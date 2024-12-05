@_windows: (_cargo-plugins) && (_node_modules)
  echo "Installing dev dependencies!"
  choco install cmake.install --installargs '"ADD_CMAKE_TO_PATH=User"'
  choco install nodejs deno
