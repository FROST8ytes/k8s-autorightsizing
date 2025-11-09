# Kubernetes Auto-Rightsizing Project

![Banner](./assets/banner.svg)

A comprehensive infrastructure-as-code solution for automated Kubernetes resource optimization using Prometheus metrics, custom resource recommendations, and GitOps workflows.

## Table of Contents

- [Kubernetes Auto-Rightsizing Project](#kubernetes-auto-rightsizing-project)
  - [Table of Contents](#table-of-contents)
  - [Overview](#overview)
    - [Key Features](#key-features)
    - [Component Flow](#component-flow)
  - [Project Structure](#project-structure)
  - [Prerequisites](#prerequisites)
    - [Required Tools](#required-tools)
    - [AWS Requirements](#aws-requirements)
    - [GitHub Requirements](#github-requirements)
  - [Quick Start](#quick-start)
    - [1. Clone Repository](#1-clone-repository)
    - [2. Deploy Infrastructure](#2-deploy-infrastructure)
    - [3. Configure kubectl](#3-configure-kubectl)
    - [4. Deploy Monitoring Stack](#4-deploy-monitoring-stack)
    - [5. Verify Deployment](#5-verify-deployment)
    - [6. Build Recommender](#6-build-recommender)
    - [7. Run Recommender](#7-run-recommender)
  - [Components](#components)
    - [Terraform Infrastructure](#terraform-infrastructure)
      - [VPC and Networking](#vpc-and-networking)
      - [EKS Cluster](#eks-cluster)
      - [Amazon Managed Prometheus](#amazon-managed-prometheus)
      - [IAM Configuration](#iam-configuration)
    - [Kubernetes Monitoring Stack](#kubernetes-monitoring-stack)
      - [Prometheus Operator](#prometheus-operator)
      - [Grafana](#grafana)
      - [Exporters](#exporters)
    - [Resource Recommender](#resource-recommender)
      - [Features](#features)
      - [Command-Line Options](#command-line-options)
      - [Usage Examples](#usage-examples)
      - [Recommendation Algorithm](#recommendation-algorithm)
    - [Sample Applications](#sample-applications)
      - [1. CPU-Intensive](#1-cpu-intensive)
      - [2. Memory-Intensive](#2-memory-intensive)
      - [3. Variable Load](#3-variable-load)
      - [4. Idle App](#4-idle-app)
    - [GitHub Actions Workflow](#github-actions-workflow)
      - [Workflow File](#workflow-file)
      - [Trigger Conditions](#trigger-conditions)
      - [Workflow Steps](#workflow-steps)
      - [Required GitHub Secrets](#required-github-secrets)
  - [Usage](#usage)
    - [Complete Workflow Example](#complete-workflow-example)
      - [Step 1: Let Workloads Run](#step-1-let-workloads-run)
      - [Step 2: Run Recommender Analysis](#step-2-run-recommender-analysis)
      - [Step 3: Review Recommendations](#step-3-review-recommendations)
      - [Step 4: Create Pull Request](#step-4-create-pull-request)
      - [Step 5: Review and Merge PR](#step-5-review-and-merge-pr)
      - [Step 6: GitHub Actions Applies Changes](#step-6-github-actions-applies-changes)
      - [Step 7: Verify Changes](#step-7-verify-changes)
    - [Monitoring Resource Usage](#monitoring-resource-usage)
      - [Via kubectl](#via-kubectl)
      - [Via Grafana](#via-grafana)
  - [Configuration](#configuration)
    - [Terraform Variables](#terraform-variables)
    - [Prometheus Configuration](#prometheus-configuration)
    - [Recommender Configuration](#recommender-configuration)
    - [Sample App Configuration](#sample-app-configuration)
  - [Monitoring and Observability](#monitoring-and-observability)
    - [Metrics Available](#metrics-available)
      - [Container Metrics (cAdvisor)](#container-metrics-cadvisor)
      - [Kubernetes State Metrics](#kubernetes-state-metrics)
      - [Node Metrics](#node-metrics)
    - [Prometheus Queries](#prometheus-queries)
    - [Grafana Dashboards](#grafana-dashboards)
  - [GitOps Workflow](#gitops-workflow)
    - [Workflow Overview](#workflow-overview)
    - [Manual Apply (Alternative)](#manual-apply-alternative)
  - [Troubleshooting](#troubleshooting)
    - [Common Issues](#common-issues)
      - [Issue: Recommender cannot connect to Prometheus](#issue-recommender-cannot-connect-to-prometheus)
      - [Issue: GitHub Actions workflow fails](#issue-github-actions-workflow-fails)
      - [Issue: Pods stuck in Pending state](#issue-pods-stuck-in-pending-state)
      - [Issue: Grafana shows no data](#issue-grafana-shows-no-data)
      - [Issue: Recommender TUI logs interfere with display](#issue-recommender-tui-logs-interfere-with-display)
    - [Debug Commands](#debug-commands)
  - [Development](#development)
    - [Building Recommender from Source](#building-recommender-from-source)
    - [Code Structure](#code-structure)

## Overview

This project provides an end-to-end solution for automatically optimizing Kubernetes resource requests and limits based on actual usage patterns. It combines:

- **Infrastructure as Code**: Terraform for AWS EKS cluster provisioning
- **Monitoring Stack**: Prometheus for metrics collection and Grafana for visualization
- **Custom Recommender**: Rust-based CLI tool for analyzing metrics and generating recommendations
- **GitOps Automation**: GitHub Actions for automated deployment of resource updates

### Key Features

- Automated EKS cluster deployment on AWS
- Complete monitoring stack with Prometheus Operator, Grafana, and exporters
- Intelligent resource recommendations based on historical metrics (CPU, memory)
- Interactive TUI and CLI modes for reviewing recommendations
- Automated PR creation for resource updates
- GitHub Actions workflow for applying changes to cluster
- Support for both immediate and scheduled lookback periods (supports fractional hours)

### Component Flow

1. **Metrics Collection**: Prometheus scrapes metrics from Kubernetes cluster components
2. **Analysis**: Recommender tool queries Prometheus for historical resource usage
3. **Recommendation Generation**: Algorithm calculates optimal resource requests/limits
4. **Review Process**: User reviews recommendations via TUI or JSON output
5. **PR Creation**: Approved changes create GitHub Pull Request with updated manifests
6. **Automated Deployment**: GitHub Actions workflow applies changes when PR is merged
7. **Verification**: Deployments roll out with new resource configurations

## Project Structure

```txt
.
├── .github/
│   └── workflows/
│       ├── apply-sample-apps.yml    # Auto-apply workflow
│       ├── README.md                # Workflow documentation
│       └── show-secrets.sh          # Helper script
├── k8s/
│   ├── cadvisor/                    # Container metrics exporter
│   ├── grafana/                     # Visualization platform
│   ├── kube-state-metrics/          # Kubernetes metrics exporter
│   ├── node-exporter/               # Node metrics exporter
│   ├── prometheus-agent/            # Prometheus in agent mode
│   ├── prometheus-operator/         # Prometheus operator
│   └── prometheus-operator-crd/     # Custom resource definitions
├── recommender/
│   ├── src/
│   │   ├── lib/
│   │   │   ├── cli.rs              # Command-line interface
│   │   │   ├── config.rs           # Configuration structures
│   │   │   ├── github.rs           # GitHub API integration
│   │   │   ├── output.rs           # Output formatting
│   │   │   ├── recommender.rs      # Core recommendation logic
│   │   │   ├── tui.rs              # Terminal user interface
│   │   │   └── updater.rs          # Manifest updater
│   │   └── main.rs                 # Application entry point
│   ├── Cargo.toml                  # Rust dependencies
│   └── README.md                   # Recommender documentation
├── sample-apps/
│   ├── 1-cpu-intensive/            # CPU test workload
│   ├── 2-memory-intensive/         # Memory test workload
│   ├── 3-variable-load/            # Variable load workload
│   └── 4-idle-app/                 # Idle workload
├── terraform/
│   ├── provider.tf                 # AWS provider configuration
│   ├── vpc.tf                      # VPC networking
│   ├── eks.tf                      # EKS cluster
│   ├── nodes.tf                    # Worker nodes
│   ├── iam-oidc.tf                 # OIDC provider
│   ├── iam-prometheus.tf           # Prometheus IAM roles
│   ├── iam-grafana.tf              # Grafana IAM roles
│   ├── prometheus.tf               # Amazon Managed Prometheus
│   └── ec2.tf                      # EC2 bastion (optional)
├── deploy.sh                       # Deployment script
├── destroy.sh                      # Cleanup script
└── validate.sh                     # Validation script
```

## Prerequisites

### Required Tools

- **Terraform**: >= 1.0
- **kubectl**: >= 1.24
- **AWS CLI**: >= 2.0
- **Rust**: >= 1.70 (for building recommender)
- **Git**: >= 2.0

### AWS Requirements

- AWS Account with appropriate permissions
- IAM user or role with permissions for:
  - EKS cluster management
  - VPC and networking
  - IAM role creation
  - Amazon Managed Prometheus
- AWS credentials configured (`~/.aws/credentials`)

### GitHub Requirements

- GitHub account
- Personal access token with `repo` permissions
- Repository access for storing manifests

## Quick Start

### 1. Clone Repository

```bash
git clone https://github.com/FROST8ytes/k8s-autorightsizing.git
cd k8s-autorightsizing
```

### 2. Deploy Infrastructure

```bash
cd terraform
terraform init
terraform plan
terraform apply
```

### 3. Configure kubectl

```bash
aws eks update-kubeconfig --name fyp-autorightsizing --region ap-southeast-1
```

### 4. Deploy Monitoring Stack

```bash
cd ../
./deploy.sh
```

### 5. Verify Deployment

```bash
./validate.sh
```

### 6. Build Recommender

```bash
cd recommender
cargo build --release
```

### 7. Run Recommender

```bash
./target/release/recommender \
  --amp-url "https://aps-workspaces.ap-southeast-1.amazonaws.com/workspaces/ws-xxxxx/" \
  --region ap-southeast-1 \
  --namespace default \
  --output table
```

![Recommended rightsizing values in a table](./assets/recommended_table.png)

## Components

### Terraform Infrastructure

#### VPC and Networking

- **VPC CIDR**: 10.0.0.0/16
- **Subnets**: Public and private subnets across 2 availability zones
- **NAT Gateway**: For private subnet internet access
- **Internet Gateway**: For public subnet access

#### EKS Cluster

- **Kubernetes Version**: 1.28
- **Node Group**: Managed node group with auto-scaling
- **Instance Type**: t3.medium (configurable)
- **Capacity**: 2-4 nodes

#### Amazon Managed Prometheus

- **Workspace**: Dedicated AMP workspace for metrics storage
- **Remote Write**: Configured for Prometheus agent
- **Retention**: Managed by AWS

#### IAM Configuration

- **OIDC Provider**: For service account authentication
- **Prometheus Role**: IAM role for remote write permissions
- **Grafana Role**: IAM role for querying AMP

### Kubernetes Monitoring Stack

#### Prometheus Operator

- **Version**: Latest stable
- **Mode**: Agent mode with remote write to AMP
- **Scrape Interval**: 30 seconds
- **Components**:
  - Prometheus Operator
  - Prometheus Agent
  - Service Monitors
  - Custom Resource Definitions

#### Grafana

- **Version**: Latest
- **Datasource**: Amazon Managed Prometheus
- **Authentication**: IAM role-based
- **Dashboards**:
  - cAdvisor metrics
  - Kube-state-metrics
  - Node metrics

**Access Grafana**:

```bash
kubectl port-forward -n monitoring svc/grafana 3000:3000
```

Then open <http://localhost:3000>

![Example Grafana dashboard](./assets/grafana_dashboard.png)

#### Exporters

**cAdvisor**:

- Container-level metrics
- CPU, memory, network, disk usage
- Deployed as DaemonSet

**Kube-State-Metrics**:

- Kubernetes object state metrics
- Deployment, pod, node status
- Resource requests and limits

**Node Exporter**:

- Host-level metrics
- CPU, memory, disk, network
- Deployed as DaemonSet

### Resource Recommender

A Rust-based CLI tool that analyzes Prometheus metrics and generates resource recommendations.

#### Features

- **Multiple Output Formats**: Table, JSON, TUI
- **Flexible Time Windows**: Support for fractional hours (e.g., 0.5 hours = 30 minutes)
- **Interactive TUI**: Review and apply recommendations with visual interface
- **GitHub Integration**: Automatic PR creation with updated manifests
- **Multi-threaded**: Progress tracking with non-blocking operations
- **Git Authentication**: Support for personal access tokens and fine-grained tokens

#### Command-Line Options

```bash
recommender [OPTIONS]

Options:
  --amp-url <URL>              Amazon Managed Prometheus workspace URL
  --region <REGION>            AWS region (default: ap-southeast-1)
  --namespace <NAMESPACE>      Kubernetes namespace (default: default)
  --lookback-hours <HOURS>     Hours to look back (supports decimals, default: 168.0)
  --output <FORMAT>            Output format: table|json|tui (default: table)
  --verbose                    Enable verbose logging
  --quiet                      Suppress logs (useful with TUI)
  --apply                      Enable apply mode (create PR)
  --manifest-url <URL>         GitHub repository URL for manifests
  --git-token <TOKEN>          GitHub personal access token
  --git-branch <BRANCH>        Target branch (default: main)
  --git-username <USERNAME>    Git username for commits
```

#### Usage Examples

**Basic recommendation (table output)**:

```bash
./recommender --amp-url "https://aps-workspaces..." --output table
```

**Interactive TUI mode**:

```bash
./recommender --amp-url "https://aps-workspaces..." --output tui --quiet
```

**JSON output for automation**:

```bash
./recommender --amp-url "https://aps-workspaces..." --output json > recommendations.json
```

**Auto-apply with PR creation**:

```bash
./recommender \
  --amp-url "https://aps-workspaces..." \
  --output tui \
  --apply \
  --manifest-url https://github.com/FROST8ytes/k8s-autorightsizing.git \
  --git-token ghp_xxxxxxxxxxxx \
  --git-branch recommender-development \
  --quiet
```

**Short lookback period (30 minutes)**:

```bash
./recommender --amp-url "https://aps-workspaces..." --lookback-hours 0.5
```

![Auto-applying recommended values to manifests](./assets/auto_apply.png)
![PR created for updated manifests](./assets/pr_creation.png)

#### Recommendation Algorithm

1. **Metrics Collection**: Query Prometheus for CPU and memory usage
2. **Statistical Analysis**: Calculate P95 (95th percentile) for resource usage
3. **Buffer Application**: Apply 20% buffer for safety margin
4. **Limit Calculation**: Set limits at 2x requests
5. **Validation**: Ensure recommendations don't exceed current limits
6. **Output**: Generate recommendations in chosen format

### Sample Applications

Four test workloads demonstrating different resource patterns:

#### 1. CPU-Intensive

- **Purpose**: Stress test CPU allocation
- **Pattern**: Constant high CPU usage
- **Initial Resources**:
  - Requests: 500m CPU, 256Mi memory
  - Limits: 1000m CPU, 512Mi memory

#### 2. Memory-Intensive

- **Purpose**: Stress test memory allocation
- **Pattern**: Constant high memory usage
- **Initial Resources**:
  - Requests: 256m CPU, 512Mi memory
  - Limits: 512m CPU, 1Gi memory

#### 3. Variable Load

- **Purpose**: Simulate real-world variable workload
- **Pattern**: Fluctuating CPU and memory usage
- **Initial Resources**:
  - Requests: 200m CPU, 128Mi memory
  - Limits: 500m CPU, 256Mi memory

#### 4. Idle App

- **Purpose**: Test over-provisioning detection
- **Pattern**: Minimal resource usage
- **Initial Resources**:
  - Requests: 500m CPU, 512Mi memory
  - Limits: 1000m CPU, 1Gi memory

### GitHub Actions Workflow

Automated workflow for applying manifest changes to the cluster.

#### Workflow File

`.github/workflows/apply-sample-apps.yml`

#### Trigger Conditions

- Push to `recommender-development` branch
- Changes in `sample-apps/**/*.yaml` files
- Manual trigger via workflow_dispatch

#### Workflow Steps

1. **Checkout**: Clone repository
2. **Configure AWS**: Authenticate using secrets
3. **Update kubeconfig**: Connect to EKS cluster
4. **Verify Connection**: Test cluster connectivity
5. **Apply Manifests**: Run `kubectl apply -f sample-apps/`
6. **Verify Deployments**: Check deployment status
7. **Wait for Rollout**: Ensure pods are ready
8. **Final Status**: Display deployment state

#### Required GitHub Secrets

| Secret | Description | Example |
|--------|-------------|---------|
| `AWS_ACCESS_KEY_ID` | AWS access key | `AKIAIOSFODNN7EXAMPLE` |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key | `wJalrXUtn...` |
| `AWS_REGION` | AWS region | `ap-southeast-1` |
| `EKS_CLUSTER_NAME` | EKS cluster name | `fyp-autorightsizing` |

**Setting up secrets**:

1. Navigate to repository settings: `https://github.com/FROST8ytes/k8s-autorightsizing/settings/secrets/actions`
2. Click "New repository secret"
3. Add each secret with its corresponding value

![Screenshot of GitHub Actions workflow run](./assets/gh_workflow.png)

## Usage

### Complete Workflow Example

#### Step 1: Let Workloads Run

Allow sample applications to run for at least 1 hour to collect metrics:

```bash
kubectl get pods -n default -w
```

#### Step 2: Run Recommender Analysis

```bash
cd recommender
./target/release/recommender \
  --amp-url "https://aps-workspaces.ap-southeast-1.amazonaws.com/workspaces/ws-xxxxx/" \
  --region ap-southeast-1 \
  --namespace default \
  --lookback-hours 1.0 \
  --output tui \
  --quiet
```

#### Step 3: Review Recommendations

In TUI mode:

- Review each recommendation
- Check current vs recommended values
- Select recommendations to apply
- Choose apply mode (JSON or TUI)

#### Step 4: Create Pull Request

If apply mode is enabled:

- Enter GitHub username
- Enter branch name
- Tool creates PR automatically
- PR contains updated manifest files

#### Step 5: Review and Merge PR

```bash
# View PR on GitHub
open https://github.com/FROST8ytes/k8s-autorightsizing/pulls

# Or use GitHub CLI
gh pr view <PR_NUMBER>
gh pr review <PR_NUMBER> --approve
gh pr merge <PR_NUMBER>
```

#### Step 6: GitHub Actions Applies Changes

- Workflow triggers automatically on merge
- Authenticates to AWS
- Applies manifests to cluster
- Waits for rollout completion

#### Step 7: Verify Changes

```bash
kubectl get deployments -n default
kubectl describe deployment cpu-intensive -n default | grep -A 5 "Requests:"
kubectl get pods -n default
```

### Monitoring Resource Usage

#### Via kubectl

```bash
# Current resource requests/limits
kubectl describe deployment cpu-intensive -n default

# Pod resource usage (requires metrics-server)
kubectl top pods -n default

# Node resource usage
kubectl top nodes
```

#### Via Grafana

```bash
# Port-forward Grafana
kubectl port-forward -n monitoring svc/grafana 3000:3000

# Access at http://localhost:3000
# Navigate to pre-configured dashboards
```

## Configuration

### Terraform Variables

Edit `terraform/terraform.tfvars` (create if doesn't exist):

```hcl
cluster_name    = "fyp-autorightsizing"
region          = "ap-southeast-1"
vpc_cidr        = "10.0.0.0/16"
instance_type   = "t3.medium"
desired_size    = 2
min_size        = 2
max_size        = 4
```

### Prometheus Configuration

Edit `k8s/prometheus-agent/4-prometheus.yaml`:

- **Scrape Interval**: Default 30s
- **Remote Write**: AMP workspace URL
- **Additional Scrape Configs**: Add custom targets

### Recommender Configuration

Configuration is primarily via CLI arguments. For persistent settings, create a wrapper script:

```bash
#!/bin/bash
# recommend.sh

./recommender/target/release/recommender \
  --amp-url "https://aps-workspaces.ap-southeast-1.amazonaws.com/workspaces/ws-xxxxx/" \
  --region ap-southeast-1 \
  --namespace default \
  --lookback-hours 168.0 \
  --output tui \
  --quiet \
  "$@"
```

### Sample App Configuration

Each sample app can be configured by editing its YAML files:

```bash
# Edit CPU-intensive deployment
vi sample-apps/1-cpu-intensive/deployment.yaml

# Edit resource requests/limits
resources:
  requests:
    cpu: "500m"
    memory: "256Mi"
  limits:
    cpu: "1000m"
    memory: "512Mi"
```

## Monitoring and Observability

### Metrics Available

#### Container Metrics (cAdvisor)

- `container_cpu_usage_seconds_total`
- `container_memory_working_set_bytes`
- `container_network_receive_bytes_total`
- `container_network_transmit_bytes_total`

#### Kubernetes State Metrics

- `kube_deployment_status_replicas`
- `kube_pod_container_resource_requests`
- `kube_pod_container_resource_limits`
- `kube_pod_status_phase`

#### Node Metrics

- `node_cpu_seconds_total`
- `node_memory_MemAvailable_bytes`
- `node_filesystem_avail_bytes`

### Prometheus Queries

**CPU usage by container**:

```promql
rate(container_cpu_usage_seconds_total{namespace="default"}[5m])
```

**Memory usage by container**:

```promql
container_memory_working_set_bytes{namespace="default"}
```

**P95 CPU usage over 24 hours**:

```promql
quantile(0.95, rate(container_cpu_usage_seconds_total{namespace="default"}[24h]))
```

### Grafana Dashboards

Pre-configured dashboards available:

1. **cAdvisor Dashboard**: Container-level metrics
2. **Kube-State-Metrics Dashboard**: Kubernetes object state
3. **Node Dashboard**: Host-level metrics

**Custom Dashboard Creation**:

1. Access Grafana: <http://localhost:3000>
2. Navigate to Dashboards > New Dashboard
3. Add panel with Prometheus datasource
4. Use PromQL queries from above

## GitOps Workflow

### Workflow Overview

```txt
Developer/Recommender
        |
        | (1) Create PR with manifest updates
        v
    GitHub Repository
        |
        | (2) Review & Merge
        v
    GitHub Actions
        |
        | (3) Authenticate to AWS
        v
    kubectl apply
        |
        | (4) Update resources
        v
    EKS Cluster
```

### Manual Apply (Alternative)

If not using GitHub Actions:

```bash
# Apply changes locally
kubectl apply -f sample-apps/1-cpu-intensive/

# Or apply all sample apps
kubectl apply -f sample-apps/ --recursive

# Verify rollout
kubectl rollout status deployment/cpu-intensive -n default
```

## Troubleshooting

### Common Issues

#### Issue: Recommender cannot connect to Prometheus

**Symptoms**: Connection timeout or authentication error

**Solutions**:

- Verify AMP workspace URL is correct
- Check AWS credentials have AMP query permissions
- Ensure region matches AMP workspace region
- Test connection: `aws amp describe-workspace --workspace-id ws-xxxxx --region ap-southeast-1`

#### Issue: GitHub Actions workflow fails

**Symptoms**: Workflow run shows authentication error

**Solutions**:

- Verify all GitHub secrets are set correctly
- Check AWS credentials have EKS permissions
- Ensure EKS cluster name matches secret value
- Test locally: `aws eks update-kubeconfig --name fyp-autorightsizing --region ap-southeast-1`

#### Issue: Pods stuck in Pending state

**Symptoms**: Pods don't schedule after resource update

**Solutions**:

- Check node resources: `kubectl describe nodes`
- Verify resource requests are not too high
- Scale nodes if needed: Edit `terraform/nodes.tf` and apply
- Check events: `kubectl get events -n default --sort-by='.lastTimestamp'`

#### Issue: Grafana shows no data

**Symptoms**: Empty graphs in Grafana dashboards

**Solutions**:

- Verify Prometheus is scraping metrics: `kubectl logs -n monitoring prometheus-agent-0`
- Check remote write to AMP: `kubectl logs -n monitoring prometheus-agent-0 | grep "remote_write"`
- Verify Grafana datasource configuration: Settings > Data Sources
- Check IAM role permissions for Grafana

#### Issue: Recommender TUI logs interfere with display

**Symptoms**: Log messages overlay TUI interface

**Solutions**:

- Use `--quiet` flag to suppress logs
- Redirect logs to file: `./recommender --output tui 2> recommender.log`

### Debug Commands

```bash
# Check all deployments
kubectl get deployments --all-namespaces

# Check pod status and events
kubectl describe pod <pod-name> -n default

# Check Prometheus agent logs
kubectl logs -n monitoring prometheus-agent-0 -f

# Check Grafana logs
kubectl logs -n monitoring deployment/grafana -f

# Check GitHub Actions workflow locally
act -j apply-changes  # Requires 'act' tool

# Verify AWS credentials
aws sts get-caller-identity

# Test EKS connectivity
kubectl cluster-info
kubectl get nodes
```

## Development

### Building Recommender from Source

```bash
cd recommender

# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- --amp-url "..." --output table
```

### Code Structure

**Recommender modules**:

- `cli.rs`: Command-line argument parsing (clap)
- `config.rs`: Configuration structures
- `github.rs`: GitHub API integration (octocrab)
- `recommender.rs`: Core recommendation logic and Prometheus queries
- `output.rs`: Output formatting (table, JSON)
- `tui.rs`: Terminal user interface (ratatui)
- `updater.rs`: YAML manifest parsing and updating
