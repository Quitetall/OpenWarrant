# Checklist

- [x] Reproduce interrupted and late header/body reads, retain early-EOF and size controls, patch the reader, and run full library/Clippy/format checks (i-308f) — done by agent://codex, 2026-10-23: Observed red/green interrupted header/body and late-read controls. All14 web UI tests and550 library tests pass, including the original failing socket test; strict all-target/schema Clippy and fmt pass. Bounds and original expectations unchanged. Original CI syscall cause remains UNKNOWN; protected full CI must rerun. Claimed unverified implementation only.
