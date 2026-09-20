---
schema: oh.war/atom/v1
warrant_uuid: 01a0ac9b-3953-7511-86f6-7face232c664
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. The SAS accepted at 0.1.0 (`war sas propose 0.1.0`, then a human's `war sign 0.1.0 --ssh-sign`).
2. This Warrant authorized (`war authorize IX-WAR-0004`, then `war sign IX-WAR-0004 --ssh-sign`).
3. Gate evidence recorded (`war evidence record IX-WAR-0004`).
4. An independent verification ingested (`war verify IX-WAR-0004 --performer <you>` handed to a separate context; `war verify IX-WAR-0004 --response <file>`).
5. This Warrant resolved (`war resolve IX-WAR-0004`, then `war sign IX-WAR-0004 --ssh-sign`).

## Frozen Surfaces

`docs/authority/` — human-written; no tool writes it.

## Premade Instructions

- `war next` before every step; it names whose act comes next.
- Never edit a record to make `war check` go green.

## Autonomy and Escalation

Tier T2: the agent drafts and records; every signature is a human's.

## Rollback

Delete `docs/warrants/IX-WAR-0004/`; nothing else depends on it yet.
