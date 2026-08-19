---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-7339-94c1-44ae5d81a7df
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — resolution binds exact snapshots
- **scope:** RQ-059.
- **evidence:** a resolution citing a contract digest that no longer matches is
  detected.

### OBL-002 — falsification is a resolution
- **scope:** §56.3.
- **evidence:** a Warrant resolved falsified retains the measurement that
  disproved it and is not reported as an error.

### OBL-003 — dispute and annulment preserve history
- **scope:** §91.6 tests 40 and 41.
- **evidence:** two plants; the original resolution is readable after both.

### OBL-004 — a required unknown BLOCKS
- **scope:** RQ-054.
- **evidence:** a plant; resolution refuses.

## Gate Adequacy

Required at `controlled` — resolution is the claim everything else supports.

**Could a Warrant resolve falsely?** Yes: with one actor, judgment authority is
self-asserted (RQ-058) and independence is none (OW-WAR-0021). A locally resolved
controlled Warrant is a self-certified one. That is a governance gap, not a code
gap, and it closes only with KF identity.

**Executed attacks:** recorded here when run.

## Residual Risk

Self-certification, as above. Every controlled Warrant in this repository will resolve under recorded-absent independence until federation exists.
