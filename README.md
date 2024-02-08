# artemis

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![codecov](https://img.shields.io/codecov/c/github/puffyCid/artemis?style=for-the-badge)](https://codecov.io/github/puffyCid/artemis)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/puffycid/artemis/nightly.yml?style=for-the-badge)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/puffycid/artemis/audit.yml?label=Audit&style=for-the-badge)

artemis is a powerful command line digital forensic and incident response (DFIR)
tool that collects forensic data from Windows, macOS, and Linux endpoints. Its
primary focus is: speed, ease of use, and low resource usage.\
Notable features _so far_:

- Setup collections using basic TOML files
- Parsing support for large amount of forensic artifacts (25+)
- Output to JSON or JSONL file(s)
- Can output results to local system or upload to cloud services.
- Embedded JavaScript runtime via [Deno](https://deno.land/)
- Can be used as a library
- MIT license

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
  filelisting        Pull filelisting
  systeminfo         Get systeminfo
  firefoxhistory     Parse Firefox History
  chromiumhistory    Parse Chromium History
  firefoxdownloads   Parse Firefox Downloads
  chromiumdownloads  Parse Chromium Downloads
  shellhistory       Parse Shellhistory
  cron               Parse Cron Jobs
  sudologs           Grab Sudo logs
  execpolicy         Parse ExecPolicy
  users              Collect local users
  fsevents           Parse FsEvents entries
  emond              Parse Emond persistence. Removed in Ventura
  loginitems         Parse LoginItems
  launchd            Parse Launch Daemons and Agents
  groups             Collect local groups
  safarihistory      Collect Safari History
  safaridownloads    Collect Safari Downloads
  unifiedlogs        Parse the Unified Logs
  help               Print this message or the help of the given subcommand(s)

Options:
      --format <FORMAT>  Output format. JSON or JSON [default: json]
  -h, --help             Print help


> artemis acquire processes
```

You can also run collections using TOML files or JavaScript code!

The online documentation contains in depth overview of using artemis
