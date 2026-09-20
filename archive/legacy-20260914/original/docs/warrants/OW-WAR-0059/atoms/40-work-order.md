---
schema: oh.war/atom/v1
warrant_uuid: 01a060c6-64a5-71e1-b994-133d3c1e19d2
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `evidence.rs` in the CLI: `GateEvidence` (a run, its receipt, both
   paths); `admissibility` — §44.5 pass, receipt present and complete,
   `receipt_digest` recomputes, verdicts agree, `subject_digests` names the
   current contract; `admissible_runs`; `load` from a Warrant's
   `gate-runs/`; `record` (`war evidence record <alias> [--gate]`); `check`
   rules `evidence.admissible`, `evidence.stale-binding` (warn),
   `evidence.receipt-invalid` (error), `evidence.not-a-pass` (warn),
   `evidence.malformed` (error).
2. The gate runner takes an output directory, so a receipt is minted into
   the Warrant's tracked `gate-runs/` rather than copied there. The
   gitignored receipts path stays the default for `war gate --run`.
3. Requirement 5 reads a Warrant's own `gate-runs/` and nothing else, in
   both `war resolve` and the corpus projection; the "gate runs are NOT
   read" caveat is replaced by one that says where they ARE read from.
4. `resolution_cmd.rs`: `war resolve <alias>` emits a `ResolutionRequest`;
   `--response <file>` ingests a `ResolutionResponse`, re-evaluates the
   thirteen, checks the digest, resolves the actor through the register,
   refuses agents, bounds `common_outcome` by §38.6, validates the
   `Resolution`, and writes `resolution.toml` once — a second ingest is
   refused as `resolution.exists`.
5. The record computes `assurance_case_snapshot_digest` under
   `DigestDomain::AssuranceCaseSnapshot` and `artifact_manifest_digest`
   over `deliverables.toml`, and cites the admissible receipts, judgments
   and residual risks by repository-relative ref.
6. `war check`: `resolution.recorded` (pass), `resolution.stale` (error —
   the contract moved after resolution; §45: dispute or annul, do not
   edit around it), `resolution.malformed`, `resolution.wrong-warrant`.
7. `war status`: a Warrant with a record binding the current contract
   reads `resolved`; `Implements.warrant_resolved` is true only for
   `common_outcome = satisfied`, so §34.3's `satisfied` keeps its meaning;
   an Objective reads `Recorded` only when its exit Warrant resolved
   satisfied. A stale record is shown and derives nothing.
8. Evidence recorded for OW-WAR-0010 and OW-WAR-0020 (OW-WAR-0016 cites no
   gate — see the basis); their resolution requests emitted.

## Frozen Surfaces

`GateReceipt`'s fields and the way `receipt_digest` seals them — a receipt
minted before this Warrant still reseals after it. `GateRun` and
`satisfies_required_pass`. `Resolution` and `validate`. The thirteen.
`DigestDomain::ALL` — fifteen.

## Premade Instructions

- Never read `docs/receipts/` for requirement 5. Not in `war resolve`, not
  in the projection. One question, one input.
- A receipt is never rewritten. If its refs are wrong, mint another.
- Do not record evidence for a Warrant whose obligations are not
  established; it would move nothing but the file count.
- Do not resolve anything. Emit the requests.

## Autonomy and Escalation

Tier T2. Escalate if any of the three would-satisfy Warrants fails its own
gate when run for real.

## Rollback

Delete the two `gate-runs/` directories and revert. Requirement 5 returns
to unmet everywhere and `war resolve` returns to dry-run only.
