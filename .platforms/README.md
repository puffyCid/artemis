# Packer Templates

Experimental [packer](https://developer.hashicorp.com/packer) for building VMs to compile artemis. These templates are useful if you want to compile artemis on niche platforms like BSD.

All steps assume you are on a Linux host system with [libvirt](https://en.wikipedia.org/wiki/Libvirt) library and QEMU installed

Sample setup steps:

1. Install [packer](https://developer.hashicorp.com/packer) and [vagrant](https://developer.hashicorp.com/vagrant)
2. Initialize the target template with packer. `packer init openbsd.pkr.hcl`
3. Validate the target template with packer. `packer validate openbsd.pkr.hcl`
4. Build the Vagrant box. `packer build openbsd.pkr.hcl`
5. Once the box is built, add it locally. `vagrant box add --name artemis-openbsd ./boxes/openbsd-78.box`
6. Initialize the box from repo root directory. `vagrant init artemis-openbsd`
7. Start the box from repo root directory. `vagrant up --name artemis-openbsd`
8. SSH into the the box. `vagrant ssh openbsd`
10. Compile for OpenBSD. `just slim`