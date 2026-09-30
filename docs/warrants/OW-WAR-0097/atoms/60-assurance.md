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

### OBL-002 — diagnostics do not mutate records or run configured commands

- **scope:** disposable initialized CLI fixture with a backend that writes a marker.
- **evidence:** exact file snapshot before/after doctor is equal; marker absent.

### OBL-003 — diagnostic coverage does not fabricate authority or admission

- **scope:** CLI JSON fixture reports, absent authority and unknown target.
- **evidence:** admission UNKNOWN, execution_authorized false, absent optional
  authority warning, unknown Warrant refused, exact remedy argument vector.
