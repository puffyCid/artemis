# Install Rust on Linux or macOS platforms
@_rust-setup-unix:
  echo "Installing Rust!"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  . "$HOME/.cargo/env"


@_cargo-plugins:
  echo "Installing Cargo plugins and binaries: deny, audit, auditable, nextest, update, and cross"
  cargo install cargo-deny --locked
  cargo install cargo-audit
  cargo install cargo-auditable
  cargo install cargo-nextest --locked
  cargo install cross --git https://github.com/cross-rs/cross
  cargo install cargo-update