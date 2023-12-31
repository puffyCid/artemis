# Small Justfile (https://github.com/casey/just and https://just.systems/man/en). 
# `just` is recommended. 
# Its useful when you want to run groups of tests and do not want to type the full test path
# Its very useful to prebuild WASM code before compiling the rest of artemis
# Windows users will need to use PowerShell `just --shell pwsh.exe --shell-arg -c`

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
server:(_wasm)
  cd server && cargo build --release --examples
  cd target/release/examples/ && ./start_server ../../../server/tests/test_data/server.toml

# Build the entire artemis project.
build:(_wasm)
  cargo build --release

# Run tests for code coverage. Used by CI
_coverage:(_wasm)
  cargo llvm-cov --release --workspace --lcov --output-path lcov.info

# Test the entire artemis project
test:(_wasm)
  cargo test --release

# Test the entire artemis project using nextest
nextest:(_wasm)
  cargo nextest run --release

# Just build the artemis binary
cli:
  cd cli && cargo build --release

# Just build artemis-core
core:
  cd artemis-core && cargo build --release

# Review complexity with scc
complex:
  scc -i rs --by-file -s complexity