# M11: concurrency at Beads' level or better

## Notes

- **2026-10-07 09:07 UTC, claude-m11:** M11 started on claude/m11 (worktree /mnt/4tb/tmp/claude-m11, base 4497597b). Plan: claims move to <git common dir>/openwarrant/claims with legacy per-worktree locks still honoured; leases ride on the lock inode's mtime so a renewal never rewrites a lock another agent may have just taken.
- **2026-10-07 10:43 UTC, claude-m11, on i-e8fe:** Merge: verified that git's text merge conflicts on adjacent ticks (and on two adds / two first notes). Layout kept (ticket bytes frozen); instead a merge driver: war merge-ticket (checklist item by item keyed by id, intents by appending, one Notes heading; a two-way edit of one item falls back to git merge-file markers + ticket.merge-conflict). Journals: merge=union. war init writes .gitattributes and configures the driver (probe-guarded so an older/missing war falls back to git merge-file); this repo's .gitattributes carries both lines; existing clones run war merge-ticket --install.
