# Checklist

- [x] Compute moved_since once per (root, tree) within one read-only evaluation (next, status, check), never across a gate run (i-0771) — done by claude, 2026-09-26: gate_cmd::source::remember_tree_reads(): moved_since's two git reads remembered per (root, tree) for the process, only when turned on by the one-shot read-only next/check/status arms of run(); off for observe()/gate runs, TUI, web UI, MCP. Errors not remembered. next 11.4->3.7 s, check --generated 45->13.7 s, status 66.7->1.7 s; outputs byte-identical to the unmemoised binary, clean and dirty.
- [ ] Refresh or tolerate a racy index (fresh clone: 181 s) so a battery clone does not pay a rehash per call (i-fa93)
- [x] 67-pins-next passes solo at HEAD again; record war next's CPU time before and after (i-7e73) — done by claude, 2026-09-26: 67 passes solo and three-wide at 5d206e5d: next 3.7-4.1 s CPU (was 17.8 s at f24aba73), next/status 2.48-2.64.
