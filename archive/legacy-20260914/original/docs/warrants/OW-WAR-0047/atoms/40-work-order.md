---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfd-7e45-b33b-749b95409cd1
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. One compatible WAR stage graph lowered into BLUT PlanSpec.
2. A real BLUT execution, with status, artifact and lineage receipts.
3. A plant for §91.7 test 47 — an unsupported lowering fails rather than
   degrades — against the shipped binary.
4. Evidence that no BLUT lineage was copied into the Warrant.

## Frozen Surfaces

§49.2's adapter duties and the lineage-reference shape.

## Premade Instructions

- Attempt an INCOMPATIBLE lowering first and record the refusal. A
  successful lowering proves the happy path; the refusal proves the adapter is a
  control rather than a translator.
- Grep the resulting Warrant for lineage content. Finding any is the defect.
- Deliverable 2 — the execution — runs from the repository root as:

      war blut OW-WAR-0047 --emit plan.json --verify <cookbook-binary>
      <cookbook-binary> plan run plan.json

  `--emit` writes the spec before the verdict is sought, so the bytes on disk
  are the bytes BLUT was asked about. Rehearse with `plan run --dry-run` first;
  it answers "would this launch?" without spending the machine on finding out.

  The stage args point at `conformance/fixtures/ow-war-0047-corpus.jsonl`, a
  three-conversation corpus committed so the run is reproducible from a
  checkout rather than depending on a path that happens to exist on one host.
  The path is repo-root-relative, which is where `war` is run from.

  Do NOT run it on a loaded box. The first stage materializes a path and the
  second filters three lines, so the work is trivial — but a run whose receipts
  are the deliverable should not be competing for admission with something else,
  and BLUT's broker is memory-gated.

- The receipts are captured AFTER the run, against the real job id, and that
  capture is deliberately not written in advance: a receipt-shaped function
  tested only against a job that never existed is the substitution §40.7
  forbids, wearing the name of the control that prevents it.

## Autonomy and Escalation

Tier T1 — the adapter's refusals decide completion.

## Rollback

Revert the adapter invocation. Stage graphs remain lowerable and unlowered.
