# -*- mode: ruby -*-
# vi: set ft=ruby :

# Calculate 30% of host cores (minimum 1)
cpus = [1, (0.30 * `nproc`.to_i).to_i].max

# All Vagrant configuration is done below. The "2" in Vagrant.configure
# configures the configuration version (we support older styles for
# backwards compatibility). Please don't change it unless you know what
# you're doing.
Vagrant.configure("2") do |config|

  config.vm.provider :libvirt do |config_vm|
    config_vm.memory = "4096"
    config_vm.cpus = cpus
    config_vm.driver = "kvm"
    config_vm.nic_model_type = "virtio"
    config_vm.cpu_mode = 'host-passthrough'
    config_vm.graphics_type = 'none'
    config_vm.graphics_type = "spice"    
    config_vm.video_type = "qxl"
  end

  config.vm.define "centos" do |centos|
    # Only the compiled musl artemis binary directory is exposed
    # Our Vagrantfile is not exposed to the box
    centos.vm.synced_folder "./target/x86_64-unknown-linux-musl/release", "/vagrant",
      type: "nfs", 
      nfs_version: 4, 
      nfs_udp: false, 
      mount_options: ["tcp", "rsize=1048576", "wsize=1048576", "hard", "intr"]
    centos.vm.box = "bento/centos-stream-9"

    # Must update firewall to allow mounting of project code
    # sudo firewall-cmd --zone=libvirt --add-service=nfs
    # sudo firewall-cmd --zone=libvirt --add-service=mountd
    # sudo firewall-cmd --zone=libvirt --add-service=rpc-bind
    centos.vm.provider :libvirt do |centosstream|
      centosstream.qemu_use_agent = true
      centos.vm.provision "shell", inline: <<-SHELL
      # Install updates for CentOS Stream
      sudo dnf upgrade -y
      SHELL
    end
  end

  config.vm.define "freebsd" do |freebsd|
    # Our Vagrantfile is not exposed to the box
    freebsd.vm.synced_folder "./target/x86_64-unknown-freebsd/release", "/vagrant",
      type: "nfs", 
      nfs_version: 4, 
      nfs_udp: false, 
      mount_options: ["tcp", "rsize=1048576", "wsize=1048576", "hard", "intr"]
    freebsd.vm.box = "bento/freebsd-14"

    # Must update firewall to allow mounting of project code
    # sudo firewall-cmd --zone=libvirt --add-service=nfs
    # sudo firewall-cmd --zone=libvirt --add-service=mountd
    # sudo firewall-cmd --zone=libvirt --add-service=rpc-bind
    freebsd.vm.provider :libvirt do |freebsd_vm|
      freebsd.vm.provision "shell", inline: <<-SHELL
      # Install updates for FreeBSD Stream
      sudo pkg update && sudo pkg upgrade -y
      SHELL
    end
  end

  config.vm.define "openbsd" do |openbsd|
    # OpenBSD specific shell and command overrides
    openbsd.ssh.shell = "ksh -l"
    openbsd.ssh.sudo_command = "doas -n %c"
    # Our Vagrantfile is not exposed to the box
    openbsd.vm.synced_folder ".", "/home/vagrant/",
      type: "rsync",
      rsync__rsync_path: "doas rsync",
      rsync__exclude: ["target/", ".platforms/", "Vagrantfile"]
    openbsd.vm.box = "artemis-openbsd"
    openbsd.vm.provider :libvirt do |obsd_libvirt|
      # FORCE VGA output to match your Packer build's default OS layout
      obsd_libvirt.graphics_type = 'vnc' 
    end
  end

  config.vm.define "archlinux" do |archlinux|
    # Only the compiled musl artemis binary directory is exposed
    # Our Vagrantfile is not exposed to the box
    archlinux.vm.synced_folder "./target/x86_64-unknown-linux-musl/release", "/vagrant",
      type: "rsync",
      rsync__rsync_path: "sudo rsync",
      rsync__exclude: ["target/", ".platforms/", "Vagrantfile"]
    archlinux.vm.box = "artemis-archlinux"

    # Must update firewall to allow mounting of project code
    # sudo firewall-cmd --zone=libvirt --add-service=nfs
    # sudo firewall-cmd --zone=libvirt --add-service=mountd
    # sudo firewall-cmd --zone=libvirt --add-service=rpc-bind
    archlinux.vm.provider :libvirt do |archlinux_vm|
      archlinux_vm.qemu_use_agent = true
    end
  end
end
