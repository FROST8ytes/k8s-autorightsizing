# Terraform outputs for automated deployment

# Data sources
data "aws_caller_identity" "current" {}

# Network outputs
output "vpc_id" {
  description = "VPC ID"
  value       = aws_vpc.main.id
}

# EKS outputs
output "cluster_name" {
  description = "EKS cluster name"
  value       = aws_eks_cluster.fyp_autorightsizing.name
}

output "cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = aws_eks_cluster.fyp_autorightsizing.endpoint
}

output "region" {
  description = "AWS region"
  value       = "ap-southeast-1"
}

output "account_id" {
  description = "AWS account ID"
  value       = data.aws_caller_identity.current.account_id
}

# OIDC Provider
output "oidc_provider" {
  description = "OIDC provider ARN"
  value       = aws_iam_openid_connect_provider.eks.arn
}

output "oidc_provider_url" {
  description = "OIDC provider URL"
  value       = aws_iam_openid_connect_provider.eks.url
}

# Prometheus outputs
output "prometheus_workspace_id" {
  description = "AWS Managed Prometheus workspace ID"
  value       = aws_prometheus_workspace.fyp_autorightsizing.id
}

output "prometheus_endpoint" {
  description = "AWS Managed Prometheus workspace endpoint URL"
  value       = aws_prometheus_workspace.fyp_autorightsizing.prometheus_endpoint
}

output "prometheus_workspace_arn" {
  description = "AWS Managed Prometheus workspace ARN"
  value       = aws_prometheus_workspace.fyp_autorightsizing.arn
}

# IAM Role outputs
output "prometheus_role_arn" {
  description = "IAM role ARN for Prometheus"
  value       = aws_iam_role.prometheus.arn
}

output "grafana_role_arn" {
  description = "IAM role ARN for Grafana"
  value       = aws_iam_role.grafana.arn
}

# EC2 outputs
output "ec2_public_ip" {
  description = "EC2 instance public IP"
  value       = aws_instance.my_app.public_ip
}

output "ec2_private_ip" {
  description = "EC2 instance private IP"
  value       = aws_instance.my_app.private_ip
}
