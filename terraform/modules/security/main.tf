# =============================================================================
# Security Groups, IAM Roles, Secrets Manager
# =============================================================================

# ---------------------------------------------------------------------------
# Security Groups
# ---------------------------------------------------------------------------

# ALB Security Group — allows HTTPS from internet
resource "aws_security_group" "alb" {
  name_prefix = "metadata-tool-${var.environment}-alb-"
  description = "ALB - allow HTTPS inbound"
  vpc_id      = var.vpc_id

  ingress {
    description = "HTTPS from internet"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = [var.alb_ingress_cidr]
  }

  # HTTP redirect (optional, Cloudflare handles this but useful for direct ALB access)
  ingress {
    description = "HTTP for redirect"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = [var.alb_ingress_cidr]
  }

  egress {
    description = "To ECS tasks"
    from_port   = var.ecs_port
    to_port     = var.ecs_port
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "metadata-tool-${var.environment}-alb-sg"
  }

  lifecycle {
    create_before_destroy = true
  }
}

# ECS Tasks Security Group — allows traffic from ALB, outbound to RDS and internet
resource "aws_security_group" "ecs" {
  name_prefix = "metadata-tool-${var.environment}-ecs-"
  description = "ECS tasks - allow ALB inbound, DB and internet outbound"
  vpc_id      = var.vpc_id

  ingress {
    description     = "From ALB"
    from_port       = var.ecs_port
    to_port         = var.ecs_port
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  # Outbound to RDS
  egress {
    description = "To RDS"
    from_port   = var.db_port
    to_port     = var.db_port
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # Outbound HTTPS for external APIs (Entra, Claude, Graph, OpenAI)
  egress {
    description = "HTTPS to internet (APIs)"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "metadata-tool-${var.environment}-ecs-sg"
  }

  lifecycle {
    create_before_destroy = true
  }
}

# RDS Security Group — allows traffic only from ECS tasks
resource "aws_security_group" "rds" {
  name_prefix = "metadata-tool-${var.environment}-rds-"
  description = "RDS - allow inbound from ECS only"
  vpc_id      = var.vpc_id

  ingress {
    description     = "PostgreSQL from ECS"
    from_port       = var.db_port
    to_port         = var.db_port
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs.id]
  }

  tags = {
    Name = "metadata-tool-${var.environment}-rds-sg"
  }

  lifecycle {
    create_before_destroy = true
  }
}

# ---------------------------------------------------------------------------
# IAM Roles for ECS
# ---------------------------------------------------------------------------

# ECS Task Execution Role — used by ECS agent to pull images and read secrets
resource "aws_iam_role" "ecs_task_execution" {
  name = "metadata-tool-${var.environment}-ecs-execution"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution_base" {
  role       = aws_iam_role.ecs_task_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

# Allow ECS to read secrets from Secrets Manager
resource "aws_iam_role_policy" "ecs_secrets_access" {
  name = "secrets-access"
  role = aws_iam_role.ecs_task_execution.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Action = [
        "secretsmanager:GetSecretValue"
      ]
      Resource = [
        aws_secretsmanager_secret.database_url.arn,
        aws_secretsmanager_secret.jwt_secret.arn,
        aws_secretsmanager_secret.settings_encryption_key.arn,
        aws_secretsmanager_secret.anthropic_api_key.arn,
        aws_secretsmanager_secret.openai_api_key.arn,
      ]
    }]
  })
}

# ECS Task Role — used by the application itself (currently minimal)
resource "aws_iam_role" "ecs_task" {
  name = "metadata-tool-${var.environment}-ecs-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })
}

# ---------------------------------------------------------------------------
# Secrets Manager
# ---------------------------------------------------------------------------

resource "aws_secretsmanager_secret" "database_url" {
  name                    = "metadata-tool/${var.environment}/database-url"
  recovery_window_in_days = 0 # Immediate deletion on destroy (no 30-day recovery)
}

resource "aws_secretsmanager_secret" "jwt_secret" {
  name                    = "metadata-tool/${var.environment}/jwt-secret"
  recovery_window_in_days = 0
}

resource "aws_secretsmanager_secret" "settings_encryption_key" {
  name                    = "metadata-tool/${var.environment}/encryption-key"
  recovery_window_in_days = 0
}

resource "aws_secretsmanager_secret" "anthropic_api_key" {
  name                    = "metadata-tool/${var.environment}/anthropic-api-key"
  recovery_window_in_days = 0
}

resource "aws_secretsmanager_secret" "openai_api_key" {
  name                    = "metadata-tool/${var.environment}/openai-api-key"
  recovery_window_in_days = 0
}

# Populate secrets with values
resource "aws_secretsmanager_secret_version" "jwt_secret" {
  secret_id     = aws_secretsmanager_secret.jwt_secret.id
  secret_string = var.jwt_secret
}

resource "aws_secretsmanager_secret_version" "settings_encryption_key" {
  secret_id     = aws_secretsmanager_secret.settings_encryption_key.id
  secret_string = var.settings_encryption_key
}

resource "aws_secretsmanager_secret_version" "anthropic_api_key" {
  secret_id     = aws_secretsmanager_secret.anthropic_api_key.id
  secret_string = var.anthropic_api_key
}

resource "aws_secretsmanager_secret_version" "openai_api_key" {
  count         = var.openai_api_key != "" ? 1 : 0
  secret_id     = aws_secretsmanager_secret.openai_api_key.id
  secret_string = var.openai_api_key
}

# DATABASE_URL is populated after RDS is created (in compute module via variable)
