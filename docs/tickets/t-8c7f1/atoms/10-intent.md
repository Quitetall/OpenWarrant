# M15: native graph executor, pluggable executors

## Notes

- **2026-10-07 19:36 UTC, claude:** M15 started in /mnt/4tb/tmp/claude-m15 (branch claude/m15 from ce243b45). Design: graph.rs builds one work graph from war plan model (items, light Warrants, stages via the frontier, records reached by depends_on/implements); estimate.rs learns claim-to-done medians from ticket journals; war evidence go runs ready nodes in worktrees; war start is the 12th daily verb.
