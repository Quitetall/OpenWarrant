---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e81-7722-b56c-53f89968bd3c
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — exact source-set proposal and acceptance
- **scope:** new source-set proposal, preview, acceptance and status interfaces; main SAS, manifest, all members, adoption decision, edition and predecessor.
- **evidence:** a valid fixture produces a captured v2 subject and request; a trusted human fixture response accepts that exact subject; preview exposes identities without writes; changed current sources remain distinct from accepted historical capture. The real RC.2 candidate is only proposed until the human acts.

### OBL-002 — stale or incomplete subjects are refused
- **scope:** changed main, normative companion, schema, reference fixture, manifest, decision or predecessor; missing/duplicate/escaping/symlink paths, oversized or inconsistent reads, destination collisions, wrong-actor and stale responses.
- **evidence:** each named mutation produces its specific refusal through the public interfaces and no accepted record. A historical v1 signature cannot satisfy a new v2 subject. No fixture may succeed solely from constant-success output.

### OBL-003 — independent verification and human acceptance
- **scope:** this Warrant's implementation revision, retained history, exact candidate subject and OBL-001/002/005 evidence.
- **evidence:** a separate verifier or authorized human reproduces observations from exact fixtures/build/source identities and supplies bounded findings. Performer tests do not clear this obligation. Required human corrections, SAS acceptance, and final resolution remain separately recorded acts.

### OBL-004 — record integrity and required checks
- **scope:** revised Warrant records, historical attestations, implementation and required repository checks.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** qualified structural receipt plus focused tests and aggregate repository check results on the pinned toolchain. Pending revision authorization or corrections remain explicit blockers; expected refusal fixtures do not stand in for a green real-corpus gate. Preserve all original planted controls.

### OBL-005 — legacy compatibility and publication safety
- **scope:** v1 proposal/acceptance/status, existing SAS pins, original revision-1 authorization/response/attestation bytes, and new snapshot publication.
- **evidence:** existing v1 tests still pass, retained signatures verify, old Warrants keep their basis, and refusal/collision tests cannot overwrite an existing snapshot or revision. The current accepted historical 1.0.0 record is never relabeled in place.

## Gate Adequacy

The source-set extension has not run. Earlier preparation observations retain
their stated bounds and establish no runtime obligation here. No disposition is
declared. Record commands, exit codes, inputs and build identities; unavailable
checks are UNKNOWN, not PASS. Required evidence and independent judgment are
needed in addition to a structurally valid record.

**Adversarial question:** can copied assertions, an unverified manifest, a forged
human-kind field, or a main-document-only signature satisfy this contract? No.
Verify all captured bytes, the full signature subject, actual authority, and
observed refusal behavior at the same public seams used by callers.
