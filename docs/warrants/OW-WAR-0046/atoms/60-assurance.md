---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfc-7c2c-b61a-06b3aad19004
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a real gate run produced a complete receipt
- **scope:** one qualified gate, one run, against a named subject.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a §44.6 receipt with all twelve required scalars
  populated, whose triple is askable + completed + pass. Plus recorded runs
  showing a missing tool reported as `not_askable`, NOT as a failure.

### OBL-002 — an adequacy review executed attacks
- **scope:** §39.3, for the Warrant being resolved.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the review records executed attacks as BULLETS, each
  naming what was planted and which control rejected it. This closes roadmap
  limit 3 for one Warrant.

### OBL-003 — all thirteen requirements are computed, and each unmet one is named
- **scope:** §56.1, for every Warrant in this corpus. NARROWED by
  `amendments/AM-001.yaml`: the original required one Warrant to be RESOLVED,
  which requirement 10 makes unreachable while one actor holds every role.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war resolve --dry-run` reporting all thirteen individually, met
  and unmet alike, plus a recorded run in which each of the thirteen is unset in
  turn and each blocks alone and is named. Recording a §56.2 resolution is out of
  scope and stays out until requirement 10 can hold.

### OBL-004 — dispute and annulment preserved history
- **scope:** §56.4 and §56.5, for the resolved Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the original resolution compared byte for byte after
  both, showing it unchanged, and a supersession showing the superseded record
  keeps `valid` standing — replacement is not invalidity.

### OBL-005 — §91.10, §91.11 and §91.12 are planted
- **scope:** tests 64 through 90.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** twenty-seven entries in `conformance/` against the
  shipped binary, each rejected by a named rule with a named detail. Tests 73 and
  74 are planted first.

## Evidence

§40's records for the work this Warrant has actually done. Every one describes
something that happened; none describes something planned.

### EV-001 — the plant battery's output
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** conformance/plant.sh, executed by cargo xtask gate
- **occurred at:** 2026-08-21

### EV-002 — the reachability measurement
- **class:** evidence
- **kind:** static_analysis
- **origin:** performer
- **admissibility:** performer_report_only
- **digest:** sha256:pending-receipt-binding
- **method:** grep for type names across the CLI and compiler crates, then
  confirmation that the validation executes on a `war check` run
- **occurred at:** 2026-08-20

### OBS-001 — every planted violation was rejected by its intended control
- **class:** observation
- **evidence:** EV-001
- **method:** each plant asserts an exit code, a named rule, AND a named detail
  string, so a rejection for the wrong reason fails
- **admissibility:** controlled_measurement

### OBS-002 — six of twenty sampled alpha types are executed by a war command
- **class:** observation
- **evidence:** EV-002
- **method:** counted by hand against the twenty sampled in the 2026-08-20
  measurement; the grep alone undercounted twice and is not the basis
- **admissibility:** performer_report_only

### INF-001 — the validators alpha shipped were not enforcing anything
- **class:** inference
- **kind:** deductive
- **premises:** OBS-002
- **claim:** roadmap-limit-2

### JDG-001 — deliverable 3 is unachievable and is narrowed rather than pursued
- **class:** judgment
- **kind:** scope_narrowing
- **actor:** QuiteTall
- **acting role:** author
- **meaning:** §56.1 requirement 10 cannot hold while one actor holds every role,
  so the deliverable is narrowed to computing and reporting the thirteen. This
  accepts that no Warrant here will be resolved, and says so.
- **basis:** OBS-002, INF-001
- **authority:** authorized
- **limitations:** one actor, so this judgment is not independently reviewed —
  §27.4 says role separation by one person is not organizational independence

## Gate Adequacy

Required at `controlled`, and this is the Warrant where a defect is
worst: everything else in the system exists to make this one true.

**Adversarial question: could a delivery close through proof that is bounded,
provenance-preserving, and wrong?** Yes, and §39.5 admits it: no generic compiler
proves that arbitrary gates entail arbitrary natural-language claims. A gate can
be qualified, askable, complete, and measure something adjacent to the obligation
it is cited against. Every mechanical control here would pass.

There is a second gap, and it is this repository's own. §46.3 wants blind review
at `controlled`, and there is one actor here. Whatever review happens is the
author reviewing the author, which §27.4 says is not independence. That is a
blocking unknown rather than an accepted risk, because pretending otherwise
would be the exact substitution §40.7 forbids.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

The independence gap does not close by effort. It closes by a second
person or an accountable external control, and until then every resolution this
repository produces is a self-assessment with good bookkeeping. That should be
stated in the resolution's `meaning` field rather than left for a reader to work
out.
