use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::json;
use serde_yaml::Value;
use tempfile::TempDir;

use crate::lib::config::{GitConnectionType, GitProvider, UpdaterConfig};
use crate::lib::error::{RecommenderError, Result};
use crate::lib::recommender::ResourceRecommendation;

pub struct ManifestUpdater {
    config: UpdaterConfig,
    temp_dir: TempDir,
    repo: Option<Repository>,
}

impl ManifestUpdater {
    /// Create a new ManifestUpdater
    pub fn new(config: UpdaterConfig) -> Result<Self> {
        let temp_dir = TempDir::new()?;
        info!("Created temporary directory: {}", temp_dir.path().display());

        Ok(Self {
            config,
            temp_dir,
            repo: None,
        })
    }

    /// Clone the repository
    pub fn clone_repo(&mut self, branch: &str) -> Result<()> {
        info!("Cloning base branch: {}", branch);
        info!("Cloning repository: {}", self.config.git_url);

        let mut callbacks = RemoteCallbacks::new();

        // Setup credentials based on connection type
        match &self.config.connection_type {
            GitConnectionType::Ssh => {
                callbacks.credentials(|_url, username_from_url, _allowed_types| {
                    if let Some(username) = username_from_url {
                        if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                            return Ok(cred);
                        }
                    }
                    Cred::default()
                });
            }
            GitConnectionType::Https => {
                let token = self.config.auth_token.clone();
                let username = self.config.auth_username.clone();

                callbacks.credentials(move |url_str, username_from_url, allowed_types| {
                    // Log for debugging (without exposing token)
                    info!("Git credential callback invoked for URL: {}", url_str);
                    info!("Username from URL: {:?}", username_from_url);
                    info!("Configured username: {:?}", username);
                    info!("Allowed credential types: {:?}", allowed_types);

                    if let Some(ref token) = token {
                        // Priority: 1) CLI provided username, 2) URL username, 3) default to "git"
                        let user = username
                            .as_ref()
                            .map(|s| s.as_str())
                            .or(username_from_url)
                            .unwrap_or("git");
                        info!("Attempting userpass authentication with username: {}", user);
                        return Cred::userpass_plaintext(user, token);
                    }

                    info!("Falling back to default credentials");
                    Cred::default()
                });
            }
        }

        // Add certificate check callback for debugging
        callbacks.certificate_check(|_cert, _host| {
            info!("Certificate check passed");
            Ok(git2::CertificateCheckStatus::CertificateOk)
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);
        builder.branch(branch);

        let repo = builder.clone(self.config.git_url.as_str(), self.temp_dir.path())?;
        info!("Repository cloned successfully");

        self.repo = Some(repo);
        Ok(())
    }

    /// Find all deployment YAML files in the repository
    pub fn find_deployment_files(&self) -> Result<Vec<PathBuf>> {
        let repo_path = self.temp_dir.path();
        let mut deployment_files = Vec::new();

        self.find_yaml_files_recursive(repo_path, &mut deployment_files)?;

        info!("Found {} YAML files to scan", deployment_files.len());
        Ok(deployment_files)
    }

