# OW-WAR-0097: proposed assurance amendment

Status: draft for human review. This document does not amend or authorize the
Warrant. Revision 1 and its signed contract remain unchanged.

## Problem

The work order requires focused CLI controls and the repository gate. The three
assurance obligations describe those controls, but name no executable gate.
`war evidence record OW-WAR-0097 --json` therefore refuses before writing evidence:

> the assurance atom cites no gate, so there is no required result to record

Adding an arbitrary structural gate would not demonstrate the doctor's behavior.
`software.repo.war-check@1.0.0` checks record structure and generated projections;
its definition explicitly says it does not execute acceptance checks.

## Proposed revision 2

Preserve OBL-001, OBL-002 and OBL-003, their scope, and their evidence statements.
Add a gate reference to each obligation:

```markdown
- **gate:** gate://software.cli.doctor@1.0.0
```

Add a separate structural obligation, without claiming structural validation
proves the behavioral obligations:

```markdown
### OBL-004 — repository records and generated projections remain consistent

- **scope:** the amended repository corpus at the recorded code revision.
- **evidence:** the repository checker passes; its registered refusal controls
  remain documented in the gate definition.
- **gate:** gate://software.repo.war-check@1.0.0
```

Add the doctor gate definition and its execution adapter to the work order and
deliverable manifest. Draft the definition as a local candidate, initially
`draft`. Qualify its declared controls before changing its lifecycle or binding
it as executable acceptance evidence. That qualification is distinct from
independent verification of this Warrant.

The doctor adapter runs:

```text
cargo +1.97.1 test --locked -p openwarrant-cli --test doctor_cli
```

It must test the exact source tree bound by the gate receipt. Builds write
artifacts and tests create temporary fixtures. The review adapter runs Cargo with
`--frozen` and a fresh temporary target directory outside the source tree. A
service-stage binding does not bypass the current runner's refusal of mutating
gates; declaring an in-place build mutating would not make it executable.

The adapter must propagate failing tests and compilation errors. Missing tools
are unavailable observations, not evidence of a doctor defect. The current gate
runner maps nonzero child exits to FAIL even for missing nested prerequisites;
registration remains blocked until that classification preserves UNKNOWN. Its
qualification needs observed passing and failing commands and a before/after
snapshot proving its claimed repository isolation. Pin its inputs sufficiently
to reject reuse after a relevant source or fixture change.

## Coverage of the existing tests

| Obligation | Existing observation in `tests/doctor_cli.rs` |
| --- | --- |
| OBL-001 | Malformed repository and authority TOML produce named errors in a valid report; authority failure leaves performer configuration visible. |
| OBL-002 | Exact recursive fixture snapshot is unchanged; a configured backend's marker is absent. |
| OBL-003 | Admission is `UNKNOWN`, execution authorization is false, optional authority is warned about, unknown target is refused, and remedy argv is exact. |
| Positive/refusal pair | A clean scaffold succeeds; a deliberately changed generated document fails with `generated.drift` and the exact file path. |

These observations establish the specified fixture behavior. They do not prove
the gate adapter propagates failure, preserves source identity, or isolates its
build. Those controls must be added and observed before gate qualification.

## Current observation

- Source checkout: `7df5723a990269deb2be56fbd3f0b5a61e8b27ec`.
- Date: 2026-09-30.
- Command: `cargo +1.97.1 test --locked -p openwarrant-cli --test doctor_cli`.
- Result: exit 0; 2 passed, 0 failed, 0 ignored. Both named tests above ran.
- The doctor implementation and test file have no committed differences between
  this source revision and `claude/friction` at `1dcbfd796b25e3ad25e53eca08c6247df978e1df`.
- This is a performer observation, not a recorded acceptance receipt or an
  independent disposition.

## Activation and closure

After review, prepare a proper amendment record and revised authored atoms through
the repository's amendment procedure. Bind revision 2 to its exact contract;
retain revision 1, its authorization, and all earlier observations. Do not write
a human authorizer or an effective signature into this proposal.

After the new gate is qualified and the revision is authorized, record fresh
gate evidence. Request independent verification of all four bounded obligations.
Only then evaluate `war resolve --dry-run OW-WAR-0097`. A human performs the
resolution signature; neither this draft nor the existing completion report
clears that act.
