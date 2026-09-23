---
schema: oh.war/atom/v1
warrant_uuid: 01a0d049-9bcc-7a20-9d41-db7d11e0fb0d
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war check` warns `independence.insufficient` for every Warrant in this
corpus: 83 at `basic`, 29 at `controlled`. The repository declares no
independence, and no verifier is configured. So no Warrant can meet §56.1
requirement 10, and none can be resolved. The one performer and the one
owner cannot supply independence by being careful (§27.4). A performer's
report cannot satisfy an independent gate (RQ-053).

`war verify --run` already exists. It compiles a verification bundle — the
authorized atoms, the deliverables' bytes, the plants that name the Warrant,
its gate runs and prior verifications, and nothing the performer said — and
hands it to `[verify] verifier_argv`. Nothing is configured there.

## Desired Outcome

A verifier this repository can run on every Warrant, whose independence is
a fact of how it is built rather than a claim anyone makes:

- **A separate process with a separate context.** `claude -p` in
  non-interactive mode, no session persistence, with every tool disallowed.
  It reads the bundle on stdin and nothing else. It cannot open a file,
  run a command or write anywhere.
- **Blind.** The bundle carries no transcript, no rationale and no journal.
- **The model chooses only the verdicts.** Per obligation: `established`,
  `refuted` or `not_established`, with the evidence it relied on. Who the
  verifier is, and every independence flag, are written by the wrapper
  script, never by the model: a model cannot grant itself independence by
  saying so.
- **Anything unclear becomes `not_established`:** an answer that does not
  parse, an obligation it skipped, a disposition outside the three. Unknown
  is not pass (Law 15).
- **`distinct_model_required` is claimed only when checked:** only when
  `CLAUDE_PERFORMER_MODEL` is set and differs from the verifier's model.
  Otherwise it is written `false`.
- The repository declares the independence this arrangement provides in
  `openwarrant.toml`, with each flag's reason beside it.

After authorization the verifier is run, Warrant by Warrant, with `war
verify <alias> --performer <performer> --run`. Its verdicts are its own. This
Warrant delivers the verifier, not any verdict.

## Non-goals

- Human verification, four-eyes review or `high` assurance. No human
  verifies here, and the declaration says `distinct_human_required = false`
  (§27.4: a view shall not claim four-eyes review when none occurred).
- Choosing the verdict for any Warrant, or re-running a verification until
  it passes.
- A verifier for another vendor's model. The seam is `verifier_argv`; this
  Warrant ships one adapter.
