# OW93 reference workbench

Implements a separate Python standard-library workflow package over the real
OpenWarrant SDK CLI. It authors persistent unverified drafts, preserves revisions,
refuses stale/concurrent writes, and serves a browser editor plus project inventory.
The Rust `war progress --snapshot --json` seam exposes the canonical viewer's
validated records; the app does not duplicate report or evidence-path validation.

Eight HTTP tests and five Rust viewer tests passed, including process restart,
field-only corruption, unauthorized input, symlink files and competing writers.
Independent Spec and Standards reviews passed after two confirmed defects were
fixed: editable fields now share a complete stored-payload checksum; project
completion counts now consume the existing canonical report validator.

Browser QA at loopback8766 created a synthetic Warrant, saved revisions1/2,
inspected revision1, then saved revision3 after the integrity fix. Original
pre-fix synthetic records remain unchanged in `/tmp/ow93-runtime/state`; their
exact payloads were copied into `/tmp/ow93-runtime/state-v1` for the final QA.
This is a development fixture migration, not shipped migration support or actual-
user study evidence. No repository Warrant was created by the browser action.

See [application/API guide](../../../../apps/openwarrant-web/README.md),
[reconciliation queue](../../../releases/warrant-reconciliation.md), and the
[full machine inventory](../../../releases/warrant-reconciliation-inventory.json).
All89 active records are inventoried; no historical resolution, signature,
retirement, reconciliation acceptance or Verified mark is fabricated.

Full repository gate is pending for this source change. Agent execution, hotline,
work-stop orchestration, real adapters and actual-user evidence are later slices.
