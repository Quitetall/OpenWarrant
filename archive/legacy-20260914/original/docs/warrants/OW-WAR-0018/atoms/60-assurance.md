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
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `grep -c 'contains("adequacy")'` returns 0, and a controlled
  Warrant with the word 'adequacy' and no review record FAILS.

### OBL-002 — a review with no executed attacks is REPORTED
- **scope:** §39.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant; and the three existing reviews in this repository, which
  are currently in exactly that state, must be reported by the same rule.

### OBL-003 — existing reviews migrate without loss
- **scope:** OW-WAR-0003, 0005, 0007, 0016, 0017 adequacy sections.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each migrates with its named gaps preserved verbatim.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could this pass while adequacy is still theatre?** Yes,
and the honest answer has two halves. A structured review can record a shallow
question, a trivial attack, and a confident outcome; §39 cannot make a reviewer
adversarial, and §39.5 says so directly. What the check CAN do is make the
absence of a question and the absence of executed attacks visible, which is the
state most of this repository is actually in.

- **outcome:** counterexample_found, gate_strengthened

The counterexample was not constructed. It was already here: OW-WAR-0023 carries
a `## Gate Adequacy` section that states a conclusion and never asks §39.1's
question. It passed the substring check for two weeks because it contains the
word. It fails this one, unplanted, on real data.

**Executed attacks:**
- deleted the `## Gate Adequacy` section from a `controlled` Warrant; refused by `assurance.adequacy-review` (§39.4)
- replaced a review body with confident prose containing no question; refused by `assurance.adequacy-review` (§39.1)
- retitled the section `## Notes On Adequacy`; the section no longer opens a review, so the missing-review rule fires rather than a substring match passing
- ran the new check against all 40 Warrants unmodified: 1 real error, 28 real warnings, 0 grandfathered

## Residual Risk

The reviews in this repository will fail their own new check on day one, by design. That is the honest starting position and must not be papered over by grandfathering them.
