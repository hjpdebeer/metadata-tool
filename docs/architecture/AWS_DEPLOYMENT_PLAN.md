# AWS Deployment Architecture for Metadata Management Tool

**Status**: Proposed
**Author**: Architecture Review
**Date**: 2026-03-22
**Target Environment**: Production SaaS
**Target Capacity**: 500 Concurrent Users

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Resource Footprint Analysis](#2-resource-footprint-analysis)
3. [AWS Architecture Overview](#3-aws-architecture-overview)
4. [Security Architecture](#4-security-architecture)
5. [PgCat Connection Pooling](#5-pgcat-connection-pooling)
6. [CI/CD Pipeline](#6-cicd-pipeline)
7. [Observability](#7-observability)
8. [Cost Estimate](#8-cost-estimate)
9. [Terraform Module Structure](#9-terraform-module-structure)
10. [Migration Path](#10-migration-path)

---

## 1. Executive Summary

This document describes the AWS deployment architecture for the Metadata Management Tool, a SaaS offering for financial institutions. The architecture prioritises security, operational simplicity, and cost-efficiency while supporting 500 concurrent users with room for horizontal scaling.

### Key Architectural Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Compute | ECS Fargate | No server management, security patching automated, pay-per-use |
| Database | RDS PostgreSQL 17 Multi-AZ | Managed service, automatic failover, point-in-time recovery |
| Connection Pooling | PgCat sidecar | Rust-native, modern observability, transaction-mode pooling |
| Frontend Hosting | S3 + CloudFront | Global CDN, edge caching, near-zero latency for static assets |
| Container Registry | ECR | Native ECS integration, IAM-based access, vulnerability scanning |
| Secrets | AWS Secrets Manager | Automatic rotation support, IAM-integrated, audit trail |

### Architecture Diagram (Conceptual)

```
                                   Internet
                                       |
                              +--------+--------+
                              |   Route 53      |
                              |   (DNS)         |
                              +--------+--------+
                                       |
                    +------------------+------------------+
                    |                                     |
           +--------v--------+                   +--------v--------+
           |   CloudFront    |                   |      ALB        |
           |   (Frontend)    |                   |   (Backend)     |
           +--------+--------+                   +--------+--------+
                    |                                     |
           +--------v--------+                   +--------v--------+
           |    S3 Bucket    |                   |  ECS Fargate    |
           | (React SPA)     |                   |  (Rust API)     |
           +-----------------+                   +--------+--------+
                                                          |
                                                 +--------v--------+
                                                 |    PgCat        |
                                                 |   (Sidecar)     |
                                                 +--------+--------+
                                                          |
                                          +---------------+---------------+
                                          |                               |
                                 +--------v--------+             +--------v--------+
                                 |  RDS Primary    |             |  RDS Standby    |
                                 |  (AZ-a)         |<----------->|  (AZ-b)         |
                                 +-----------------+  Sync Repl  +-----------------+
```

---

## 2. Resource Footprint Analysis

### 2.1 Why Rust is Efficient

The Metadata Tool backend is written in Rust using the Axum async web framework. This choice provides significant operational advantages:

| Aspect | Rust/Axum | Node.js (Express) | Java (Spring) |
|--------|-----------|-------------------|---------------|
| Memory per instance | 50-100 MB | 200-500 MB | 512 MB-1 GB |
| Cold start time | <100 ms | 500-2000 ms | 5-30 sec |
| CPU efficiency | Near C-level | JIT overhead | JIT + GC pauses |
| Concurrency model | Async/await, zero-cost | Event loop, single-threaded | Thread pools, context switching |
| GC pauses | None | Mark-sweep | Stop-the-world |

For a metadata management tool where requests involve moderate database queries and JSON serialisation, Rust's lack of garbage collection and efficient async runtime translates directly to lower infrastructure costs.

### 2.2 Backend Compute Sizing

**Assumptions for 500 concurrent users:**
- Average request rate: 5 requests/user/minute = 2,500 requests/minute = ~42 requests/second
- Peak factor: 3x = ~125 requests/second
- P95 response time target: <200 ms
- Request payload: mostly small JSON (1-10 KB), occasional Excel uploads (up to 10 MB)

**Axum Benchmarks (typical):**
- Single-core throughput: 50,000-100,000 simple requests/second
- Database-bound requests: 1,000-5,000 requests/second per core (depending on query complexity)

**Recommended configuration:**

| Component | Specification | Justification |
|-----------|--------------|---------------|
| ECS Task CPU | 1 vCPU | Rust efficiency means single core handles 125 req/s easily |
| ECS Task Memory | 512 MB | ~100 MB baseline + 400 MB for request spikes and file processing |
| Task Count (min) | 2 | High availability across AZs |
| Task Count (max) | 6 | Auto-scale for 3x peak capacity |
| Auto-scale trigger | CPU > 60% or Request Count > 100/task | Conservative threshold for headroom |

**Why 512 MB memory is sufficient:**
- Rust binary: ~20 MB
- Tokio runtime: ~5 MB
- Connection pool overhead: ~10 MB (handles, buffers)
- Request processing: ~50 MB peak (Excel uploads processed in streaming chunks)
- Safety margin: ~400 MB

### 2.3 Database Sizing

**Schema complexity:**
- 42 migration files creating approximately 70+ tables
- Full-text search via TSVECTOR on glossary_terms and data_elements
- Complex triggers for CDE propagation and naming standards
- Moderate JOIN complexity (typically 2-4 table joins)

**Estimated data volumes (first year, single tenant):**

| Entity | Estimated Records | Row Size | Total Storage |
|--------|------------------|----------|---------------|
| Glossary Terms | 5,000 | 2 KB | 10 MB |
| Data Elements | 50,000 | 1 KB | 50 MB |
| Technical Columns | 500,000 | 500 B | 250 MB |
| Workflow History | 100,000 | 500 B | 50 MB |
| Audit Trail | 1,000,000 | 1 KB | 1 GB |
| AI Suggestions | 50,000 | 2 KB | 100 MB |
| **Total (Year 1)** | | | **~1.5 GB** |

**Connection requirements:**
- Active ECS tasks: 2-6
- PgCat connections per task: 20 client-side, 5 server-side pooled
- Total backend connections to RDS: 10-30 (via PgCat pooling)
- RDS max_connections headroom: 100 (for admin, migrations, monitoring)

**Recommended RDS configuration:**

| Parameter | Value | Justification |
|-----------|-------|---------------|
| Instance Class | db.t4g.medium | 2 vCPU, 4 GB RAM, burstable |
| Storage | 100 GB gp3 | 3,000 IOPS baseline, expandable |
| Multi-AZ | Yes | Automatic failover, synchronous replication |
| Backup Retention | 14 days | Point-in-time recovery |
| Performance Insights | Enabled | Query analysis, wait event visibility |
| Parameter Group | Custom | Enable pg_stat_statements, tune for workload |

**Why db.t4g.medium:**
- Graviton3 (ARM) provides ~40% better price-performance vs x86
- 4 GB RAM supports ~70 tables with indexes in shared_buffers
- Burstable CPU handles peak workloads while keeping baseline costs low
- Can upgrade to db.r6g.large (16 GB RAM) if needed without migration

---

## 3. AWS Architecture Overview

### 3.1 VPC Layout

```
Region: eu-west-1 (Ireland) or your preferred region
VPC CIDR: 10.0.0.0/16

+-----------------------------------------------------------------------------------+
|                                    VPC (10.0.0.0/16)                              |
|                                                                                   |
|  +----------------------------------+  +----------------------------------+       |
|  |        Availability Zone A       |  |        Availability Zone B       |       |
|  |                                  |  |                                  |       |
|  |  +----------------------------+  |  |  +----------------------------+  |       |
|  |  | Public Subnet (10.0.1.0/24)|  |  |  | Public Subnet (10.0.2.0/24)|  |       |
|  |  |  - NAT Gateway             |  |  |  |  - NAT Gateway             |  |       |
|  |  |  - ALB Nodes               |  |  |  |  - ALB Nodes               |  |       |
|  |  +----------------------------+  |  |  +----------------------------+  |       |
|  |                                  |  |                                  |       |
|  |  +----------------------------+  |  |  +----------------------------+  |       |
|  |  | Private Subnet(10.0.11.0/24)| |  |  | Private Subnet(10.0.12.0/24)| |       |
|  |  |  - ECS Fargate Tasks       |  |  |  |  - ECS Fargate Tasks       |  |       |
|  |  +----------------------------+  |  |  +----------------------------+  |       |
|  |                                  |  |                                  |       |
|  |  +----------------------------+  |  |  +----------------------------+  |       |
|  |  | DB Subnet (10.0.21.0/24)   |  |  |  | DB Subnet (10.0.22.0/24)   |  |       |
|  |  |  - RDS Primary             |  |  |  |  - RDS Standby             |  |       |
|  |  +----------------------------+  |  |  +----------------------------+  |       |
|  +----------------------------------+  +----------------------------------+       |
|                                                                                   |
+-----------------------------------------------------------------------------------+
```

### 3.2 Component Inventory

| Component | AWS Service | Configuration | Purpose |
|-----------|-------------|---------------|---------|
| DNS | Route 53 | Hosted zone + health checks | Domain routing, failover |
| CDN | CloudFront | Single distribution | Frontend delivery, edge caching |
| Frontend Storage | S3 | Private bucket, OAC | React SPA static files |
| Load Balancer | ALB | Multi-AZ, HTTPS only | Backend traffic distribution |
| API Certificate | ACM | Auto-renewed, DNS validation | TLS termination at ALB |
| Container Registry | ECR | Private repository | Backend container images |
| Compute | ECS Fargate | Service with auto-scaling | Rust API containers |
| Database | RDS PostgreSQL 17 | Multi-AZ, db.t4g.medium | Persistent data storage |
| Secrets | Secrets Manager | Automatic rotation capable | Database creds, API keys, JWT secret |
| Encryption Keys | KMS | Customer-managed key | Encryption at rest |
| Logs | CloudWatch Logs | 30-day retention | Application and access logs |
| Metrics | CloudWatch Metrics | Standard + custom | Performance monitoring |
| Alarms | CloudWatch Alarms | CPU, latency, errors | Operational alerting |
| VPC Flow Logs | CloudWatch Logs | 14-day retention | Network traffic audit |
| Audit Trail | CloudTrail | S3 archival | API call auditing |

### 3.3 Frontend Hosting (S3 + CloudFront)

**S3 Bucket Configuration:**
```hcl
# Terraform snippet (conceptual)
resource "aws_s3_bucket" "frontend" {
  bucket = "metadata-tool-frontend-${var.environment}"
}

resource "aws_s3_bucket_public_access_block" "frontend" {
  bucket                  = aws_s3_bucket.frontend.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}
```

**CloudFront Configuration:**
- Origin Access Control (OAC) for S3 access (not legacy OAI)
- Default root object: `index.html`
- Custom error response: 404 returns `/index.html` with 200 (SPA routing)
- Cache policy: CachingOptimized for static assets
- Origin request policy: CORS-S3Origin
- Web Application Firewall (WAF) attached

**DNS Setup:**
- `app.metadata-tool.example.com` -> CloudFront distribution
- `api.metadata-tool.example.com` -> ALB

### 3.4 Backend Compute (ECS Fargate)

**Why Fargate over EC2:**

| Factor | Fargate | EC2 |
|--------|---------|-----|
| Server management | None | OS patching, AMI updates |
| Scaling | Task-level, seconds | Instance-level, minutes |
| Security | AWS-managed kernel | You manage kernel, host OS |
| Cost (500 users) | ~$80/month | ~$60/month (but + ops overhead) |
| Right-sizing | Per-task CPU/memory | Instance-level, often over-provisioned |

For a small team managing a SaaS product, Fargate's operational simplicity outweighs the ~30% cost premium. Security patching is automatic, and the attack surface is reduced.

**ECS Service Configuration:**

```hcl
resource "aws_ecs_service" "api" {
  name            = "metadata-tool-api"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.api.arn
  desired_count   = 2

  deployment_configuration {
    minimum_healthy_percent = 100
    maximum_percent         = 200
  }

  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.api.arn
    container_name   = "api"
    container_port   = 8080
  }
}
```

**Task Definition:**

```json
{
  "family": "metadata-tool-api",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "512",
  "containerDefinitions": [
    {
      "name": "api",
      "image": "${ECR_REPO_URL}:${IMAGE_TAG}",
      "portMappings": [{ "containerPort": 8080, "protocol": "tcp" }],
      "environment": [
        { "name": "HOST", "value": "0.0.0.0" },
        { "name": "PORT", "value": "8080" },
        { "name": "RUST_LOG", "value": "metadata_tool=info,tower_http=info" },
        { "name": "FRONTEND_URL", "value": "https://app.metadata-tool.example.com" }
      ],
      "secrets": [
        { "name": "DATABASE_URL", "valueFrom": "${DATABASE_SECRET_ARN}:url::" },
        { "name": "JWT_SECRET", "valueFrom": "${JWT_SECRET_ARN}" },
        { "name": "SETTINGS_ENCRYPTION_KEY", "valueFrom": "${ENCRYPTION_KEY_SECRET_ARN}" },
        { "name": "ANTHROPIC_API_KEY", "valueFrom": "${ANTHROPIC_SECRET_ARN}" },
        { "name": "OPENAI_API_KEY", "valueFrom": "${OPENAI_SECRET_ARN}" }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/metadata-tool-api",
          "awslogs-region": "eu-west-1",
          "awslogs-stream-prefix": "api"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8080/api/v1/health || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 10
      }
    },
    {
      "name": "pgcat",
      "image": "ghcr.io/postgresml/pgcat:latest",
      "portMappings": [{ "containerPort": 6432, "protocol": "tcp" }],
      "environment": [
        { "name": "PGCAT_CONFIG", "value": "/etc/pgcat/pgcat.toml" }
      ],
      "mountPoints": [
        { "sourceVolume": "pgcat-config", "containerPath": "/etc/pgcat", "readOnly": true }
      ],
      "dependsOn": [],
      "essential": true
    }
  ]
}
```

### 3.5 Database (RDS PostgreSQL 17)

**Instance Configuration:**

| Parameter | Value |
|-----------|-------|
| Engine | PostgreSQL 17.x |
| Instance Class | db.t4g.medium (production), db.t4g.micro (staging) |
| Storage | 100 GB gp3, auto-scaling enabled |
| IOPS | 3,000 baseline (gp3 included) |
| Multi-AZ | Enabled |
| Encryption | KMS CMK |
| Backup Window | 03:00-04:00 UTC |
| Maintenance Window | Sun 04:00-05:00 UTC |
| Parameter Group | Custom (pg_stat_statements, logging) |
| Deletion Protection | Enabled |

**Custom Parameter Group Settings:**

```
shared_preload_libraries = 'pg_stat_statements'
pg_stat_statements.track = all
log_min_duration_statement = 1000  # Log queries > 1 second
log_connections = on
log_disconnections = on
log_lock_waits = on
log_statement = 'ddl'
```

---

## 4. Security Architecture

### 4.1 Network Security Groups

**Security Group Matrix:**

| Source | Destination | Port | Protocol | Purpose |
|--------|-------------|------|----------|---------|
| 0.0.0.0/0 | ALB | 443 | TCP | HTTPS ingress |
| ALB SG | ECS Tasks SG | 8080 | TCP | Application traffic |
| ECS Tasks SG | RDS SG | 5432 | TCP | Database connections |
| ECS Tasks SG | 0.0.0.0/0 | 443 | TCP | Outbound to AI APIs, Entra ID, Graph API |
| RDS SG | - | - | - | No outbound required |

**ALB Security Group:**
```hcl
resource "aws_security_group" "alb" {
  name        = "metadata-tool-alb"
  description = "ALB security group"
  vpc_id      = aws_vpc.main.id

  ingress {
    description = "HTTPS from internet"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    description     = "To ECS tasks"
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs_tasks.id]
  }
}
```

**ECS Tasks Security Group:**
```hcl
resource "aws_security_group" "ecs_tasks" {
  name        = "metadata-tool-ecs-tasks"
  description = "ECS tasks security group"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "From ALB"
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  egress {
    description = "To RDS"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    security_groups = [aws_security_group.rds.id]
  }

  egress {
    description = "HTTPS outbound (AI APIs, Entra, Graph)"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
```

**RDS Security Group:**
```hcl
resource "aws_security_group" "rds" {
  name        = "metadata-tool-rds"
  description = "RDS security group"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "From ECS tasks"
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.ecs_tasks.id]
  }

  # No egress rules - RDS does not initiate outbound connections
}
```

### 4.2 WAF Configuration

**CloudFront WAF Rules:**

| Rule | Action | Purpose |
|------|--------|---------|
| AWS-AWSManagedRulesCommonRuleSet | Count (then Block) | OWASP Top 10 protection |
| AWS-AWSManagedRulesKnownBadInputsRuleSet | Block | Log4j, path traversal |
| AWS-AWSManagedRulesSQLiRuleSet | Block | SQL injection |
| Rate limiting | Block > 2000 req/5min/IP | DoS protection |
| Geo-restriction | Block (optional) | Restrict to allowed countries |

**ALB WAF Rules:**

| Rule | Action | Purpose |
|------|--------|---------|
| AWS-AWSManagedRulesCommonRuleSet | Block | OWASP Top 10 |
| AWS-AWSManagedRulesKnownBadInputsRuleSet | Block | Known bad inputs |
| Custom rate limit | Block > 100 req/5min/IP on /api/v1/auth/* | Auth brute-force protection |

### 4.3 Secrets Management

**Secrets in AWS Secrets Manager:**

| Secret Name | Contents | Rotation |
|-------------|----------|----------|
| `metadata-tool/database/master` | RDS master credentials | 30 days (Lambda rotation) |
| `metadata-tool/database/app` | Application DB user credentials | 30 days |
| `metadata-tool/jwt-secret` | JWT signing secret | Manual (triggers app redeploy) |
| `metadata-tool/encryption-key` | Settings encryption key | Manual |
| `metadata-tool/entra/credentials` | Entra ID client secret | Manual (tied to Entra rotation) |
| `metadata-tool/graph/credentials` | Graph API client secret | Manual |
| `metadata-tool/anthropic/api-key` | Anthropic API key | Manual |
| `metadata-tool/openai/api-key` | OpenAI API key | Manual |

**Secret Access:**
- ECS Task Role has `secretsmanager:GetSecretValue` for specific ARNs only
- No broad `secretsmanager:*` permissions
- KMS decrypt permission on the CMK used for secret encryption

### 4.4 KMS Encryption

**Customer Managed Key (CMK):**
```hcl
resource "aws_kms_key" "main" {
  description             = "Metadata Tool encryption key"
  deletion_window_in_days = 30
  enable_key_rotation     = true

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "Enable IAM User Permissions"
        Effect    = "Allow"
        Principal = { AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root" }
        Action    = "kms:*"
        Resource  = "*"
      },
      {
        Sid       = "Allow RDS to use the key"
        Effect    = "Allow"
        Principal = { Service = "rds.amazonaws.com" }
        Action    = ["kms:Encrypt", "kms:Decrypt", "kms:GenerateDataKey*"]
        Resource  = "*"
      }
    ]
  })
}
```

**Encryption at Rest:**
- RDS: KMS CMK encryption
- S3: SSE-S3 (frontend assets are public anyway via CloudFront)
- Secrets Manager: KMS CMK encryption
- CloudWatch Logs: KMS CMK encryption
- EBS volumes (Fargate-managed): AWS-managed encryption

### 4.5 IAM Roles

**ECS Task Execution Role:**
- Pulls container images from ECR
- Retrieves secrets from Secrets Manager
- Sends logs to CloudWatch

**ECS Task Role:**
- No AWS service permissions needed (application only talks to RDS and external APIs)
- If future features need S3 or other AWS services, add minimal permissions here

```hcl
resource "aws_iam_role" "ecs_task_execution" {
  name = "metadata-tool-ecs-task-execution"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "ecs-tasks.amazonaws.com" }
    }]
  })
}

resource "aws_iam_role_policy" "ecs_task_execution" {
  name = "metadata-tool-ecs-task-execution"
  role = aws_iam_role.ecs_task_execution.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ecr:GetAuthorizationToken",
          "ecr:BatchCheckLayerAvailability",
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchGetImage"
        ]
        Resource = "*"
      },
      {
        Effect   = "Allow"
        Action   = ["secretsmanager:GetSecretValue"]
        Resource = "arn:aws:secretsmanager:${var.region}:${var.account_id}:secret:metadata-tool/*"
      },
      {
        Effect   = "Allow"
        Action   = ["kms:Decrypt"]
        Resource = aws_kms_key.main.arn
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "arn:aws:logs:${var.region}:${var.account_id}:log-group:/ecs/metadata-tool-*:*"
      }
    ]
  })
}
```

### 4.6 VPC Flow Logs and CloudTrail

**VPC Flow Logs:**
- Destination: CloudWatch Logs
- Retention: 14 days
- Format: Version 2 (includes additional fields like pkt-src-aws-service)

**CloudTrail:**
- Multi-region trail
- S3 bucket with lifecycle policy (90 days to Glacier, 7 years retention for compliance)
- CloudWatch Logs integration for real-time alerting
- Data events: S3 and Lambda (if used)

---

## 5. PgCat Connection Pooling

### 5.1 Why PgCat over PgBouncer

| Feature | PgCat | PgBouncer |
|---------|-------|-----------|
| Language | Rust | C |
| Modern async | Tokio-based | libevent |
| Observability | Prometheus metrics built-in | Basic stats |
| Configuration | TOML, hot-reload | INI, requires restart |
| Prepared statements | Full support in transaction mode | Problematic in transaction mode |
| Query routing | Built-in read/write splitting | Requires external solution |
| Sharding | Native support | Not supported |
| Memory safety | Rust guarantees | Manual memory management |
| Active development | Very active (PostgresML team) | Stable but slower evolution |

For a Rust application, PgCat provides a natural fit with excellent observability and future-proofing for read replicas and multi-tenancy.

### 5.2 Configuration

**PgCat TOML Configuration:**

```toml
[general]
host = "0.0.0.0"
port = 6432
admin_username = "pgcat_admin"
admin_password = "${PGCAT_ADMIN_PASSWORD}"
connect_timeout = 5000
idle_timeout = 60000
server_lifetime = 86400000
log_client_connections = false
log_client_disconnections = false
prometheus_exporter_port = 9930

[pools.metadata_tool]
pool_mode = "transaction"
default_role = "primary"
query_parser_enabled = true

[pools.metadata_tool.shards.0]
servers = [
    ["${RDS_PRIMARY_ENDPOINT}", 5432, "primary"],
]
database = "metadata_tool"

[pools.metadata_tool.users.0]
username = "app_user"
password = "${DB_APP_PASSWORD}"
pool_size = 10
min_pool_size = 2
statement_timeout = 30000
```

### 5.3 Connection Limits

**Connection flow:**

```
ECS Task 1 ----[20 client conns]----> PgCat 1 ----[5 server conns]----> RDS
ECS Task 2 ----[20 client conns]----> PgCat 2 ----[5 server conns]----> RDS
...
```

**Sizing rationale:**

| Layer | Connections | Calculation |
|-------|-------------|-------------|
| Axum connection pool | 20 per task | Tokio runtime can handle 1000s, but 20 is sufficient for typical load |
| PgCat client-side | 20 per sidecar | Matches Axum pool |
| PgCat server-side | 5 per sidecar | Transaction mode reuses connections efficiently |
| Total to RDS | 10-30 | 2-6 tasks x 5 = 10-30 connections |
| RDS max_connections | 112 (db.t4g.medium default) | Plenty of headroom |

**Why sidecar deployment:**
- Each ECS task gets its own PgCat instance
- No cross-AZ latency for connection pooling
- Failure isolation (one PgCat failure affects one task, not all)
- Simpler networking (localhost communication within task)

### 5.4 Future: Read Replicas

When read scaling is needed, PgCat configuration extends easily:

```toml
[pools.metadata_tool.shards.0]
servers = [
    ["${RDS_PRIMARY_ENDPOINT}", 5432, "primary"],
    ["${RDS_REPLICA_ENDPOINT}", 5432, "replica"],
]

[pools.metadata_tool]
default_role = "any"  # or "replica" for read-heavy workloads
```

Application code marks read-only queries with `SET TRANSACTION READ ONLY` or uses a query comment, and PgCat routes appropriately.

---

## 6. CI/CD Pipeline

### 6.1 Pipeline Overview

```
+----------+     +---------+     +--------+     +------------+     +---------+
|  GitHub  | --> | Build & | --> |  ECR   | --> | ECS Deploy | --> |  Smoke  |
|  Push    |     |  Test   |     |  Push  |     | (Rolling)  |     |  Tests  |
+----------+     +---------+     +--------+     +------------+     +---------+
     |                                               |
     v                                               v
+----------+                                    +----------+
| Frontend |                                    | DB       |
| S3 Sync  |                                    | Migrate  |
+----------+                                    +----------+
     |
     v
+------------+
| CloudFront |
| Invalidate |
+------------+
```

### 6.2 GitHub Actions Workflow

**Backend Workflow (`.github/workflows/deploy-backend.yml`):**

```yaml
name: Deploy Backend

on:
  push:
    branches: [main]
    paths:
      - 'backend/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/deploy-backend.yml'

env:
  AWS_REGION: eu-west-1
  ECR_REPOSITORY: metadata-tool-api
  ECS_SERVICE: metadata-tool-api
  ECS_CLUSTER: metadata-tool
  CONTAINER_NAME: api

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache Cargo
        uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --workspace

      - name: Run clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Check formatting
        run: cargo fmt --all -- --check

  build-and-deploy:
    needs: test
    runs-on: ubuntu-latest
    permissions:
      id-token: write
      contents: read

    steps:
      - uses: actions/checkout@v4

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::${{ secrets.AWS_ACCOUNT_ID }}:role/github-actions-deploy
          aws-region: ${{ env.AWS_REGION }}

      - name: Login to ECR
        id: login-ecr
        uses: aws-actions/amazon-ecr-login@v2

      - name: Build and push image
        env:
          ECR_REGISTRY: ${{ steps.login-ecr.outputs.registry }}
          IMAGE_TAG: ${{ github.sha }}
        run: |
          docker build -t $ECR_REGISTRY/$ECR_REPOSITORY:$IMAGE_TAG -t $ECR_REGISTRY/$ECR_REPOSITORY:latest -f backend/Dockerfile .
          docker push $ECR_REGISTRY/$ECR_REPOSITORY:$IMAGE_TAG
          docker push $ECR_REGISTRY/$ECR_REPOSITORY:latest

      - name: Run database migrations
        env:
          DATABASE_URL: ${{ secrets.DATABASE_URL }}
        run: |
          # Run migrations using sqlx CLI in a temporary container
          docker run --rm \
            -e DATABASE_URL=$DATABASE_URL \
            ${{ steps.login-ecr.outputs.registry }}/$ECR_REPOSITORY:${{ github.sha }} \
            /bin/sh -c "sqlx migrate run"

      - name: Update ECS service
        run: |
          aws ecs update-service \
            --cluster $ECS_CLUSTER \
            --service $ECS_SERVICE \
            --force-new-deployment

      - name: Wait for deployment
        run: |
          aws ecs wait services-stable \
            --cluster $ECS_CLUSTER \
            --services $ECS_SERVICE

      - name: Smoke test
        run: |
          curl -f https://api.metadata-tool.example.com/api/v1/health
```

**Frontend Workflow (`.github/workflows/deploy-frontend.yml`):**

```yaml
name: Deploy Frontend

on:
  push:
    branches: [main]
    paths:
      - 'frontend/**'
      - '.github/workflows/deploy-frontend.yml'

env:
  AWS_REGION: eu-west-1
  S3_BUCKET: metadata-tool-frontend-production
  CLOUDFRONT_DISTRIBUTION_ID: E1234567890ABC

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    permissions:
      id-token: write
      contents: read

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: frontend/package-lock.json

      - name: Install dependencies
        working-directory: frontend
        run: npm ci

      - name: Build
        working-directory: frontend
        env:
          VITE_API_URL: https://api.metadata-tool.example.com
        run: npm run build

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::${{ secrets.AWS_ACCOUNT_ID }}:role/github-actions-deploy
          aws-region: ${{ env.AWS_REGION }}

      - name: Sync to S3
        working-directory: frontend
        run: |
          aws s3 sync dist/ s3://$S3_BUCKET/ \
            --delete \
            --cache-control "max-age=31536000" \
            --exclude "index.html" \
            --exclude "*.json"

          # index.html and manifest files should not be cached
          aws s3 cp dist/index.html s3://$S3_BUCKET/index.html \
            --cache-control "no-cache, no-store, must-revalidate"

      - name: Invalidate CloudFront
        run: |
          aws cloudfront create-invalidation \
            --distribution-id $CLOUDFRONT_DISTRIBUTION_ID \
            --paths "/index.html" "/*.json"
```

### 6.3 Database Migrations Strategy

**Safe migration approach:**

1. **Backward-compatible migrations only** - New columns must be nullable or have defaults
2. **Pre-deployment migrations** - Run migrations before deploying new code
3. **Rollback plan** - Each migration has a corresponding down migration
4. **Connection draining** - PgCat handles graceful connection handoff during rolling deploy

**Migration execution in CI:**
- Uses a short-lived container with network access to RDS (via VPC peering to GitHub Actions or using a bastion/jump box)
- Alternatively, migrations run as an ECS task before the main deployment

**For complex migrations (rare):**
1. Deploy migration-only release
2. Monitor for issues
3. Deploy application code that uses new schema
4. Clean up old columns in subsequent release

### 6.4 Dockerfile

**Backend Dockerfile (`backend/Dockerfile`):**

```dockerfile
# Stage 1: Build
FROM rust:1.79-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY backend/ backend/

# Build release binary
RUN cargo build --release --bin metadata-tool

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/metadata-tool /app/metadata-tool

# Copy migrations for sqlx
COPY backend/migrations /app/migrations

# Non-root user
RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 8080

CMD ["/app/metadata-tool"]
```

---

## 7. Observability

### 7.1 Logging

**Log Streams:**

| Source | Log Group | Retention | Format |
|--------|-----------|-----------|--------|
| Backend API | `/ecs/metadata-tool-api` | 30 days | JSON (tracing-subscriber) |
| PgCat | `/ecs/metadata-tool-pgcat` | 14 days | Plain text |
| ALB Access | `/alb/metadata-tool` | 90 days | W3C extended |
| VPC Flow | `/vpc/metadata-tool` | 14 days | Flow log format |

**Structured Logging Configuration:**

The backend already uses `tracing-subscriber` with JSON output capability. Enable in production:

```rust
// In main.rs, when RUST_LOG is set
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .json()  // Enable JSON format in production
    .init();
```

### 7.2 Metrics

**CloudWatch Metrics to Monitor:**

| Metric | Namespace | Alarm Threshold | Action |
|--------|-----------|-----------------|--------|
| CPUUtilization | AWS/ECS | > 80% for 5 min | Scale out |
| MemoryUtilization | AWS/ECS | > 85% for 5 min | Alert |
| TargetResponseTime | AWS/ApplicationELB | > 500ms P95 | Alert |
| HTTPCode_ELB_5XX_Count | AWS/ApplicationELB | > 10/min | Alert |
| DatabaseConnections | AWS/RDS | > 80 | Alert |
| FreeableMemory | AWS/RDS | < 500 MB | Alert |
| ReadLatency | AWS/RDS | > 20ms | Alert |
| WriteLatency | AWS/RDS | > 20ms | Alert |

**Custom Metrics (via CloudWatch Embedded Metric Format):**

Consider adding application-level metrics:
- Request count by endpoint
- AI API call latency
- Workflow state transitions
- Active user sessions

### 7.3 Health Check Endpoints

**Existing endpoint:** `GET /api/v1/health`

**Recommended enhancements:**

```rust
// Current simple health check
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

// Enhanced for production
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,          // "healthy", "degraded", "unhealthy"
    pub version: String,         // Application version from build
    pub database: String,        // "connected", "disconnected"
    pub uptime_seconds: u64,
}
```

**ALB Health Check Configuration:**
- Path: `/api/v1/health`
- Interval: 30 seconds
- Timeout: 5 seconds
- Healthy threshold: 2
- Unhealthy threshold: 3

### 7.4 RDS Performance Insights

Enable Performance Insights on the RDS instance:
- Retention: 7 days (free tier) or 2 years (paid)
- Top SQL queries by wait time
- Database load visualisation
- Host metrics correlation

This is invaluable for diagnosing slow queries without enabling expensive query logging.

### 7.5 Alerting

**CloudWatch Alarms with SNS:**

```hcl
resource "aws_sns_topic" "alerts" {
  name = "metadata-tool-alerts"
}

resource "aws_sns_topic_subscription" "email" {
  topic_arn = aws_sns_topic.alerts.arn
  protocol  = "email"
  endpoint  = "ops@metadata-tool.example.com"
}

resource "aws_cloudwatch_metric_alarm" "api_5xx" {
  alarm_name          = "metadata-tool-api-5xx"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "HTTPCode_ELB_5XX_Count"
  namespace           = "AWS/ApplicationELB"
  period              = 60
  statistic           = "Sum"
  threshold           = 10
  alarm_description   = "API returning 5xx errors"
  alarm_actions       = [aws_sns_topic.alerts.arn]

  dimensions = {
    LoadBalancer = aws_lb.api.arn_suffix
  }
}
```

---

## 8. Cost Estimate

### 8.1 Monthly Cost Breakdown (500 Concurrent Users)

**Compute (ECS Fargate):**

| Resource | Specification | Hours/Month | Unit Price | Monthly Cost |
|----------|--------------|-------------|------------|--------------|
| ECS Tasks (min) | 2 x (1 vCPU, 0.5 GB) | 730 | $0.04048/vCPU-hr + $0.004445/GB-hr | ~$65 |
| ECS Tasks (avg) | 3 x (1 vCPU, 0.5 GB) | 730 | | ~$95 |
| ECS Tasks (max) | 6 x (1 vCPU, 0.5 GB) | occasional | | peak: ~$190 |

**Database (RDS):**

| Resource | Specification | Monthly Cost |
|----------|--------------|--------------|
| RDS db.t4g.medium | Multi-AZ | ~$130 |
| RDS Storage | 100 GB gp3 | ~$12 |
| RDS Backup | 14 days retention | ~$5 |
| **Subtotal** | | **~$147** |

**Networking:**

| Resource | Specification | Monthly Cost |
|----------|--------------|--------------|
| ALB | Fixed + LCU | ~$25 |
| NAT Gateway | 2x (per AZ) | ~$65 |
| NAT Data Processing | ~50 GB outbound | ~$2 |
| Data Transfer Out | ~50 GB | ~$5 |
| **Subtotal** | | **~$97** |

**Storage and CDN:**

| Resource | Specification | Monthly Cost |
|----------|--------------|--------------|
| S3 (frontend) | ~100 MB | ~$0 |
| CloudFront | ~100 GB/month | ~$9 |
| ECR | ~500 MB images | ~$0.50 |
| **Subtotal** | | **~$10** |

**Security and Observability:**

| Resource | Specification | Monthly Cost |
|----------|--------------|--------------|
| Secrets Manager | 8 secrets | ~$4 |
| KMS | 1 CMK + requests | ~$3 |
| CloudWatch Logs | ~10 GB/month | ~$5 |
| CloudWatch Alarms | 10 alarms | ~$1 |
| WAF | 2 Web ACLs | ~$10 |
| **Subtotal** | | **~$23** |

### 8.2 Total Monthly Cost

| Category | Cost |
|----------|------|
| Compute | $65-95 |
| Database | $147 |
| Networking | $97 |
| Storage/CDN | $10 |
| Security/Observability | $23 |
| **Total (typical)** | **~$340-370/month** |

### 8.3 Cost Scaling Factors

**Linear scaling (usage-based):**
- NAT data processing
- CloudFront bandwidth
- CloudWatch Logs ingestion
- ALB LCU hours

**Step scaling (capacity-based):**
- ECS Fargate tasks (add 1 task at ~$30/month)
- RDS instance upgrade (db.t4g.medium -> db.r6g.large: +$200/month)

**Fixed costs:**
- NAT Gateway (majority of networking cost)
- Secrets Manager
- KMS

### 8.4 Cost Optimisation Opportunities

1. **Savings Plans**: 1-year Fargate Savings Plan saves ~30% (~$20/month)
2. **Reserved Instances**: 1-year RDS Reserved Instance saves ~30% (~$45/month)
3. **Single NAT Gateway**: For non-critical environments, use 1 NAT Gateway in 1 AZ (~$32/month savings)
4. **Spot Fargate**: For staging environments, use Fargate Spot (~70% savings)

---

## 9. Terraform Module Structure

### 9.1 Directory Layout

```
terraform/
├── environments/
│   ├── production/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── terraform.tfvars
│   │   └── backend.tf
│   └── staging/
│       ├── main.tf
│       ├── variables.tf
│       ├── terraform.tfvars
│       └── backend.tf
├── modules/
│   ├── network/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── outputs.tf
│   │   └── README.md
│   ├── compute/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── outputs.tf
│   │   ├── task-definition.json.tpl
│   │   └── README.md
│   ├── database/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── outputs.tf
│   │   └── README.md
│   ├── security/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── outputs.tf
│   │   └── README.md
│   ├── cdn/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── outputs.tf
│   │   └── README.md
│   └── monitoring/
│       ├── main.tf
│       ├── variables.tf
│       ├── outputs.tf
│       └── README.md
└── README.md
```

### 9.2 Module Breakdown

**network/** - VPC, Subnets, Route Tables, NAT Gateways, Internet Gateway

| Input | Type | Description |
|-------|------|-------------|
| vpc_cidr | string | VPC CIDR block |
| availability_zones | list(string) | AZs to deploy into |
| environment | string | Environment name |

| Output | Description |
|--------|-------------|
| vpc_id | VPC ID |
| public_subnet_ids | Public subnet IDs |
| private_subnet_ids | Private subnet IDs |
| database_subnet_ids | Database subnet IDs |

**compute/** - ECR, ECS Cluster, Service, Task Definition, ALB, Auto Scaling

| Input | Type | Description |
|-------|------|-------------|
| vpc_id | string | VPC ID |
| private_subnet_ids | list(string) | Private subnets for tasks |
| public_subnet_ids | list(string) | Public subnets for ALB |
| security_group_ids | object | Security group IDs |
| secrets_arns | map(string) | Secret ARNs for task |
| container_image | string | ECR image URI |
| desired_count | number | Desired task count |
| min_capacity | number | Min tasks for auto-scaling |
| max_capacity | number | Max tasks for auto-scaling |

**database/** - RDS Instance, Subnet Group, Parameter Group

| Input | Type | Description |
|-------|------|-------------|
| vpc_id | string | VPC ID |
| database_subnet_ids | list(string) | Database subnets |
| security_group_ids | list(string) | Security group IDs |
| instance_class | string | RDS instance class |
| engine_version | string | PostgreSQL version |
| kms_key_arn | string | KMS key for encryption |
| master_password_secret_arn | string | Secret ARN for master password |

**security/** - Security Groups, KMS Key, Secrets Manager, IAM Roles, WAF

| Input | Type | Description |
|-------|------|-------------|
| vpc_id | string | VPC ID |
| environment | string | Environment name |

| Output | Description |
|--------|-------------|
| alb_security_group_id | ALB security group |
| ecs_security_group_id | ECS tasks security group |
| rds_security_group_id | RDS security group |
| kms_key_arn | KMS key ARN |
| ecs_task_execution_role_arn | Task execution role ARN |
| ecs_task_role_arn | Task role ARN |

**cdn/** - S3 Bucket, CloudFront Distribution, OAC

| Input | Type | Description |
|-------|------|-------------|
| domain_name | string | Frontend domain |
| certificate_arn | string | ACM certificate ARN |
| waf_acl_arn | string | WAF Web ACL ARN |

**monitoring/** - CloudWatch Log Groups, Metrics, Alarms, SNS Topics

| Input | Type | Description |
|-------|------|-------------|
| environment | string | Environment name |
| ecs_cluster_name | string | ECS cluster name |
| alb_arn_suffix | string | ALB ARN suffix |
| rds_instance_id | string | RDS instance ID |
| alert_email | string | Email for alerts |

### 9.3 Environment Configuration Example

**environments/production/main.tf:**

```hcl
terraform {
  required_version = ">= 1.5.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

module "network" {
  source = "../../modules/network"

  vpc_cidr           = "10.0.0.0/16"
  availability_zones = ["eu-west-1a", "eu-west-1b"]
  environment        = "production"
}

module "security" {
  source = "../../modules/security"

  vpc_id      = module.network.vpc_id
  environment = "production"
}

module "database" {
  source = "../../modules/database"

  vpc_id                     = module.network.vpc_id
  database_subnet_ids        = module.network.database_subnet_ids
  security_group_ids         = [module.security.rds_security_group_id]
  instance_class             = "db.t4g.medium"
  engine_version             = "17"
  kms_key_arn                = module.security.kms_key_arn
  master_password_secret_arn = module.security.db_master_secret_arn
}

module "compute" {
  source = "../../modules/compute"

  vpc_id             = module.network.vpc_id
  private_subnet_ids = module.network.private_subnet_ids
  public_subnet_ids  = module.network.public_subnet_ids
  security_group_ids = {
    alb = module.security.alb_security_group_id
    ecs = module.security.ecs_security_group_id
  }
  secrets_arns = module.security.secrets_arns
  container_image = "${var.ecr_repository_url}:${var.image_tag}"
  desired_count   = 2
  min_capacity    = 2
  max_capacity    = 6
}

module "cdn" {
  source = "../../modules/cdn"

  domain_name     = "app.metadata-tool.example.com"
  certificate_arn = var.acm_certificate_arn
  waf_acl_arn     = module.security.waf_acl_arn
}

module "monitoring" {
  source = "../../modules/monitoring"

  environment      = "production"
  ecs_cluster_name = module.compute.ecs_cluster_name
  alb_arn_suffix   = module.compute.alb_arn_suffix
  rds_instance_id  = module.database.instance_id
  alert_email      = var.alert_email
}
```

---

## 10. Migration Path

### 10.1 Prerequisites

Before starting the migration:

1. **AWS Account Setup**
   - Create dedicated AWS account (or use existing)
   - Enable AWS Organizations (recommended for billing isolation)
   - Configure IAM Identity Center (SSO) for admin access

2. **Domain and SSL**
   - Register or delegate domain to Route 53
   - Request ACM certificates for `app.metadata-tool.example.com` and `api.metadata-tool.example.com`
   - Validate certificates via DNS

3. **Tooling**
   - Install Terraform 1.5.7+
   - Install AWS CLI v2
   - Configure AWS credentials

4. **Secrets**
   - Generate production JWT secret: `openssl rand -base64 48`
   - Generate settings encryption key: `openssl rand -base64 48`
   - Obtain Entra ID production credentials
   - Obtain Graph API production credentials
   - Confirm Anthropic/OpenAI API keys

### 10.2 Migration Phases

**Phase 1: Infrastructure Foundation (Week 1)**

| Step | Action | Verification |
|------|--------|--------------|
| 1.1 | Create S3 bucket for Terraform state | `terraform init` succeeds |
| 1.2 | Deploy network module | VPC visible in console, NAT Gateways active |
| 1.3 | Deploy security module | Security groups, KMS key, IAM roles created |
| 1.4 | Create secrets in Secrets Manager | Secrets accessible via CLI |

**Phase 2: Database (Week 1-2)**

| Step | Action | Verification |
|------|--------|--------------|
| 2.1 | Deploy RDS module | Instance running, Multi-AZ confirmed |
| 2.2 | Connect from bastion/Cloud9 | `psql` connection works |
| 2.3 | Run initial migrations | All 42 migrations apply |
| 2.4 | Seed lookup data | Roles, statuses present |

**Phase 3: Application (Week 2)**

| Step | Action | Verification |
|------|--------|--------------|
| 3.1 | Build Docker image locally | Image builds, runs locally |
| 3.2 | Push to ECR | Image visible in ECR |
| 3.3 | Deploy compute module | ECS tasks running, health check passing |
| 3.4 | Configure ALB | HTTPS endpoint responds |
| 3.5 | Test API endpoints | `/api/v1/health` returns 200 |

**Phase 4: Frontend (Week 2)**

| Step | Action | Verification |
|------|--------|--------------|
| 4.1 | Deploy CDN module | CloudFront distribution active |
| 4.2 | Build frontend with production API URL | `npm run build` succeeds |
| 4.3 | Upload to S3 | Files visible in bucket |
| 4.4 | Test frontend | SPA loads, login works |

**Phase 5: Observability (Week 3)**

| Step | Action | Verification |
|------|--------|--------------|
| 5.1 | Deploy monitoring module | Log groups created |
| 5.2 | Verify logs flowing | Logs visible in CloudWatch |
| 5.3 | Configure alarms | Alarms in OK state |
| 5.4 | Test alerting | Test alarm triggers email |

**Phase 6: CI/CD (Week 3)**

| Step | Action | Verification |
|------|--------|--------------|
| 6.1 | Configure GitHub OIDC provider | Role assumable from Actions |
| 6.2 | Add backend workflow | Push triggers build, deploy succeeds |
| 6.3 | Add frontend workflow | Push triggers build, S3 sync, invalidation |
| 6.4 | Document rollback procedure | Tested, documented |

**Phase 7: Go-Live (Week 4)**

| Step | Action | Verification |
|------|--------|--------------|
| 7.1 | Final security review | WAF rules active, all secrets rotated |
| 7.2 | Performance testing | 500 concurrent user load test passes |
| 7.3 | DNS cutover | Update DNS to point to AWS resources |
| 7.4 | Monitor for 24-48 hours | No errors, latency within SLA |

### 10.3 Data Migration

**For fresh deployment (no existing data):**
- Migrations create empty tables
- Seed data applied via migrations
- Users onboard fresh

**For migration from existing system:**
1. Export data from source system during maintenance window
2. Transform to match schema (if needed)
3. Import using `pg_restore` or custom scripts
4. Verify row counts and data integrity
5. Update sequences to continue from last ID

### 10.4 DNS Cutover Strategy

**Option A: Instant Cutover (simple)**
1. Lower TTL to 60 seconds 24-48 hours before
2. Update DNS records to point to AWS
3. Monitor for issues
4. Raise TTL back to normal after 24 hours

**Option B: Weighted Routing (gradual)**
1. Create weighted record sets
2. Start with 10% traffic to AWS
3. Monitor errors and latency
4. Increase to 50%, then 100%
5. Remove old backend

**Recommended:** Option A for initial SaaS launch (no existing users to migrate gradually).

### 10.5 Rollback Plan

**Application rollback:**
1. Revert to previous ECR image tag
2. Force new ECS deployment
3. Verify health checks pass

**Database rollback:**
- Point-in-time recovery to before migration
- Requires application code compatible with previous schema

**Full environment rollback:**
- Terraform state allows `terraform destroy`
- DNS reverts to previous configuration
- RDS snapshots provide data recovery

---

## Appendix A: Architecture Decision Records

### ADR-AWS-001: ECS Fargate over EC2

**Status:** Accepted

**Context:** Need to choose compute platform for the Rust backend.

**Decision:** Use ECS Fargate.

**Rationale:**
- No server management reduces operational burden
- Security patching is automatic
- Pay-per-use aligns with early-stage SaaS
- Simpler IAM model (task roles, not instance profiles)

**Consequences:**
- ~30% cost premium over EC2
- Cannot customise host OS
- No GPU support (not needed for this workload)

### ADR-AWS-002: PgCat Sidecar over Centralised Pool

**Status:** Accepted

**Context:** Need connection pooling for PostgreSQL.

**Decision:** Deploy PgCat as an ECS sidecar container alongside each API task.

**Rationale:**
- No cross-AZ latency for pooling
- Failure isolation (one PgCat failure affects one task)
- Simpler networking (localhost communication)
- Scales automatically with application

**Consequences:**
- Slightly higher memory overhead per task
- Configuration must be consistent across tasks

### ADR-AWS-003: Multi-AZ from Day One

**Status:** Accepted

**Context:** Financial institution customers expect high availability.

**Decision:** Deploy across 2 AZs with Multi-AZ RDS from initial launch.

**Rationale:**
- Automatic failover for RDS
- ECS tasks distributed across AZs
- Demonstrates enterprise readiness

**Consequences:**
- 2x NAT Gateway cost (~$65/month)
- More complex networking
- Worth it for customer confidence

---

## Appendix B: Checklist

### Pre-Deployment Checklist

- [ ] AWS account created and configured
- [ ] Domain delegated to Route 53
- [ ] ACM certificates issued and validated
- [ ] Terraform state bucket created
- [ ] All secrets generated and stored in Secrets Manager
- [ ] Entra ID application registered for production
- [ ] Graph API permissions granted
- [ ] AI API keys tested and valid

### Post-Deployment Checklist

- [ ] Health endpoint returning 200
- [ ] Login flow working end-to-end
- [ ] Database connectivity confirmed
- [ ] AI enrichment working
- [ ] Email notifications working (if configured)
- [ ] CloudWatch logs flowing
- [ ] All alarms in OK state
- [ ] WAF rules in Block mode
- [ ] SSL certificate valid and auto-renewing
- [ ] Backup verified (test restore from snapshot)

---

## Appendix C: Glossary

| Term | Definition |
|------|------------|
| AZ | Availability Zone - isolated data centre within a region |
| ALB | Application Load Balancer - Layer 7 load balancer |
| ACM | AWS Certificate Manager - managed SSL/TLS certificates |
| CDN | Content Delivery Network - edge caching for static content |
| CMK | Customer Managed Key - KMS key you control |
| ECR | Elastic Container Registry - Docker image storage |
| ECS | Elastic Container Service - container orchestration |
| gp3 | General Purpose SSD v3 - current-gen EBS storage type |
| Multi-AZ | Deployment across multiple availability zones |
| NAT | Network Address Translation - allows private subnets to reach internet |
| OAC | Origin Access Control - CloudFront to S3 authentication |
| PgCat | PostgreSQL connection pooler written in Rust |
| RDS | Relational Database Service - managed database |
| WAF | Web Application Firewall - application-layer protection |
