# Archive coverage diagnostics

`war archive export` and `war archive inspect` now return the archive's exact
coverage declarations and a sorted `unavailable_categories` list in their JSON
result. Text output names unavailable categories. Inspection after source removal
retains the same report; it does not infer completeness from reconstructed IR.

These are source declarations, not independent verification. Export still reports
`complete: false`; unresolved historical/provider categories still block import.
No archive wire bytes, authority, signature or assurance disposition changes.

Validation: ten preservation integration tests passed, including equality between
export and source-detached inspect reports, retained canonical IR and unavailable
artifact classification. All-target CLI Clippy with warnings denied passed.
Rust toolchain: repository pin 1.97.1. Full release qualification remains open.
