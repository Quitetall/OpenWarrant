---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f41-7a13-9d41-14fd18657244
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — every human act has a row, and no row changes an unregistered Warrant
- **scope:** the act table in OW-ADR-0024, against the acts `war sign`
  performs in this repository at the reviewed commit. No claim about acts
  added later.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - the reviewer lists the acts `war sign --list` and `sign.rs` handle and
    finds a row for each;
  - every §67 name in the table appears in `seam.rs`; a name that does not
    is a defect;
  - every "before registration" cell says the act is unchanged from today.

### OBL-002 — every changed act names the refusal a plant can test
- **scope:** the rows whose "after registration" cell is refused or
  forwarded.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - each such row names a diagnostic rule id and the input that triggers
    it;
  - the reviewer refuses a row that only discourages ("should not",
    "avoid") without naming what the tool refuses.

### OBL-003 — the ADR claims nothing about KF that it cannot cite
- **scope:** every sentence in OW-ADR-0024 about Knowledge Fabric's
  behaviour.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - each such sentence cites a KF file at a named commit, or sits in a list
    of open questions for KF's owner;
  - the gate's `citation-missing` rule fires on a planted citation to a KF
    file that does not exist.

### OBL-004 — the ADR is a parsed, proposed ADR that records the owner's answers
- **scope:** `docs/adr/atoms/OW-ADR-0024-kf-owns-registered-lifecycle.md`
  and `war check` on this repository.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - `war check` counts it among parsed ADRs and reports no `adr.malformed`;
  - its status is `proposed` and its `governs` names this Warrant's uuid;
  - it records the answers to Q-001 to Q-003 and the options not chosen;
  - on a scratch copy, removing its `adr_uuid` makes `war check` report
    `adr.malformed` for it.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-002. An ADR that
says what should happen, without naming what the tool refuses, cannot be
planted, and the follow-on implementation would decide the rule itself.
