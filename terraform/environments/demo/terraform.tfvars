# Environment
environment = "demo"
aws_region  = "eu-west-1"

# Network — Single AZ with NAT Gateway
availability_zones = ["eu-west-1a"]
enable_nat_gateway = true

# Compute — Minimal but production-like
ecs_cpu           = 256   # 0.25 vCPU
ecs_memory        = 512   # 512 MB
ecs_desired_count = 1

# Database — Smallest viable
rds_instance_class    = "db.t4g.micro"
rds_allocated_storage = 20
rds_multi_az          = false

# Domain — Cloudflare-managed
domain          = "metadata.hjpdebeer.com"
cloudflare_zone = "hjpdebeer.com"

# Sensitive values — set via environment variables or .auto.tfvars (gitignored):
#   TF_VAR_db_password
#   TF_VAR_cloudflare_api_token
#   TF_VAR_jwt_secret
#   TF_VAR_settings_encryption_key
#   TF_VAR_anthropic_api_key
#   TF_VAR_openai_api_key
