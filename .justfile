# Small Justfile (https://github.com/casey/just and https://just.systems/man/en). 
# `just` is recommended. 
# Its useful when you want to run groups of tests and do not want to type the full test path
# Its very useful to prebuild WASM code before compiling the rest of artemis
# Windows users will need to use PowerShell `just --shell pwsh.exe --shell-arg -c`

import ".setup/ubuntu.just"
import ".setup/fedora.just"
import ".setup/windows.just"
import ".setup/macos.just"

# Run cargo clippy on artemis project 
default:(_wasm)
  cargo clippy

_test target:
  cargo test --release {{target}}

_wasm:
  # Ignore Windows errors any prexisting directories
  -mkdir -p target/dist/web
  # Trying both trunk configs. Windows has seperate config. Continue if we get an error
  -cd webui && trunk build --release
  -cd webui && trunk build --config TrunkWin.toml --release

_pretest:(_wasm)
  cargo test --no-run --release

# Test only the ESE parsing functions
[group('artifacts')]
ese: (_test "artifacts::os::windows::ese")

# Test only the WMI parsing functions
[group('artifacts')]
wmi: (_test "artifacts::os::windows::wmi")

# Test only the ShellItems parsing functions
[group('artifacts')]
shellitems: (_test "artifacts::os::windows::shellitems")

# Test only the Outlook parsing functions
[group('artifacts')]
outlook: (_test "artifacts::os::windows::outlook")

# Test only the Spotlight parsing functions
[group('artifacts')]
spotlight: (_test "artifacts::os::macos::spotlight")

# Test only the Registry parsing functions
[group('artifacts')]
registry: (_test "artifacts::os::windows::registry")

# Test only the JavaScript runtime
runtime: (_test "runtime::")

# Test only the FileSystem functions
filesystem: (_test "filesystem::")

# Test all the Windows artifacts
[group('os')]
windows: (_test "artifacts::os::windows")

# Test all the macOS artifacts
[group('os')]
macos: (_test "artifacts::os::macos")

# Test all the Linux artifacts
[group('os')]
linux: (_test "artifacts::os::linux")

# Test all the Unix artifacts
[group('os')]
unix: (_test "artifacts::os::unix")

# Spawn single client and attempt to connect to server
[group('workspace')]
client:
  cd client && cargo build --release --examples
  cd target/release/examples && ./start_client ../../../client/tests/test_data/client.toml

# Compile WASM and server code then start the server
[group('workspace')]
server:(_wasm)
  cd server && cargo build --release --examples
  cd target/release/examples/ && ./start_server ../../../server/tests/test_data/server.toml

# Build the entire artemis project.
build:(_wasm)
  cargo build --release

# Run tests for code coverage. Used by CI
_coverage:(_wasm)
  cargo llvm-cov --release --workspace --exclude artemis-webui --lcov --output-path lcov.info

# Build Artemis for GitHub Actions
_ci_release target:
  cargo auditable build --profile release-action --bin artemis --target {{target}}

# Build Artemis for GitHub Actions using Cross
_ci_release_cross target:
  cross build --profile release-action --bin artemis --target {{target}}

# Test the entire artemis project
test:(_wasm)
  cargo test --release

# Test the entire artemis project using nextest
nextest:(_wasm)
  cargo nextest run --release

# Just build the artemis binary
[group('workspace')]
cli:
  cd cli && cargo build --release

# Just build core library
[group('workspace')]
core:
  cd artemis-core && cargo build --release

# Review complexity with scc
complex:
  scc -i rs --by-file -s complexity

# Setup Artemis development environment for Ubuntu
[group('setup')]
setup-ubuntu: (_ubuntu)

# Setup Artemis development environment for Fedora
[group('setup')]
setup-fedora: (_fedora)

# Setup Artemis development environment for Windows
[group('setup')]
setup-windows: (_windows)

# Setup Artemis development environment for macOS
[group('setup')]
setup-macos: (_macos)