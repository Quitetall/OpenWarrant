# Work Order

## Deliverables

1. The SAS accepted at 0.1.0 (`war sas propose 0.1.0`, then a human's `war sign 0.1.0 --ssh-sign`).
2. This Warrant authorized (`war authorize {{alias}}`, then `war sign {{alias}} --ssh-sign`).
3. Gate evidence recorded (`war evidence record {{alias}}`).
4. An independent verification ingested (`war verify {{alias}} --performer <you>` handed to a separate context; `war verify {{alias}} --response <file>`).
5. This Warrant resolved (`war resolve {{alias}}`, then `war sign {{alias}} --ssh-sign`).

## Frozen Surfaces

`docs/authority/` — human-written; no tool writes it.

## Premade Instructions

- `war next` before every step; it names whose act comes next.
- Never edit a record to make `war check` go green.

## Autonomy and Escalation

Tier T2: the agent drafts and records; every signature is a human's.

## Rollback

Delete `docs/warrants/{{alias}}/`; nothing else depends on it yet.
