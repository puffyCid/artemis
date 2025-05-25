# Artemis architecture

This document provides a very high level overview of the artemis code
architecture. Inspired by
https://matklad.github.io//2021/02/06/ARCHITECTURE.md.html

A more detailed overview can be found at
https://puffycid.github.io/artemis-api/docs/Contributing/overview

### Artemis repo structure

The artemis repository is composed of multiple workspaces

- `cli/` - Contains the code the powers the CLI application
- `core/` - Contains the code related to all forensic parsers and the Boa (JS)
  runtime
- `common/` - Collection of structs shared between workspaces
- `timeline/`- Contains the code related to timelining supported artifacts

### Core structure

The `core` crate (also sometimes referred to as `artemis-core`) is primarily
grouped by forensic artifacts based on the OS.

- `artifacts/` - Contains all the code associated with parsing forensic
  artifacts. The bulk of artemis code is located here. It is further broken down
  by OS.
- `filesystem/` - Contains helper functions to access the filesystem
- `output/` - Code related to outputting the forensic artifacts
- `runtime/` - Code related to the Boa (JS) runtime
- `structs/` - Collection of structs used by `core` crate
- `utils/` - Contains misc helper functions
