---
schema: oh.war/atom/v1
warrant_uuid: 01a0b24f-8786-7331-9380-a5a780410433
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

- Repair `crates/openwarrant-cli/src/telemetry.rs` unsupported claims.
- Public CLI regression in `crates/openwarrant-cli/tests/telemetry_integrity_cli.rs`.
- Preserve prior content digests in unresolved OW39/OW41 delivery provenance.
- Retain original diagnostic observation, corrected evidence and failed/successful checks under OW41; link this follow-up.

## Frozen Surfaces

Historical baseline, signed contracts, judgments, resolutions, schema and metric names stay unchanged. Existing `not_measurable_yet` representation suffices.

## Autonomy and rollback

Prompt-authorized unverified execution. No model calls or signatures. Required regression and full gate; revert unaccepted code if needed without deleting history.
