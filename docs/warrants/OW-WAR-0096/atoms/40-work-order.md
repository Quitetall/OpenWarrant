---
schema: oh.war/atom/v1
warrant_uuid: 01a0ad2f-c0fd-7471-a00e-c7a9eb8f89d1
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work order

Deliver core authority_transition module, authority CLI (draft/propose/approve/check/
bootstrap/activate/status/history/allows), transactional snapshot store, Linux
sandbox runner, public seam tests, candidate ADR, threat model and operator guide.

Preserve existing legacy files and signatures. No effective grant, SAS acceptance,
production account creation or human signing is authorized by this implementation
report. Core functions have no I/O; CLI uses system OpenSSH. All provider calls
must remain local/free unless reliable spend accounting permits an explicit cap.

One writer per file; independent reviewers inspect fixed base and candidate diff.
Rollback: stop using the candidate CLI/runner; retained legacy records remain.
Do not roll back protected accepted state to resurrect revoked keys. Recovery is
a forward signed transition under previously configured recovery permissions.
