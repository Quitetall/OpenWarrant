---
schema: oh.war/atom/v1
warrant_uuid: 01a0b286-9220-7191-a979-34dc8b2ef31d
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables
- Treat only missing question storage as empty; preserve enumeration/read failures.
- Refuse non-directory stores and nonregular TOML records, including symlinks.
- Keep healthy questions visible in partial listings with blocking diagnostics.
- Require complete reads for answers, watch and console; preserve MCP diagnostics
  and error status without fabricating an emitted-success report.
- Public CLI and JSON-RPC tests for valid records, missing stores, directory replaced
  by file, malformed companion, nonregular record, symlink store and valid answer.

## Limits
No schema/canonicalization change, signatures, authority activation, independent
verification, execution pause/resume, or claim of malicious-owner filesystem isolation.
No resolved pin is changed. Roll back code if needed; retain observed failure evidence.
