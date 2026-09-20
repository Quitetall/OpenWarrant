---
schema: oh.war/atom/v1
warrant_uuid: 01a0b4a5-58bf-7aa2-b621-7e3254ec72dc
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — deterministic structural declarations
- **scope:** current 15 schema families and explicitly supported generation constructs.
- **evidence:** repeat generation is identical; real TypeScript positive fixtures
  compile and wrong required fields/literals/tuple shapes fail.

### OBL-002 — honest limits and drift refusal
- **scope:** generator and artifact/source identity manifest.
- **evidence:** unsupported construct/reference refuses; edited output fails normal
  check; original JSON pack bytes and digest remain unchanged; runtime-only rules documented.

### OBL-003 — actual KF consumption
- **scope:** KF Warrant package's real build/import boundary.
- **evidence:** pinned generated declaration imports compile in the provider build;
  incompatible field fixture fails; runtime rejection tests remain passing.

### OBL-004 — preserved history and shared integration evidence
- **scope:** this cross-project work and original OW32 records.
- **evidence:** both source revisions and artifact identities retained; one shared
  contract referenced; no original signed atom, verdict or resolution rewritten.
