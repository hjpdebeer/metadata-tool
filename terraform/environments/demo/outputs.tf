output "vpc_id" {
  description = "VPC ID"
  value       = module.network.vpc_id
}

output "alb_dns_name" {
  description = "ALB DNS name (use Cloudflare domain instead)"
  value       = module.compute.alb_dns_name
}

output "ecr_repository_url" {
  description = "ECR repository URL for Docker images"
  value       = module.compute.ecr_repository_url
}

output "rds_endpoint" {
  description = "RDS endpoint"
  value       = module.database.endpoint
}

output "frontend_bucket_name" {
  description = "S3 bucket name for frontend static files"
  value       = module.frontend.bucket_name
}

output "frontend_website_endpoint" {
  description = "S3 website endpoint for frontend"
  value       = module.frontend.website_endpoint
}

output "app_url" {
  description = "Application URL"
  value       = "https://${var.domain}"
}
