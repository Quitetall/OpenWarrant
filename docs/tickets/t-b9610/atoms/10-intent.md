# Reject stale independent verification after its reviewed subject changes

Real public CLI reproduction on a disposable inbox fixture: ingest a synthetic independent v1 response, change OBL-001's requirement, and prepare still reports verify current 2/2. The same old response is accepted again after the contract changes. No live verdict was fabricated. Existing gates still block full resolution, but stale-verifier skipping violates the required verification loop. Core verification.rs is an active resolved pin: do not edit it without an approved correction. CLI verify.rs and resolve.rs have successor ownership. Preserve historical results; missing reviewed-subject binding is UNKNOWN, not current assurance. Use the existing canonical verification-bundle digest and actual source/evidence snapshots; no placeholder digest or inferred review from a timestamp. No paid calls.

## Notes

- **2026-10-01 01:26 UTC, codex:** Public CLI reproduces stale current 2/2 verification after changing OBL-001 and accepts the old response again. Evidence under evidence/ is explicitly synthetic and isolated; no live dispositions written. Fix reviewed-subject binding before treating prepare --all as unattended-safe.
