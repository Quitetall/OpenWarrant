---
schema: oh.war/atom/v1
warrant_uuid: 01a0746d-d84b-7401-bc57-fb7d6304aeb1
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — signed phases preserve program identity
- **scope:** canonical roadmap references and selected SAS phase declarations.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** all fourteen LIM phases -1–12 parse and group as LIM; OW's 0–10
  remain valid; malformed, duplicate, undeclared and wrong-program refs refuse.

### OBL-002 — phase authority is bound to exact source bytes
- **scope:** selected SAS and explicitly requested revision snapshots.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** matching source/digest yields declarations; missing/multiple
  documents, digest drift, duplicate phases/requirements and mismatched pins
  produce unavailable/refusal, without another program's fallback.

### OBL-003 — inspection cannot manufacture completion
- **scope:** corpus status release authority, objective achievement and SAS
  requirement status under unavailable or proposed/unregistered authority.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** draft inspection is labelled; false completion prose establishes
  nothing; unavailable authority emits no substituted phase or requirement index.

### OBL-004 — compatibility and projections are independently checked
- **scope:** this code change, unchanged historical documents, retained OW
  declaration behavior and regenerated status projections.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** targeted tests, Rust 1.97.1 aggregate gate, generated-view drift
  check, and independent review tied to exact final commit. No review or
  acceptance is claimed by this authored assurance atom.

## Gate Adequacy

**Adversarial question:** Can a record pass the declared gates while using a
phase from another program, an unbound SAS, or a completion claim unsupported
by accepted authority?

An independent reviewer must check that syntactic parsing does not substitute
for membership, that draft status cannot be read as accepted, and that missing
authority cannot make an objective appear complete.
