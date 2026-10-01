# Checklist

- [x] Reject contract, stage, and attempt mismatches through war submit without writing a submission (i-9eba) — done by codex, 2026-10-01: CLI tests and real control pass: wrong contract/stage/attempt refused without writes; missing bindings UNKNOWN; exact submission accepted and replayed without writes. Unverified implementation, not independent assurance.
- [x] Add planted controls and retain scoped CLI evidence (i-7d65) — done by codex, 2026-10-01: Actual selected battery: 42 passed, 0 failed, in disposable clone. Covers idempotency, service execution, three binding refusals, missing metadata UNKNOWN, and exact replay. CLI 3/3 and Clippy pass. Unverified scoped evidence; combined full integration gate pending.
