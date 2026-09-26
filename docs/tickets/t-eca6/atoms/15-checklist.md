# Checklist

- [ ] Compute moved_since once per (root, tree) within one read-only evaluation (next, status, check), never across a gate run (i-0771)
- [ ] Refresh or tolerate a racy index (fresh clone: 181 s) so a battery clone does not pay a rehash per call (i-fa93)
- [ ] 67-pins-next passes solo at HEAD again; record war next's CPU time before and after (i-7e73)
