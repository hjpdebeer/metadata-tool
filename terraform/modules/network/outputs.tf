output "vpc_id" {
  value = aws_vpc.main.id
}

output "public_subnet_ids" {
  value = concat(
    aws_subnet.public[*].id,
    aws_subnet.public_secondary[*].id
  )
}

output "private_subnet_ids" {
  value = aws_subnet.private[*].id
}

output "database_subnet_ids" {
  value = concat(
    aws_subnet.database[*].id,
    aws_subnet.database_secondary[*].id
  )
}
