---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be74-73ce-8f9d-105d80ab82fc
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a real Katana executed, over a versioned seam
- **scope:** one Katana instance, named and version-pinned.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** Katana, by exact commit or release.
- **evidence:** a §48.4 receipt with all eleven minimum fields populated and a
  `dispatch_digest` matching the Dispatch we compiled. A receipt for a different
  Dispatch is evidence about different work and is refused.

### OBL-002 — the agent received the Dispatch and nothing else
- **scope:** one stage, one attempt.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the exact bytes handed to the agent, digested and
  compared against the compiled Dispatch. Plus a recorded run where a required
  input was deliberately omitted and the agent raised a §53.1 BLOCKER rather than
  improvising.

### OBL-003 — the performer could not and did not resolve
- **scope:** §51.2, for the executed attempt.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the Submission requests one of continue, verify, block,
  amend, cancel. Plus a plant submitting `resolve` as text, refused by name
  against the shipped binary.

### OBL-004 — realized capability was within authorization
- **scope:** §48.3, for the executed attempt.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the receipt's `realized_capabilities` compared against
  the Dispatch's authorization, with the comparison recorded. A plant adding one
  unauthorized capability to a receipt is refused by
  `UnauthorizedCapabilityRealized`.

### OBL-005 — §91.7 and §91.9 are planted
- **scope:** §91.7 tests 43 through 51 and §91.9 tests 59 through 63.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** fourteen entries in `conformance/`, each running the
  shipped binary and rejected by a named rule with a named detail.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could a Dispatch execute cleanly while authority is
confused?** Yes, in a way no schema can catch. A capable agent handed an
insufficient Dispatch will often succeed anyway, using what it already knows
about this repository. Every field validates, the receipt is well formed, and the
conclusion "the Dispatch was sufficient" is false.

That is why OBL-002 requires a deliberate omission and a recorded blocker. It is
the only obligation here that tests the compiler rather than the runtime, and it
is the one most likely to be quietly dropped as awkward.

The related limit is that statelessness is asserted, not proved. We control what
we send; we do not control what the agent remembers.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

§91.7 test 47 (unsupported BLUT lowering fails rather than degrades) belongs
to §91.7 but is discharged by OW-WAR-0047, not here. Recorded so the subsection is
not double-counted as complete by either Warrant.
