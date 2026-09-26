# war prepare: take a Warrant to its sign-off unattended

Getting a Warrant ready for the owner's one sitting is a multi-step procedure an agent currently improvises: declare delivery (war deliver), run each service stage its milestones name through `war run` (obligations may require the dispatch-bound receipt, as OW-WAR-0066's do), record the cited gates (`war evidence record`), run the blind verifier (`war verify --run`), then the document gates last (document.review needs verifications). Order matters (t-22fd, t-5d82, t-fed6). Add `war prepare <alias>...` (and a corpus mode) that does exactly this, idempotently, resumable, never signing, printing what is left for the human (the authorize and resolve lines) — the command an agent runs so the human never has to babysit. Batteries may run in parallel (t-d052 made that sound).

## Notes

- **2026-09-26 15:21 UTC, claude:** i-2644 (run it over the wave) is left open on purpose: the owner runs war prepare over the real corpus after merge (the implementer was told not to). Suggested: war prepare --all --dry-run first to see the plan; then war prepare --all --jobs 4 from a clean tree with a git identity; the commit it makes before evidence has commit signing off.
