@_windows: (_cargo-plugins)
  echo "Installing dev dependencies!"
  choco install cmake.install --installargs '"ADD_CMAKE_TO_PATH=User"'
  choco install nodejs podman-cli

  cargo install --locked trunk
