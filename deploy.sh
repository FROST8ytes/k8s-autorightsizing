#!/bin/bash
set -e  # Exit on any error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check required tools
check_dependencies() {
    log_info "Checking dependencies..."

    local deps=("terraform" "kubectl" "aws" "jq")
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            log_error "$dep is not installed. Please install it first."
            exit 1
        fi
    done

    log_success "All dependencies are installed"
}

# Step 1: Deploy infrastructure with Terraform
deploy_infrastructure() {
    log_info "Deploying infrastructure with Terraform..."

    cd terraform
    terraform init -upgrade
    terraform apply -auto-approve

    # Extract outputs
    export CLUSTER_NAME=$(terraform output -raw cluster_name)
    export REGION=$(terraform output -raw region)
    export AWS_ACCOUNT_ID=$(terraform output -raw account_id)
    export PROMETHEUS_WORKSPACE_ID=$(terraform output -raw prometheus_workspace_id)
    export PROMETHEUS_ENDPOINT=$(terraform output -raw prometheus_endpoint)
    export OIDC_PROVIDER=$(terraform output -raw oidc_provider)

    log_success "Infrastructure deployed"
    log_info "Cluster: $CLUSTER_NAME"
    log_info "Region: $REGION"
    log_info "Prometheus Workspace: $PROMETHEUS_WORKSPACE_ID"

    cd ..
}

# Step 2: Update kubeconfig
update_kubeconfig() {
    log_info "Updating kubeconfig..."

    aws eks update-kubeconfig \
        --region "$REGION" \
        --name "$CLUSTER_NAME"

    # Wait for cluster to be ready
    log_info "Waiting for cluster to be ready..."
    kubectl wait --for=condition=Ready nodes --all --timeout=300s

    log_success "Kubeconfig updated and cluster is ready"
}

# Step 3: Update Kubernetes YAML files with dynamic values
update_k8s_configs() {
    log_info "Updating Kubernetes configurations with dynamic values..."

    # Update Prometheus Agent with workspace endpoint
    log_info "Updating Prometheus Agent remote_write URL..."
    sed -i.bak "s|url: https://aps-workspaces\..*\.amazonaws\.com/workspaces/[^/]*/*api/v1/remote_write|url: ${PROMETHEUS_ENDPOINT%/}/api/v1/remote_write|g" \
        k8s/prometheus-agent/4-prometheus.yaml

    # Update Grafana datasource with workspace endpoint
    log_info "Updating Grafana datasource URL..."
    sed -i.bak "s|url: https://aps-workspaces\..*\.amazonaws\.com/workspaces/[^/]*|url: ${PROMETHEUS_ENDPOINT%/}|g" \
        k8s/grafana/3-datasources.yaml

    # Update IAM role annotations with correct OIDC provider
    log_info "Updating IAM role annotations..."

    # Prometheus service account
    sed -i.bak "s|eks\.amazonaws\.com/role-arn: arn:aws:iam::[0-9]*:role/fyp-autorightsizing-prometheus|eks.amazonaws.com/role-arn: arn:aws:iam::${AWS_ACCOUNT_ID}:role/fyp-autorightsizing-prometheus|g" \
        k8s/prometheus-agent/0-service-account.yaml

    # Grafana service account
    sed -i.bak "s|eks\.amazonaws\.com/role-arn: arn:aws:iam::[0-9]*:role/fyp-autorightsizing-grafana|eks.amazonaws.com/role-arn: arn:aws:iam::${AWS_ACCOUNT_ID}:role/fyp-autorightsizing-grafana|g" \
        k8s/grafana/0-service-account.yaml

    # Clean up backup files
    find k8s -name "*.bak" -delete

    log_success "Kubernetes configurations updated"
}

# Step 4: Deploy Prometheus Operator CRDs
deploy_prometheus_crds() {
    log_info "Deploying Prometheus Operator CRDs..."

    kubectl create -f k8s/prometheus-operator-crd/ 2>/dev/null || kubectl apply -f k8s/prometheus-operator-crd/

    # Wait for CRDs to be established
    log_info "Waiting for CRDs to be established..."
    kubectl wait --for condition=established --timeout=60s \
        crd/prometheuses.monitoring.coreos.com \
        crd/servicemonitors.monitoring.coreos.com \
        crd/podmonitors.monitoring.coreos.com \
        crd/prometheusrules.monitoring.coreos.com

    log_success "Prometheus Operator CRDs deployed"
}

# Step 5: Deploy monitoring namespace
deploy_monitoring_namespace() {
    log_info "Creating monitoring namespace..."

    kubectl apply -f k8s/prometheus-operator/0-namespace.yaml

    # Label namespace for service monitor selection
    kubectl label namespace monitoring monitoring=prometheus-agent --overwrite

    log_success "Monitoring namespace created"
}

# Step 6: Deploy Prometheus Operator
deploy_prometheus_operator() {
    log_info "Deploying Prometheus Operator..."

    kubectl apply -f k8s/prometheus-operator/

    # Wait for operator to be ready
    log_info "Waiting for Prometheus Operator to be ready..."
    kubectl wait --for=condition=Available --timeout=300s \
        -n monitoring deployment/prometheus-operator

    log_success "Prometheus Operator deployed"
}

# Step 7: Deploy Prometheus Agent
deploy_prometheus_agent() {
    log_info "Deploying Prometheus Agent..."

    kubectl apply -f k8s/prometheus-agent/

    # Wait for Prometheus to be ready
    log_info "Waiting for Prometheus Agent to be ready..."
    sleep 10  # Give it time to create the statefulset
    kubectl wait --for=condition=Ready --timeout=300s \
        -n monitoring pod -l app.kubernetes.io/name=prometheus

    log_success "Prometheus Agent deployed"
}

