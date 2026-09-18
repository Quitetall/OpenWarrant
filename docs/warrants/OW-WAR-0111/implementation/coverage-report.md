# Archive coverage diagnostics

`war archive export` and `war archive inspect` now return the archive's exact
coverage declarations and a sorted `unavailable_categories` list in their JSON
result. Text output names unavailable categories. Inspection after source removal
retains the same report; it does not infer completeness from reconstructed IR.

These are source declarations, not independent verification. The original diagnostic slice reported `complete: false` for its incomplete
sources. The current assembler computes completeness from checked coverage and
reports `completeness_scope: selected-local-current-and-history`. Complete local
archives can import; unresolved historical/provider categories still block import.
See `declared-context-coverage.md` and `kf-source-complete-roundtrip.md` for later
source-complete observations.
No archive wire bytes, authority, signature or assurance disposition changes.

Validation: ten preservation integration tests passed, including equality between
export and source-detached inspect reports, retained canonical IR and unavailable
artifact classification. All-target CLI Clippy with warnings denied passed.
Rust toolchain: repository pin 1.97.1. Full release qualification remains open.
