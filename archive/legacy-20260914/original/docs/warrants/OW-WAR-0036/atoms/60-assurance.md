---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7951-bda1-e484a57f9940
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — one classifier, not two
- **scope:** OW-WAR-0010 and this Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** both call the same function; a test asserts it.

### OBL-002 — an undocumented normative decision FAILS
- **scope:** §91.4 test 23.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant and its refusal.

### OBL-003 — execution choices are NOT flagged
- **scope:** §19.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant of an in-envelope choice, which must pass. Over-detection is the failure mode that gets the whole feature disabled.

## Gate Adequacy

Not required at `basic`; `controlled` when executed. Asked: a classifier sees a diff, not an intent, so a normative decision expressed as a small edit will be classified as execution. This raises the cost of hiding a decision; it does not prevent it.

## Residual Risk

Over-detection is the practical risk: a detector that flags routine work will be switched off, taking the real detections with it.
