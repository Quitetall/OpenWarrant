# Checklist

- [x] Measure bundle sizes across the wave; find what dominates (i-c18e) — done by claude, 2026-09-26: sizes per Warrant in the ticket note; deliverables 68%, gate output 22%, atoms 4%
- [x] Bound the bundle (per-obligation, or excerpts with whole-file digests), keeping it blind and deterministic (i-a20b) — done by claude, 2026-09-26: per-obligation bundles over [verify] max_bundle_tokens, excerpts with whole-file digests, obligation_evidence; plant 84-bundle-budget.sh
- [x] Every wave Warrant verifies within the timeout (i-3071) — done by claude, 2026-09-26: all 37 wave Warrants <= 48k tokens per bundle; largest three verified in 11-23 s each; 0112 end to end 129 s
