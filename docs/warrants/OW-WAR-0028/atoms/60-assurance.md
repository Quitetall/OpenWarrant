---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-7118-8a05-0e8bf46ef650
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — actions are typed and enveloped
- **scope:** §67.
- **evidence:** every state change goes through an action.

### OBL-002 — direct status edits are REFUSED
- **scope:** RQ-076.
- **evidence:** a plant; ideally it does not compile.

### OBL-003 — concurrency and idempotency hold
- **scope:** §67.3, §67.4.
- **evidence:** a stale write refused; a duplicated action applied once.

## Gate Adequacy

Not required at `basic`; should be `controlled` when executed. Asked: typed actions constrain the CLIENT. A compromised or buggy server applies whatever it accepts, and nothing here checks the server.

## Residual Risk

This is a client-side contract only. Server-side enforcement is KF's, and testing it is beta.
