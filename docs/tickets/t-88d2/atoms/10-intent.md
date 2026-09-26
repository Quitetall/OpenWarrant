# war prepare's commit carried only tree-bound paths, so HEAD's projections drifted from HEAD's records and every battery (a clone of HEAD) failed its clean-corpus check

## Notes

- **2026-09-26 21:52 UTC, claude:** Seen in prepare --all at 5cc5e3cf: 0066/0112/0113 plants FAILED — clean corpus does not pass, CURRENT.md not fresh; a clone of 5cc5e3cf reads 5 drift errors (CURRENT, HISTORY, CORPUS_STATUS x3). Fix: war compile before the commit, and carry every excludes_from_tree path (records + projections). Plant 55-prepare: a clone of prepare's commit reads no drift — FAILS on the old binary (WARRANT_OVERVIEW drift, then CORPUS_STATUS from an uncommitted signature when scoped to evidence only), passes on the fix.
