# Contributing Guidelines

First, thanks for expressing possible interest in contributing to artemis!

The following document is a summary guidelines for contributing to artemis. The
artemis book contains a more detailed information on how to start contributing
to artemis.

## Reporting Issues

If you encounter an issue running artemis please open an Issue. To best help
figure out your issue, the more details you include the better.\
For example, the following would be a poor issue:

- `Issue: artemis crashed on Windows. <Insert crash details>`

A better way to write the issue above would be:

- `Issue: artemis crashed while parsing Windows Event Logs on Windows 11. <Insert crash details>`
- Additional details that could also be helpful related to the issue above:
  - Explict Windows version. Ex: Windows 11 22H2
  - The specific Event Log file such as Security.evtx
  - Size of Event Log

If you can provide the artemis log file `<uuid>.log` that could also be very
useful to figuring out an artemis issue. **PLEASE** make sure you review the log
file before providing it to make sure you are comfortable sharing its contents.
If you want to share the log file privately, please mention that in the issue
and we can figure something out.

There is no discussion group for artemis so if you have a question. Please open
an issue.

## Contributing

If you want to add a new feature or fix a bug, below are some key point to think
about:

1. Before you work on your feature or bug fix **please** open an issue!. This is
   to ensure that everyone knows if someone else is working a specific feature.
2. Try to limit external dependencies. Rust has a huge ecosystem of really cool
   third-party crates and cargo makes it really easy to include and compile
   external dependencies in your project.
   \
   A small consequence of this is the number of dependencies in a Rust project
   can explode very quickly depending on the third-party crate that is added.
3. Usage of unsafe is **not** allowed. Exceptions may be granted for limited
   edge case scenarios.
4. Try to avoid system APIs. When possible artemis should try to avoid calling
   system APIs. Similar to unsafe, exceptions may allowed in **some** scenarios:
   - Volatile artifact such processes or network connections. These artifacts
     only exist in memory and typically require OS APIs to parse them
   - Complex proprietary formats such compression or encryption. These formats
     are extremely complex and could be very difficult to implement natively in
     Rust.
5. Shelling out to other processes or tools is not allowed.
6. Submitting data to third-party sites (ex: VirusTotal) is not allowed.
