# RelaisDesk security backport (2026-09-08)

Base: unmodified crates.io glib 0.15.12, with its MIT LICENSE and COPYRIGHT retained.
The compatibility version remains 0.15.12; no upstream release is being invented.

Only the two-line `VariantStrIter::impl_get` fix from gtk-rs/gtk-rs-core
PR #1343 is backported: use `let mut p` and pass `&mut p` to the variadic
C function. This fixes RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g without changing
the GTK3 API required by this application.

Upstream patch:
https://github.com/gtk-rs/gtk-rs-core/pull/1343
Merge commit: 05dff0ee696f9bcd8617cd48c4b812d046d440cb
Advisory: https://rustsec.org/advisories/RUSTSEC-2024-0429.html

The added regression test covers forward, reverse and mixed iteration,
nth, nth_back, last and exhaustion, including UTF-8 strings. Run it in release
mode, because optimization exposed the original undefined behaviour.

Registry glib <0.20 remains affected. A scanner reporting only the retained
version may still alert: the path patch and regression tests are required
evidence, not a blanket exception for that version.
