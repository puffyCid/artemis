import "rust.just"
import "node.just"
import "deno.just"

@_ubuntu: (_rust-setup-unix) && (_cargo-plugins) (_deno-setup-unix) (_node_modules)
  echo "Installing dev dependencies!"
  sudo apt install nodejs npm podman cmake gcc pkg-config libssl-dev

  echo "Installing trunk via cargo"
  cargo install trunk --locked