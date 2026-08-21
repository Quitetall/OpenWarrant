---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b570-7f57-85b2-0f8189873d9e
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a real agent answered over the §75.2 seam
- **scope:** one drafting agent, named and version-pinned, on this host.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** the drafting agent, by name and exact version
  or commit.
- **evidence:** a recorded exchange showing the canonical request on stdin and a
  canonical Draft Proposal on stdout, produced by a SEPARATE PROCESS. A proposal
  constructed in-process is this project grading itself.

### OBL-002 — one vague sentence produced an applied draft
- **scope:** a single request, recorded verbatim before the run.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the verbatim request, the returned proposal, the §74.4
  eight-step trace, and the resulting Warrant. `war check` reports the result
  WELL-FORMED with 0 errors.

### OBL-003 — the model wrote no file
- **scope:** the whole run in OBL-002.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** every mutation is one of §74.3's seven operations, and
  a filesystem audit over the agent's lifetime shows no write outside them. A
  plant offering `write_file` as an operation is refused by name.

### OBL-004 — §91.8's seven tests are planted, not unit-tested
- **scope:** §91.8 tests 52 through 58.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** seven entries in `conformance/`, each running the
  shipped binary and each rejected by a named rule with a named detail. Tests 53
  and 54 in particular must refuse an agent that attempts to authorize or to
  allocate an enterprise identifier.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could this pass while the planner is still useless?**
Yes. An agent can satisfy every obligation here by returning a syntactically
perfect Draft Proposal that proposes nothing worth doing. §74 constrains the
SHAPE of a proposal and its honesty about evidence; it says nothing about whether
the plan is any good, and no schema can.

The narrower thing these obligations do buy is the one Phase 2's exit actually
names: the model cannot write files, and a vague sentence reaches a reviewable
artifact. Quality is a human judgment at §74.4 step 6, which is why that step
cannot be skipped.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

§74.8 is enforced against carelessness, not intent. A determined agent
citing a plausible but nonexistent source produces a proposal that validates.
Detecting that needs resolution of every citation, which is OW-WAR-0044's
federation work, not this Warrant's.
