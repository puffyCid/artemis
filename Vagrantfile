# -*- mode: ruby -*-
# vi: set ft=ruby :

# Calculate 30% of host cores (minimum 1)
cpus = [1, (0.30 * `nproc`.to_i).to_i].max

# All Vagrant configuration is done below. The "2" in Vagrant.configure
# configures the configuration version (we support older styles for
# backwards compatibility). Please don't change it unless you know what
# you're doing.
Vagrant.configure("2") do |config|
  # Generic CentOS Stream 10 box (Optimized for libvirt/qemu)
  config.vm.box = "bento/centos-stream-9"

  # Disable automatic box update checking. If you disable this, then
  # boxes will only be checked for updates when the user runs
  # `vagrant box outdated`. This is not recommended.
  # config.vm.box_check_update = false

  # Create a forwarded port mapping which allows access to a specific port
  # within the machine from a port on the host machine. In the example below,
  # accessing "localhost:8080" will access port 80 on the guest machine.
  # NOTE: This will enable public access to the opened port
  # config.vm.network "forwarded_port", guest: 80, host: 8080

  # Create a forwarded port mapping which allows access to a specific port
  # within the machine from a port on the host machine and only allow access
  # via 127.0.0.1 to disable public access
  # config.vm.network "forwarded_port", guest: 80, host: 8080, host_ip: "127.0.0.1"

  # Create a private network, which allows host-only access to the machine
  # using a specific IP.
  # config.vm.network "private_network", ip: "192.168.33.10"

  # Create a public network, which generally matched to bridged network.
  # Bridged networks make the machine appear as another physical device on
  # your network.
  # config.vm.network "public_network"

  # Share an additional folder to the guest VM. The first argument is
  # the path on the host to the actual folder. The second argument is
  # the path on the guest to mount the folder. And the optional third
  # argument is a set of non-required options.
  # config.vm.synced_folder "../data", "/vagrant_data"

  # Must update firewall to allow mounting of project code
  # sudo firewall-cmd --zone=libvirt --add-service=nfs
  # sudo firewall-cmd --zone=libvirt --add-service=mountd
  # sudo firewall-cmd --zone=libvirt --add-service=rpc-bind

  # Only the compiled musl artemis binary directory is exposed
  # Our Vagrantfile is not exposed to the box
  config.vm.synced_folder "./target/x86_64-unknown-linux-musl/release", "/vagrant",
    type: "nfs", 
    nfs_version: 4, 
    nfs_udp: false, 
    mount_options: ["tcp", "rsize=1048576", "wsize=1048576", "hard", "intr"]


  # Provider-specific configuration so you can fine-tune various
  # backing providers for Vagrant. These expose provider-specific options.
  # Example for VirtualBox:
  #
  # config.vm.provider "virtualbox" do |vb|
  #   # Display the VirtualBox GUI when booting the machine
  #   vb.gui = true
  #
  #   # Customize the amount of memory on the VM:
  #   vb.memory = "1024"
  # end
  #
  # View the documentation for the provider you are using for more
  # information on available options.
  # Exclusive Libvirt/QEMU configuration
  config.vm.provider :libvirt do |centosstream|
    centosstream.memory = "4096"
    centosstream.cpus = cpus
    centosstream.driver = "kvm"
    centosstream.nic_model_type = "virtio"
    centosstream.qemu_use_agent = true
    centosstream.cpu_mode = 'host-passthrough'
    centosstream.graphics_type = 'none' 
  end

  config.vm.provision "shell", inline: <<-SHELL
    # Install updates for CentOS Stream
    sudo dnf upgrade -y
  SHELL
end

