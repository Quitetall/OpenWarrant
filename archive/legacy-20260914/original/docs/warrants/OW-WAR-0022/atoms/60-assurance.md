---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-7339-94c1-44ae5d81a7df
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — resolution binds exact snapshots
- **scope:** RQ-059.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a resolution citing a contract digest that no longer matches is
  detected.

### OBL-002 — falsification is a resolution
- **scope:** §56.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a Warrant resolved falsified retains the measurement that
  disproved it and is not reported as an error.

### OBL-003 — dispute and annulment preserve history
- **scope:** §91.6 tests 40 and 41.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two plants; the original resolution is readable after both.

### OBL-004 — a required unknown BLOCKS
- **scope:** RQ-054.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant; resolution refuses.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could a resolution be recorded that asserts more than was
verified?** Yes, and §56.1's thirteen requirements are the only thing standing in
the way. They are implemented as thirteen named booleans rather than a count,
because "11 of 13" tells a reader nothing about whether to worry, and each one is
tested to block on its own — a conjunction where one term does nothing is a
conjunction with a hole in it.

- **outcome:** gate_added, gate_strengthened, gap_accepted

`gate_added`: three vocabulary distinctions the SAS draws and that are easy to
flatten now cannot be. §56.3's `falsified` requires a falsifiable claim in the
PROFILE, not in the resolver's account after the fact — otherwise a failed
delivery gets dressed up as a finding. §56.5's annulment leaves the original
historical. §56.6's supersession records replacement, NOT invalidity, so a
superseded resolution keeps `valid` standing; it was and remains correct for the
contract it was recorded against.

`gap_accepted`: this repository has no authorization or resolution records for
these types to govern. The machinery is implemented and unit-tested; nothing in
the corpus exercises it, because the local journal arrives with OW-WAR-0031. That
gap is recorded in PRODUCTION_ROADMAP.md rather than hidden behind the word
"resolved".

**Executed attacks:**
- unset each of §56.1's thirteen requirements in turn; each blocked alone and was named in the error
- recorded `falsified` against a profile carrying no falsifiable claim; refused
- annulled a resolution and compared the original byte for byte; unchanged
- superseded a resolution and checked its standing; still `valid`, because replacement is not invalidity
- blanked each of §56.4's six dispute fields in turn; each named

## Residual Risk

Self-certification, as above. Every controlled Warrant in this repository will resolve under recorded-absent independence until federation exists.
