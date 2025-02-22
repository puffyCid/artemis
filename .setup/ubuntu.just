import "rust.just"
import "node.just"

@_ubuntu: (_rust-setup-unix) && (_cargo-plugins) (_node_modules)
  echo "Installing dev dependencies!"
  sudo apt install nodejs npm podman cmake gcc pkg-config libssl-dev
