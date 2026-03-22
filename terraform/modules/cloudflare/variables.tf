variable "domain" {
  description = "Full domain name (e.g., metadata.hjpdebeer.com)"
  type        = string
}

variable "cloudflare_zone" {
  description = "Root domain managed by Cloudflare (e.g., hjpdebeer.com)"
  type        = string
}

variable "alb_dns_name" {
  description = "ALB DNS name to point the domain to"
  type        = string
}
