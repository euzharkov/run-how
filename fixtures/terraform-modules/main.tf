terraform {
  required_version = ">= 1.6"
}

module "vpc" {
  source = "./modules/vpc"
}
