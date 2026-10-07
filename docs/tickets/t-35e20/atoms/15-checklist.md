# Checklist

- [x] Shared claims across worktrees in the git common dir; the cross-worktree double claim becomes a refused plant (i-12e7) — done by claude-m11, 2026-10-07: Claims live under <git common dir>/openwarrant/claims (found by reading .git/gitdir/commondir, no git process); pre-M11 per-worktree locks are read from every worktree and honoured; [tickets] claims_dir still overrides. Plant 100-concurrency: the two-worktree double claim refused by name, same-worktree refused, a pre-M11 lock honoured. Plants 45 and 46 updated for the new lock path.
- [ ] Leases with heartbeat and reclaim of expired leases, journaled (i-0b6e)
- [ ] Compare-and-set writes on record revision (warrant.stale-revision) (i-5812)
- [ ] Cross-machine claims via refs/openwarrant/claims with atomic push; optional war serve HTTP API (i-5e12)
- [ ] Alias safety: warrant.alias-duplicate, cross-branch allocation, war renumber for unsigned Warrants (i-75d0)
- [ ] Merge-friendly files and union-merged per-actor journal; 30-worktree + 2-clone stress plant (i-e8fe)
