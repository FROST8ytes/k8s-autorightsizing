#!/bin/bash
# Test script to verify deployment automation without actually deploying

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[CHECK]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

echo ""
echo "=========================================="
echo "  Pre-Deployment Validation"
echo "=========================================="
echo ""

# Check 1: Required tools
log_info "Checking required tools..."
MISSING_TOOLS=()

for tool in terraform kubectl aws jq; do
    if command -v "$tool" &> /dev/null; then
        log_success "$tool is installed"
    else
        log_error "$tool is NOT installed"
        MISSING_TOOLS+=("$tool")
    fi
done

if [ ${#MISSING_TOOLS[@]} -gt 0 ]; then
    echo ""
    log_error "Missing tools: ${MISSING_TOOLS[*]}"
    echo ""
    echo "Install instructions:"
    echo "  brew install terraform kubectl awscli jq"
    echo ""
    exit 1
fi

# Check 2: AWS credentials
log_info "Checking AWS credentials..."
if aws sts get-caller-identity &> /dev/null; then
    ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
    log_success "AWS credentials configured (Account: $ACCOUNT_ID)"
else
    log_error "AWS credentials not configured"
    echo ""
    echo "Configure AWS credentials:"
    echo "  aws configure"
    echo ""
    exit 1
fi

# Check 3: Terraform files
log_info "Checking Terraform files..."
REQUIRED_TF_FILES=("provider.tf" "network.tf" "eks.tf" "iam.tf" "monitoring.tf" "ec2.tf" "outputs.tf")
MISSING_TF_FILES=()

cd terraform 2>/dev/null || {
    log_error "terraform/ directory not found"
    exit 1
}

for file in "${REQUIRED_TF_FILES[@]}"; do
    if [ -f "$file" ]; then
        log_success "$file exists"
    else
        log_error "$file is missing"
        MISSING_TF_FILES+=("$file")
    fi
done

if [ ${#MISSING_TF_FILES[@]} -gt 0 ]; then
    echo ""
    log_error "Missing Terraform files: ${MISSING_TF_FILES[*]}"
    exit 1
fi

# Check 4: Terraform init
log_info "Checking Terraform initialization..."
if terraform init -upgrade &> /dev/null; then
    log_success "Terraform initialized successfully"
else
    log_error "Terraform initialization failed"
    exit 1
fi

# Check 5: Terraform validate
log_info "Validating Terraform configuration..."
if terraform validate &> /dev/null; then
    log_success "Terraform configuration is valid"
else
    log_error "Terraform configuration is invalid"
    terraform validate
    exit 1
fi

cd ..

# Check 6: Kubernetes manifests
log_info "Checking Kubernetes manifest directories..."
K8S_DIRS=("prometheus-operator-crd" "prometheus-operator" "prometheus-agent" "node-exporter" "cadvisor" "kube-state-metrics" "grafana")
MISSING_K8S_DIRS=()

for dir in "${K8S_DIRS[@]}"; do
    if [ -d "k8s/$dir" ]; then
        FILE_COUNT=$(find "k8s/$dir" -name "*.yaml" | wc -l)
        log_success "k8s/$dir exists ($FILE_COUNT YAML files)"
    else
        log_error "k8s/$dir is missing"
        MISSING_K8S_DIRS+=("$dir")
    fi
done

if [ ${#MISSING_K8S_DIRS[@]} -gt 0 ]; then
    echo ""
    log_error "Missing Kubernetes directories: ${MISSING_K8S_DIRS[*]}"
    exit 1
fi

# Check 7: Sample apps
log_info "Checking sample applications..."
SAMPLE_APPS=("1-cpu-intensive" "2-memory-intensive" "3-variable-load" "4-idle-app")
MISSING_APPS=()

for app in "${SAMPLE_APPS[@]}"; do
    if [ -d "sample-apps/$app" ]; then
        if [ -d "sample-apps/$app/k8s" ]; then
            log_success "sample-apps/$app exists with k8s manifests"
        else
            log_warning "sample-apps/$app exists but missing k8s/ directory"
        fi
    else
        log_error "sample-apps/$app is missing"
        MISSING_APPS+=("$app")
    fi
done

if [ ${#MISSING_APPS[@]} -gt 0 ]; then
    echo ""
    log_error "Missing sample apps: ${MISSING_APPS[*]}"
    exit 1
fi

# Check 8: Deployment scripts
log_info "Checking deployment scripts..."
if [ -f "deploy.sh" ] && [ -x "deploy.sh" ]; then
    log_success "deploy.sh exists and is executable"
else
    log_error "deploy.sh is missing or not executable"
    echo "  Run: chmod +x deploy.sh"
    exit 1
fi

if [ -f "destroy.sh" ] && [ -x "destroy.sh" ]; then
    log_success "destroy.sh exists and is executable"
else
    log_error "destroy.sh is missing or not executable"
    echo "  Run: chmod +x destroy.sh"
    exit 1
fi

# Check 9: Python dependencies
log_info "Checking Python dependencies..."
if command -v python3 &> /dev/null; then
    log_success "Python 3 is installed"

    if python3 -c "import requests" 2>/dev/null; then
        log_success "requests library is installed"
    else
        log_warning "requests library not installed"
        echo "  Install: pip3 install requests"
    fi

    if python3 -c "import yaml" 2>/dev/null; then
        log_success "pyyaml library is installed"
    else
        log_warning "pyyaml library not installed"
        echo "  Install: pip3 install pyyaml"
    fi
else
    log_warning "Python 3 not found"
fi

echo ""
echo "=========================================="
log_success "Pre-deployment validation complete!"
echo "=========================================="
echo ""
echo "You're ready to deploy! Run:"
echo "  ./deploy.sh"
echo "  ./destroy.sh  (before leaving)"
echo ""
