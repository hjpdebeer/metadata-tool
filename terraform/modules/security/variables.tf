variable "environment" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "alb_ingress_cidr" {
  type    = string
  default = "0.0.0.0/0"
}

variable "ecs_port" {
  type    = number
  default = 8080
}

variable "db_port" {
  type    = number
  default = 5432
}

variable "db_password" {
  type      = string
  sensitive = true
}

variable "db_username" {
  type = string
}

variable "db_name" {
  type = string
}

variable "jwt_secret" {
  type      = string
  sensitive = true
}

variable "settings_encryption_key" {
  type      = string
  sensitive = true
}

variable "anthropic_api_key" {
  type      = string
  sensitive = true
}

variable "openai_api_key" {
  type      = string
  sensitive = true
  default   = ""
}
