# VPC
resource "aws_vpc" "main" {
  cidr_block = "10.0.0.0/16"

  tags = {
    Name = "fyp-autorightsizing"
  }
}

# Internet Gateway
resource "aws_internet_gateway" "igw" {
  vpc_id = aws_vpc.main.id

  tags = {
    Name = "fyp-autorightsizing-igw"
  }
}

# Elastic IP for NAT Gateway
resource "aws_eip" "nat" {
  vpc = true

  tags = {
    Name = "fyp-autorightsizing-nat"
  }
}

# NAT Gateway
resource "aws_nat_gateway" "nat" {
  allocation_id = aws_eip.nat.id
  subnet_id     = aws_subnet.public_ap_southeast_1a.id

  tags = {
    Name = "fyp-autorightsizing-nat"
  }

  depends_on = [aws_internet_gateway.igw]
}

# Private Subnets
resource "aws_subnet" "private_ap_southeast_1a" {
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.0.0.0/19"
  availability_zone = "ap-southeast-1a"

  tags = {
    "Name"                                      = "fyp-autorightsizing-private-ap-southeast-1a"
    "kubernetes.io/role/internal-elb"           = "1"
    "kubernetes.io/cluster/fyp-autorightsizing" = "owned"
  }
}

resource "aws_subnet" "private_ap_southeast_1b" {
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.0.32.0/19"
  availability_zone = "ap-southeast-1b"

  tags = {
    "Name"                                      = "fyp-autorightsizing-private-ap-southeast-1b"
    "kubernetes.io/role/internal-elb"           = "1"
    "kubernetes.io/cluster/fyp-autorightsizing" = "owned"
  }
}

# Public Subnets
resource "aws_subnet" "public_ap_southeast_1a" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.0.64.0/19"
  availability_zone       = "ap-southeast-1a"
  map_public_ip_on_launch = true

  tags = {
    "Name"                                      = "fyp-autorightsizing-public-ap-southeast-1a"
    "kubernetes.io/role/elb"                    = "1"
    "kubernetes.io/cluster/fyp-autorightsizing" = "owned"
  }
}

resource "aws_subnet" "public_ap_southeast_1b" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.0.96.0/19"
  availability_zone       = "ap-southeast-1b"
  map_public_ip_on_launch = true

  tags = {
    "Name"                                      = "fyp-autorightsizing-public-ap-southeast-1b"
    "kubernetes.io/role/elb"                    = "1"
    "kubernetes.io/cluster/fyp-autorightsizing" = "owned"
  }
}

# Route Tables
resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.nat.id
  }

  tags = {
    Name = "fyp-autorightsizing-private"
  }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.igw.id
  }

  tags = {
    Name = "fyp-autorightsizing-public"
  }
}

# Route Table Associations
resource "aws_route_table_association" "private_ap_southeast_1a" {
  subnet_id      = aws_subnet.private_ap_southeast_1a.id
  route_table_id = aws_route_table.private.id
}

resource "aws_route_table_association" "private_ap_southeast_1b" {
  subnet_id      = aws_subnet.private_ap_southeast_1b.id
  route_table_id = aws_route_table.private.id
}

resource "aws_route_table_association" "public_ap_southeast_1a" {
  subnet_id      = aws_subnet.public_ap_southeast_1a.id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "public_ap_southeast_1b" {
  subnet_id      = aws_subnet.public_ap_southeast_1b.id
  route_table_id = aws_route_table.public.id
}
