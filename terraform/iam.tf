# EKS Cluster IAM Role
resource "aws_iam_role" "eks_cluster" {
  name = "fyp-autorightsizing-eks-cluster"

  assume_role_policy = <<POLICY
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Service": "eks.amazonaws.com"
      },
      "Action": "sts:AssumeRole"
    }
  ]
}
POLICY
}

resource "aws_iam_role_policy_attachment" "eks_cluster_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
  role       = aws_iam_role.eks_cluster.name
}

# EKS Node Group IAM Role
resource "aws_iam_role" "eks_nodes" {
  name = "fyp-autorightsizing-eks-nodes"

  assume_role_policy = jsonencode({
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ec2.amazonaws.com"
      }
    }]
    Version = "2012-10-17"
  })
}

resource "aws_iam_role_policy_attachment" "eks_nodes_worker_node_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
  role       = aws_iam_role.eks_nodes.name
}

resource "aws_iam_role_policy_attachment" "eks_nodes_cni_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.eks_nodes.name
}

resource "aws_iam_role_policy_attachment" "eks_nodes_container_registry_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
  role       = aws_iam_role.eks_nodes.name
}

# Prometheus IAM Role
data "aws_iam_policy_document" "prometheus_assume_role" {
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]
    effect  = "Allow"

    condition {
      test     = "StringEquals"
      variable = "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub"
      values   = ["system:serviceaccount:monitoring:prometheus"]
    }

    principals {
      identifiers = [aws_iam_openid_connect_provider.eks.arn]
      type        = "Federated"
    }
  }
}

resource "aws_iam_role" "prometheus" {
  assume_role_policy = data.aws_iam_policy_document.prometheus_assume_role.json
  name               = "fyp-autorightsizing-prometheus"
}

resource "aws_iam_policy" "prometheus_ingest_access" {
  name = "FypAutorightsizingPrometheusIngestAccess"

  policy = jsonencode({
    Statement = [{
      Action = [
        "aps:RemoteWrite"
      ]
      Effect   = "Allow"
      Resource = aws_prometheus_workspace.fyp_autorightsizing.arn
    }]
    Version = "2012-10-17"
  })
}

resource "aws_iam_role_policy_attachment" "prometheus_ingest_access" {
  role       = aws_iam_role.prometheus.name
  policy_arn = aws_iam_policy.prometheus_ingest_access.arn
}

resource "aws_iam_role_policy_attachment" "prometheus_ec2_access" {
  role       = aws_iam_role.prometheus.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ReadOnlyAccess"
}

# Grafana IAM Role
data "aws_iam_policy_document" "grafana_assume_role" {
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]
    effect  = "Allow"

    condition {
      test     = "StringEquals"
      variable = "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub"
      values   = ["system:serviceaccount:monitoring:grafana"]
    }

    principals {
      identifiers = [aws_iam_openid_connect_provider.eks.arn]
      type        = "Federated"
    }
  }
}

resource "aws_iam_role" "grafana" {
  assume_role_policy = data.aws_iam_policy_document.grafana_assume_role.json
  name               = "fyp-autorightsizing-grafana"
}

resource "aws_iam_role_policy_attachment" "grafana_query_access" {
  role       = aws_iam_role.grafana.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonPrometheusQueryAccess"
}
