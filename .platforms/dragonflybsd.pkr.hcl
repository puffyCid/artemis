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

variable "dragonflybsd_version" {
  type    = string
  default = "642"
}

variable "headless" {
  type    = bool
  default = true
}

source "qemu" "dragonflybsd" {
  # DragonflyBSD ISO Target
  iso_url      = "https://mirror-master.dragonflybsd.org/iso-images/dfly-x86_64-6.4.2_REL.iso"
  iso_checksum = "file:https://mirror-master.dragonflybsd.org/iso-images/md5.txt"

  headless = var.headless

  # VM Hardware mapping
  accelerator      = "kvm" # Toggle to "tcg" if running in a non-KVM environment
  cpus             = 2
  memory           = 2048
  disk_size        = "20G"
  disk_interface   = "virtio"
  net_device       = "virtio-net"
  format           = "qcow2"
  output_directory = "output-dragonflybsd"
  vm_name          = "box.img" # Required by Vagrant-libvirt

  # Automated Installation Keystrokes for DragonflyBSD Console
  boot_wait = "1m"
  boot_command = [
    "installer<enter>", # Start installer process
    "<wait3s>",
    "<enter>", # Install DragonflyBSD
    "<wait3s>",
    "<enter>", # Confirm install DragonflyBSD
    "<wait3s>",
    "<right><enter>", # Use Legacy BIOS
    "<wait3s>",
    "<enter>", # Use default disk
    "<wait3s>",
    "<enter>", # Confirm disk
    "<wait3s>",
    "<enter>", # Confirm formatting
    "<wait3s>",
    "<enter>", # Accept formatting success
    "<wait3s>",
    "<enter>", # Use HAMMER2
    "<wait3s>",
    "<up><enter>", # Accept defaults
    "<wait3s>",
    "<enter>", # Use defaults
    "<wait3s>",
    "<enter>", # Accept small filesystem
    "<wait3s>",
    "<enter>",       # Install files
    "<wait60s>",     # Wait for installation of files
    "<down><enter>", # Install boot blocks
    "<wait3s>",
    "<enter>", # Accept info
    "<wait3s>",
    "<enter>", # Configure system
    "<wait3s>",
    "<down><down><down><down><down><enter>", # Navigate to network interface
    "<wait3s>",
    "<enter>", # Select the active network interface
    "<wait3s>",
    "<enter>",  # Select "Use DHCP"
    "<wait3s>", # Wait for it to pull a bridge IP
    "<enter>",  # Save and return to the menu
    "<wait3s>",
    "<down><down><down><enter>", # Set root password
    "vagrant<enter>",            # root password
    "<wait2s>",
    "vagrant<enter>", # confirm root password
    "<enter>",        # Accept
    "<wait2s>",
    "<enter>",                                     # Accept
    "<down><down><down><down><down><down><enter>", # Navigate to Hostname
    "dragonflybsd<enter>",                         # Hostname
    "<enter>",                                     # Empty domain 
    "<enter>",                                     # Accept
    "<wait2s>",
    "<leftAltOn><f3><leftAltOff>", # Move to the "Exit to Shell" option in the curses menu
    "<wait2s>",
    "root<enter>",
    "<wait1s>",
    "echo 'PermitRootLogin yes' >> /mnt/etc/ssh/sshd_config<enter>", # Allow the root user to log in natively via password (required for Packer)
    "<wait1s>",
    "sed -i '' 's/.*PasswordAuthentication.*/PasswordAuthentication yes/' /mnt/etc/ssh/sshd_config<enter>", # Allow password auth
    "<wait2s>",
    "echo 'ifconfig_vtnet0=\"DHCP\"' >> /mnt/etc/rc.conf<enter>",
    "<wait2s>",
    "reboot<enter>",
  ]

  # SSH specifications for Packer to hand-off commands
  ssh_username     = "root"
  ssh_password     = "vagrant"
  ssh_timeout      = "30m"
  shutdown_command = "halt -p"
}

build {
  sources = ["source.qemu.dragonflybsd"]

  provisioner "shell" {
    # Since we log in as root, we do not need the 'su - root' wrapper anymore
    execute_command = "sh -c '{{.Vars}} {{.Path}}'"
    inline = [
      # 1. Install sudo first
      "env ASSUME_ALWAYS_YES=yes pkg bootstrap",
      "pkg install -y sudo",

      # 2. Create the 'vagrant' user with password 'vagrant' and add to the wheel group
      # On DragonFlyBSD/FreeBSD, -h 0 reads the password from standard input
      "echo 'vagrant' | pw useradd vagrant -m -G wheel -s /bin/sh -h 0",

      # 3. Grant the wheel group passwordless sudo privileges (Vagrant requirement)
      "echo '%wheel ALL=(ALL) NOPASSWD: ALL' >> /usr/local/etc/sudoers",

      # 4. Inject the insecure Vagrant public key into the new vagrant user's home
      "mkdir -p /home/vagrant/.ssh",
      "chmod 0700 /home/vagrant/.ssh",
      "fetch -o /home/vagrant/.ssh/authorized_keys https://raw.githubusercontent.com/hashicorp/vagrant/master/keys/vagrant.pub",
      "chmod 0600 /home/vagrant/.ssh/authorized_keys",
      "chown -R vagrant:vagrant /home/vagrant/.ssh",

      # 5. Install Rust, Git, and supporting compiler tools
      "echo 'Installing requested development packages...'",
      "pkg install -y git cmake rsync just rust",
    ]
  }

  post-processor "vagrant" {
    keep_input_artifact = false
    output              = "./boxes/dragonflybsd-${var.dragonflybsd_version}.box"
    provider_override   = "libvirt"
  }
}
