# AWS MVP Deployment Plan

**Status**: Proposed
**Author**: Architecture Review
**Date**: 2026-03-22
**Target Environment**: Demo / Proof of Concept
**Target Capacity**: 5 Concurrent Users
**Budget Target**: ~$72-85/month (production-like architecture at minimal scale)
**Domain**: `metadata.hjpdebeer.com` (DNS managed by Cloudflare)

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Component Sizing](#2-component-sizing)
3. [Security (Minimum Viable)](#3-security-minimum-viable)
4. [Cost Estimate](#4-cost-estimate)
5. [Terraform Structure](#5-terraform-structure)
6. [Implementation Steps](#6-implementation-steps)
7. [DNS and HTTPS](#7-dns-and-https)
8. [Database Migrations](#8-database-migrations)
9. [Environment Variables and Secrets](#9-environment-variables-and-secrets)

---

## 1. Architecture Overview

### What This Plan Is

A production-like AWS deployment at minimal scale for demonstrating the Metadata Management Tool to stakeholders. Uses Cloudflare (free tier) for DNS, CDN, TLS, WAF, and DDoS protection — replacing AWS CloudFront and Route 53. The path to scaling is changing instance sizes and AZ count, not re-architecting.

### Why Cloudflare Instead of CloudFront + Route 53

Hendrik already manages `hjpdebeer.com` on Cloudflare. Using Cloudflare as the edge layer provides:

| Capability | CloudFront + Route 53 | Cloudflare (free) |
|------------|----------------------|-------------------|
| CDN | ~$1/month | Free |
| TLS 1.3 | Included | Free |
| Security headers | Response Headers Policy | Transform Rules |
| WAF | ~$10/month (was deferred) | Free (included now) |
| DDoS protection | AWS Shield Standard | Free (superior) |
| DNS hosting | $0.50/month | Free |
| Bot protection | Not included | Free (basic) |

**Total savings**: ~$12/month, plus WAF and DDoS from day one instead of deferred.

### MVP vs Production — What's Different

| Aspect | MVP | Production |
|--------|-----|------------|
| Availability zones | Single AZ | Multi-AZ |
| ECS task count | 1 | 2-6 (auto-scaling) |
| RDS Multi-AZ | No | Yes |
| NAT Gateway | 1 (single AZ) | 2 (one per AZ) |
| PgCat connection pooler | Yes (sidecar) | Yes (sidecar) |
| Cloudflare WAF | Free rules | Free + paid rules |
| VPC Flow Logs | No | Yes |
| CloudTrail | No | Yes |
| Domain | `metadata.hjpdebeer.com` | Custom product domain |

### Architecture Diagram

```
                              Internet
                                  |
                         +--------v--------+
                         |   Cloudflare    |
                         | (DNS, CDN, TLS, |
                         |  WAF, DDoS)     |
                         +---+--------+----+
                             |        |
                  frontend   |        |  api
                  (cached)   |        |  (proxied)
                             |        |
                   +---------v--+  +--v-----------+
                   |  S3 Bucket |  |     ALB      |
                   | (React SPA)|  | (HTTPS:443)  |
                   | (Website   |  |  + ACM cert  |
                   |  Endpoint) |  +------+-------+
                   +------------+         |
                                   +------v-------+
                                   | ECS Fargate  |
                                   | (Rust API +  |
                                   |  PgCat)      |
                                   | Private      |
                                   | Subnet       |
                                   +------+-------+
                                          |
                                   +------v-------+
                                   |  RDS Postgres|
                                   |  Single-AZ   |
                                   |  db.t4g.micro|
                                   +--------------+

VPC: 10.0.0.0/16, Single AZ (eu-west-1a)
Public Subnet:   10.0.1.0/24  (ALB, NAT Gateway)
Private Subnet:  10.0.11.0/24 (ECS Fargate tasks)
Database Subnet: 10.0.21.0/24 (RDS)

Cloudflare DNS:
  metadata.hjpdebeer.com      → CNAME → ALB DNS name (proxied, orange cloud)
  frontend.metadata.hjpdebeer.com → CNAME → S3 website endpoint (proxied)
  OR: single domain with Cloudflare Page Rules to split /api/* vs /*
```

### Network Flow

1. **Frontend**: Internet → Cloudflare (CDN + TLS + WAF) → S3 website endpoint (React SPA)
2. **API**: Internet → Cloudflare (proxy + TLS + WAF) → ALB (HTTPS, ACM cert) → ECS Fargate (private subnet) → PgCat sidecar → RDS
3. **Outbound**: ECS → NAT Gateway (public subnet) → Internet (for Entra SSO, Claude API, Graph API)

### Cloudflare ↔ ALB TLS (Full Strict Mode)

Cloudflare terminates the client's TLS connection, then opens a new TLS connection to the ALB.
This is "Full (Strict)" mode — Cloudflare validates the ALB's ACM certificate:

```
Client ──TLS 1.3──▸ Cloudflare ──TLS 1.2/1.3──▸ ALB (ACM cert) ──▸ ECS (HTTP:8080)
```

The ALB still needs an ACM certificate for `metadata.hjpdebeer.com`. ACM validates domain
ownership via a DNS CNAME record — created at Cloudflare (not Route 53).

### PgCat Sidecar

PgCat runs as a sidecar container within the same ECS task as the Rust backend. This is included from day one to:
- Manage database connection pooling (transaction mode)
- Decouple the application from direct database connections
- Prepare for read replicas when scaling horizontally
- Match the production architecture exactly

Configuration for MVP:
- **Client connections** (app → PgCat): up to 100
- **Server connections** (PgCat → RDS): 5 (db.t4g.micro supports ~85 total)
- **Mode**: Transaction pooling

### What's Deferred to Production

- Multi-AZ redundancy (second AZ, second NAT Gateway)
- Auto-scaling (fixed at 1 task)
- Cloudflare paid WAF rules (free rules active from day one)
- VPC Flow Logs and CloudTrail audit
- Automated secret rotation
- RDS Performance Insights extended retention
- Read replicas (PgCat ready to route when added)

---

## 2. Component Sizing

### MVP vs Production Comparison

| Component | MVP (5 users) | Production (500 users) | Notes |
|-----------|---------------|------------------------|-------|
| **ECS Task Count** | 1 | 2-6 | Fixed vs auto-scaling |
| **ECS CPU** | 0.25 vCPU | 1 vCPU | Rust is efficient |
| **ECS Memory** | 512 MB | 512 MB | Same (handles Excel uploads) |
| **PgCat Sidecar** | 0.25 vCPU / 256 MB | 0.25 vCPU / 256 MB | Same per task |
| **RDS Instance** | db.t4g.micro | db.t4g.medium | 2 vCPU / 1 GB vs 2 vCPU / 4 GB |
| **RDS Multi-AZ** | No | Yes | No automatic failover |
| **RDS Storage** | 20 GB gp2 | 100 GB gp3 | Minimum viable |
| **ALB** | 1 | 1 | Same |
| **NAT Gateway** | 1 | 2 | One per AZ in production |
| **Cloudflare** | Free tier | Free or Pro ($20/mo) | DNS, CDN, TLS, WAF, DDoS |
| **S3 (frontend)** | 1 bucket (website) | 1 bucket (website) | Same storage |
| **Secrets Manager** | 5 secrets | 8 secrets | Fewer secrets needed |
| **KMS** | AWS-managed | Customer-managed | Simpler |

### Why These Sizes Work

**ECS 0.25 vCPU / 512 MB** (backend):
- Rust/Axum can handle 100+ requests/second on 0.25 vCPU
- 5 users generate maybe 1 request/second
- 512 MB handles the largest Excel upload scenario

**PgCat 0.25 vCPU / 256 MB** (sidecar):
- PgCat is Rust-native, extremely lightweight
- Handles connection pooling with near-zero overhead
- 256 MB is more than sufficient for connection state management

**RDS db.t4g.micro**:
- 2 vCPU, 1 GB RAM
- Baseline 10% CPU (burstable)
- Free tier eligible for 12 months (750 hours/month)
- Supports 85 max connections (PgCat holds 5 server connections)

---

## 3. Security (Minimum Viable)

### Required Even for Demo

| Security Control | Implementation | Rationale |
|------------------|----------------|-----------|
| HTTPS everywhere | ACM certificate on ALB | No plaintext credentials |
| Secrets Manager | API keys, JWT secret, DB password | Never in code or env vars |
| Security groups | Restrictive ingress rules | Principle of least privilege |
| Non-root container | Dockerfile USER directive | Container hardening |
| RDS encryption | AWS-managed KMS | Data at rest encryption |
| Private DB subnet | No public IP for RDS | Database not internet-accessible |

### Deferred to Production

| Security Control | Reason to Defer |
|------------------|-----------------|
| WAF | Adds ~$10/month, overkill for demo |
| VPC Flow Logs | Adds cost, not needed for demo |
| CloudTrail | Can enable later if needed |
| Customer-managed KMS | AWS-managed sufficient for demo |
| Secret rotation | Manual rotation acceptable for demo |

### Security Group Rules

**ALB Security Group**:
```
Inbound:
  - 443 TCP from 0.0.0.0/0 (HTTPS from internet)
Outbound:
  - 8080 TCP to ECS SG (backend traffic)
```

**ECS Tasks Security Group**:
```
Inbound:
  - 8080 TCP from ALB SG (API requests via backend container)
Outbound:
  - 6432 TCP to localhost (PgCat sidecar, same task)
  - 443 TCP to 0.0.0.0/0 (AI APIs, Entra, Graph — via NAT Gateway)
```

Note: The PgCat sidecar shares the same network namespace as the backend
container within the ECS task, so backend connects to PgCat on localhost:6432.
PgCat then connects to RDS on 5432.

**PgCat (sidecar, same SG as ECS task)**:
```
Outbound:
  - 5432 TCP to RDS SG (database connections)
```

**RDS Security Group**:
```
Inbound:
  - 5432 TCP from ECS SG (connections from PgCat)
Outbound:
  - None (RDS does not initiate connections)
```

---

## 4. Cost Estimate

### Monthly Breakdown

| Resource | Specification | Monthly Cost | Notes |
|----------|--------------|--------------|-------|
| **NAT Gateway** | 1 gateway, ~5 GB processed | ~$35 | $32 fixed + data |
| **ALB** | Fixed hourly + LCU | ~$16 | HTTPS termination for API |
| **RDS db.t4g.micro** | Single-AZ, 20 GB | ~$13 | Or $0 if free tier |
| **ECS Fargate (backend)** | 0.25 vCPU, 512 MB, 730 hrs | ~$10 | Rust API container |
| **ECS Fargate (PgCat)** | 0.25 vCPU, 256 MB, 730 hrs | ~$7 | Sidecar container |
| **Secrets Manager** | 5 secrets | ~$2 | $0.40/secret/month |
| **RDS Storage** | 20 GB gp2 | ~$2.30 | Included in free tier |
| **S3** | Frontend static files | ~$0.05 | < 100 MB |
| **CloudWatch Logs** | ~1 GB/month | ~$0.50 | Basic logging |
| **Data Transfer** | ~10 GB outbound | ~$0.90 | Minimal for demo |
| **ECR** | ~200 MB images | ~$0.02 | Negligible |
| **Cloudflare** | Free tier | $0 | DNS, CDN, TLS, WAF, DDoS |

### Total Monthly Cost

| Scenario | Cost |
|----------|------|
| **With RDS free tier** | ~$72/month |
| **Without free tier** | ~$85/month |

### Free Tier Eligibility

New AWS accounts get 12 months of:
- **RDS**: 750 hours/month of db.t4g.micro (covers 1 instance)
- **RDS Storage**: 20 GB
- **S3**: 5 GB
- **Data Transfer**: 100 GB outbound

After free tier expires, expect ~$85/month.

### Biggest Cost Drivers

1. **NAT Gateway** (~$35) — Largest single cost, but essential for private subnet security
2. **ALB** (~$16) — Fixed cost for HTTPS termination + health checks
3. **RDS** (~$13) — Smallest instance that runs PostgreSQL 17
4. **ECS Fargate** (~$17 combined) — Backend + PgCat sidecar

### What Cloudflare Provides for Free

- DNS hosting for `metadata.hjpdebeer.com`
- Global CDN caching for frontend static assets
- TLS 1.3 termination (client ↔ Cloudflare)
- Basic WAF (OWASP rules, bot protection)
- DDoS mitigation (L3/L4/L7)
- Security headers via Transform Rules
- Analytics and traffic insights

### Cost Comparison

| Environment | Monthly Cost | Notes |
|-------------|--------------|-------|
| MVP (this plan) | ~$72-85 | Production-like architecture at minimum scale |
| Production (500 users) | ~$310-340 | Multi-AZ, larger instances, monitoring |

---

## 5. Terraform Structure

### Directory Layout

Same structure as production, but with a `demo` environment:

```
terraform/
├── environments/
│   ├── demo/
│   │   ├── main.tf           # Module composition
│   │   ├── variables.tf      # Variable declarations
│   │   ├── terraform.tfvars  # Demo-specific values
│   │   ├── outputs.tf        # Useful outputs
│   │   └── backend.tf        # S3 state backend
│   └── production/
│       └── ...               # (from existing plan)
├── modules/
│   ├── network/              # VPC, subnets, NAT Gateway, routing
│   ├── compute/              # ECR, ECS (backend + PgCat sidecar), ALB
│   ├── database/             # RDS PostgreSQL
│   ├── security/             # SGs, IAM, Secrets Manager
│   ├── frontend/             # S3 bucket (website hosting)
│   ├── cloudflare/           # DNS records, TLS mode, WAF rules, security headers
│   └── pgcat/                # PgCat config generation
└── README.md
```

### Key Variable Differences

**environments/demo/terraform.tfvars**:
```hcl
# Environment
environment = "demo"
aws_region  = "eu-west-1"

# Network - Single AZ, with NAT Gateway for private subnets
availability_zones = ["eu-west-1a"]
enable_nat_gateway = true

# Compute - Minimal but production-like
ecs_cpu           = 256      # 0.25 vCPU (backend)
ecs_memory        = 512      # 512 MB (backend)
pgcat_cpu         = 256      # 0.25 vCPU (sidecar)
pgcat_memory      = 256      # 256 MB (sidecar)
ecs_desired_count = 1
ecs_min_capacity  = 1
ecs_max_capacity  = 1

# PgCat connection pooling
enable_pgcat            = true
pgcat_pool_size         = 5       # Server connections to RDS
pgcat_max_client_conns  = 100     # Client connections from app

# Database - Smallest viable
rds_instance_class      = "db.t4g.micro"
rds_allocated_storage   = 20
rds_multi_az            = false
rds_deletion_protection = false  # Easy teardown for demo

# Security - Simplified (Cloudflare WAF is free and always on)
enable_flow_logs   = false
enable_cloudtrail  = false

# Cloudflare (DNS, CDN, TLS, WAF)
cloudflare_zone    = "hjpdebeer.com"
domain             = "metadata.hjpdebeer.com"
cloudflare_tls_mode = "full_strict"
```

**environments/production/terraform.tfvars** (for comparison):
```hcl
environment = "production"
aws_region  = "eu-west-1"

availability_zones = ["eu-west-1a", "eu-west-1b"]
enable_nat_gateway = true

ecs_cpu           = 1024     # 1 vCPU
ecs_memory        = 512
ecs_desired_count = 2
ecs_min_capacity  = 2
ecs_max_capacity  = 6

rds_instance_class     = "db.t4g.medium"
rds_allocated_storage  = 100
rds_multi_az           = true
rds_deletion_protection = true

enable_waf         = true
enable_flow_logs   = true
enable_cloudtrail  = true
enable_cloudfront  = true
```

### Module Conditional Logic

Modules handle MVP vs production via variables:

```hcl
# modules/network/main.tf — NAT Gateway (1 for demo, 1 per AZ for production)
resource "aws_nat_gateway" "main" {
  count = var.enable_nat_gateway ? length(var.availability_zones) : 0
  # ...
}

# modules/compute/main.tf — ECS always in private subnet (NAT for outbound)
resource "aws_ecs_service" "api" {
  # ...
  network_configuration {
    subnets          = var.private_subnet_ids
    assign_public_ip = false
    security_groups  = [var.ecs_security_group_id]
  }
}

# modules/compute/main.tf — PgCat as sidecar container
resource "aws_ecs_task_definition" "api" {
  container_definitions = jsonencode([
    {
      name  = "api"
      image = "${var.ecr_repo_url}:latest"
      portMappings = [{ containerPort = 8080 }]
      environment = [
        # App connects to PgCat on localhost:6432 instead of RDS directly
        { name = "DATABASE_URL", value = "" }  # Injected from Secrets Manager
      ]
    },
    var.enable_pgcat ? {
      name  = "pgcat"
      image = "ghcr.io/postgresml/pgcat:latest"
      portMappings = [{ containerPort = 6432 }]
      # PgCat config mounted from S3 or inline
    } : null
  ])
}

# modules/cloudflare/main.tf — DNS records pointing to AWS resources
resource "cloudflare_record" "api" {
  zone_id = data.cloudflare_zone.main.id
  name    = var.domain                          # metadata.hjpdebeer.com
  content = aws_lb.api.dns_name                 # ALB DNS name
  type    = "CNAME"
  proxied = true                                # Orange cloud = CDN + WAF + DDoS
}

resource "cloudflare_record" "frontend" {
  zone_id = data.cloudflare_zone.main.id
  name    = "app.${var.domain}"                 # app.metadata.hjpdebeer.com
  content = aws_s3_bucket_website.frontend.website_endpoint
  type    = "CNAME"
  proxied = true
}

# TLS mode: Full (Strict) — validates ALB's ACM certificate
resource "cloudflare_zone_settings_override" "tls" {
  zone_id = data.cloudflare_zone.main.id
  settings {
    ssl              = "strict"
    min_tls_version  = "1.2"
    tls_1_3          = "on"
  }
}
```

---

## 6. Implementation Steps

### Prerequisites (Do Once)

#### Step 0.1: AWS Account Setup

```bash
# 1. Create AWS account at https://aws.amazon.com
# 2. Enable MFA on root account
# 3. Create IAM user for Terraform with AdministratorAccess (or scoped policy)
# 4. Configure AWS CLI
aws configure
# Enter: Access Key ID, Secret Access Key, eu-west-1, json
```

#### Step 0.2: Install Tools

```bash
# macOS (you have these via Homebrew)
brew install terraform awscli

# Verify
terraform --version  # Should be 1.5.7+
aws --version        # Should be 2.x
```

#### Step 0.3: Create Terraform State Bucket

```bash
# Create S3 bucket for Terraform state (globally unique name required)
aws s3 mb s3://metadata-tool-terraform-state-YOURNAME --region eu-west-1

# Enable versioning
aws s3api put-bucket-versioning \
  --bucket metadata-tool-terraform-state-YOURNAME \
  --versioning-configuration Status=Enabled
```

### Implementation Order

Execute in this order due to dependencies:

#### Step 1: Terraform Backend Configuration

**File**: `terraform/environments/demo/backend.tf`

```hcl
terraform {
  required_version = ">= 1.5.0"

  backend "s3" {
    bucket         = "metadata-tool-terraform-state-YOURNAME"
    key            = "demo/terraform.tfstate"
    region         = "eu-west-1"
    encrypt        = true
  }

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "metadata-tool"
      Environment = var.environment
      ManagedBy   = "terraform"
    }
  }
}
```

#### Step 2: Network Module

Creates VPC, subnets, internet gateway (no NAT for MVP).

```bash
cd terraform/environments/demo
terraform init
terraform plan -target=module.network
terraform apply -target=module.network
```

#### Step 3: Security Module

Creates security groups, IAM roles, and Secrets Manager entries.

```bash
terraform plan -target=module.security
terraform apply -target=module.security
```

**After apply, populate secrets**:
```bash
# Generate secrets
JWT_SECRET=$(openssl rand -base64 48)
SETTINGS_KEY=$(openssl rand -base64 48)

# Store in Secrets Manager
aws secretsmanager put-secret-value \
  --secret-id metadata-tool/demo/jwt-secret \
  --secret-string "$JWT_SECRET"

aws secretsmanager put-secret-value \
  --secret-id metadata-tool/demo/encryption-key \
  --secret-string "$SETTINGS_KEY"

# Store API keys (replace with your actual keys)
aws secretsmanager put-secret-value \
  --secret-id metadata-tool/demo/anthropic-api-key \
  --secret-string "sk-ant-..."

aws secretsmanager put-secret-value \
  --secret-id metadata-tool/demo/openai-api-key \
  --secret-string "sk-..."
```

#### Step 4: Database Module

Creates RDS PostgreSQL instance.

```bash
terraform plan -target=module.database
terraform apply -target=module.database

# Note the RDS endpoint from output
terraform output rds_endpoint
```

**Verify connectivity** (from a Cloud9 instance or your machine with VPN):
```bash
psql -h <rds-endpoint> -U postgres -d postgres
```

#### Step 5: Build and Push Docker Image

**Create Dockerfile** at `backend/Dockerfile`:

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

# Copy migrations for auto-run on startup
COPY backend/migrations /app/migrations

# Non-root user
RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 8080

CMD ["/app/metadata-tool"]
```

**Build and push**:

```bash
cd /Users/hjpdebeer/Projects/metadata-tool

# Get ECR repository URL from Terraform output
ECR_REPO=$(terraform -chdir=terraform/environments/demo output -raw ecr_repository_url)

# Login to ECR
aws ecr get-login-password --region eu-west-1 | docker login --username AWS --password-stdin $ECR_REPO

# Build for Linux (if on macOS ARM)
docker buildx build --platform linux/amd64 \
  -t $ECR_REPO:latest \
  -f backend/Dockerfile \
  --push .
```

#### Step 6: Compute Module (ECS + ALB)

```bash
terraform plan -target=module.compute
terraform apply -target=module.compute

# Get ALB DNS name
terraform output alb_dns_name
```

**Verify backend is running**:
```bash
curl https://$(terraform output -raw alb_dns_name)/api/v1/health
# Should return: {"status":"healthy"}
```

#### Step 7: Cloudflare DNS + ACM Certificate

```bash
terraform plan -target=module.cloudflare
terraform apply -target=module.cloudflare

# This creates:
# - ACM certificate request for metadata.hjpdebeer.com
# - CNAME validation record at Cloudflare (grey cloud)
# - Wait for ACM validation (may take 5-10 minutes)
# - CNAME record pointing to ALB (orange cloud, proxied)
# - TLS and security header settings
```

#### Step 8: Frontend Deployment

```bash
# Build frontend pointing to the Cloudflare domain
cd /Users/hjpdebeer/Projects/metadata-tool/frontend
VITE_API_URL=https://metadata.hjpdebeer.com npm run build

# Upload to S3
S3_BUCKET=$(terraform -chdir=../terraform/environments/demo output -raw frontend_bucket_name)
aws s3 sync dist/ s3://$S3_BUCKET/ --delete
```

#### Step 9: Verify Full Stack

```bash
# Verify security headers
curl -sI https://metadata.hjpdebeer.com/api/v1/health

# Open in browser
open "https://metadata.hjpdebeer.com"

# Test login with dev mode credentials (email: admin@example.com, password: metadata123)
# Or configure Entra SSO credentials in Secrets Manager
```

### Teardown

When demo is complete:

```bash
cd terraform/environments/demo
terraform destroy
```

This removes all resources. Cost stops immediately.

---

## 7. DNS, HTTPS, and Security (Cloudflare)

DNS is managed at Cloudflare — no Route 53 needed. Cloudflare provides CDN, TLS, WAF, and DDoS for free.

### DNS Records (Cloudflare)

| Record | Type | Content | Proxy |
|--------|------|---------|-------|
| `metadata.hjpdebeer.com` | CNAME | ALB DNS name | Proxied (orange cloud) |
| `_acm-validation...` | CNAME | ACM validation value | DNS only (grey cloud) |

The proxied (orange cloud) record means traffic flows through Cloudflare's CDN/WAF/DDoS network.

### TLS Configuration — Two Hops

```
Client ──TLS 1.3──▸ Cloudflare ──TLS 1.2/1.3──▸ ALB (ACM cert) ──HTTP──▸ ECS (port 8080)
```

1. **Client → Cloudflare**: TLS 1.3 (Cloudflare manages the certificate automatically)
2. **Cloudflare → ALB**: TLS with ACM certificate (Full Strict mode validates it)
3. **ALB → ECS**: Plain HTTP on port 8080 (internal VPC, no internet exposure)

### ACM Certificate (for ALB)

Even though Cloudflare terminates the client TLS, the ALB still needs an ACM certificate so
Cloudflare can connect to it over HTTPS (Full Strict mode).

```hcl
# Request ACM certificate for the domain
resource "aws_acm_certificate" "main" {
  domain_name       = "metadata.hjpdebeer.com"
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }
}

# Validation CNAME — created at Cloudflare (not Route 53)
resource "cloudflare_record" "acm_validation" {
  for_each = {
    for dvo in aws_acm_certificate.main.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
    }
  }

  zone_id = data.cloudflare_zone.main.id
  name    = each.value.name
  content = each.value.record
  type    = "CNAME"
  proxied = false    # Must be DNS-only (grey cloud) for ACM validation
}

# Wait for ACM to validate
resource "aws_acm_certificate_validation" "main" {
  certificate_arn         = aws_acm_certificate.main.arn
  validation_record_fqdns = [for record in cloudflare_record.acm_validation : record.hostname]
}

# ALB listener with TLS 1.3
resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.api.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = aws_acm_certificate_validation.main.certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }
}
```

### Cloudflare Settings (Terraform)

```hcl
# TLS mode: Full (Strict) — Cloudflare validates ALB's ACM cert
resource "cloudflare_zone_settings_override" "main" {
  zone_id = data.cloudflare_zone.main.id
  settings {
    ssl                      = "strict"
    min_tls_version          = "1.2"
    tls_1_3                  = "on"
    always_use_https         = "on"
    automatic_https_rewrites = "on"
  }
}

# CNAME record pointing to ALB (proxied = CDN + WAF + DDoS)
resource "cloudflare_record" "api" {
  zone_id = data.cloudflare_zone.main.id
  name    = "metadata"
  content = aws_lb.api.dns_name
  type    = "CNAME"
  proxied = true
}
```

### Security Headers — Two Layers

**Layer 1: Cloudflare Transform Rules (frontend + API)**

Cloudflare adds security headers to all responses passing through the proxy:

```hcl
resource "cloudflare_ruleset" "security_headers" {
  zone_id = data.cloudflare_zone.main.id
  name    = "Security Headers"
  kind    = "zone"
  phase   = "http_response_headers_transform"

  rules {
    action = "rewrite"
    action_parameters {
      headers {
        name      = "Strict-Transport-Security"
        operation = "set"
        value     = "max-age=31536000; includeSubDomains; preload"
      }
      headers {
        name      = "X-Content-Type-Options"
        operation = "set"
        value     = "nosniff"
      }
      headers {
        name      = "X-Frame-Options"
        operation = "set"
        value     = "DENY"
      }
      headers {
        name      = "Referrer-Policy"
        operation = "set"
        value     = "strict-origin-when-cross-origin"
      }
      headers {
        name      = "Permissions-Policy"
        operation = "set"
        value     = "camera=(), microphone=(), geolocation=(), payment=()"
      }
    }
    expression  = "true"
    description = "Add security headers to all responses"
    enabled     = true
  }
}
```

**Layer 2: Axum Security Headers Middleware (backend API)**

Already implemented in `backend/src/main.rs`. Applied to all API responses as defence-in-depth:

| Header | Value | Purpose |
|--------|-------|---------|
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains; preload` | Enforce HTTPS |
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-Frame-Options` | `DENY` | Clickjacking protection |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Control referrer leakage |
| `Permissions-Policy` | `camera=(), microphone=(), geolocation=(), payment=()` | Restrict browser APIs |
| `Cache-Control` | `no-store` | Prevent caching sensitive API data |

Both layers are active — Cloudflare headers cover the frontend, Axum headers cover the API.
If Cloudflare is ever bypassed (e.g., direct ALB access), the Axum headers still protect API responses.

### Terraform Provider Configuration

The Cloudflare Terraform provider requires an API token:

```hcl
# In terraform/environments/demo/backend.tf
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
  }
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token  # Scoped to zone:edit for hjpdebeer.com
}
```

The Cloudflare API token (you already have one in `~/.zshrc`) needs `Zone:DNS:Edit` and
`Zone:Zone Settings:Edit` permissions for `hjpdebeer.com`.

---

## 8. Database Migrations

### How It Works

The Rust backend uses SQLx with embedded migrations. On startup, it automatically runs any pending migrations:

```rust
// In db.rs (already implemented)
sqlx::migrate!("./migrations").run(&pool).await?;
```

### First Deployment

1. Deploy RDS (Step 4)
2. Deploy ECS with the backend image (Step 6)
3. The first ECS task startup runs all 30 migrations automatically

**No manual intervention required**.

### Verifying Migrations Ran

Check ECS task logs in CloudWatch:

```bash
aws logs tail /ecs/metadata-tool-api --follow
```

Look for:
```
Applied migration 001_extensions
Applied migration 002_users
...
Applied migration 030_xxx
```

### Manual Migration (If Needed)

If you need to run migrations manually (e.g., testing):

```bash
# From a machine with database access
cd /Users/hjpdebeer/Projects/metadata-tool
DATABASE_URL=postgres://postgres:PASSWORD@RDS_ENDPOINT:5432/metadata_tool cargo sqlx migrate run
```

### Rollback

SQLx migrations are forward-only by default. For rollback:
1. Create a new migration that undoes changes
2. Or restore from RDS snapshot

---

## 9. Environment Variables and Secrets

### Complete List

| Variable | Secret? | Source | Example Value |
|----------|---------|--------|---------------|
| `DATABASE_URL` | Yes | Secrets Manager | `postgres://app:xxx@rds:5432/metadata_tool` |
| `JWT_SECRET` | Yes | Secrets Manager | 48+ character random string |
| `SETTINGS_ENCRYPTION_KEY` | Yes | Secrets Manager | 48+ character random string |
| `ANTHROPIC_API_KEY` | Yes | Secrets Manager | `sk-ant-api03-...` |
| `OPENAI_API_KEY` | Yes | Secrets Manager | `sk-...` |
| `HOST` | No | Task Definition | `0.0.0.0` |
| `PORT` | No | Task Definition | `8080` |
| `RUST_LOG` | No | Task Definition | `metadata_tool=info,tower_http=info` |
| `FRONTEND_URL` | No | Task Definition | `https://metadata.hjpdebeer.com` |
| `AI_PRIMARY_PROVIDER` | No | Task Definition | `claude` |
| `ANTHROPIC_MODEL` | No | Task Definition | `claude-sonnet-4-6` |
| `OPENAI_MODEL` | No | Task Definition | `gpt-4o` |

### Secrets Manager Structure

Create these secrets before deploying ECS:

```bash
# Secret names (Terraform creates the empty secrets)
metadata-tool/demo/database-url
metadata-tool/demo/jwt-secret
metadata-tool/demo/encryption-key
metadata-tool/demo/anthropic-api-key
metadata-tool/demo/openai-api-key
```

### ECS Task Definition (Backend + PgCat Sidecar)

The task contains two containers sharing a network namespace (localhost):

```json
{
  "containerDefinitions": [
    {
      "name": "api",
      "image": "ACCOUNT.dkr.ecr.eu-west-1.amazonaws.com/metadata-tool:latest",
      "essential": true,
      "portMappings": [{ "containerPort": 8080 }],
      "secrets": [
        {
          "name": "DATABASE_URL",
          "valueFrom": "arn:aws:secretsmanager:eu-west-1:ACCOUNT:secret:metadata-tool/demo/database-url"
        },
        {
          "name": "JWT_SECRET",
          "valueFrom": "arn:aws:secretsmanager:eu-west-1:ACCOUNT:secret:metadata-tool/demo/jwt-secret"
        },
        {
          "name": "SETTINGS_ENCRYPTION_KEY",
          "valueFrom": "arn:aws:secretsmanager:eu-west-1:ACCOUNT:secret:metadata-tool/demo/encryption-key"
        },
        {
          "name": "ANTHROPIC_API_KEY",
          "valueFrom": "arn:aws:secretsmanager:eu-west-1:ACCOUNT:secret:metadata-tool/demo/anthropic-api-key"
        },
        {
          "name": "OPENAI_API_KEY",
          "valueFrom": "arn:aws:secretsmanager:eu-west-1:ACCOUNT:secret:metadata-tool/demo/openai-api-key"
        }
      ],
      "environment": [
        { "name": "DATABASE_URL", "value": "postgres://app:PASSWORD@localhost:6432/metadata_tool" },
        { "name": "HOST", "value": "0.0.0.0" },
        { "name": "PORT", "value": "8080" },
        { "name": "RUST_LOG", "value": "metadata_tool=info,tower_http=info" },
        { "name": "FRONTEND_URL", "value": "https://metadata.hjpdebeer.com" },
        { "name": "AI_PRIMARY_PROVIDER", "value": "claude" },
        { "name": "ANTHROPIC_MODEL", "value": "claude-sonnet-4-6" },
        { "name": "OPENAI_MODEL", "value": "gpt-4o" }
      ],
      "dependsOn": [{ "containerName": "pgcat", "condition": "START" }]
    },
    {
      "name": "pgcat",
      "image": "ghcr.io/postgresml/pgcat:latest",
      "essential": true,
      "portMappings": [{ "containerPort": 6432 }],
      "environment": [
        { "name": "PGCAT_CONFIG", "value": "/etc/pgcat/pgcat.toml" }
      ],
      "healthCheck": {
        "command": ["CMD-SHELL", "pg_isready -h localhost -p 6432 || exit 1"],
        "interval": 10,
        "timeout": 5,
        "retries": 3
      }
    }
  ]
}
```

**Note**: The `DATABASE_URL` in the environment block points to `localhost:6432` (PgCat sidecar),
not directly to RDS. PgCat's config contains the actual RDS endpoint. The `DATABASE_URL` secret
in Secrets Manager holds the direct RDS connection string (used by PgCat's config and for
running migrations before PgCat is available).

### Terraform Secrets Module

```hcl
# Create secrets (empty, to be populated manually)
resource "aws_secretsmanager_secret" "database_url" {
  name = "metadata-tool/${var.environment}/database-url"
}

resource "aws_secretsmanager_secret" "jwt_secret" {
  name = "metadata-tool/${var.environment}/jwt-secret"
}

resource "aws_secretsmanager_secret" "encryption_key" {
  name = "metadata-tool/${var.environment}/encryption-key"
}

resource "aws_secretsmanager_secret" "anthropic_api_key" {
  name = "metadata-tool/${var.environment}/anthropic-api-key"
}

resource "aws_secretsmanager_secret" "openai_api_key" {
  name = "metadata-tool/${var.environment}/openai-api-key"
}

# Populate DATABASE_URL after RDS is created
resource "aws_secretsmanager_secret_version" "database_url" {
  secret_id     = aws_secretsmanager_secret.database_url.id
  secret_string = "postgres://${var.db_username}:${var.db_password}@${module.database.endpoint}:5432/${var.db_name}"
}
```

### Entra SSO (Optional for Demo)

For demo purposes, you can use **dev mode** authentication (email + password):

```
ENTRA_TENANT_ID=      # Leave empty to enable dev mode
```

Dev mode users are seeded by migration 004:
- admin@example.com / admin123 (ADMIN role)
- steward@example.com / steward123 (STEWARD role)
- analyst@example.com / analyst123 (ANALYST role)
- viewer@example.com / viewer123 (VIEWER role)

If you want Entra SSO for the demo, add these secrets:
```
metadata-tool/demo/entra-tenant-id
metadata-tool/demo/entra-client-id
metadata-tool/demo/entra-client-secret
```

And these environment variables:
```json
{ "name": "ENTRA_REDIRECT_URI", "value": "https://metadata.hjpdebeer.com/api/v1/auth/callback" }
```

---

## Quick Reference

### Useful Commands

```bash
# Check ECS task status
aws ecs describe-services --cluster metadata-tool-demo --services metadata-tool-api

# View task logs
aws logs tail /ecs/metadata-tool-api --follow

# Force new deployment (after image update)
aws ecs update-service --cluster metadata-tool-demo --service metadata-tool-api --force-new-deployment

# Get RDS connection info
terraform -chdir=terraform/environments/demo output rds_endpoint

# Estimate cost before applying
terraform -chdir=terraform/environments/demo plan

# Destroy everything
terraform -chdir=terraform/environments/demo destroy
```

### Expected Timeline

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Prerequisites | 1 hour | AWS account, tools |
| Network + Security | 30 min | Prerequisites |
| Database | 15 min | Network |
| Docker build + push | 20 min | Database |
| ECS + PgCat deployment | 20 min | Docker image, RDS endpoint |
| Cloudflare DNS + ACM cert | 15 min | ALB DNS name |
| Frontend (S3 upload) | 10 min | S3 bucket, domain |
| **Total** | **3-4 hours** | |

### Scaling to Production

When ready to go production:

1. Create `terraform/environments/production/terraform.tfvars` using production values
2. Run `terraform apply` in production environment
3. Same modules, different variables

No code changes required. The Terraform modules handle both configurations.
