# RelaisDesk compatibility update (2026-09-08)

Base: crates.io phf_generator 0.8.0, MIT license retained.
Only the normalized Cargo.toml dependency on rand changes from 0.7 to 0.8.6+.
The PHF algorithm and public API remain unchanged. This removes the final
rand 0.7 dependency from the historical Tauri UI build (RUSTSEC-2026-0097).

This component uses seeded SmallRng for compile-time table generation, not
ThreadRng. The reported custom-logger/reseed path was not used here. Migrating
the dependency avoids retaining its unsafe implementation nonetheless.

Rand's SmallRng algorithm may change the generated keys. The table, key and
displacements are generated together, so lookup remains consistent. The added
test verifies the map against phf_shared's runtime lookup and determinism.
Cargo.toml.orig and .cargo_vcs_info.json preserve the original provenance;
this is a local compatibility patch, not an upstream 0.8.0 release.

Reference: https://rustsec.org/advisories/RUSTSEC-2026-0097.html
