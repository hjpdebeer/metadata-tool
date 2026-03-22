# =============================================================================
# ECR, ECS Cluster, Task Definition, Service, ALB
# =============================================================================

# ---------------------------------------------------------------------------
# ECR Repository
# ---------------------------------------------------------------------------

resource "aws_ecr_repository" "api" {
  name                 = "metadata-tool-${var.environment}"
  image_tag_mutability = "MUTABLE"
  force_delete         = true

  image_scanning_configuration {
    scan_on_push = true
  }
}

# Lifecycle policy — keep last 5 images
resource "aws_ecr_lifecycle_policy" "api" {
  repository = aws_ecr_repository.api.name

  policy = jsonencode({
    rules = [{
      rulePriority = 1
      description  = "Keep last 5 images"
      selection = {
        tagStatus   = "any"
        countType   = "imageCountMoreThan"
        countNumber = 5
      }
      action = {
        type = "expire"
      }
    }]
  })
}

# ---------------------------------------------------------------------------
# ECS Cluster
# ---------------------------------------------------------------------------

resource "aws_ecs_cluster" "main" {
  name = "metadata-tool-${var.environment}"

  setting {
    name  = "containerInsights"
    value = "disabled" # Enable in production
  }
}

# ---------------------------------------------------------------------------
# CloudWatch Log Groups
# ---------------------------------------------------------------------------

resource "aws_cloudwatch_log_group" "api" {
  name              = "/ecs/metadata-tool-${var.environment}/api"
  retention_in_days = 14
}

resource "aws_cloudwatch_log_group" "pgcat" {
  name              = "/ecs/metadata-tool-${var.environment}/pgcat"
  retention_in_days = 14
}

# ---------------------------------------------------------------------------
# ECS Task Definition (Backend API + PgCat sidecar)
# ---------------------------------------------------------------------------

