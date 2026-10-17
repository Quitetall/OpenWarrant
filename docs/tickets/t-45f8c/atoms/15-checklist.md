# Checklist

- [x] Exercise actual preview shell with checked fixture assets and refusal controls (i-43c1) — done by agent://codex, 2026-10-16: Observed RED: actual workflow shell omitted installer from upload. After patch, all 29 release tests pass; missing installer refuses before upload or edit. No live publication attempted.
- [x] Include source installer and digest in alpha upload; integrate after checks (i-89b5) — done by agent://codex, 2026-10-17: Preview workflow installer/checksum inclusion landed in main395168 via PR312 after protected gate PASS. Earlier29 release workflow tests execute the actual shell with Git/GitHub fixtures, including missing-installer refusal. This completes code integration; no new live publication through that workflow is claimed.
