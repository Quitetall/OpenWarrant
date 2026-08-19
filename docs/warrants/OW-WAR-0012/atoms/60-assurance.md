---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-7dc6-a3b7-92bb556e3569
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — context items carry role and trust class
- **scope:** the vocabularies of §33.2 and §33.3.
- **evidence:** parsed values asserted per class.

### OBL-002 — conflicts and unsourced summaries are REFUSED
- **scope:** two context items conflicting with no precedence; a summary with no
  named source.
- **evidence:** two plants, two refusals.

### OBL-003 — the context manifest has a digest
- **scope:** `oh.war/context-manifest/v1`.
- **evidence:** a digest computed under that domain, distinct from every other.

## Gate Adequacy

Not required at `basic`. Asked: trust classes are self-declared, so a mislabelled source defeats precedence entirely. Verification of trust class is out of scope everywhere in the SAS — it is a governance control, not a parser control.

## Residual Risk

Self-declared trust is the whole model's soft spot and it is inherited from the specification, not introduced here.
