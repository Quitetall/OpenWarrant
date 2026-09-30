# Progress lookup

Human-facing progress uses HTML. Run `war progress --html`; return the emitted
artifact path as the work-stop link. Default output is
`.openwarrant/state/progress.html`. An explicit path selects another snapshot.
Regenerate before claiming current state. `war progress --serve` starts the same
read-only view on loopback, refreshed every five seconds by default; consult
`--help` for port and interval options. Return its URL when a live view is wanted.

For machine lookup, use `war overview --json` (`war progress` is the same command).
Default JSON/text lists unresolved legacy records; `--all` includes resolutions.
Remaining means unresolved, not unfinished code. On an older binary without
HTML support, link the existing generated `CORPUS_STATUS.html` and disclose its
legacy-only snapshot semantics; do not substitute raw JSON as the human artifact.

For implementation reporting, read the viewer adapter contract at
`docs/warrants/OW-WAR-0092/implementation/viewer-contract.md` in this source repo.
At a work stop, record actual scope, source revision and evidence in the Warrant's
`implementation/progress.json`, then regenerate HTML. Report only observed work;
missing evidence is not invented. This adapter is an attributed display claim,
not an SDK assurance record. Unknown implementation stays unknown, and legacy
resolution alone never supplies the new Verified mark.

For one legacy record: `war status <alias> --json`. Human acts: `war next --json`.
Open questions: `war questions --open --json`. Stages: `war frontier --json`.
Follow named actors; pending signatures are not universal prototype start gates.
On lookup failure, report UNKNOWN and diagnostics, never zero remaining. Live
view retains its last good snapshot and labels stale/disconnected state.
