---
schema: oh.war/atom/v1
warrant_uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/bundle.rs`: each `gate_runs` entry gains
   `stdout` and `stderr` objects: `captured`, `sha256`, `bytes`,
   `truncated`, `text` (the tail within `max_excerpt_bytes`), and
   `mismatch` when the receipt records a digest for that stream and it
   differs. Deterministic ordering is kept.
2. `conformance/plants.d/47-bundle-output.sh`, on a scratch corpus with a
   recorded gate run.

## Frozen Surfaces

Every record schema other than the bundle's additive fields; the gate
runner; receipts; `claude-verifier.sh`.

## Autonomy and Escalation

Tier T2. Escalate rather than decide any change to what counts as
admissible evidence.

## Rollback

Revert `bundle.rs`. Bundles already written stay as history.
