---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be73-76f7-9aa7-d883cc39d51e
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a real KF instance answered typed actions
- **scope:** one running Knowledge Fabric instance, named and version-pinned.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** Knowledge Fabric, by exact commit or release.
- **evidence:** a recorded action envelope and its receipt, where `recorded_at`
  was assigned BY THE SERVER. A client-supplied value is refused, and that
  refusal is recorded too.

### OBL-002 — the enterprise identifier came from KF, not from us
- **scope:** §12.4 and §91.3 test 20.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** Knowledge Fabric's allocator.
- **evidence:** an identifier returned in a KF receipt, plus a recorded refusal
  of a locally-derived identifier of identical SHAPE. The contrast is the
  evidence: shape alone cannot distinguish them, provenance can.

### OBL-003 — Git remained Source Holder after registration
- **scope:** §91.3 test 21, for the registered Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** after registration, the authored atoms still resolve to
  Git and their digests are unchanged. A plant asserting KF as Source Holder for
  an authored atom is refused.

### OBL-004 — a §68 round trip preserved semantic and digest identity
- **scope:** the registered Warrant, exported into an empty compatible instance.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** export, reconnect preserved bytes, re-export, compare.
  Digest identity holds AND the recorded semantic-difference list is empty. A
  comparison run without reconnecting the bytes is refused as vacuous.

### OBL-005 — §91.3 and §91.13 are planted
- **scope:** §91.3 tests 19 and 22, §91.13 tests 91 through 95.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** seven entries in `conformance/` against the shipped
  binary, each rejected by a named rule with a named detail.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could a Warrant be registered while authority is quietly
confused?** Yes, and this is the failure the exit criterion is worded to prevent.
The tempting implementation registers a Warrant and, in the process, starts
treating KF as the answer to every question about it — including where its source
lives. Nothing would visibly break. `war check` would pass. The property that
died is one nobody looks at until Git and KF disagree about a byte.

OBL-003 exists because that failure is silent, and it is written as a digest
comparison rather than a design review for the same reason.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

One instance. §91.3 test 18 (the same local alias in two repositories must
not collide) is genuinely cross-instance and cannot be discharged against a single
KF, so it stays open here and is not claimed.
