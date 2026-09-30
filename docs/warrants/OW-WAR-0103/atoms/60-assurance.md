---
schema: oh.war/atom/v1
warrant_uuid: 01a0b286-9220-7191-a979-34dc8b2ef31d
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — damaged storage never becomes empty success
- **scope:** CLI questions, answers, watch and console using fixture repository.
- **evidence:** original regression fails before fix and passes after; missing store
  remains valid, valid answer survives, damaged store/record refuses.

### OBL-002 — partial diagnostics survive MCP
- **scope:** real stdio JSON-RPC war_questions and war_answers against same fixture.
- **evidence:** healthy blocking record remains visible beside malformed companion;
  report exit status and MCP error flag identify incomplete observation; answers refuse.

### OBL-003 — integration remains intact
- **scope:** current repository and inbox/MCP consumers.
- **evidence:** targeted suites, clippy, generated checks and full repository gate.
