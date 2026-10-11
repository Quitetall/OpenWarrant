# Dependency measurement, 2026-10-11

PR #216 updates six direct dependencies and their locked transitive graph.
OW-ADR-0014 requires graph and license measurements when the exact MCP pin changes.
Its historical evidence remains unchanged. This is a new measurement, not an ADR
amendment, independent verdict, human acceptance or release qualification.

| Measurement | Current main graph | PR #216 graph |
| --- | --- | --- |
| Unique packages in default-host normal graph | 161 | 160 |
| Cargo deny license expressions | 14 | 13 |
| Unlicensed packages | 0 | 0 |
| `cargo deny --locked check licenses` | PASS | PASS |

No license expression was added. BSD-2-Clause was removed. `arrayref` is the
only removed package name; no package name was added. These counts are
current observations, not a comparison with the ADR's older 86/108-crate graph.
Cargo deny examines its configured graph; the normal tree uses this Linux host
and the workspace's defaults. Optional feature and native-host qualification
remain separate requirements. No license exception was added.

`comparison.json` pins commits, graph/config file hashes, exact commands, package
sets and uncompressed log hashes. Baseline commands ran at b8cc7ff1: Git verified
Cargo.toml, Cargo.lock and crates/openwarrant-cli/Cargo.toml byte-identical to
main 9b73d834. Candidate commands ran at ec5bdaac. Both use Rust 1.97.1 and
cargo-deny 0.20.2. Compressed stdout/stderr preserve the observations, including
existing warnings. The initial candidate offline tree could not run because
rmcp 3.5.1 was not cached. The subsequent locked online tree fetched it and
completed successfully; an unavailable cache was not labeled a passing check.

The full protected-main gate for the candidate remains a separate merge condition.
