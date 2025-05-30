# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html),
and is generated by [Changie](https://github.com/miniscruff/changie).


## v0.14.0 - 2025-05-24
### Added
* Support for OS distribution packages (ex: MSI or RPM)
### Fixed
* Bug where artemis would not compress csv files
* Issue where artemis would not timeline wmipersist or srum entries
* Bug where artemis improperly handled MFT entries with a size of zero
### Dependencies
* Removed tauri
### ArtemisApi
* Support for timelining additional artifacts
* More URLs added to Unfurl
* Support for additional iOS artifacts
* Extract browser data from GNOME Epiphany
* Support for newer Windows Update History sqlite db
* Added background activities manager parser
* Support for parsing SSH known_hosts file
* Support for assembling PowerShell ScriptBlocks
* Extract Windows Firewall Rules from Registry
* Support for parsing VSCode recently opened files and folders

## v0.13.0 - 2025-03-30
### Added
* Support for more platforms! Ex: FreeBSD, NetBSD, Android!!, Linux musl, and more!
* Support for collecting network connections!
### Changed
* Improved BITs carving support
* Moved to Rust 2024 edition
* Faster WMI parsing
* Replaced Deno with Boa for JS runtime
### Fixed
* Updated user registry file regex to capture lowercase filenames
* Bug where artemis would not get all ESE rows
* Issue where artemis would not parse alt WMI directories
### Dependencies
* Made Yara-X an optional dependency (enabled by default)
* Add lumination library to collect network connections
### ArtemisApi
* Exposed rest of Outlook parser to API
* Support for additional Firefox artifacts

## v0.12.0 - 2025-02-09

### Added

- Timelining support
- Apollo project (a Tauri GUI application)
- GZIP compression option to CLI
- MFT parsing support!
- Option to provide alternative $MFT path when parsing UsnJrnl

### Changed

- Lowered memory usage by reducing clones and streamlining output workflow
- Improved handling of extracting strings from bytes
- Reduced memory usage when pulling a process listing and enabling binary
  parsing
- Reduced memory usage when parsing fsevents
- Huge speed increase when parsing UsnJrnl

### Fixed

- Journal parsing bug where artemis would not parse all entries
- Search bug where artemis would treat alt-files as ESE db instead of sqlite

### Dependencies

- Updated all dependencies
- Add sunlight protobuf parser and URL parser

### ArtemisApi

- Support for parsing GNOME Application usage
- Support for parsing GNOME Extensions
- Support for parsing gedit recently opened files
- Support for parsing GVFS metadata files
- Support for listing installed Snap packages
- Unfold URL parsing. Inspired by the Unfurl project
- Support for parsing Safari cookies
- Initial support for parsing unencrypted iTunes/iOS backups!
- Support for extracting WordWheel Registry entries

## v0.11.0 - 2024-11-05
### Added
* Outlook OST parser!
* CSV output support
* Support for providing custom output directory in when using cli
* Option to include template strings when parsing EventLogs
### Changed
* Reduced memory usage of eventlogs parser
* Improved ESE parsing speed
* Prefetch version 31 supported
* Additional minor updates
### Fixed
* Panic in huffman decompression code when running with Rust 1.81
### Dependencies
* Updated all dependencies to latest versions
### New Contributors
* maxspl

## v0.10.0 - 2024-07-21
### Added
* Exposed macOS bookmark parsing to JS runtime
* Support for parsing Archive ShellItems added in Windows 11
* Support for uploading files to AWS
* Support for uploading files to Azure
* Linux ARM support!
* Embedded Software Bill of Materials into release binaries via cargo auditable
### Changed
* Major improvements to the ESE parser
* Improvements to the macOS loginitem artifact
* Migrated to ISO8601 RFC 3339 timestamps for artifacts
* Major updates to client and server code
* Added timestamps to macOS FsEvents and Launch artifacts
* Ability to filter filelistings using yara rules!
* Improved compiled binary performance via cargo LTO
### Fixed
* Incorrect args to users and groups artifacts
* Path value not getting populated for processes artifact
### Dependencies
* Updated all dependencies
* Added Yara-X
### ArtemisApi
* Support for looking up software EOL status via https://endoflife.date
* Support for looking up browser extension reports on https://crxcavator.io
* Support for circlu Hashlookup service
* Support for parsing Microsoft Office MRU entries
* Support for parsing macOS Gatekeeper entries
* Initial OneDrive parser support
* Extract service install entries from Windows EventLog
* Extract logon entries from macOS UnifiedLog

## v0.9.0 - 2024-05-08
### Added
* Support for parsing version 3 of fsevents
* Zlib decompression support
* Initial code for artemis client
* Initial script for macOS app sigining
### Changed
* Improved JS HTTP client
### Fixed
* Processes not containing args or env values
* Issue where artemis would parse a URI shellitem as a ZIP shellitem
* Issue where artemis-api would not return all sqlite results
* Removed some improper async code in JS runtime
### ArtemisApi
* Initial support for Timesketch!!
* Initial support for timelining artifacts!
* Experimental Protobuf parser
* Experimental macOS BIOME parser
* Extract macOS Lulu info
* Extract macOS Munki application usage info
* Experimental support for parsing Windows Defender signatures
* Extract Chromium DIPS info
* Extract macOS Quarantine Events
* Extract Chromium Preferences
* Initial support for acquiring files
* Started adding tests that run via GitHub Actions

## v0.8.0 - 2024-03-18
### Added
* Support for querying any SQLITE database via artemis API
* macOS Spotlight parser!
* Optional args to all Linux artifacts
* Windows XPRESS decompression support without API calls. Code from https://github.com/ForensicRS/frnsc-prefetch project (MIT)
### Changed
* Updates to webui
* Made most Windows artifacts use alt_file or alt_dir arguements. Removed alt_drive options for most artifacts
* Combined all supported forensic artifacts. Can parse all supported forensic artifacts on any OS that can run artemis
### Fixed
* Issue where artemis would fail to parse NTFS $SDS file data
### Dependencies
* Updated all dependencies
### ArtemisApi
* Support for querying macOS TCC.db files
* Support for parsing RPM sqlite database
* Updated UnifiedLog macOS support
* Support for querying Chromium Cookies database
* Support for querying Chromium Autofill database
* Support for querying Firefox Cookies database
* Support for parsing Chromium bookmarks
* Support for parsing VSCode extensions
* Parse some macOS Xprotect entries

## v0.7.0 - 2024-02-08
### Added
* Optional parameters for all macos artifacts
* WebUI improvements
* Insomnia config for server interaction
* Support for parsing ShellItems from JS runtime
* Support for extracting UTF16 strings to JS runtime
* Added cargo deny workflow to github actions
* Support for FILETIME timestamps in ESE databases
* WMI parsing!
### Changed
* Moved sudo logs into macos and Linux artifacts. Instead of Unix artifacts
### Fixed
* Server fixes and improvements
### Dependencies
* Updated all dependencies
### Tests
* BITS benchmarking test
* Improved test speed for firefox and chromium JS tests
### ArtemisApi
* BOM parsing support
* Support for parsing multiple MRU Registry keys
* Support for getting macOS System Extensions
* User Access Log (UAL) parsing support for Windows servers!

## v0.6.0 - 2023-12-02
### Added
* Initial idea for WASM webUI
* Just tool now recommended to build artemis
* Support for Registry Security Keys
* Cargo deny file
### Changed
* Better support for macOS loginitems
* Made folder description lookups optional for userassist entries
* Improved artifact bindings to JS runtime
### Fixed
* Error when parsed ESE tables did not return all data
* Incorrect ESE timestamps
### Dependencies
* Updated to latest versions
### ArtemisApi
* Added HTTP client for JS runtime
* Added command execution to JS runtime
* Basic support for VirusTotal lookups!
* Can now parse and dump table(s) in ESE dbs
* Retrieve installed homebrew packages and casks
* Retrieve installed deb packages
* Retrieve installed Chocolatey packages
* Parse history of Windows Updates
* List joined Wifi networks on macOS
* Get Windows PowerShell history

## v0.5.0 - 2023-10-30
### Added
* Server upload support for compressed jsonl data. Also more async code.
* Support for collecting artifacts using command args. Example: `artemis acquire processes`
* Simple support for just command runner
### Dependencies
* Removed redb
* Updated all dependencies to latest versions
### ArtemisApi
* Lots of features added to API: LibreOffice and VSCode file history, macOS Firewall status, macOS App listing, and so much more!
* New documentation website!: https://puffycid.github.io/artemis-api

## v0.4.0 - 2023-09-14
### Added
* Basic support for Windows PropertyStores
* Exposed several nom parsers to JavaScript (Deno) runtime
* Recycle Bin parser
* Initial idea for embedded server
* Support for parsing all Windows shortcut (LNK) extra properties
* Initial benchmarking tests
* Linux logon parser
### Changed
* Github Actions support for macOS AMR binaries in nightly and stable relases
### Fixed
* Added some error handling when calling JS runtime functions
* Bug when parsing ESE pages and not parsing the last page
### Dependencies
* Updated dependencies to latest version
* Added axum and redb for server and database storage
* Added xml2json-rs crate for better xml to json parsing

## v0.3.0 - 2023-08-14
### Added
* Async deno scripts support
* Support for parsing Windows Schedule Tasks
* Deno bindings for globbing and reading XML files to JSON
* Windows Services parsing support
* Support for executing JavaScript file directly
* Nightly releases
* Basic support for parsing OLE data
* Support for parsing Windows Jumplists
### Changed
* Overhauled deno scripting runtime
### Fixed
* String extraction on UTF16 vs UTF8 (ASCII) Registry values
* Bug when extracting BigData cells and multiString value data from Regsitry
### Dependencies
* Removed `deno_runtime`
* Update all dependencies
* Added glob crate for globbing support
* Added quick-xml crate for parsing XML files

## v0.2.0 - 2023-07-12
### Added
* Initial Linux support. Supports filelisting, processes, systeminfo, cron, shellhistory, chromium, firefox, and ELF binary artifacts
* Initial remote upload support for: GCP, Azure, and AWS
* Support for setting logging level from TOML input. error, warn, info, debug are supported
* Support for parsing ExecPolicy db on macOS
* Support for programatically outputting data through artemis via Deno runtime
* Journal parsing support on Linux
* Sudo log parser support for macOS and Linux
### Changed
* Minor improvements to filelisting when PE or MACHO parsing is enabled
* Release binaries are now stripped
* Faster ESE parsing
### Fixed
* Possible array out bounds error when trying to get browser user info
* Dont throw error if artemis can not carve out BITS Job info
* Additional fixes and enhancements
* Duplicated ESE values when parsing branched data
### Dependencies
* Updated all dependencies
* Added rusty-s3, jsonwebtoken, reqwest for remote upload support. elf for ELF parsing
* Added ruzstd to decompress Journal data
* Added lz4_flex for decompressing older Journal files
* Added xz2 for decompressing older Journal files
### Tests
* Enabled additional tests
