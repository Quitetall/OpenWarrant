# Checklist

- [x] Preserve actual two-control failure with exact source and receipt identities (i-f6ea) — done by agent://codex, 2026-10-14: Retained actual failed GateRun receipt, full stdout/stderr and journal with byte digests. Exact source 379acb09: 1662 passed, 2 failed; no failure erased or disposition fabricated.
- [x] Record bounded same-namespace targeted control and actual Git stderr observations (i-8cce) — done by agent://codex, 2026-10-14: Unchanged required-sources control passes in the original namespace shape with 18 bounded Git inventory observations. Expected non-Git exit 128 with no .git ancestor correctly falls back. This excludes blanket namespace failure; original underlying cause remains UNKNOWN, not repaired.
- [ ] Capture full-sequence Git failure cause and repair only the established defect without weakening checks (i-9f40)
