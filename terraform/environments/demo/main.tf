# =============================================================================
# Module composition — all infrastructure for the demo environment
# =============================================================================

module "network" {
  source = "../../modules/network"

  environment        = var.environment
  vpc_cidr           = var.vpc_cidr
  availability_zones = var.availability_zones
  enable_nat_gateway = var.enable_nat_gateway
}

module "security" {
  source = "../../modules/security"

  environment = var.environment
  vpc_id      = module.network.vpc_id

  alb_ingress_cidr = "0.0.0.0/0"
  ecs_port         = 8080
  db_port          = 5432

  # Secrets
  db_password             = var.db_password
  db_username             = var.db_username
  db_name                 = var.db_name
  jwt_secret              = var.jwt_secret
  settings_encryption_key = var.settings_encryption_key
  anthropic_api_key       = var.anthropic_api_key
  openai_api_key          = var.openai_api_key
}

module "database" {
  source = "../../modules/database"

  environment        = var.environment
  db_name            = var.db_name
  db_username        = var.db_username
  db_password        = var.db_password
  instance_class     = var.rds_instance_class
  allocated_storage  = var.rds_allocated_storage
  multi_az           = var.rds_multi_az
  subnet_ids         = module.network.database_subnet_ids
  security_group_id  = module.security.rds_security_group_id
}

module "compute" {
  source = "../../modules/compute"

  environment       = var.environment
  aws_region        = var.aws_region
  vpc_id            = module.network.vpc_id
  public_subnet_ids = module.network.public_subnet_ids
  private_subnet_ids = module.network.private_subnet_ids
  enable_nat_gateway = var.enable_nat_gateway

  ecs_cpu           = var.ecs_cpu
  ecs_memory        = var.ecs_memory
  ecs_desired_count = var.ecs_desired_count

  alb_security_group_id = module.security.alb_security_group_id
  ecs_security_group_id = module.security.ecs_security_group_id
  ecs_task_execution_role_arn = module.security.ecs_task_execution_role_arn
  ecs_task_role_arn           = module.security.ecs_task_role_arn

  acm_certificate_arn = module.cloudflare.acm_certificate_arn

  # Database connection via PgCat sidecar
  database_url_secret_arn         = module.security.database_url_secret_arn
  jwt_secret_arn                  = module.security.jwt_secret_arn
  settings_encryption_key_arn     = module.security.settings_encryption_key_arn
  anthropic_api_key_arn           = module.security.anthropic_api_key_arn
  openai_api_key_arn              = module.security.openai_api_key_arn

  db_host     = module.database.endpoint
  db_port     = 5432
  db_name     = var.db_name
  db_username = var.db_username
  db_password = var.db_password

  domain       = var.domain

  # Entra SSO (passed as variables, not hardcoded)
  entra_tenant_id     = var.entra_tenant_id
  entra_client_id     = var.entra_client_id
  entra_client_secret = var.entra_client_secret
}

module "frontend" {
  source = "../../modules/frontend"

  environment = var.environment
  domain      = var.domain
}

module "cloudflare" {
  source = "../../modules/cloudflare"

  domain          = var.domain
  cloudflare_zone = var.cloudflare_zone
  alb_dns_name    = module.compute.alb_dns_name
}
