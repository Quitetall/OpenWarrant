# A whole-Warrant verification bundle cut a long gate stream to its tail, so the Warrant's own plant section (printed early) never reached the blind verifier

## Notes

- **2026-09-27 07:25 UTC, claude:** Run 3 (fde64f4d): ~15 of 45 not-established obligations in the wave said the plant section was not shown — 0118 OBL-001..003, 0120 OBL-001..003, 0144 OBL-001..003, and partials. 0118's bundle was warrant scope: stdout 109934 bytes carried as its last 65486, section at line 211. Fix: warrant scope excerpts streams like obligation scope (tail + sections + terms + FAIL/ERROR); heads include headers echoed by delivered plant files. Unit tests: excerpt keeps a far section, tail-only does not, non-matching head offers nothing; header reader skips expansions. Real 0118 bundle: section and 'a timed human step' carried, 30288 tokens.
