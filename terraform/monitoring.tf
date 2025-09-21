# CloudWatch Log Group for Prometheus
resource "aws_cloudwatch_log_group" "prometheus" {
  name              = "/aws/prometheus/fyp-autorightsizing"
  retention_in_days = 14
}

# AWS Managed Prometheus Workspace
resource "aws_prometheus_workspace" "fyp_autorightsizing" {
  alias = "fyp-autorightsizing"

  logging_configuration {
    log_group_arn = "${aws_cloudwatch_log_group.prometheus.arn}:*"
  }
}
