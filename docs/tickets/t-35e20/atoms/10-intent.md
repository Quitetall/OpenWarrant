# M11: concurrency at Beads' level or better

## Notes

- **2026-10-07 09:07 UTC, claude-m11:** M11 started on claude/m11 (worktree /mnt/4tb/tmp/claude-m11, base 4497597b). Plan: claims move to <git common dir>/openwarrant/claims with legacy per-worktree locks still honoured; leases ride on the lock inode's mtime so a renewal never rewrites a lock another agent may have just taken.
