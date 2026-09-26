# war ready and war claim can disagree on whether an item is blocked

45-tickets.sh 'blockers keep items out of ready' failed once (OW-WAR-0132's evidence run, loaded machine): after `war done` on the blocker i-9468, `war ready` listed t-b314/i-240e as ready, and the next `war claim` refused it (ticket.blocked: waits on i-9468). Two commands disagreeing about the same state is a correctness bug in the ticket loop, not a plant flake. Find the source: a cache keyed on a timestamp too coarse for same-second writes, a read of the checklist racing the atomic rename, or the two computing 'blocked' differently.

## Notes

- **2026-09-26 03:51 UTC, claude:** Root cause: the plant, not the tool. OW-WAR-0132's line 'claim 2: refused ... waits on i-9468' was the plant's deliberate pre-done claim; the failing term was an empty TK5_SC or TK5_R from a war add that did not run (concurrent relink, exit 127, unchecked). Separately: 67-pins-next 'next answers this corpus in time' fails on this machine at base too (war next 12-21 s, debug build, load ~10), unrelated to this ticket.
