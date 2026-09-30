---
schema: oh.war/atom/v1
warrant_uuid: 01a0b230-6c1e-76a3-acf0-82ea0221814a
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

### OBL-001 — readable history has an open stage

- **scope:** disposable inbox CLI fixture.
- **evidence:** successful frontier with open count greater than zero.

### OBL-002 — damaged history cannot produce open work

- **scope:** malformed journal, directory container and invalid milestones fixtures.
- **evidence:** nonzero CLI result naming damaged input, no frontier result on malformed journal.

### OBL-003 — absent journal is distinct from damaged journal

- **scope:** fixture after removing journal.
- **evidence:** frontier succeeds.
