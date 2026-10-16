# Checklist

- [x] Align release gate timeout and version preview with truthful release notes (i-35d1) — done by agent://codex, 2026-10-15: Prepared alpha.4 CLI manifest/lock version and truthful notes; release gate now matches CI at 75 minutes. Preview validation 4 tests and distribution 8 tests pass; locked Cargo metadata confirms CLI version; existing alpha-only and Stable qualification guards retained. Full CI, native release builds and publication remain pending.
- [ ] Implement bounded compressed source capsules, preserving all source bytes and v1 reader support (i-06f4)
- [ ] Run native packaging and install checks for the repaired alpha preview, then publish only after required checks pass (i-69f4, after t-5501f/i-06f4)
