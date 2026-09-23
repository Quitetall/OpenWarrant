---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f55-7171-906f-45d79a408ce3
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a native reference must name a holder that is not Git
- **scope:** `deliverable.rs` validation, exercised by
  `61-native-authority.sh` on a scratch corpus, for the reference form in
  OW-ADR-0026. No claim about other forms.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a native deliverable with provenance and a non-`git` holder passes
    `war check`;
  - `war check` refuses, each by its named rule:
    - the same deliverable with no provenance;
    - one with Source Holder `git`;
    - one with an empty Source Holder;
    - one with `content_addressed = true` and no content digest.

### OBL-002 — nothing reads, rewrites or corrects a native-held artifact
- **scope:** `war pins --refresh`, `war pins`, `war correct` and
  `war check`'s drift pass, on a scratch Warrant with one native and one
  repository deliverable.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - after `war pins --refresh`, the native deliverable's recorded digest is
    byte-identical; the repository deliverable's moves as today;
  - `war pins` lists the native one as held by its system;
  - `war correct <alias> <native D-id>` is refused, names the holder, and
    leaves the tree unchanged;
  - `war check` reports no drift and no `pins.unreadable` for it;
  - a file created at the path the reference's text would name, if joined
    to the root, is never read (the plant changes its bytes; nothing moves).

### OBL-003 — a native deliverable neither passes nor reads as missing
- **scope:** `war resolve --dry-run` and `war status` on the scratch
  Warrant. §56.1 requirements 2 and 3 only.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - with a required native deliverable, requirements 2 and 3 are unmet,
    and an UNKNOWN diagnostic names the holder;
  - no line says the deliverable does not exist;
  - `war status` shows "held by <system>", not unreadable;
  - the same Warrant with the native deliverable made `required = false`
    and a present repository deliverable meets requirement 2, so the plant
    shows the rule is specific to native references.

### OBL-004 — the verifier is told what it cannot see
- **scope:** `war verify --bundle` on the scratch Warrant.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** the bundle holds the native deliverable's reference, holder
  and recorded digest, marked not read, and no bytes for it. The repository
  deliverable's bytes are present as today.

### OBL-005 — the chosen form is a parsed, proposed ADR
- **scope:** `docs/adr/atoms/OW-ADR-0026-native-held-deliverables.md`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check` parses it with no `adr.malformed`; its status is
  `proposed`; its `governs` names this Warrant's uuid; it records the
  owner's answer to Q-001 and the forms not chosen.

### OBL-006 — every existing Warrant is unaffected
- **scope:** every `deliverables.toml` in this repository at the delivery
  commit.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - every existing deliverable classifies as a repository path;
  - `ownership::set_digest` is identical before and after for every
    authorized Warrant (a test compares them);
  - `war check` on the corpus reports no error it did not report before.

## Gate Adequacy

Required at `basic`. The load-bearing obligations are OBL-002 and OBL-003.
A native reference that any reader still joins to the repository root
either re-owns the artifact or reports it missing; the planted file at the
joined path is what shows no reader does.
