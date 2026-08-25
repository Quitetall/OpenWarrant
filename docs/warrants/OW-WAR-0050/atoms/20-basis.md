---
schema: oh.war/atom/v1
warrant_uuid: 01a0399d-05b9-7ad0-b8dc-bf1a226fa641
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

OpenWarrant's separation of authorization, execution, evidence, independent
verification, and resolution; Bonsai's local-first architecture contracts; and
the repository's existing `gate` required check.

## Assumptions and Unknowns

- The supplied Bonsai binary and pinned source revision are build inputs, not
  claims made by a Warrant.
- GitHub settings are repository-administration state. This record can require
  and test workflow behaviour, but cannot truthfully claim an administrator
  enabled every setting until an external check records it.

## Constraints and Invariants

- Scope bytes and policy bytes are digest-bound before a pass is emitted.
- Candidate `head` must be checked out, preventing a worktree report from being
  mislabeled as another commit.
- Only named scope violations and Bonsai architecture errors block this pilot.
- A missing executable or malformed machine result is `unknown`, never pass.
