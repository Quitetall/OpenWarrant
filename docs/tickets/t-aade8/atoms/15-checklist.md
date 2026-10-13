# Checklist

- [ ] Capture child exit/status/stderr and establish cause without weakening the five-second checkpoint requirement (i-73f6)
- [x] Add bounded child diagnostics and cleanup; retain the original deadline and durable checkpoint semantics (i-a17e) — done by agent://codex, 2026-10-13: Test-only bounded child diagnostics and cleanup implemented. Actual missed checkpoint caught live child in Linux jbd2_log_wait_commit; original 5-second deadline and sync_all unchanged. Controlled terminal/live child checks and all 11 module replay pass; strict Clippy passes. Cause/storage repair and original item remain open.
