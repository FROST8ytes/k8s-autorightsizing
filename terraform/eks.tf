# EKS Cluster
resource "aws_eks_cluster" "fyp_autorightsizing" {
  name     = "fyp-autorightsizing"
  role_arn = aws_iam_role.eks_cluster.arn

  vpc_config {
    subnet_ids = [
      aws_subnet.private_ap_southeast_1a.id,
      aws_subnet.private_ap_southeast_1b.id,
      aws_subnet.public_ap_southeast_1a.id,
      aws_subnet.public_ap_southeast_1b.id
    ]
  }

  depends_on = [aws_iam_role_policy_attachment.eks_cluster_policy]
}

# EKS Node Group
resource "aws_eks_node_group" "private_nodes" {
  cluster_name    = aws_eks_cluster.fyp_autorightsizing.name
  node_group_name = "fyp-autorightsizing-private-nodes"
  node_role_arn   = aws_iam_role.eks_nodes.arn

  subnet_ids = [
    aws_subnet.private_ap_southeast_1a.id,
    aws_subnet.private_ap_southeast_1b.id
  ]

  capacity_type  = "ON_DEMAND"
  instance_types = ["t3.medium"]

  scaling_config {
    desired_size = 2
    max_size     = 2
    min_size     = 2
  }

  update_config {
    max_unavailable = 1
  }

  labels = {
    role = "general"
  }

  depends_on = [
    aws_iam_role_policy_attachment.eks_nodes_worker_node_policy,
    aws_iam_role_policy_attachment.eks_nodes_cni_policy,
    aws_iam_role_policy_attachment.eks_nodes_container_registry_policy,
  ]
}

# OIDC Provider for EKS
data "tls_certificate" "eks" {
  url = aws_eks_cluster.fyp_autorightsizing.identity[0].oidc[0].issuer
}

resource "aws_iam_openid_connect_provider" "eks" {
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = [data.tls_certificate.eks.certificates[0].sha1_fingerprint]
  url             = aws_eks_cluster.fyp_autorightsizing.identity[0].oidc[0].issuer
}
