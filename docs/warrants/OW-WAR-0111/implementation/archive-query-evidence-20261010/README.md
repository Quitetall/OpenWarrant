# Read-only archive queries with external records

Unverified OW-WAR-0111 implementation. `war archive runtime-basis <archive> --evidence <directory>` resolves separately retained records by their declared SHA-256 addresses. Every actual byte is checked, with aggregate limits and the existing descriptor-relative reader. Missing, changed or linked evidence refuses. All-embedded callers need no evidence directory.

The SDK `Archive::resolve_records` supplies exact bytes for queries while preserving unavailable category coverage. Whole import still calls `reconnect` and refuses unavailable required coverage before writing a destination. Neither path establishes native provider authentication or activates imported authority.

The public CLI regression failed on the old unsupported option, then passed with the new resolver. Source/configuration/native receipt inputs were removed; separately retained records reproduce the embedded query's contract, stage inventory and capture inventory. Controls refuse missing evidence, altered bytes, symlinks and an incomplete whole import.

A real retained BLUT native receipt archive was converted to 32 external records and queried from a directory without repository sources. Its contract, stage and provider inventories exactly match the earlier embedded query. Historical IR omissions remain visible; both retained native observations remain UNKNOWN. This does not make the native receipt's separate verifier inputs complete or qualify native trust. Native runtime coverage, historical reconstruction, actual KF native roundtrip and independent/human qualification remain open.

Validation: two public archive controls passed; all-target/all-feature CLI/compiler Clippy with warnings denied passed. Additional preservation checks and generated integrity are recorded when terminal. Rust toolchain 1.97.1. These observations are implementation evidence, not an independent verdict.
