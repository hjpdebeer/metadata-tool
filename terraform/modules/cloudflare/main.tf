# =============================================================================
# Cloudflare DNS, ACM Certificate Validation, TLS Settings, Security Headers
# =============================================================================

# ---------------------------------------------------------------------------
# Look up the Cloudflare zone
# ---------------------------------------------------------------------------

data "cloudflare_zone" "main" {
  name = var.cloudflare_zone
}

# ---------------------------------------------------------------------------
# ACM Certificate (for ALB — Cloudflare validates via DNS)
# ---------------------------------------------------------------------------

resource "aws_acm_certificate" "main" {
  domain_name       = var.domain
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }

  tags = {
    Name = "metadata-tool-${var.domain}"
  }
}

# Create the DNS validation CNAME at Cloudflare
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
  proxied = false # Must be DNS-only for ACM validation
}

# Wait for ACM certificate to be validated
resource "aws_acm_certificate_validation" "main" {
  certificate_arn         = aws_acm_certificate.main.arn
  validation_record_fqdns = [for record in cloudflare_record.acm_validation : record.hostname]
}

# ---------------------------------------------------------------------------
# DNS Record — point domain to ALB (proxied through Cloudflare CDN/WAF)
# ---------------------------------------------------------------------------

resource "cloudflare_record" "api" {
  zone_id = data.cloudflare_zone.main.id
  name    = split(".", var.domain)[0] # "metadata" from "metadata.hjpdebeer.com"
  content = var.alb_dns_name
  type    = "CNAME"
  proxied = true # Orange cloud = CDN + WAF + DDoS

  depends_on = [aws_acm_certificate_validation.main]
}

# ---------------------------------------------------------------------------
# TLS Settings — Full (Strict) mode, TLS 1.3, always HTTPS
# ---------------------------------------------------------------------------

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

# ---------------------------------------------------------------------------
# Security Headers
# ---------------------------------------------------------------------------
# Security headers are handled by the Axum middleware in the Rust backend
# (defence-in-depth). Cloudflare Transform Rules can be configured manually
# in the dashboard if additional edge-level headers are needed. The API token
# would need "Zone Rulesets Edit" permission to manage this via Terraform.
