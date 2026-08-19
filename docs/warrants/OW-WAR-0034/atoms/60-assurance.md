---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7188-bfa4-d386a25f4b66
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — proposals are typed
- **scope:** §74.2-§74.3.
- **evidence:** each operation kind round trips.

### OBL-002 — validation precedes application
- **scope:** §74.4.
- **evidence:** an invalid proposal is refused with nothing written; the working tree is unchanged, asserted rather than assumed.

### OBL-003 — no write path in the agent crate
- **scope:** §74.5.
- **evidence:** the existing `no_proposal_kind_names_a_write_target` test, extended to every type in the crate.

## Gate Adequacy

Not required at `basic`; `controlled` when executed. Asked: validation proves a proposal is well-formed, never that it is a good idea. Human review is the only control on intent, and §74 is explicit that the agent proposes rather than decides.

## Residual Risk

A well-formed proposal to do the wrong thing passes every check here.
