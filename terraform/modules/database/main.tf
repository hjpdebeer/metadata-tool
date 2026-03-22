# =============================================================================
# RDS PostgreSQL 17
# =============================================================================

resource "aws_db_subnet_group" "main" {
  name       = "metadata-tool-${var.environment}"
  subnet_ids = var.subnet_ids

  tags = {
    Name = "metadata-tool-${var.environment}-db-subnet-group"
  }
}

resource "aws_db_parameter_group" "postgres17" {
  name   = "metadata-tool-${var.environment}-pg17"
  family = "postgres17"

  # Log slow queries (> 1 second)
  parameter {
    name  = "log_min_duration_statement"
    value = "1000"
  }

  tags = {
    Name = "metadata-tool-${var.environment}-pg17-params"
  }
}

resource "aws_db_instance" "main" {
  identifier = "metadata-tool-${var.environment}"

  engine         = "postgres"
  engine_version = "17"
  instance_class = var.instance_class

  db_name  = var.db_name
  username = var.db_username
  password = var.db_password

  allocated_storage     = var.allocated_storage
  max_allocated_storage = var.allocated_storage * 2 # Auto-scale up to 2x
  storage_type          = "gp2"
  storage_encrypted     = true

  multi_az               = var.multi_az
  db_subnet_group_name   = aws_db_subnet_group.main.name
  vpc_security_group_ids = [var.security_group_id]
  parameter_group_name   = aws_db_parameter_group.postgres17.name

  # Backups
  backup_retention_period = 7
  backup_window           = "03:00-04:00"
  maintenance_window      = "sun:04:00-sun:05:00"

  # Demo settings
  deletion_protection = false
  skip_final_snapshot = true

  # Performance Insights (free tier)
  performance_insights_enabled          = true
  performance_insights_retention_period = 7

  tags = {
    Name = "metadata-tool-${var.environment}"
  }
}
