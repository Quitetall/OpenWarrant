---
schema: oh.war/atom/v1
warrant_uuid: 01a0a2da-52d6-75b0-835e-7cef44cb0a1f
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — Offline human-readable artifact

- **scope:** public CLI --html in a repository with explicit work reports.
- **evidence:** offline browser renders embedded tracker rows and claimed work state,
  notes/evidence links, filters and source identity without network access; hostile
  title/report strings cannot execute. Existing source destinations refuse unchanged.

### OBL-002 — Local refresh and refusal

- **scope:** public loopback --serve endpoint and browser.
- **evidence:** record edits appear on next refresh; failures keep previous content
  visibly stale; recovered reads clear stale state. Invalid reports show unknown.
  Wrong host/origin, POST, traversal and oversize requests refuse; ordinary GET works.

### OBL-003 — Completion differs from qualification

- **scope:** displayed work reports and existing legacy records.
- **evidence:** completed unverified report renders completed/unverified; resolved
  legacy Warrant without a report has unknown implementation state; no mark granted.

### OBL-004 — Independent review and repository checks

- **scope:** exact delivered source and fixtures.
- **evidence:** independent review, CLI tests, browser scenarios and cargo xtask gate.
  Secure human acceptance remains qualification-only and is not claimed here.
