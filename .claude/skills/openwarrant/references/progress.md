# Progress lookup

Run `war overview --json` (`war progress` is the same command). Default lists every
unresolved legacy record; `--all` includes resolutions. Read totals and row details;
show a short answer unless a full list was requested. Save human output with
`war overview > <chosen-output.md>` when a persistent overview link is wanted.
This explicit output is a snapshot; regenerate it before claiming current state.

For one result: `war status <alias> --json`. For human acts: `war next --json`.
For open questions: `war questions --open --json`. For stages: `war frontier --json`.
Follow the named actor; a pending signature is never an agent action.

Current command reports legacy record state. `remaining` means unresolved, not a
measurement of unfinished code. It cannot observe future SDK prototype completion
records or award Verified marks. Read the candidate roadmap for intended phase
ownership; keep that planning view separate from observed runtime status.

On an older binary without overview, use `war status --json` and filter unresolved
rows from its live result; disclose fallback. On command failure, report UNKNOWN
and the diagnostic, not zero remaining. Do not silently use stale generated views.
