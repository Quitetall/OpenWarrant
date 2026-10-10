# Deliver OW-WAR-0148 implementations at the signed file paths

The existing authorization and delivery declarations specify state_cmd.rs, 76-ticket-features.sh and 77-typed-demo.sh. Move the actual implementations to those locations rather than changing signed scope or inserting stubs. Preserve old paths through Git rename history; retain the unused amendment analysis. Run preparation dry-run and existing state/ticket/demo checks.

## Notes

- **2026-10-08 21:54 UTC, codex:** Moved actual implementations to the existing signed target paths, retaining exact file bytes and the public states module via a Rust path attribute. All authorization, declaration, work-order and assurance source hashes match the recorded prestate. Scoped preparation dry-run passes with no deliver.missing. Fresh module compilation and full gate remain pending; no contract amendment or human act.
