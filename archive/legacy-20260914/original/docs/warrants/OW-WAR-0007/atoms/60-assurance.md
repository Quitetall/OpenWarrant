---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e1-71e3-a237-3554610254d3
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-000 — the structured-atom parser is chosen by ADR
- **scope:** the parser used for `.yaml` atoms.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a merged ADR naming it, with its licence checked against the
  Apache-2.0 path.

### OBL-001 — milestones and stages parse into typed values
- **scope:** the `oh.war/milestones/v1` schema as used by this repository's Warrants.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** parsed values asserted field by field, including ports.

### OBL-002 — every malformed graph is REFUSED
- **scope:** dangling `stage_refs`, dangling `obligation_refs`, duplicate
- **gate:** `gate://software.repo.war-check@1.0.0`
  milestone id, duplicate stage id, `depends_on` cycle.
- **evidence:** five plants, five observed refusals, each naming its rule.

### OBL-003 — the existing corpus validates
- **scope:** all Warrants under `docs/warrants/`, enumerated at run time.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check` exit 0 with the milestone graph reported.

## Gate Adequacy

Required at `controlled` (§39.4).

**Adversarial question: could every obligation pass while a Warrant's plan is
still nonsense?**

Yes, in two ways, both accepted. First, a well-formed graph can describe work
nobody intends to do — structure is not intent, and no parser fixes that.
Second, and more sharply: this Warrant makes milestones READABLE but not
ANSWERABLE. Whether a milestone is MET still cannot be computed, because that
needs the state model and gate runs. Anyone reading a parsed milestone list may
reasonably assume the tool knows whether it is done. It does not, and the
Overview must say so until OW-WAR-0020 lands.

- **outcome:** no_counterexample_found, gap_accepted

**Executed attacks:**
- planted a `stage_ref` naming a stage that is not declared; refused by `milestones.invalid`
- planted a dependency cycle among milestones; refused by `milestones.invalid`
- planted a milestone carrying a stage-only field; refused by `milestones.invalid`
- each of the three asserts a distinct detail string, so one branch cannot answer for another

## Residual Risk

The structured-atom parser widens the input surface OW-ADR-0002 deliberately narrowed. If the decision is to adopt a full YAML parser for `.yaml` atoms, the expansion-DoS argument from that ADR applies again and must be answered, not inherited.
