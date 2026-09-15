---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd4-ab15-7fcf-9b6b-25f71a48d7c5
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the six classes are distinct
- **scope:** claim, evidence item, observation, inference, judgment, resolution.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a test that no class converts silently into another.

### OBL-002 — every prohibited substitution is REFUSED
- **scope:** each substitution enumerated in §40.7, individually.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** one plant per substitution, each naming its own rule. A shared
  error would prove only that something was rejected.

### OBL-003 — evidence carries origin and admissibility
- **scope:** §41.1 origins, §41.2 admissibility, §41.5 custody.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** an evidence item missing custody is refused.

### OBL-004 — a judgment names who made it
- **scope:** §42.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** an anonymous judgment is refused.

## Gate Adequacy

Required at `controlled` — this is the mechanism by which false completion is
prevented, so a defect here defeats every other control.

**Could the obligations pass while a false claim still closes a Warrant?** Yes,
in one way that cannot be closed by typing: an evidence item can be
well-formed, admissible, correctly attributed, and simply WRONG — a measurement
of the wrong thing. §38.4's bounded scope and OW-WAR-0021's verifier independence
reduce that; nothing eliminates it.

- **outcome:** gap_accepted

**Executed attacks:** none against the corpus. The epistemic classes are enforced
in types and unit tests — §40.7's six prohibited substitutions each have a test
asserting they are refused — but no plant exercises them, because no Warrant in
this repository yet records an inference or a judgment to substitute. The
distinction between a tested rule and a planted one is real, and the roadmap
states it: a rule with no plant is a rule nothing has tried to break.

## Residual Risk

Admissibility involves judgement about independence that a type system structures but does not decide. The rule that a performer report is not evidence is mechanical; the rule that a verifier is genuinely independent is not.
