# -*- mode: ruby -*-
# vi: set ft=ruby :

# All Vagrant configuration is done below. The "2" in Vagrant.configure
# configures the configuration version (we support older styles for
# backwards compatibility). Please don't change it unless you know what
# you're doing.
# Calculate 30% of host cores (minimum 1)
cpus = [1, (0.30 * `nproc`.to_i).to_i].max






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

  # Disable the default share of the current code directory. Doing this
  # provides improved isolation between the vagrant box and your host
  # by making sure your Vagrantfile isn't accessible to the vagrant box.
  # If you use this you may want to enable additional shared subfolders as
  # shown above.
  # config.vm.synced_folder ".", "/vagrant", disabled: true

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

  # Setup Rust, Just, and Nextest
  config.vm.provision "shell", inline: <<-SHELL
    # Install build dependencies for CentOS Stream
    sudo dnf groupinstall -y "Development Tools"
    sudo dnf install -y curl gcc

    # Install Rust for the vagrant user if not present
    if [ ! -d "/home/vagrant/.cargo" ]; then
      curl --proto '=https' --tlsv1.2 -sSf https://rustup.rs | sh -s -- -y
      echo 'source "$HOME/.cargo/env"' >> /home/vagrant/.bashrc
    fi


    # Enable provisioning with a shell script. Additional provisioners such as
    # Ansible, Chef, Docker, Puppet and Salt are also available. Please see the
    # documentation for more information about their specific syntax and use.
    # config.vm.provision "shell", inline: <<-SHELL
    #   apt-get update
    #   apt-get install -y apache2
    # SHELL

    # Install tools via cargo as the vagrant user
    # --locked is required for cargo-nextest
    sudo -u vagrant -i bash -c "cargo install just && cargo install cargo-nextest --locked"
  SHELL
end

