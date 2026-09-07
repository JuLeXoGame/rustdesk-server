# RelaisDesk atty compatibility adapter

This is new, safe compatibility code, not the upstream atty implementation.
It preserves the public `Stream`, `is` and `isnt` API needed by clap,
env_logger and flexi_logger, and delegates to Rust's `std::io::IsTerminal`.
No unsafe code, Windows pointer casts, libc, winapi or custom allocation remain.

This addresses GHSA-g98v-hv3f-hcfr / RUSTSEC-2021-0145. Upstream atty is
unmaintained (RUSTSEC-2024-0375); this adapter requires Rust 1.70+.
Its 0.2.14 compatibility version is deliberately retained: this is not a
new upstream release. Version-only scanners may still flag it; verify the
path patch and implementation rather than dismissing registry atty globally.

Terminal recognition follows the standard library, including returning false
when a stream is redirected or cannot be identified as a terminal.
Tests cover API parity and child-process pipes/null input on Windows and Linux.

Reference: https://rustsec.org/advisories/RUSTSEC-2021-0145.html
