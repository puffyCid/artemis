import "rust.just"

@_ubuntu: (_rust-setup-unix) && (_cargo-plugins)
  echo "Installing dev dependencies!"
  sudo apt install nodejs npm podman gcc pkg-config libssl-dev
