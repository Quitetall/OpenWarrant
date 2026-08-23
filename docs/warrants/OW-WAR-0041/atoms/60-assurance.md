---
schema: oh.war/atom/v1
warrant_uuid: 01a021a1-4e5e-7fab-b57f-5ff71bea4973
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a baseline exists, recorded before any tuning
- **scope:** §94's eighteen measures, against this repository only, at a named
  commit. No claim about any other corpus.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a recorded artifact naming the commit and the value
  of every measure, produced BEFORE any assurance default or amendment policy is
  changed. A baseline taken after tuning is not a baseline.

### OBL-002 — measures that cannot be taken are named, not zeroed
- **scope:** the subset of §94 requiring instrumented authoring.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each unmeasurable measure carries an explicit
  `not_measurable_yet` and a stated reason. A grep for `= 0` across the recorded
  baseline returns no measure that was never actually taken.

### OBL-003 — §95 detection runs against real history and fabricates nothing
- **scope:** this repository's commit history at a named commit.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a run producing real untracked-work candidates, and a
  plant that attempts to attach a relation with an empty reviewer and is refused
  by `RelationFabricated`. Run against the shipped binary, not a unit test.

### OBL-004 — no §100 reduction is claimed
- **scope:** all sixteen §100 metrics.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the recorded artifact states, for every §100 metric,
  either a baseline value or `no baseline`. It states no delta, because one
  measurement cannot support one.

## Evidence

### EV-001 — the telemetry baseline
- **class:** evidence
- **kind:** telemetry_baseline
- **origin:** performer
- **admissibility:** performer_report_only
- **digest:** sha256:pending-receipt-binding
- **method:** `war telemetry --commit <sha>`, written to
  `artifacts/telemetry-baseline.json`
- **occurred at:** 2026-08-22

### EV-002 — the §95 fabrication plants
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** conformance/plant.sh — an attachment with no reviewer, and a
  positive control with one
- **occurred at:** 2026-08-22

### OBS-001 — 7 of §94's 18 measures are takeable; 11 carry a stated reason
- **class:** observation
- **evidence:** EV-001
- **method:** every measure is either a value with a stated method or
  `not_measurable_yet` with a reason; no measure has neither. Two measures record
  a value of ZERO and both are real zeros — `auto-authorizable fraction` (no
  Warrant is auto-authorizable while §56.1 requirement 10 is unmet) and, before
  correction, `adequacy counterexamples`.
- **admissibility:** performer_report_only

### OBS-002 — a private counter reported 0 adequacy counterexamples; the shared parser reports 51
- **class:** observation
- **evidence:** EV-001
- **method:** the first implementation counted markdown bullets with its own
  rule and produced 0. Replacing it with `adequacy::parse` — the same parser
  `war check` uses — produced 51. The baseline would have published a measured
  zero for a quantity that is fifty-one.
- **admissibility:** controlled_measurement

### OBS-003 — §95's fabrication refusal runs from the shipped binary
- **class:** observation
- **evidence:** EV-002
- **method:** `war telemetry --attach` with no `--reviewer` exits 1 with
  `RelationFabricated`; with a reviewer it exits 0. The positive control matters:
  a build that refused EVERY attachment would satisfy the refusal plant and look
  like a working review requirement.
- **admissibility:** controlled_measurement

### INF-001 — the unmeasurable eleven are blocked on instrumentation, not on effort
- **class:** inference
- **kind:** deductive
- **premises:** OBS-001
- **claim:** telemetry-baseline
- **reasoning:** each of the eleven names a property of an authoring SESSION —
  minutes spent, questions asked, wall time elapsed, cost metered — and a git
  repository records commits, not sessions. Three of them (`reopenings`,
  `post-resolution escapes`, and the interview counts) are blocked for a second
  reason: they are undefined while no Warrant has been resolved and no agent has
  answered over §75.2. Recording any of them as `0` would assert that the thing
  was looked for and not found.
- **admissibility:** performer_report_only

### JDG-001 — no §100 delta is claimed, and none can be
- **class:** judgment
- **kind:** scope_holding
- **actor:** QuiteTall
- **acting role:** author
- **meaning:** all sixteen §100 metrics record `no baseline`. §100 is a list of
  things the system succeeds by REDUCING or INCREASING, and a direction of travel
  needs two measurements. This is the first. OBL-004 asks for exactly this, and
  the temptation a success-metrics section creates is precisely to report one
  sample as an improvement.
- **basis:** OBS-001
- **authority:** authorized
- **limitations:** one actor, so this judgment is not independently reviewed —
  §27.4 says role separation by one person is not organizational independence

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could every obligation pass while the measurements are
meaningless?** Yes, and this is the honest limit. A baseline of one repository,
authored by one actor, measured once, describes that repository on that day. It
supports no generalisation about authoring cost, and §38.4 would refuse a
universal claim built on it.

What the obligations do buy is narrower and still worth having: the difference
between a measure that was taken and one that was never looked at is recorded,
which is the distinction that makes a later comparison possible at all.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

A single sample cannot detect a trend, and the temptation once a second
sample exists will be to report the difference as a reduction. §100's verbs make
that tempting and §38.4 makes it refusable. The check on it is OBL-004, which
forbids stating a delta, and it will need re-reading when the second measurement
lands.
