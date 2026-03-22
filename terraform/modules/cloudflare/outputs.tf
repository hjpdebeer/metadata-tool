output "acm_certificate_arn" {
  description = "Validated ACM certificate ARN for ALB"
  value       = aws_acm_certificate_validation.main.certificate_arn
}
