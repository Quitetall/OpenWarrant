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
