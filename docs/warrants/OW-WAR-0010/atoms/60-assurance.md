---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd1-b0d3-7be9-a602-9d2144de440d
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the envelope parses and classifies
- **scope:** the three classes of §30.1.
- **evidence:** classification asserted per class.

### OBL-002 — ambiguity ESCALATES
- **scope:** a change matching no class cleanly.
- **evidence:** a plant; the classifier must escalate, never default to local.

### OBL-003 — an unjustified amendment is refused
- **scope:** an amendment record with a class and no justification.
- **evidence:** a plant and its refusal.

## Gate Adequacy

Not required at `basic`. Asked: could a real unauthorized amendment pass? Yes — the classifier sees a diff, not an intention, so a change deliberately shaped to look local will be classified local. This raises the cost of an unauthorized amendment; it does not prevent one.

## Residual Risk

Conservative classification will produce escalations a human considers unnecessary. If that becomes frequent the envelope will be widened, and widening it is how it becomes decorative — so widening requires an ADR.
