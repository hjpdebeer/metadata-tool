variable "environment" {
  description = "Environment name"
  type        = string
}

variable "aws_region" {
  description = "AWS region"
  type        = string
}

variable "availability_zones" {
  description = "List of availability zones"
  type        = list(string)
}

# Network
variable "vpc_cidr" {
  description = "VPC CIDR block"
  type        = string
  default     = "10.0.0.0/16"
}

variable "enable_nat_gateway" {
  description = "Enable NAT Gateway for private subnets"
  type        = bool
}

# Compute
variable "ecs_cpu" {
  description = "ECS task CPU units (256 = 0.25 vCPU)"
  type        = number
}

variable "ecs_memory" {
  description = "ECS task memory in MB"
  type        = number
}

variable "ecs_desired_count" {
  description = "Number of ECS tasks to run"
  type        = number
}

# Database
variable "db_name" {
  description = "Database name"
  type        = string
  default     = "metadata_tool"
}

variable "db_username" {
  description = "Database master username"
  type        = string
  default     = "metadata_tool"
}

variable "db_password" {
  description = "Database master password"
  type        = string
  sensitive   = true
}

variable "rds_instance_class" {
  description = "RDS instance class"
  type        = string
}

variable "rds_allocated_storage" {
  description = "RDS allocated storage in GB"
  type        = number
}

variable "rds_multi_az" {
  description = "Enable Multi-AZ for RDS"
  type        = bool
}

# Domain
variable "domain" {
  description = "Domain name for the application"
  type        = string
}

variable "cloudflare_zone" {
  description = "Cloudflare zone (root domain)"
  type        = string
}

variable "cloudflare_api_token" {
  description = "Cloudflare API token with DNS and Zone Settings edit permissions"
  type        = string
  sensitive   = true
}

# Secrets
variable "jwt_secret" {
  description = "JWT signing secret"
  type        = string
  sensitive   = true
}

variable "settings_encryption_key" {
  description = "Settings encryption key"
  type        = string
  sensitive   = true
}

variable "anthropic_api_key" {
  description = "Anthropic API key for AI enrichment"
  type        = string
  sensitive   = true
}

variable "openai_api_key" {
  description = "OpenAI API key (fallback)"
  type        = string
  sensitive   = true
  default     = ""
}

# Entra SSO
variable "entra_tenant_id" {
  description = "Microsoft Entra tenant ID"
  type        = string
  default     = ""
}

variable "entra_client_id" {
  description = "Microsoft Entra application (client) ID"
  type        = string
  default     = ""
}

variable "entra_client_secret" {
  description = "Microsoft Entra client secret"
  type        = string
  sensitive   = true
  default     = ""
}
