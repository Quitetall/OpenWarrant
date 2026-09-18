---
schema: oh.war/atom/v1
warrant_uuid: 01a0b286-9220-7191-a979-34dc8b2ef31d
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

Owner authorized remaining implementation and verified bug fixes. The actual
fixture reproduced `questions` and `answers` returning success after their question
directory became a regular file. `inbox` already refused through its own path guard.
`questions.rs` swallowed enumeration errors; tolerant reads dropped them; watch,
console and MCP discarded diagnostics. Existing schema and authority semantics
remain binding. No new signing or responder authority is introduced.
