# run-1 — the record, as it happened

`run.json` is the record of the run that produced `proposal.json`: its argv
disallowed `MultiEdit` and `Task`, and `stderr.txt` shows Claude Code
reporting that `MultiEdit` matches no known tool. The committed wrapper
dropped those two names afterwards. The record is not edited to match the
wrapper; the wrapper is what changed.

`sentence.txt` was committed before the run (OW-WAR-0042 work order: the
vague sentence is recorded verbatim, not rewritten to suit the agent).
`request.json` is the canonical `oh.war/draft-request/v1` `war plan` sent on
stdin; `proposal.raw.json` is the drafter's stdout verbatim; `proposal.json` is
what `war plan --draft --out` wrote after the §74.4 steps 1–4 passed.

Not applied: steps 5–8 need a human's `--reviewed`.
