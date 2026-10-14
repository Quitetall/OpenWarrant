# Checklist

- [x] Preflight exact mutable manifests and retain original versions (i-d65a) — done by agent://codex, 2026-10-14: Preflighted 39 mutable manifests: no resolution or correction chain, regular local static artifacts, unique concrete digest occurrences. Exact original manifest bytes retained under unresolved-digest-refresh/previous with original structure.
- [x] Use pins refresh for selected unresolved manifests; preserve all non-digest metadata (i-2c23) — done by agent://codex, 2026-10-14: Observed 187 current-byte digest updates through pins.refresh in 39 unresolved manifests. Full parsed metadata comparison and exact textual comparison confirm all producer, attempt, contract, target and other fields preserved. All 29 resolved manifest hashes unchanged. No approval, verification or production attribution inferred.
- [ ] Recompile, verify record integrity and retain a committed checkpoint (i-27da)
