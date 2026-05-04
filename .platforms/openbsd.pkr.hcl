# Full credit to Gemini (google.com/ai) O.o
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

variable "openbsd_version" {
  type    = string
  default = "78"
}

variable "headless" {
  type    = bool
  default = true
}

source "qemu" "openbsd" {
  # OpenBSD 7.8 ISO Target
  # iso_url = "file:///home/<user>/Downloads/install78.iso"
  iso_url      = "https://cdn.openbsd.org/pub/OpenBSD/7.8/amd64/install78.iso"
  iso_checksum = "file:https://cdn.openbsd.org/pub/OpenBSD/7.8/amd64/SHA256"

  headless = var.headless

  # VM Hardware mapping
  accelerator      = "kvm" # Toggle to "tcg" if running in a non-KVM environment
  cpus             = 2
  memory           = 2048
  disk_size        = "20G"
  disk_interface   = "virtio"
  net_device       = "virtio-net"
  format           = "qcow2"
  output_directory = "output-openbsd"
  vm_name          = "box.img" # Required by Vagrant-libvirt

  # Automated Installation Keystrokes for OpenBSD 7.8 Console
  boot_wait = "20s"
  boot_command = [
    "i<enter>",               # Enter the standard install routine
    "us<enter>",              # Target keyboard layout
    "openbsd<enter>",         # Hostname
    "<enter>",                # Default network interface (vio0)
    "<enter>",                # Use autoconf 
    "<enter>",                # Accept default IP/IPv6
    "<enter>",                # Finish network setup
    "vagrant<enter>",         # Root password
    "vagrant<enter>",         # Confirm root password
    "y<enter>",               # Start sshd by default
    "n<enter>",               # Decline running the X Window system
    "n<enter>",               # Change default console to com0
    "vagrant<enter>",         # Setup primary user 'vagrant'
    "vagrant<enter>",         # Full name
    "vagrant<enter>",         # User password
    "vagrant<enter>",         # Confirm user password
    "y<enter>",               # Prohibit root SSH login (or 'n' if you want it)
    "<enter>",                # Default timezone
    "<enter>",                # Default disk
    "n<enter>",               # Encrypt the disk
    "w<enter>",               # Use the whole disk
    "<enter>",                # auto layout
    "http<enter>",            # Install from CD-ROM
    "<enter>",                # No proxy
    "cdn.openbsd.org<enter>", # Default mirror
    "<enter>",                # Default directory
    "done<enter>",            # Finished with OS file sets
    "y<enter>",               # Install without verification (normal for CD ISOs)
    "<enter>",                # Continue
    "y<enter>",               # Set correct time
    "<enter>",                # Reboot machine after layout completion
  ]

  # SSH specifications for Packer to hand-off commands
  ssh_username     = "vagrant"
  ssh_password     = "vagrant"
  ssh_timeout      = "30m"
  shutdown_command = "echo 'vagrant' | su - root -c 'halt -p'"
}

build {
  sources = ["source.qemu.openbsd"]

  # Provisioning step to handle Vagrant mapping and your requested packages
  provisioner "shell" {
    execute_command = "echo 'vagrant' | su - root -c 'sh -c \"{{.Vars}} {{.Path}}\"'"
    inline = [
      # Setup doas for passwordless execution (Vagrant requirement)
      "echo 'permit nopass vagrant as root' > /etc/doas.conf",

      # Inject the insecure Vagrant SSH key for initial connections
      "mkdir -p /home/vagrant/.ssh",
      "chmod 0700 /home/vagrant/.ssh",
      "ftp -o /home/vagrant/.ssh/authorized_keys https://raw.githubusercontent.com/hashicorp/vagrant/master/keys/vagrant.pub",
      "chmod 0600 /home/vagrant/.ssh/authorized_keys",
      "chown -R vagrant:vagrant /home/vagrant/.ssh",

      # Install Rust, Just, Rsync, Git, and CMake via OpenBSD package management
      "echo 'Installing requested development packages...'",
      "pkg_add -I rust just cmake rsync-- git"
    ]
  }

  # Compile everything directly into a .box tarball mapped for libvirt
  post-processor "vagrant" {
    keep_input_artifact = false
    output              = "./boxes/openbsd-${var.openbsd_version}.box"
    provider_override   = "libvirt"
  }
}
