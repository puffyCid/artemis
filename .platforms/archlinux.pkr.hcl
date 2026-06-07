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

variable "archlinux_version" {
  type    = string
  default = "2026.05.01"
}

variable "headless" {
  type    = bool
  default = true
}

source "qemu" "archlinux" {
  # Arch Linux ISO Target
  iso_url      = "https://fastly.mirror.pkgbuild.com/iso/2026.05.01/archlinux-2026.05.01-x86_64.iso"
  iso_checksum = "file:https://fastly.mirror.pkgbuild.com/iso/2026.05.01/sha256sums.txt"

  headless = var.headless

  # VM Hardware mapping
  accelerator      = "kvm" # Toggle to "tcg" if running in a non-KVM environment
  cpus             = 2
  memory           = 2048
  disk_size        = "20G"
  disk_interface   = "virtio"
  net_device       = "virtio-net"
  format           = "qcow2"
  output_directory = "output-archlinux"
  vm_name          = "box.img" # Required by Vagrant-libvirt

  # Automated Installation Keystrokes for OpenBSD 7.8 Console
  boot_wait = "20s"
  boot_command = [
    # Clear boot screen and enter live environment
    "<enter><wait45>",
    
    # 1. Partition, format, and mount /dev/vda
    "parted -s /dev/vda mklabel msdos mkpart primary ext4 1MiB 100% && mkfs.ext4 -F /dev/vda1 && mount /dev/vda1 /mnt<enter><wait10s>",
    
    # 2. Bootstrap Core Packages and Services (Takes a few minutes, so we wait longer)
    "pacstrap -K /mnt base linux linux-firmware sudo openssh rsync networkmanager qemu-guest-agent && genfstab -U /mnt >> /mnt/etc/fstab<enter><wait300s>",
    
    # 3. Enter chroot environment and configure localizations, system services, and users
    "arch-chroot /mnt /bin/bash -c '",
    "ln -sf /usr/share/zoneinfo/UTC /etc/localtime && ",
    "echo \"archlinux-libvirt\" > /etc/hostname && ",
    "systemctl enable NetworkManager sshd qemu-guest-agent && ",
    "useradd -m -G wheel -s /bin/bash vagrant && ",
    "echo \"vagrant:vagrant\" | chpasswd && ",
    "echo \"root:vagrant\" | chpasswd && ",
    "echo \"vagrant ALL=(ALL) NOPASSWD: ALL\" > /etc/sudoers.d/vagrant && chmod 440 /etc/sudoers.d/vagrant && ",
    "mkdir -p /home/vagrant/.ssh && ",
    "curl -Lo /home/vagrant/.ssh/authorized_keys https://raw.githubusercontent.com/hashicorp/vagrant/master/keys/vagrant.pub && ",
    "chown -R vagrant:vagrant /home/vagrant/.ssh && chmod 700 /home/vagrant/.ssh && chmod 600 /home/vagrant/.ssh/authorized_keys && ",
    "pacman -S --noconfirm grub && grub-install --target=i386-pc /dev/vda && grub-mkconfig -o /boot/grub/grub.cfg",
    "'<enter><wait45>",
    
    # 4. Unmount systems and reboot into the new Vagrant base image
    "umount -R /mnt && reboot<enter>"
  ]

  # SSH specifications for Packer to hand-off commands
  ssh_username     = "vagrant"
  ssh_password     = "vagrant"
  ssh_timeout      = "30m"
  shutdown_command = "sudo shutdown -h now"

  qemuargs = [
    ["-boot", "strict=on,menu=on,order=cd"]
  ]
}

build {
  sources = ["source.qemu.archlinux"]

  # Compile everything directly into a .box tarball mapped for libvirt
  post-processor "vagrant" {
    keep_input_artifact = false
    output              = "./boxes/archlinux-${var.archlinux_version}.box"
    provider_override   = "libvirt"
  }
}
