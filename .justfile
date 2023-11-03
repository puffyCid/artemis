# Small Justfile (https://github.com/casey/just and https://just.systems/man/en). 
# `just` is optional. Useful when you want to run groups of tests and do not want to type the full test path
# Windows users will need to use PowerShell `just --shell powershell.exe --shell-arg -c`

# Run cargo clippy on artemis project 
default:
  cargo clippy

_test target:
  cargo test --release {{target}}

# Test only the ESE parsing functions
ese: (_test "artifacts::os::windows::ese")

# Test only the ShellItems parsing functions
shellitems: (_test "artifacts::os::windows::shellitems")

# Test only the JavaScript runtime
runtime: (_test "runtime::")

# Test only the FileSystem functions
filesystem: (_test "filesystem::")

# Test all the Windows artifacts
windows: (_test "artifacts::os::windows")

# Test all the macOS artifacts
macos: (_test "artifacts::os::macos")

# Test all the Linux artifacts
linux: (_test "artifacts::os::linux")

# Test all the Unix artifacts
unix: (_test "artifacts::os::unix")

# Compile WASM and server code then start the server
server:
  cd webui && trunk build --release
  cd server && cargo build --release
  cd target/release/examples/ && ./start_server ../../../server/tests/test_data/server.toml