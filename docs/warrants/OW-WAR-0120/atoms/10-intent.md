---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eaf-7041-b99b-eaed6ac66f86
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

On 2026-09-22 the owner asked three questions: do journals need
compacting, should resolved Warrants be archived, and what does the tool
cost at 1,000 Warrants. Nothing measures any of them.

What this repository shows on 2026-09-23 (136 Warrant directories):

- **Journals are small.** 963 events and 438 KB in all, at most 27 events
  and 13 KB in one Warrant. That is about 1% of `docs/warrants/`.
- **What grows is elsewhere.** `implementation/` holds 24.5 MB (logs,
  captured check output), `gate-runs/` 4.1 MB, `generated/` 1.4 MB. The
  largest Warrant directory (OW-WAR-0107, 952 KB) is mostly two logs.
- **Time is not yet a problem, and nobody knows at 1,000.** With a debug
  build, `war check` takes 2.9 s, `war next` 6.8 s and `war status` 1.2 s
  on this corpus. On a scratch program of 1,000 *draft* Warrants they take
  4.0 s, 4.2 s and 0.8 s, and `war compile` 13.4 s. Resolved Warrants cost
  more per Warrant than drafts (pins, receipts, committed journals), and no
  corpus of 1,000 with resolved Warrants exists to measure.

The current rules also limit the answers. A journal is append-only:
`journal.rewritten` refuses any edit of committed lines, §66.2 forbids a
competing ledger, and §33.8 says compaction shall not launder. RQ-084 keeps
historical records available. `war archive` already exists, and is
preservation transport, not a way to move Warrants aside.

## Desired Outcome

The owner's three questions answered with measurements, and a budget that
a gate holds:

- **A 1,000-Warrant corpus that can be regenerated.** A script builds it in
  a scratch directory through the real acts (`new`, `authorize`, `sign`,
  `evidence record`, `verify`, `resolve`) with throwaway identities,
  at least half resolved. Nothing it writes is committed or leaves its
  scratch directory.
- **A declared budget.** `tools/scale/budget.toml` names the commands and
  their limits (40-work-order). Authorizing this Warrant accepts those
  numbers.
- **A budget gate.** `tools/scale/budget.sh` times each command on the
  corpus, compares it with the budget, and exits 1 naming every command
  over it. A command that fails is `unknown`, never within budget.
- **A recorded measurement at 1,000**, from a release build.
- **`docs/RETENTION.md`** answers the three questions from the numbers:
  whether journals need compaction and why rewriting is not an option;
  which files grow and which of them are referenced evidence that must stay
  (the product spec: temporary work may be pruned only "without removing
  referenced evidence"); and whether the budget holds.

## Non-goals

- Rewriting, truncating or checkpointing any journal. If the measurement
  says journals must shrink, the mechanism is a normative decision for an
  ADR and a later Warrant.
- Moving, deleting or pruning any file. `docs/RETENTION.md` may name
  candidates; removing them is a later Warrant with the owner's decision.
- Optimizing any command. If a command is over budget, the obligation that
  holds the budget is refuted and a later Warrant carries the fix. §88
  requires an ADR and differential conformance for an optimization that
  could change output.
- Measuring the human's time. OW-WAR-0118 carries friction.
