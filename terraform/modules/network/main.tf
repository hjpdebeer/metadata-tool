# =============================================================================
# VPC, Subnets, Internet Gateway, NAT Gateway, Route Tables
# =============================================================================

resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_support   = true
  enable_dns_hostnames = true

  tags = {
    Name = "metadata-tool-${var.environment}"
  }
}

# Internet Gateway — required for public subnets
resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id

  tags = {
    Name = "metadata-tool-${var.environment}-igw"
  }
}

# ---------------------------------------------------------------------------
# Public Subnets (ALB, NAT Gateway)
# ---------------------------------------------------------------------------

resource "aws_subnet" "public" {
  count = length(var.availability_zones)

  vpc_id                  = aws_vpc.main.id
  cidr_block              = cidrsubnet(var.vpc_cidr, 8, count.index + 1) # 10.0.1.0/24, 10.0.2.0/24
  availability_zone       = var.availability_zones[count.index]
  map_public_ip_on_launch = true

  tags = {
    Name = "metadata-tool-${var.environment}-public-${var.availability_zones[count.index]}"
    Tier = "public"
  }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }

  tags = {
    Name = "metadata-tool-${var.environment}-public-rt"
  }
}

resource "aws_route_table_association" "public" {
  count = length(aws_subnet.public)

  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

# ALB requires subnets in at least 2 AZs — create a secondary public subnet if single-AZ
resource "aws_subnet" "public_secondary" {
  count = length(var.availability_zones) < 2 ? 1 : 0

  vpc_id                  = aws_vpc.main.id
  cidr_block              = cidrsubnet(var.vpc_cidr, 8, 2) # 10.0.2.0/24
  availability_zone       = "${substr(var.availability_zones[0], 0, length(var.availability_zones[0]) - 1)}b"
  map_public_ip_on_launch = true

  tags = {
    Name = "metadata-tool-${var.environment}-public-secondary"
    Tier = "public"
  }
}

resource "aws_route_table_association" "public_secondary" {
  count = length(aws_subnet.public_secondary)

  subnet_id      = aws_subnet.public_secondary[0].id
  route_table_id = aws_route_table.public.id
}

# ---------------------------------------------------------------------------
# NAT Gateway (for private subnet outbound internet access)
# ---------------------------------------------------------------------------

resource "aws_eip" "nat" {
  count  = var.enable_nat_gateway ? 1 : 0
  domain = "vpc"

  tags = {
    Name = "metadata-tool-${var.environment}-nat-eip"
  }
}

resource "aws_nat_gateway" "main" {
  count = var.enable_nat_gateway ? 1 : 0

  allocation_id = aws_eip.nat[0].id
  subnet_id     = aws_subnet.public[0].id

  tags = {
    Name = "metadata-tool-${var.environment}-nat"
  }

  depends_on = [aws_internet_gateway.main]
}

# ---------------------------------------------------------------------------
# Private Subnets (ECS Fargate tasks)
# ---------------------------------------------------------------------------

resource "aws_subnet" "private" {
  count = length(var.availability_zones)

  vpc_id            = aws_vpc.main.id
  cidr_block        = cidrsubnet(var.vpc_cidr, 8, count.index + 11) # 10.0.11.0/24, 10.0.12.0/24
  availability_zone = var.availability_zones[count.index]

  tags = {
    Name = "metadata-tool-${var.environment}-private-${var.availability_zones[count.index]}"
    Tier = "private"
  }
}

resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id

  dynamic "route" {
    for_each = var.enable_nat_gateway ? [1] : []
    content {
      cidr_block     = "0.0.0.0/0"
      nat_gateway_id = aws_nat_gateway.main[0].id
    }
  }

  tags = {
    Name = "metadata-tool-${var.environment}-private-rt"
  }
}

resource "aws_route_table_association" "private" {
  count = length(aws_subnet.private)

  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private.id
}

# ---------------------------------------------------------------------------
# Database Subnets (RDS — isolated, no internet access)
# ---------------------------------------------------------------------------

resource "aws_subnet" "database" {
  count = length(var.availability_zones)

  vpc_id            = aws_vpc.main.id
  cidr_block        = cidrsubnet(var.vpc_cidr, 8, count.index + 21) # 10.0.21.0/24, 10.0.22.0/24
  availability_zone = var.availability_zones[count.index]

  tags = {
    Name = "metadata-tool-${var.environment}-database-${var.availability_zones[count.index]}"
    Tier = "database"
  }
}

# Database subnet group for RDS (needs at least 2 AZs)
# For single-AZ demo, we create a second subnet in another AZ just for the subnet group
resource "aws_subnet" "database_secondary" {
  count = length(var.availability_zones) < 2 ? 1 : 0

  vpc_id            = aws_vpc.main.id
  cidr_block        = cidrsubnet(var.vpc_cidr, 8, 22)
  availability_zone = "${substr(var.availability_zones[0], 0, length(var.availability_zones[0]) - 1)}b"

  tags = {
    Name = "metadata-tool-${var.environment}-database-secondary"
    Tier = "database"
  }
}
