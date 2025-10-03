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

# Main destroy flow
main() {
    echo ""
    echo "=========================================="
    echo "  FYP Auto-Rightsizing Teardown"
    echo "=========================================="
    echo ""

    log_warning "This will destroy ALL resources including:"
    echo "  - EKS Cluster and worker nodes"
    echo "  - AWS Managed Prometheus workspace"
    echo "  - VPC and networking resources"
    echo "  - All monitoring stack"
    echo "  - All sample applications"
    echo ""

    read -p "Are you sure you want to continue? (yes/no) " -r
    echo
    if [[ ! $REPLY == "yes" ]]; then
        log_info "Teardown cancelled"
        exit 0
    fi

    # Record start time
    START_TIME=$(date +%s)

    # Step 1: Delete Kubernetes resources first
    log_info "Deleting Kubernetes resources..."

    # Delete sample apps
    log_info "Deleting sample applications..."
    kubectl delete -f sample-apps/1-cpu-intensive/k8s/ --ignore-not-found=true || true
    kubectl delete -f sample-apps/2-memory-intensive/k8s/ --ignore-not-found=true || true
    kubectl delete -f sample-apps/3-variable-load/k8s/ --ignore-not-found=true || true
    kubectl delete -f sample-apps/4-idle-app/k8s/ --ignore-not-found=true || true

    # Delete monitoring stack
    log_info "Deleting monitoring stack..."
    kubectl delete -f k8s/grafana/ --ignore-not-found=true || true
    kubectl delete -f k8s/kube-state-metrics/ --ignore-not-found=true || true
    kubectl delete -f k8s/cadvisor/ --ignore-not-found=true || true
    kubectl delete -f k8s/node-exporter/ --ignore-not-found=true || true
    kubectl delete -f k8s/prometheus-agent/ --ignore-not-found=true || true
    kubectl delete -f k8s/prometheus-operator/ --ignore-not-found=true || true

    # Delete CRDs (this will clean up all remaining CRD resources)
    log_info "Deleting Prometheus Operator CRDs..."
    kubectl delete -f k8s/prometheus-operator-crd/ --ignore-not-found=true || true

    # Wait a bit for cleanup
    log_info "Waiting for Kubernetes resources to be cleaned up..."
    sleep 30

    log_success "Kubernetes resources deleted"

    # Step 2: Destroy Terraform infrastructure
    log_info "Destroying Terraform infrastructure..."

    cd terraform
    terraform destroy -auto-approve
    cd ..

    log_success "Terraform infrastructure destroyed"

    # Step 3: Clean up local files
    log_info "Cleaning up local files..."
    rm -f deployment-info.txt

    # Calculate duration
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    echo ""
    echo "=========================================="
    log_success "Teardown completed in ${DURATION} seconds!"
    echo "=========================================="
    echo ""
    log_info "All resources have been destroyed"
    log_info "To redeploy: ./deploy.sh"
    echo ""
}

# Run main function
main
