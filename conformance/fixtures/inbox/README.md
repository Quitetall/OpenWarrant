# Inbox conformance inputs

All records here are **synthetic test data**, not authorizations, signatures,
assurance or production history. Do not ingest them into a real project.

The repository was scaffolded with `war init --namespace IX --program Inbox`.
Its four records exercise the existing loader:

- IX-WAR-0001: unsigned, waiting for authorization.
- IX-WAR-0002: synthetic authorization plus an unanswered blocking question;
  waits for an answer.
- IX-WAR-0003: synthetic current authorization; agent work remains; omitted.
- IX-WAR-0004: journal records resolved history, with no binding resolution
  claim. No current signing request; omitted. This tests the existing distinction
  between recorded phase history and a currently binding satisfied resolution.

The first two timestamps denote the same instant with different offsets; alias
order must break that tie. Tests also remove and corrupt timestamps, append a
non-transition event, corrupt each input kind, and check byte-for-byte read-only
behavior.

`classifier.json` covers all seven phases and the agent/gate/none defaults.
`inbox_classify.rs` adds every human act over every phase, including precedence.
The legacy journal cannot encode every core phase, so gate-only coverage belongs
to this pure classifier fixture rather than a fabricated new journal event.

`inbox.schema.json` is the OW-ADR-0018 candidate, not an adopted schema-pack entry.

Run schema checks with `python3 check-schema.py /absolute/path/to/war` from this
directory after installing `jsonschema[format]`. The check refuses missing date-time
validation support and exercises malformed-field controls, not just positive output.