resource "aws_ecs_task_definition" "api" {
  family                   = "metadata-tool-${var.environment}"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"

  # Fargate valid combos: 256/512/1024/2048/4096 CPU with specific memory ranges
  # 512 CPU supports 1024-4096 MB memory
  cpu    = 512
  memory = 1024

  execution_role_arn = var.ecs_task_execution_role_arn
  task_role_arn      = var.ecs_task_role_arn

  container_definitions = jsonencode([
    {
      name      = "api"
      image     = "${aws_ecr_repository.api.repository_url}:latest"
      essential = true
      cpu       = var.ecs_cpu
      memory    = var.ecs_memory

      portMappings = [{
        containerPort = 8080
        protocol      = "tcp"
      }]

      # Secrets from Secrets Manager
      secrets = [
        { name = "JWT_SECRET", valueFrom = var.jwt_secret_arn },
        { name = "SETTINGS_ENCRYPTION_KEY", valueFrom = var.settings_encryption_key_arn },
        { name = "ANTHROPIC_API_KEY", valueFrom = var.anthropic_api_key_arn },
      ]

      # Environment variables (non-sensitive)
      environment = [
        { name = "DATABASE_URL", value = "postgres://${var.db_username}:${var.db_password}@${var.db_host}:${var.db_port}/${var.db_name}?sslmode=require" },
        { name = "HOST", value = "0.0.0.0" },
        { name = "PORT", value = "8080" },
        { name = "RUST_LOG", value = "metadata_tool=info,tower_http=info" },
        { name = "FRONTEND_URL", value = "https://${var.domain}" },
        { name = "AI_PRIMARY_PROVIDER", value = "claude" },
        { name = "ANTHROPIC_MODEL", value = "claude-sonnet-4-6" },
        { name = "OPENAI_MODEL", value = "gpt-4o" },
        # Entra SSO
        { name = "ENTRA_TENANT_ID", value = "28972789-b904-42eb-aafb-c4eebc7efde3" },
        { name = "ENTRA_CLIENT_ID", value = "ea1513ea-1834-4448-9e80-5450b923994a" },
        { name = "ENTRA_CLIENT_SECRET", value = "pNx8Q~zGwG6DdEL6cf6OB_QJxicCxKVKVKv5~ceF" },
        { name = "ENTRA_REDIRECT_URI", value = "https://${var.domain}/api/v1/auth/callback" },
        # Graph API — leave empty (email notifications disabled)
        { name = "GRAPH_TENANT_ID", value = "" },
        { name = "GRAPH_CLIENT_ID", value = "" },
        { name = "GRAPH_CLIENT_SECRET", value = "" },
        { name = "GRAPH_SENDER_EMAIL", value = "" },
      ]

      healthCheck = {
        command     = ["CMD-SHELL", "curl -f http://localhost:8080/api/v1/health || exit 1"]
        interval    = 30
        timeout     = 5
        retries     = 3
        startPeriod = 60
      }

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.api.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "api"
        }
      }

      # PgCat sidecar is non-essential for now (direct RDS connection)
      # Re-enable when PgCat auth is configured correctly
    },
    {
      name      = "pgcat"
      image     = "ghcr.io/postgresml/pgcat:latest"
      essential = false
      cpu       = 256
      memory    = 256

      portMappings = [{
        containerPort = 6432
        protocol      = "tcp"
      }]

      environment = [
        { name = "PGCAT_CONFIG", value = "/tmp/pgcat.toml" },
      ]

      # PgCat config is injected via command
      command = [
        "sh", "-c",
        join("", [
          "cat > /tmp/pgcat.toml << 'TOML'\n",
          "[general]\n",
          "host = \"0.0.0.0\"\n",
          "port = 6432\n",
          "admin_username = \"pgcat\"\n",
          "admin_password = \"pgcat\"\n",
          "server_lifetime = 86400\n",
          "idle_timeout = 600\n",
          "\n",
          "[pools.metadata_tool]\n",
          "pool_mode = \"transaction\"\n",
          "default_role = \"primary\"\n",
          "query_parser_enabled = true\n",
          "primary_reads_enabled = true\n",
          "load_balancing_mode = \"random\"\n",
          "\n",
          "[pools.metadata_tool.users.0]\n",
          "username = \"${var.db_username}\"\n",
          "password = \"${var.db_password}\"\n",
          "pool_size = 5\n",
          "min_pool_size = 1\n",
          "pool_mode = \"transaction\"\n",
          "\n",
          "[pools.metadata_tool.shards.0]\n",
          "servers = [[\"${var.db_host}\", ${var.db_port}, \"primary\"]]\n",
          "database = \"${var.db_name}\"\n",
          "TOML\n",
          "pgcat /tmp/pgcat.toml",
        ])
      ]

      healthCheck = {
        command     = ["CMD-SHELL", "pg_isready -h localhost -p 6432 || exit 1"]
        interval    = 10
        timeout     = 5
        retries     = 3
        startPeriod = 10
      }

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.pgcat.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "pgcat"
        }
      }
    }
  ])
}

# ---------------------------------------------------------------------------
# Application Load Balancer
# ---------------------------------------------------------------------------

resource "aws_lb" "api" {
  name               = "metadata-tool-${var.environment}"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [var.alb_security_group_id]
  subnets            = var.public_subnet_ids

  tags = {
    Name = "metadata-tool-${var.environment}-alb"
  }
}

resource "aws_lb_target_group" "api" {
  name        = "mt-${var.environment}-api"
  port        = 8080
  protocol    = "HTTP"
  vpc_id      = var.vpc_id
  target_type = "ip"

  health_check {
    path                = "/api/v1/health"
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout             = 5
    interval            = 30
    matcher             = "200"
  }
}

# HTTPS Listener
resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.api.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.acm_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }
}

# HTTP → HTTPS redirect
resource "aws_lb_listener" "http_redirect" {
  load_balancer_arn = aws_lb.api.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = "redirect"
    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}

# ---------------------------------------------------------------------------
# ECS Service
# ---------------------------------------------------------------------------

resource "aws_ecs_service" "api" {
  name            = "metadata-tool-api"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.api.arn
  desired_count   = var.ecs_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    assign_public_ip = false
    security_groups  = [var.ecs_security_group_id]
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.api.arn
    container_name   = "api"
    container_port   = 8080
  }

  depends_on = [aws_lb_listener.https]
}
