# Artemis

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![codecov](https://img.shields.io/codecov/c/github/puffyCid/artemis?style=for-the-badge)](https://codecov.io/github/puffyCid/artemis)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/puffycid/artemis/nightly.yml?style=for-the-badge)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/puffycid/artemis/audit.yml?label=Audit&style=for-the-badge)

Artemis is a powerful command line digital forensic and incident response (DFIR)
tool that collects forensic data from Windows, macOS, and Linux endpoints. Its
primary focus is: speed, ease of use, and low resource usage.\
Notable features _so far_:

- Setup collections using basic TOML files
- Parsing support for large amount of forensic artifacts (25+)
- Output to JSON, JSONL, or CSV file(s)
- Can output results to local system or upload to cloud services.
- Embedded JavaScript runtime via [Boa](https://boajs.dev)

Checkout the online guide at https://puffycid.github.io/artemis-api for indepth
walkthrough on using artemis

## Quick Guide

1. Download the latest stable release binary from GitHub. Nightly versions also
   [available](https://github.com/puffyCid/artemis/releases/tag/nightly)
2. Run artemis!

```
artemis -h
Usage: artemis [OPTIONS] [COMMAND]

Commands:
  acquire  Acquire forensic artifacts
  help     Print this message or the help of the given subcommand(s)

Options:
  -t, --toml <TOML>              Full path to TOML collector
  -d, --decode <DECODE>          Base64 encoded TOML file
  -j, --javascript <JAVASCRIPT>  Full path to JavaScript file
  -h, --help                     Print help
  -V, --version                  Print version
```

An example to example collect a process listing on macOS

```
> artemis acquire -h
Acquire forensic artifacts

Usage: artemis acquire [OPTIONS] [COMMAND]

Commands:
  processes          Collect processes
  connections        Collect network connections
  filelisting        Pull filelisting
  systeminfo         Get systeminfo
  prefetch           windows: Parse Prefetch
  eventlogs          windows: Parse EventLogs
  rawfilelisting     windows: Parse NTFS to get filelisting
  shimdb             windows: Parse ShimDatabase
  registry           windows: Parse Registry
  userassist         windows: Parse Userassist
  shimcache          windows: Parse Shimcache
  shellbags          windows: Parse Shellbags
  amcache            windows: Parse Amcache
  shortcuts          windows: Parse Shortcuts
  usnjrnl            windows: Parse UsnJrnl
  bits               windows: Parse BITS
  srum               windows: Parse SRUM
  users-windows      windows: Parse Users
  search             windows: Parse Windows Search
  tasks              windows: Parse Windows Tasks
  services           windows: Parse Windows Services
  jumplists          windows: Parse Jumplists
  recyclebin         windows: Parse RecycleBin
  wmipersist         windows: Parse WMI Repository
  outlook            windows: Parse Outlook messages
  mft                windows: Parse MFT file
  execpolicy         macos: Parse ExecPolicy
  users-macos        macos: Collect local users
  fsevents           macos: Parse FsEvents entries
  emond              macos: Parse Emond persistence. Removed in Ventura
  loginitems         macos: Parse LoginItems
  launchd            macos: Parse Launch Daemons and Agents
  groups-macos       macos: Collect local groups
  safari-history     macos: Collect Safari History
  safari-downloads   macos: Collect Safari Downloads
  unifiedlogs        macos: Parse the Unified Logs
  sudologs-macos     macos: Parse Sudo log entries from Unified Logs
  spotlight          macos: Parse the Spotlight database
  sudologs-linux     linux: Grab Sudo logs
  journals           linux: Parse systemd Journal files
  logons             linux: Parse Logon files
  help               Print this message or the help of the given subcommand(s)

Options:
      --format <FORMAT>          Output format. JSON or JSONL or CSV [default: JSON]
      --output-dir <OUTPUT_DIR>  Optional output directory for storing results [default: ./tmp]
      --compress                 GZIP Compress results
      --timeline                 Timeline parsed data. Output is always JSONL
  -h, --help                     Print help



> artemis acquire processes
```

You can also run collections using TOML files or JavaScript code!

The online documentation contains in depth overview of using artemis
