---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfd-7e45-b33b-749b95409cd1
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a real BLUT execution produced real receipts
- **scope:** one stage graph, one execution.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** BLUT, by exact commit.
- **evidence:** status, artifact and lineage receipts returned by BLUT, with the
  registry digest recorded and the backend and stage identities pinned.

### OBL-002 — an incompatible lowering was refused
- **scope:** §49.2 and §91.7 test 47.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant offering an incompatible port kind, refused by
  `LoweringRejected` naming the port, against the shipped binary. A refusal is
  the evidence; a successful lowering is not.

### OBL-003 — BLUT's lineage was referenced, not reproduced
- **scope:** §49.3, for the executed stage.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the Warrant carries a `lineage_ref` and a search of
  its committed bytes finds no lineage events or graph. A second copy of an
  authoritative fact is the defect being tested for.

## Evidence

§40's records for what this Warrant has actually done. The first entry is the
first thing in this repository whose origin is not this repository.

### EV-001 — BLUT's verdict on a lowering OpenWarrant generated
- **class:** evidence
- **kind:** external_tool_verdict
- **origin:** blut
- **admissibility:** authoritative_external
- **digest:** sha256:pending-receipt-binding
- **method:** `war blut OW-WAR-0047 --verify <blut-binary>` wrote the lowered
  PlanSpec to a file and ran `<binary> plan check --json` on it; the verdict and
  exit status were read from that process's output
- **occurred at:** 2026-08-21

### EV-002 — the neighbour-trust plant battery
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** conformance/plant.sh, executed by cargo xtask gate — five plants
  supply a missing, lying, mute and mumbling neighbour
- **occurred at:** 2026-08-21

### EV-003 — the §49.3 lineage plants
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** conformance/plant.sh — one plant pastes BLUT lineage keys into an
  atom and requires `lineage.reproduced`; a second appends prose naming the same
  fields and requires that NOTHING fires
- **occurred at:** 2026-08-21

### OBS-001 — BLUT refused the lowering, naming the unresolvable stage
- **class:** observation
- **evidence:** EV-001
- **method:** BLUT reported `accepted: false` and exit 1 with the message
  "stage 'STAGE-002' is not in any registered cookbook"; the exit status and the
  verdict agreed, which is separately checked before either is recorded
- **admissibility:** authoritative_external
- **superseded by:** OBS-004. The observation is kept because it happened and is
  what INF-001 reasoned from. What it did NOT establish, and was read as
  establishing, is *why* — see INF-001.

### EV-004 — BLUT's verdict after the adapter stopped guessing stage names
- **class:** evidence
- **kind:** external_tool_verdict
- **origin:** blut
- **admissibility:** authoritative_external
- **digest:** sha256:pending-receipt-binding
- **method:** `war blut OW-WAR-0047 --verify <blut-binary>` after `executor_ref`
  bound each computational stage to a stage a registered cookbook compiles in
- **occurred at:** 2026-08-21

### OBS-004 — BLUT ACCEPTED a two-stage lowering
- **class:** observation
- **evidence:** EV-004
- **method:** BLUT reported `accepted: true`, exit 0, fingerprint
  `a2005e3c9535…` for a graph of `materialize_dataset_path` -> `filter_dataset`.
  Both the stage names and the graph typecheck: the root takes `()` and the edge
  carries `dataset.jsonl` to a stage expecting `dataset.jsonl`.
- **admissibility:** authoritative_external

### OBS-002 — a verdict OpenWarrant cannot attribute is refused, not recorded
- **class:** observation
- **evidence:** EV-002
- **method:** four plants each assert an exit code, a named rule and a named
  detail; the exit-code cross-check was additionally disabled and the
  corresponding plant then failed, so the battery is known to be able to fail
- **admissibility:** controlled_measurement

### OBS-003 — a Warrant restating BLUT's lineage is refused; one describing the rule is not
- **class:** observation
- **evidence:** EV-003
- **method:** both directions are planted, and both were falsified before being
  trusted — regressing the detector to a substring scan makes the prose control
  fail, and removing the key-position requirement makes it fail the same way
- **admissibility:** controlled_measurement

### INF-001 — the refusal is the correct answer, not a defect in the adapter
- **class:** inference
- **kind:** deductive
- **premises:** OBS-001
- **claim:** roadmap-limit-1
- **reasoning:** BLUT forbids dynamic stage loading, so a stage name resolves
  only if some cookbook compiles it in. Every stage this repository names is a
  `STAGE-NNN` identifier from the milestones grammar, which no cookbook has. A
  lowering of these Warrants is therefore unacceptable to any BLUT binary, and
  an acceptance would mean the pinned-registry rule had stopped applying.
- **admissibility:** authoritative_external
- **falsified by:** OBS-004, INF-002. Kept rather than deleted: a recorded
  inference that later evidence overturned is the record working, and removing
  it would hide that the conclusion was once believed.

### INF-002 — the refusal was the adapter's defect, misread as the registry rule
- **class:** inference
- **kind:** deductive
- **premises:** OBS-001, OBS-004
- **claim:** roadmap-limit-1
- **reasoning:** INF-001 concluded that no lowering here could be accepted, from
  the true premise that no cookbook compiles a `STAGE-NNN`. The step it skipped
  is that a WAR stage id was never required to BE the BLUT stage name. The
  adapter used the id because nothing else was available, so every lowering
  named `STAGE-NNN` and was refused — and the refusal was then read as the
  pinned-registry rule working correctly. Once a stage could declare the name
  its executor knows it by (`executor_ref`), BLUT accepted (OBS-004). The
  registry rule was never the obstacle; the adapter's guess was.
- **admissibility:** authoritative_external

### JDG-001 — OBL-001 remains open, and this does not narrow it
- **class:** judgment
- **kind:** scope_holding
- **actor:** QuiteTall
- **acting role:** author
- **meaning:** OBL-001 asks for status, artifact and lineage receipts returned
  by BLUT from a real execution. A typecheck returns none of those. Recording
  this verdict against OBL-001 would substitute a cheaper measurement for the
  one required (§40.7), so it is recorded against the Warrant and not against
  the obligation. The obligation is left open rather than narrowed to fit what
  was achieved.
- **basis:** OBS-001, INF-001
- **authority:** authorized
- **limitations:** one actor, so this judgment is not independently reviewed —
  §27.4 says role separation by one person is not organizational independence

## Gate Adequacy

Required at `basic` only, so §39.4 does not compel a review. Asked
anyway, briefly, because the answer is short.

**Could this pass while BLUT authority is duplicated?** Only by copying lineage
into the Warrant, which OBL-003 tests for directly by searching the committed
bytes. That is a mechanical check on a mechanical failure, and unusually for this
project, it is close to complete.

## Residual Risk

BLUT's PlanSpec may change shape faster than the adapter. The pinned
registry digest converts that into a refusal rather than a silent remap, which
trades availability for correctness deliberately.
