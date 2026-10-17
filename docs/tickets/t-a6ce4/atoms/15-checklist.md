# Checklist

- [x] Preserve schema negotiation cases and measure the failing command (i-15ba) — done by agent://codex, 2026-10-17: Preserved baseline and optimized schema-negotiation results; six paired outputs identical. Original CPU/safety block passed without changing 15000 ms or 3.4 bounds.
- [x] Avoid unnecessary full TOML parsing for nonfuture schema headers (i-9f01) — done by agent://codex, 2026-10-17: Committed conservative first-root-schema fast path (09d4a888); newer known versions and unusual headers retain full TOML parsing. Baseline/optimized public schema cases pass; paired JSON outputs unchanged. Integration and required CI remain separate pending work.
- [ ] Rerun unchanged next/status CPU control and integrate only after required checks (i-8836)
