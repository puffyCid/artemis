# artemis

artemis is a powerful command line digital forensic and incident response (DFIR)
tool that collects forensic data from Windows, macOS, and Linux endpoints. Its
primary focus is: speed, ease of use, and low resource usage.\
Notable features _so far_:

- Setup collections using basic TOML files
- Parsing support for large amount of forensic artifacts (25+)
- Output to JSON or JSONL file(s)
- Can output results to local system or upload to cloud services.
- Embedded JavaScript runtime via [Deno](https://deno.land/)
- Can be used as a library via
  [artemis-core](https://puffycid.github.io/artemis-book/core/overview.html)
- MIT license

Checkout the online guide at https://puffycid.github.io/artemis-book for indepth
walkthrough on using artemis

## Quick Guide

1. Download the latest release binary from GitHub
2. Download an
   [example](https://github.com/puffyCid/artemis/tree/main/artemis-core/tests/test_data)
   TOML collection
3. Execute artemis using a provided TOML file with elevated privileges
4. Review the output

```
artemis -t processes.toml
[artemis] Starting artemis collection!
[artemis] Finished artemis collection!

puffycid> ls -R
process_collection

./process_collection:
692f6c76-8312-472f-8005-2a3ecd2203f9.jsonl	d97b86bb-a762-4bae-b8e8-16dad8708fa4.log	status.log
```
