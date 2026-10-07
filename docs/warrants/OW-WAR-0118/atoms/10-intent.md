---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e81-73d3-b28f-cedb19f93c32
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The product spec ("Friction targets and proof") sets two targets:

- ordinary repository setup in five to ten minutes or less;
- routine Warrant administration after setup in at most 60 seconds.

It also says they are "targets, not measured achievements", and it lists
"the real-work baseline and measurement procedure" among the engineering
contracts still to specify. Nothing in this repository measures either
target. `QUICKSTART.md` says "Times are what the tool takes; the human steps
take as long as reading takes", and no number backs that sentence.

A probe on 2026-09-23 (debug build, scratch directory, throwaway ssh-agent
key) ran the governed setup from `war init --program` to an authorized first
Warrant. Every tool step took under 30 ms. So the tool's time is not where
setup friction lives. The human steps are: writing `roles.toml` and
`allowed_signers`, loading a key, reading each request, and confirming each
signature. No script can time those, and a measurement that folds them into
a number would be false.

## Desired Outcome

A repeatable measurement, a recorded baseline, and a document that says
what the numbers mean and what they do not.

- **Scripted where a human step can be simulated.** The script builds a
  program in a throwaway directory, from an empty directory to an
  authorized first Warrant, then runs a set of routine acts. It stands in
  for the human's key with a throwaway ssh-agent key, the way
  `conformance/plants.d/96-batch.sh` does. Each step records wall time and
  exit code.
- **Human steps are counted, not timed.** For each human step the record
  says what the human must do: files edited by hand, commands typed, and
  dialogs confirmed. Its time is `not_measured`, never zero.
- **A failed step is unknown, not fast.** If a step exits non-zero, its time
  and every total that includes it are `unknown`, and the script exits
  non-zero (Law 15).
- **A baseline is recorded once**, from a release build, with the binary's
  version, commit, OS and CPU, before any change made to improve a number.
- `docs/FRICTION.md` states the targets, the procedure, the baseline and
  its limits, and gives a manual protocol a human can follow to time the
  human steps. Anything a human times is recorded as a separate record and
  never merged into the scripted number.

## Non-goals

- Improving any number. This Warrant measures. Tuning is later work.
- Claiming either target is met. The human part of setup is not measured
  here, so the setup target can be neither met nor missed by this
  Warrant's evidence.
- Measuring on every supported OS. The product spec asks for each supported
  OS. This Warrant measures on the machine it runs on (Linux x86_64) and
  names the others as unmeasured.
- Scaling to large corpora. The routine acts are also timed against this
  repository's own corpus, read-only, but a 1,000-Warrant budget belongs to
  `roadmap://OW-PHASE-1/retention` (OW-WAR-0120).
- §94's "human authoring minutes" and the other telemetry measures.
  OW-WAR-0039 and OW-WAR-0041 carry those.
