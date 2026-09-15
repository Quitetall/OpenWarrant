---
schema: oh.war/atom/v1
warrant_uuid: 01a0650e-5702-7f52-ba66-dbaee871efba
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a requirement that does not exist cannot be implemented
- **scope:** §34.1; `implements` refs against §106 of the pinned SAS.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the plant "a requirement reference §106 does not contain"
  refused as `traceability.unknown-requirement` naming `RQ-999`; the
  committed corpus passes the same check.

### OBL-002 — every named control has an executed attack
- **scope:** the plants and tests listed in the work order.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each plant in the battery under this Warrant's section,
  rejected by its named rule; each test named, passing.

### OBL-003 — the §94 baseline is current and declared
- **scope:** `artifacts/telemetry-baseline.json`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war telemetry --commit <sha> --verify` passing at the
  commit it names; the artifact declared in OW-WAR-0041's deliverables.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can a control be claimed without having been made
to refuse? The attacks are the plants themselves; each names the rule and
the detail it expects, and the battery fails if a plant's mutation is a
no-op.

- **outcome:** counterexample_found, gate_added

## Residual Risk

- A test that reads the committed corpus passes on whatever is committed;
  it says the corpus is consistent, not that any one Warrant is done.
- The unknown-requirement check reads §106 of the pinned SAS; a SAS with a
  malformed §106 table is treated as unreadable, and then nothing is
  refused and nothing is vouched for.
