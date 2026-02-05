packer {
  required_plugins {
    digitalocean = {
      version = ">= 1.1.0"
      source  = "github.com/digitalocean/digitalocean"
    }
  }
}

variable "do_token" {
  type      = string
  sensitive = true
}

variable "region" {
  type    = string
  default = "nyc3"
}

variable "size" {
  type    = string
  default = "s-1vcpu-1gb"
}

variable "source_image" {
  type    = string
  default = "ubuntu-24-04-x64"
}

variable "snapshot_name" {
  type = string
}

variable "droplet_name" {
  type    = string
  default = "opencode-cloud-marketplace"
}

variable "application_name" {
  type    = string
  default = "opencode-cloud"
}

variable "application_version" {
  type    = string
  default = "15.2.0"
}

variable "container_image" {
  type    = string
  default = "prizz/opencode-cloud-sandbox:15.2.0"
}

variable "container_name" {
  type    = string
  default = "opencode-cloud-sandbox"
}

variable "container_username" {
  type    = string
  default = "opencode"
}

variable "opencode_cloud_env" {
  type    = string
  default = "digitalocean_docker_droplet"
}

source "digitalocean" "marketplace" {
  api_token     = var.do_token
  region        = var.region
  size          = var.size
  image         = var.source_image
  droplet_name  = var.droplet_name
  snapshot_name = var.snapshot_name
  monitoring    = false
  ipv6          = false
  # Keep the build droplet close to the base image (avoid extra DO config).
  private_networking = false
  droplet_agent      = false
  ssh_username       = "root"
  ssh_timeout        = "10m"
}

build {
  sources = ["source.digitalocean.marketplace"]

  provisioner "shell" {
    inline = [
      "set -euo pipefail",
      "cloud-init status --wait"
    ]
  }

  provisioner "shell" {
    inline = [
      "set -euo pipefail",
      "export DEBIAN_FRONTEND=noninteractive",
      "apt-get update -y",
      "apt-get install -y docker.io curl jq build-essential pkg-config libssl-dev ufw",
      "systemctl enable --now docker",
      "ufw allow OpenSSH",
      "ufw allow 3000/tcp",
      "ufw --force enable"
    ]
  }

  provisioner "file" {
    source      = "scripts/provisioning/opencode-cloud-setup.sh"
    destination = "/tmp/opencode-cloud-setup.sh"
  }

  provisioner "file" {
    source      = "scripts/provisioning/opencode-cloud-setup-cloud-init.sh"
    destination = "/tmp/opencode-cloud-setup-cloud-init.sh"
  }

  provisioner "file" {
    source      = "scripts/provisioning/opencode-cloud-setup-digitalocean.sh"
    destination = "/tmp/opencode-cloud-setup-digitalocean.sh"
  }

  provisioner "shell" {
    inline = [
      "install -m 0755 /tmp/opencode-cloud-setup.sh /usr/local/bin/opencode-cloud-setup.sh",
      "install -m 0755 /tmp/opencode-cloud-setup-cloud-init.sh /usr/local/bin/opencode-cloud-setup-cloud-init.sh",
      "install -m 0755 /tmp/opencode-cloud-setup-digitalocean.sh /usr/local/bin/opencode-cloud-setup-digitalocean.sh"
    ]
  }

  provisioner "file" {
    source      = "infra/digitalocean/scripts/per-instance-opencode-cloud.sh"
    destination = "/tmp/001-opencode-cloud.sh"
  }

  provisioner "shell" {
    inline = [
      "install -d -m 0755 /var/lib/cloud/scripts/per-instance",
      "install -m 0755 /tmp/001-opencode-cloud.sh /var/lib/cloud/scripts/per-instance/001-opencode-cloud.sh"
    ]
  }

  provisioner "shell" {
    environment_vars = [
      "HOST_CONTAINER_IMAGE=${var.container_image}",
      "HOST_CONTAINER_NAME=${var.container_name}",
      "CONTAINER_USERNAME=${var.container_username}",
      "OPENCODE_CLOUD_ENV=${var.opencode_cloud_env}"
    ]
    inline = [
      "install -d -m 0700 /etc/opencode-cloud",
      "printf '%s\\n' \"HOST_CONTAINER_IMAGE=$HOST_CONTAINER_IMAGE\" \"HOST_CONTAINER_NAME=$HOST_CONTAINER_NAME\" \"CONTAINER_USERNAME=$CONTAINER_USERNAME\" \"OPENCODE_CLOUD_ENV=$OPENCODE_CLOUD_ENV\" \"PUBLIC_OPENCODE_DOMAIN_URL=\" \"PUBLIC_OPENCODE_ALB_URL=\" > /etc/opencode-cloud/stack.env",
      "chmod 0600 /etc/opencode-cloud/stack.env"
    ]
  }

  provisioner "shell" {
    environment_vars = [
      "APPLICATION_NAME=${var.application_name}",
      "APPLICATION_VERSION=${var.application_version}"
    ]
    inline = [
      "install -d -m 0755 /var/lib/digitalocean",
      "printf '%s\\n' \"application_name=$APPLICATION_NAME\" \"application_version=$APPLICATION_VERSION\" > /var/lib/digitalocean/application.info"
    ]
  }

  # DigitalOcean Marketplace cleanup + validation scripts MUST be the final step
  # (they remove SSH keys and clear cloud-init state).
  provisioner "shell" {
    scripts = [
      "infra/digitalocean/marketplace/90-cleanup.sh",
      "infra/digitalocean/marketplace/99-img-check.sh"
    ]
  }
}
