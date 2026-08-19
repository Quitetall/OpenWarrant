---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f34-92db-54b2dca5446d
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `Manifest` — the §61 shape, parsed and validated.
2. `AtomRole` — the eleven roles of §16.1 with their canonical ordinals.
3. `Jurisdiction` — `authored | bound | generated` (§13).
4. `Profile` — `delivery` and `decision`, each with its required-role set (§16.3).
5. An atom-frontmatter reader for the §62 format.
6. `war new "<title>"` — allocates the next alias and scaffolds a draft.
7. Refusal paths for §91.2 tests 7, 8, 9, and 12, each with a test that plants
   the violation and asserts the refusal.

## Frozen Surfaces

- The role names and their ordinals. They are protocol vocabulary from §16.1.
- The jurisdiction tri-state. Collapsing `bound` into `authored` would erase the
  distinction that makes §13.4's jurisdiction law enforceable.

## Premade Instructions

- Allocate aliases with `O_EXCL`. Two concurrent `war new` invocations must
  produce two distinct aliases or one failure, never one file. This is not
  hypothetical: while these Warrants were being planned, ADR 0183 was committed
  in the parent project while two other ADRs sat untracked holding their
  numbers, and a different pick would have collided.
- Preserve unknown optional namespaced roles; refuse unknown required ones.
- Do not resolve `ref =` atoms over the network under this Warrant.

## Resources and Capabilities

Repository-local filesystem read and write. No network. No secrets.

## Autonomy and Escalation

Tier T2. Escalate if the frontmatter parser choice cannot be made without
adopting an unmaintained dependency — that is a security-boundary decision under
§87.2, not an implementation detail.

## Rollback

Revert the commits. The five authored Warrants are inert data and survive
untouched; nothing else consumes the parser yet.
