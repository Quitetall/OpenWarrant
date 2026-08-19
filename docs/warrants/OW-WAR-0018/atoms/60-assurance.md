---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd4-ab16-7823-bf23-99904a35aea0
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the substring check is gone
- **scope:** `check.rs`.
- **evidence:** `grep -c 'contains("adequacy")'` returns 0, and a controlled
  Warrant with the word 'adequacy' and no review record FAILS.

### OBL-002 — a review with no executed attacks is REPORTED
- **scope:** §39.3.
- **evidence:** a plant; and the three existing reviews in this repository, which
  are currently in exactly that state, must be reported by the same rule.

### OBL-003 — existing reviews migrate without loss
- **scope:** OW-WAR-0003, 0005, 0007, 0016, 0017 adequacy sections.
- **evidence:** each migrates with its named gaps preserved verbatim.

## Gate Adequacy

Required at `controlled`.

**Could this pass while adequacy is still theatre?** Yes. A structured review can
record a shallow question, a trivial attack, and a confident outcome. §39 cannot
make a reviewer adversarial. What it CAN do — and what OBL-002 forces — is make
the absence of executed attacks visible, which is the state this repository is
actually in today.

**Executed attacks:** recorded here when run.

## Residual Risk

The reviews in this repository will fail their own new check on day one, by design. That is the honest starting position and must not be papered over by grandfathering them.
