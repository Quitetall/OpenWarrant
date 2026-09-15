---
schema: oh.war/atom/v1
warrant_uuid: 01a09762-3fa2-7dd0-b794-0190b44fe879
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `src/inbox/classify.rs` — pure function `next_act(&Warrant) -> NextAct` returning `Human(HumanAct)`, `Agent`, `Gate`, or `None`, plus the state-to-act table as a single match.
2. `src/cli/inbox.rs` — the `war inbox` subcommand: loads all Warrants in the namespace, applies the classifier, prints a table (alias, title, state, awaited act, age) or, with `--json`, an `oh.war/inbox/v1` document.
3. `src/cli/mod.rs` — register the `inbox` subcommand alongside the existing commands.
4. `tests/inbox_classify.rs` — one test per Warrant state asserting its `NextAct`.
5. `tests/inbox_cli.rs` — integration test over `conformance/fixtures/inbox/` asserting the exact set of aliases listed and the `--json` document shape.
6. `conformance/fixtures/inbox/` — a small record set with at least one Warrant per act kind (human, agent, gate, none).
7. `docs/cli/inbox.md` — command reference, the state-to-act table, and an example of both outputs.
8. `CHANGELOG.md` — entry under Unreleased.

## Frozen Surfaces

- The Warrant state machine and its state names.
- All record schemas under `war schemas` and their transitive digest.
- `war check`, `war run`, `war submit` behavior and exit codes.
- The on-disk layout of `docs/warrants/`.

## Premade Instructions

- Reuse the existing record loader; do not write a second parser.
- The classifier is a total function over the state enum; the compiler must reject an unhandled state (no wildcard arm).
- Human-readable output goes to stdout, diagnostics to stderr; exit 0 on an empty inbox, non-zero only on load failure.
- `--json` output must round-trip through the schema added by the accompanying ADR; add it to the `war schemas` pack only if the ADR is accepted.
- Sort rows by wait age descending, then alias ascending, so the longest-waiting Warrant is first.
- Do not touch any existing Warrant record while implementing.
- Run `war check` on this Warrant before claiming completion.

## Autonomy and Escalation

Tier T1. The agent may implement all deliverables without check-in. Escalate to the human if: a Warrant state is found whose next act is genuinely ambiguous between human and agent; the record loader cannot expose transition timestamps; or the ADR on the JSON format is declined (then ship without `--json`).

## Rollback

Single commit on a feature branch; rollback is `git revert` of that commit. No records, schemas, or migrations are altered, so no data rollback is required. If the ADR is rejected after merge, remove the `--json` flag in a follow-up revert of `src/cli/inbox.rs` only.