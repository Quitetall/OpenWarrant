---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd7-eb6b-7e8f-88ff-6849c6e49e2f
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a Dispatch maps to a Katana request without a PromptIR
- **scope:** §48.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** no PromptIR construction anywhere in the crate; grep proves it.

### OBL-002 — receipts are consumed, never minted
- **scope:** §48.4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant where OpenWarrant fabricates a receipt, refused.

### OBL-003 — the seam declares itself unverified
- **scope:** the absence of a live Katana.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** every run against the seam reports that it has not been exercised
  against a real runtime. This is UNKNOWN, not PASS.

## Gate Adequacy

Not required at `basic`; should be `controlled`. Asked: a seam built against a document rather than a system is a guess with good manners. OBL-003 makes the guess visible; only beta closes it.

## Residual Risk

Katana's schema may have moved since `651ba435`, and this host cannot check. Everything here is provisional until a checkout exists.
