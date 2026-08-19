---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-76a1-b0c8-afc3276d6ae1
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — dimensions are declared per verification
- **scope:** §46.1.
- **evidence:** a verification with no declared independence is refused.

### OBL-002 — self-verification is REFUSED
- **scope:** RQ-053.
- **evidence:** a plant where performer and verifier are the same actor.

### OBL-003 — single-actor independence reports as NONE
- **scope:** this repository today.
- **evidence:** every verification reports independence: none, on every
  dimension, rather than silently passing the minimum.

## Gate Adequacy

Not required at `basic`, but this Warrant should be `controlled` when executed. Asked: independence is self-declared, so a single actor can declare two identities. Nothing local prevents it; KF identity (OW-WAR-0029) is what would.

## Residual Risk

This repository cannot satisfy any independence minimum, by construction. Every controlled Warrant here is therefore resolvable only by recording that gap — which is the honest position and will look like an obstruction.
