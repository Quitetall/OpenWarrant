---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd0-d818-7a39-bd24-88638a843026
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the five axes exist and are independent
- **scope:** the vocabularies of §24.1–§24.5.
- **evidence:** a test asserting no axis can be computed from another.

### OBL-002 — illegal transitions and untruthful combinations are REFUSED
- **scope:** §91.6 tests 36–41.
- **evidence:** one plant per test, each naming its rule. Includes proving that
  blocking does not erase phase and annulment does not erase outcome.

### OBL-003 — reported state distinguishes derived from recorded
- **scope:** the Overview's status column.
- **evidence:** a Warrant whose state is derived renders differently from one
  whose state is recorded. Until storage exists every state is derived, so this
  obligation is what stops the column from over-claiming on day one.

## Gate Adequacy

Required at `controlled`.

**Could the obligations pass while the reported state is wrong?** Yes. Derived
state is inferred from the record's shape, and a record can be shaped like an
authorized Warrant without having been authorized by anyone — there is no
authority model until OW-WAR-0028. So "authorized" will mean "looks authorized"
until KF actions exist. OBL-003's derived/recorded distinction is the mitigation
and it is a label, not a control.

- **outcome:** gap_accepted

**Executed attacks:** none. The state model is derived from the record's shape,
and this repository has no authorization or resolution records to shape it — those
arrive with OW-WAR-0028 and OW-WAR-0031. Planting a false state means planting a
false authorization, which cannot be done here yet. Recorded as absent rather than
omitted, and the roadmap says the same thing in one place for every Warrant in
this position.

## Residual Risk

State without storage is state without history. Transitions cannot be audited until the journal (OW-WAR-0031) exists, so this Warrant delivers the vocabulary and the rules but not the trail.
