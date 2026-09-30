---
schema: oh.war/atom/v1
warrant_uuid: 01a0adb3-c076-7db1-b217-b3a727183342
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

### OBL-001 — malformed configuration produces an actionable doctor report

- **scope:** CLI fixtures with invalid repository TOML and invalid authority TOML.
- **evidence:** nonzero exit, specific error diagnostic, valid report envelope;
  authority failure does not suppress performer configuration observations.
- **gate:** gate://software.cli.doctor@1.0.0

### OBL-002 — diagnostics do not mutate records or run configured commands

- **scope:** disposable initialized CLI fixture with a backend that writes a marker.
- **evidence:** exact file snapshot before/after doctor is equal; marker absent.
- **gate:** gate://software.cli.doctor@1.0.0

### OBL-003 — diagnostic coverage does not fabricate authority or admission

- **scope:** CLI JSON fixture reports, absent authority and unknown target.
- **evidence:** admission UNKNOWN, execution_authorized false, absent optional
  authority warning, unknown Warrant refused, exact remedy argument vector.
- **gate:** gate://software.cli.doctor@1.0.0

### OBL-004 — repository records and generated projections remain consistent

- **scope:** the amended repository corpus at the recorded code revision.
- **evidence:** the repository checker passes; its registered refusal controls
  remain documented in the gate definition.
- **gate:** gate://software.repo.war-check@1.0.0