# Step 8: Deploy Node Exporter
deploy_node_exporter() {
    log_info "Deploying Node Exporter..."

    kubectl apply -f k8s/node-exporter/

    # Wait for daemonset to be ready
    log_info "Waiting for Node Exporter to be ready..."
    kubectl rollout status daemonset/node-exporter -n monitoring --timeout=180s

    log_success "Node Exporter deployed"
}

# Step 9: Deploy cAdvisor
deploy_cadvisor() {
    log_info "Deploying cAdvisor..."

    kubectl apply -f k8s/cadvisor/

    # Wait for daemonset to be ready
    log_info "Waiting for cAdvisor to be ready..."
    kubectl rollout status daemonset/cadvisor -n monitoring --timeout=180s

    log_success "cAdvisor deployed"
}

# Step 10: Deploy Kube State Metrics
deploy_kube_state_metrics() {
    log_info "Deploying Kube State Metrics..."

    kubectl apply -f k8s/kube-state-metrics/

    # Wait for deployment to be ready
    log_info "Waiting for Kube State Metrics to be ready..."
    kubectl wait --for=condition=Available --timeout=180s \
        -n monitoring deployment/kube-state-metrics

    log_success "Kube State Metrics deployed"
}

# Step 11: Deploy Grafana
deploy_grafana() {
    log_info "Deploying Grafana..."

    kubectl apply -f k8s/grafana/
    
    # Deploy dashboard ConfigMaps
    log_info "Deploying Grafana dashboard ConfigMaps..."
    kubectl apply -f k8s/grafana/dashboards/

    # Wait for Grafana to be ready
    log_info "Waiting for Grafana to be ready..."
    kubectl wait --for=condition=Available --timeout=180s \
        -n monitoring deployment/grafana

    log_success "Grafana deployed"
    log_info "To access Grafana: kubectl -n monitoring port-forward svc/grafana 3000"
}

# Step 12: Deploy sample applications
deploy_sample_apps() {
    log_info "Deploying sample applications..."

    local apps=("1-cpu-intensive" "2-memory-intensive" "3-variable-load" "4-idle-app")

    for app in "${apps[@]}"; do
        log_info "Deploying ${app}..."
        kubectl apply -f "sample-apps/${app}/k8s/"
    done

    # Wait for all deployments to be ready
    log_info "Waiting for sample applications to be ready..."
    for app in "${apps[@]}"; do
        local app_name=$(echo "$app" | cut -d'-' -f2-)
        kubectl wait --for=condition=Available --timeout=180s \
            -n default deployment/"$app_name" 2>/dev/null || true
    done

    log_success "Sample applications deployed"
}

# Step 14: Verify deployment
verify_deployment() {
    log_info "Verifying deployment..."

    echo ""
    echo "=== Cluster Info ==="
    kubectl cluster-info

    echo ""
    echo "=== Monitoring Stack ==="
    kubectl get pods -n monitoring

    echo ""
    echo "=== Sample Applications ==="
    kubectl get pods -n default

    echo ""
    echo "=== Services ==="
    kubectl get svc -n monitoring
    kubectl get svc -n default

    log_success "Deployment verification complete"
}

# Step 14: Save outputs to file
save_outputs() {
    log_info "Saving deployment outputs..."

    cat > deployment-info.txt << EOF
FYP Auto-Rightsizing Deployment Information
Generated: $(date)

=== AWS Resources ===
Region: $REGION
Account ID: $AWS_ACCOUNT_ID
EKS Cluster: $CLUSTER_NAME
Prometheus Workspace ID: $PROMETHEUS_WORKSPACE_ID
Prometheus Endpoint: $PROMETHEUS_ENDPOINT

=== Access Commands ===
Update kubeconfig:
  aws eks update-kubeconfig --region $REGION --name $CLUSTER_NAME

Access Grafana:
  kubectl -n monitoring port-forward svc/grafana 3000
  Default credentials: admin / prom-operator

Access Prometheus (if needed):
  kubectl -n monitoring port-forward svc/prometheus-operated 9090

View monitoring pods:
  kubectl get pods -n monitoring

View sample apps:
  kubectl get pods -n default

=== Prometheus Workspace ===
Query Endpoint: $PROMETHEUS_ENDPOINT
Workspace ID: $PROMETHEUS_WORKSPACE_ID

=== Destroy Resources ===
To tear down everything:
  ./destroy.sh
EOF

    log_success "Deployment info saved to deployment-info.txt"
}

# Main deployment flow
main() {
    echo ""
    echo "=========================================="
    echo "  FYP Auto-Rightsizing Deployment"
    echo "=========================================="
    echo ""

    check_dependencies

    # Ask for confirmation
    read -p "This will deploy the entire infrastructure. Continue? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_warning "Deployment cancelled"
        exit 0
    fi

    # Record start time
    START_TIME=$(date +%s)

    # Execute deployment steps
    deploy_infrastructure
    update_kubeconfig
    update_k8s_configs
    deploy_prometheus_crds
    deploy_monitoring_namespace
    deploy_prometheus_operator
    deploy_prometheus_agent
    deploy_node_exporter
    deploy_cadvisor
    deploy_kube_state_metrics
    deploy_grafana
    deploy_sample_apps
    verify_deployment
    save_outputs

    # Calculate duration
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    echo ""
    echo "=========================================="
    log_success "Deployment completed in ${DURATION} seconds!"
    echo "=========================================="
    echo ""
    log_info "Check deployment-info.txt for access details"
    echo ""
}

# Run main function
main
