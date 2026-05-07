# Full credit to Gemini (google.com/ai)
packer {
  required_plugins {
    qemu = {
      version = ">= 1.1.0"
      source  = "github.com/hashicorp/qemu"
    }
    vagrant = {
      version = ">= 1.0.0"
      source  = "github.com/hashicorp/vagrant"
    }
  }
}

variable "esxi_version" {
  type    = string
  default = "8"
}

variable "headless" {
  type    = bool
  default = true
}

source "qemu" "esxi" {
  # ESXi ISO Target
  iso_url = "file:///home/<user>/Downloads/esxi8.iso"
  iso_checksum = "md5:<hash>"

  headless = var.headless

  # VM Hardware mapping
  accelerator      = "kvm" # Toggle to "tcg" if running in a non-KVM environment
  cpus             = 4
  memory           = 8192
  disk_interface   = "ide"
  disk_size        = "40G"
  net_device       = "e1000e"
  format           = "qcow2"
  output_directory = "output-esxi"
  vm_name          = "box.img" # Required by Vagrant-libvirt
  net_bridge = "virbr0"

  boot_wait = "10s"
  boot_command = [
    "<enter>"
  ]

  qemuargs = [
    ["-cpu", "host"]
  ]

  ssh_username     = "root"
  ssh_password     = "Vagrant123!"
  ssh_timeout      = "30m"
  shutdown_command = "poweroff"
}

build {
  sources = ["source.qemu.esxi"]

  # Compile everything directly into a .box tarball mapped for libvirt
  post-processor "vagrant" {
    keep_input_artifact = false
    output              = "./boxes/esxi-${var.esxi_version}.box"
    provider_override   = "libvirt"
  }
}