    /// Recursively find YAML files
    fn find_yaml_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip .git directory
            if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }

            if path.is_dir() {
                self.find_yaml_files_recursive(&path, files)?;
            } else if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    files.push(path);
                }
            }
        }

        Ok(())
    }

    /// Apply all recommendations
    pub fn apply_recommendations(
        &self,
        recommendations: &[ResourceRecommendation],
    ) -> Result<HashMap<String, usize>> {
        let deployment_files = self.find_deployment_files()?;
        let mut updates = HashMap::new();

        for recommendation in recommendations {
            let updated = self.find_and_update_deployment(&deployment_files, recommendation)?;

            if updated > 0 {
                let key = format!("{}/{}", recommendation.namespace, recommendation.deployment);
                updates.insert(key, updated);
            }
        }

        Ok(updates)
    }

    /// Find and update deployment in YAML files
    fn find_and_update_deployment(
        &self,
        files: &[PathBuf],
        recommendation: &ResourceRecommendation,
    ) -> Result<usize> {
        let mut updates = 0;

        for file in files {
            let content = fs::read_to_string(file)?;

            // Parse YAML (handle multiple documents)
            let docs_result: Result<Vec<Value>> = serde_yaml::Deserializer::from_str(&content)
                .map(|doc| serde_yaml::Value::deserialize(doc).map_err(|e| e.into()))
                .collect();

            let mut docs = docs_result?;

            let mut modified = false;

            for doc in &mut docs {
                if self.is_matching_deployment(doc, recommendation) {
                    debug!("Found matching deployment in: {}", file.display());
                    if self.update_container_resources(doc, recommendation)? {
                        modified = true;
                        updates += 1;
                    }
                }
            }

            if modified {
                // Write back to file
                let mut output = String::new();
                for (i, doc) in docs.iter().enumerate() {
                    if i > 0 {
                        output.push_str("\n---\n");
                    }
                    output.push_str(&serde_yaml::to_string(doc)?);
                }

                fs::write(file, output)?;
                info!("Updated file: {}", file.display());
            }
        }

        Ok(updates)
    }

    /// Check if YAML document matches the deployment we're looking for
    fn is_matching_deployment(&self, doc: &Value, recommendation: &ResourceRecommendation) -> bool {
        // Check kind
        if let Some(kind) = doc.get("kind").and_then(|v| v.as_str()) {
            if kind != "Deployment" {
                return false;
            }
        } else {
            return false;
        }

        // Check name
        if let Some(name) = doc
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
        {
            if name != recommendation.deployment {
                return false;
            }
        } else {
            return false;
        }

        // Check namespace (if specified)
        if let Some(namespace) = doc
            .get("metadata")
            .and_then(|m| m.get("namespace"))
            .and_then(|n| n.as_str())
        {
            if namespace != recommendation.namespace {
                return false;
            }
        }

        true
    }

    /// Update container resources in deployment YAML
    fn update_container_resources(
        &self,
        doc: &mut Value,
        recommendation: &ResourceRecommendation,
    ) -> Result<bool> {
        let mut updated = false;

        // Navigate to spec.template.spec.containers
        if let Some(containers) = doc
            .get_mut("spec")
            .and_then(|s| s.get_mut("template"))
            .and_then(|t| t.get_mut("spec"))
            .and_then(|s| s.get_mut("containers"))
            .and_then(|c| c.as_sequence_mut())
        {
            for container in containers {
                // Check if this is the container we're looking for
                // Clone the name first to avoid borrow checker issues
                let container_name = container
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());

                if let Some(name) = container_name {
                    if name == recommendation.container {
                        // Update resources
                        if container.get("resources").is_none() {
                            container.as_mapping_mut().unwrap().insert(
                                Value::String("resources".to_string()),
                                Value::Mapping(Default::default()),
                            );
                        }

                        let resources = container
                            .get_mut("resources")
                            .unwrap()
                            .as_mapping_mut()
                            .unwrap();

                        // Update requests
                        if !resources.contains_key(&Value::String("requests".to_string())) {
                            resources.insert(
                                Value::String("requests".to_string()),
                                Value::Mapping(Default::default()),
                            );
                        }

                        let requests = resources
                            .get_mut(&Value::String("requests".to_string()))
                            .unwrap()
                            .as_mapping_mut()
                            .unwrap();

                        requests.insert(
                            Value::String("cpu".to_string()),
                            Value::String(recommendation.recommended_cpu_request.clone()),
                        );
                        requests.insert(
                            Value::String("memory".to_string()),
                            Value::String(recommendation.recommended_memory_request.clone()),
                        );

                        // Update limits
                        if !resources.contains_key(&Value::String("limits".to_string())) {
                            resources.insert(
                                Value::String("limits".to_string()),
                                Value::Mapping(Default::default()),
                            );
                        }

                        let limits = resources
                            .get_mut(&Value::String("limits".to_string()))
                            .unwrap()
                            .as_mapping_mut()
                            .unwrap();

                        limits.insert(
                            Value::String("cpu".to_string()),
                            Value::String(recommendation.recommended_cpu_limit.clone()),
                        );
                        limits.insert(
                            Value::String("memory".to_string()),
                            Value::String(recommendation.recommended_memory_limit.clone()),
                        );

                        updated = true;
                        debug!("Updated resources for container: {}", name);
                    }
                }
            }
        }

        Ok(updated)
    }

    /// Commit changes
    pub fn commit_changes(&self, message: &str) -> Result<git2::Oid> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| RecommenderError::ApplyError("Repository not cloned".to_string()))?;

        // Add all changes
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Create commit
        let signature = repo.signature()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let parent_commit = repo.head()?.peel_to_commit()?;

        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        info!("Created commit: {}", oid);
        Ok(oid)
    }

    /// Push changes to remote
    pub fn push_changes(&self, branch: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| RecommenderError::ApplyError("Repository not cloned".to_string()))?;

        info!("Pushing changes to remote...");

        let mut remote = repo.find_remote("origin")?;

        let mut callbacks = RemoteCallbacks::new();

        // Setup credentials based on connection type
        match &self.config.connection_type {
            GitConnectionType::Ssh => {
                callbacks.credentials(|_url, username_from_url, _allowed_types| {
                    if let Some(username) = username_from_url {
                        if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                            return Ok(cred);
                        }
                    }
                    Cred::default()
                });
            }
            GitConnectionType::Https => {
                let token = self.config.auth_token.clone();
                callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                    if let Some(ref token) = token {
                        let username = username_from_url.unwrap_or("git");
                        return Cred::userpass_plaintext(username, token);
                    }
                    Cred::default()
                });
            }
        }

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
        remote.push(&[&refspec], Some(&mut push_options))?;

        info!("Changes pushed successfully");
        Ok(())
    }

    /// Create a new branch for changes
    pub fn create_branch(&mut self, branch_name: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| RecommenderError::ApplyError("Repository not cloned".to_string()))?;

        let head = repo.head()?;
        let commit = head.peel_to_commit()?;

        repo.branch(branch_name, &commit, false)?;
        repo.set_head(&format!("refs/heads/{}", branch_name))?;

        info!("Created and switched to branch: {}", branch_name);

        Ok(())
    }

    /// Get the commit SHA
    pub fn get_commit_sha(&self) -> Result<String> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| RecommenderError::ApplyError("Repository not cloned".to_string()))?;

        let head = repo.head()?;
        let oid = head
            .target()
            .ok_or_else(|| RecommenderError::ApplyError("Could not get HEAD target".to_string()))?;

        Ok(oid.to_string())
    }

    /// Get the repository path
    pub fn repo_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Complete workflow: clone, create branch, apply, commit, push, and create PR
    /// Returns (branch_name, commit_sha, pr_url)
    pub async fn apply_and_create_pr(
        &mut self,
        base_branch: &str,
        recommendations: &[ResourceRecommendation],
    ) -> Result<(String, String, Option<String>)> {
        // 1. Clone the base branch
        info!("Cloning base branch: {}", base_branch);
        self.clone_repo(base_branch)?;

        // 2. Create new branch with timestamp
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let new_branch = format!("autorightsizing-{}", timestamp);
        info!("Creating new branch: {}", new_branch);
        self.create_branch(&new_branch)?;

        // 3. Apply recommendations
        info!("Applying recommendations...");
        let updates = self.apply_recommendations(recommendations)?;

        if updates.is_empty() {
            return Err(RecommenderError::ApplyError(
                "No matching deployments found in repository".to_string(),
            ));
        }

        info!("Updated {} deployments", updates.len());

        // 4. Commit changes
        let commit_message = self.generate_commit_message(&updates);
        info!("Committing changes...");
        self.commit_changes(&commit_message)?;

        let commit_sha = self.get_commit_sha()?;
        info!("Commit SHA: {}", commit_sha);

        // 5. Push to remote
        info!("Pushing branch to remote...");
        self.push_changes(&new_branch)?;

        // 6. Create Pull Request
        info!("Creating pull request...");
        let pr_url = match self
            .create_pull_request(&new_branch, base_branch, &updates)
            .await
        {
            Ok(url) => {
                info!("Pull request created: {}", url);
                Some(url)
            }
            Err(e) => {
                warn!("Failed to create pull request automatically: {}", e);
                warn!(
                    "Please create PR manually from {} to {}",
                    new_branch, base_branch
                );
                None
            }
        };

        Ok((new_branch, commit_sha, pr_url))
    }

    /// Generate a detailed commit message
    fn generate_commit_message(&self, updates: &HashMap<String, usize>) -> String {
        let mut message = String::from("chore: apply resource recommendations\n\n");
        message.push_str(&format!(
            "Updated resource requests and limits for {} deployment(s):\n",
            updates.len()
        ));

        for deployment in updates.keys() {
            message.push_str(&format!("  - {}\n", deployment));
        }

        message.push_str("\nGenerated by Kubernetes Resource Recommender");
        message
    }

    /// Create a Pull Request (supports multiple Git providers)
    async fn create_pull_request(
        &self,
        head_branch: &str,
        base_branch: &str,
        updates: &HashMap<String, usize>,
    ) -> Result<String> {
        match &self.config.provider {
            GitProvider::GitHub => self.create_github_pr(head_branch, base_branch, updates).await,
            GitProvider::GitLab => self.create_gitlab_mr(head_branch, base_branch, updates).await,
            GitProvider::Bitbucket => {
                self.create_bitbucket_pr(head_branch, base_branch, updates)
                    .await
            }
            GitProvider::Gitea => self.create_gitea_pr(head_branch, base_branch, updates).await,
            GitProvider::Generic => Err(RecommenderError::ApplyError(
                "Automatic PR creation not supported for this Git provider. Please create PR manually.".to_string(),
            )),
        }
    }

    /// Prepare PR/MR description (common across providers)
    fn prepare_pr_description(&self, updates: &HashMap<String, usize>) -> String {
        format!(
            "## Automated Resource Recommendations\n\n\
             This PR applies resource recommendations generated by the Kubernetes Resource Recommender.\n\n\
             ### Changes\n\n\
             Updated {} deployment(s):\n{}\n\n\
             ### Review Guidelines\n\n\
             - Review the resource changes for each deployment\n\
             - Ensure the new values are appropriate for your workload\n\
             - Test in a non-production environment first\n\n\
             ---\n\
             *Generated automatically by Kubernetes Resource Recommender*",
            updates.len(),
            updates
                .keys()
                .map(|k| format!("- `{}`", k))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Create a GitHub Pull Request
    async fn create_github_pr(
        &self,
        head_branch: &str,
        base_branch: &str,
        updates: &HashMap<String, usize>,
    ) -> Result<String> {
        let (owner, repo) = self.parse_repo_owner_name()?;
        let token = self.get_auth_token()?;
        let api_base = self
            .config
            .provider
            .api_base_url(&self.config.git_url)
            .ok_or_else(|| {
                RecommenderError::ApplyError("Could not determine API base URL".to_string())
            })?;

        let client = reqwest::Client::new();
        let api_url = format!("{}/repos/{}/{}/pulls", api_base, owner, repo);

        let pr_request = json!({
            "title": format!("chore: apply resource recommendations ({})", Utc::now().format("%Y-%m-%d")),
            "head": head_branch,
            "base": base_branch,
            "body": self.prepare_pr_description(updates),
        });

        let response = client
            .post(&api_url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "kubernetes-recommender")
            .header("Accept", "application/vnd.github.v3+json")
            .json(&pr_request)
            .send()
            .await
            .map_err(|e| {
                RecommenderError::ApplyError(format!("Failed to send PR request: {}", e))
            })?;

        self.handle_api_response(response, "html_url").await
    }

    /// Create a GitLab Merge Request
    async fn create_gitlab_mr(
        &self,
        head_branch: &str,
        base_branch: &str,
        updates: &HashMap<String, usize>,
    ) -> Result<String> {
        let (owner, repo) = self.parse_repo_owner_name()?;
        let token = self.get_auth_token()?;
        let api_base = self
            .config
            .provider
            .api_base_url(&self.config.git_url)
            .ok_or_else(|| {
                RecommenderError::ApplyError("Could not determine API base URL".to_string())
            })?;

        // GitLab uses URL-encoded project path (owner/repo -> owner%2Frepo)
        let project_path = format!("{}/{}", owner, repo);
        let encoded_project = urlencoding::encode(&project_path);

        let client = reqwest::Client::new();
        let api_url = format!("{}/projects/{}/merge_requests", api_base, encoded_project);

        let mr_request = json!({
            "source_branch": head_branch,
            "target_branch": base_branch,
            "title": format!("chore: apply resource recommendations ({})", Utc::now().format("%Y-%m-%d")),
            "description": self.prepare_pr_description(updates),
        });

        let response = client
            .post(&api_url)
            .header("PRIVATE-TOKEN", token)
            .header("User-Agent", "kubernetes-recommender")
            .json(&mr_request)
            .send()
            .await
            .map_err(|e| {
                RecommenderError::ApplyError(format!("Failed to send MR request: {}", e))
            })?;

        self.handle_api_response(response, "web_url").await
    }

    /// Create a Bitbucket Pull Request
    async fn create_bitbucket_pr(
        &self,
        head_branch: &str,
        base_branch: &str,
        updates: &HashMap<String, usize>,
    ) -> Result<String> {
        let (owner, repo) = self.parse_repo_owner_name()?;
        let token = self.get_auth_token()?;

        let client = reqwest::Client::new();
        let api_url = format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests",
            owner, repo
        );

        let pr_request = json!({
            "title": format!("chore: apply resource recommendations ({})", Utc::now().format("%Y-%m-%d")),
            "source": {
                "branch": {
                    "name": head_branch
                }
            },
            "destination": {
                "branch": {
                    "name": base_branch
                }
            },
            "description": self.prepare_pr_description(updates),
        });

        let response = client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "kubernetes-recommender")
            .json(&pr_request)
            .send()
            .await
            .map_err(|e| {
                RecommenderError::ApplyError(format!("Failed to send PR request: {}", e))
            })?;

        // Bitbucket uses nested structure: links.html.href
        let pr_response: serde_json::Value = response.json().await.map_err(|e| {
            RecommenderError::ApplyError(format!("Failed to parse PR response: {}", e))
        })?;

        let pr_url = pr_response["links"]["html"]["href"]
            .as_str()
            .ok_or_else(|| RecommenderError::ApplyError("No PR URL in response".to_string()))?
            .to_string();

        Ok(pr_url)
    }

    /// Create a Gitea Pull Request
    async fn create_gitea_pr(
        &self,
        head_branch: &str,
        base_branch: &str,
        updates: &HashMap<String, usize>,
    ) -> Result<String> {
        let (owner, repo) = self.parse_repo_owner_name()?;
        let token = self.get_auth_token()?;
        let api_base = self
            .config
            .provider
            .api_base_url(&self.config.git_url)
            .ok_or_else(|| {
                RecommenderError::ApplyError("Could not determine API base URL".to_string())
            })?;

        let client = reqwest::Client::new();
        let api_url = format!("{}/repos/{}/{}/pulls", api_base, owner, repo);

        let pr_request = json!({
            "title": format!("chore: apply resource recommendations ({})", Utc::now().format("%Y-%m-%d")),
            "head": head_branch,
            "base": base_branch,
            "body": self.prepare_pr_description(updates),
        });

        let response = client
            .post(&api_url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "kubernetes-recommender")
            .json(&pr_request)
            .send()
            .await
            .map_err(|e| {
                RecommenderError::ApplyError(format!("Failed to send PR request: {}", e))
            })?;

        self.handle_api_response(response, "html_url").await
    }

    /// Handle API response and extract URL
    async fn handle_api_response(
        &self,
        response: reqwest::Response,
        url_field: &str,
    ) -> Result<String> {
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RecommenderError::ApplyError(format!(
                "API error ({}): {}",
                status, error_text
            )));
        }

        let pr_response: serde_json::Value = response.json().await.map_err(|e| {
            RecommenderError::ApplyError(format!("Failed to parse API response: {}", e))
        })?;

        let pr_url = pr_response[url_field]
            .as_str()
            .ok_or_else(|| RecommenderError::ApplyError("No URL in API response".to_string()))?
            .to_string();

        Ok(pr_url)
    }

    /// Get authentication token
    fn get_auth_token(&self) -> Result<&String> {
        self.config.auth_token.as_ref().ok_or_else(|| {
            RecommenderError::ApplyError(
                "Authentication token required for creating pull requests".to_string(),
            )
        })
    }

    /// Parse repository owner and name from git URL (generic)
    fn parse_repo_owner_name(&self) -> Result<(String, String)> {
        let url_str = self.config.git_url.as_str();

        // Handle HTTPS URLs: https://provider.com/owner/repo.git
        if url_str.starts_with("https://") || url_str.starts_with("http://") {
            // Extract path after hostname
            if let Some(host_start) = url_str.find("://") {
                let after_protocol = &url_str[host_start + 3..];
                if let Some(path_start) = after_protocol.find('/') {
                    let path = &after_protocol[path_start + 1..].trim_end_matches(".git");

                    let parts: Vec<&str> = path.split('/').collect();
                    if parts.len() >= 2 {
                        return Ok((parts[0].to_string(), parts[1].to_string()));
                    }
                }
            }
        }

        // Handle SSH URLs: git@provider.com:owner/repo.git
        if url_str.contains("git@") {
            if let Some(colon_pos) = url_str.find(':') {
                let path = &url_str[colon_pos + 1..].trim_end_matches(".git");

                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() >= 2 {
                    return Ok((parts[0].to_string(), parts[1].to_string()));
                }
            }
        }

        Err(RecommenderError::ApplyError(format!(
            "Could not parse owner/repo from URL: {}",
            url_str
        )))
    }
}
