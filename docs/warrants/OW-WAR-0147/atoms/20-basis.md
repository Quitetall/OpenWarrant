---
schema: oh.war/atom/v1
warrant_uuid: 01a0da65-9e85-7531-a20e-8e695ecfa9ee
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §2.2 and §16.3: a profile is added as data without redesign; the
  core profiles are fixed. OW-WAR-0140's registry (`role.rs`,
  `profiles/*.toml`) is the seam this Warrant extends with a working form.
- §16.4 and RQ-015: an unknown or missing required role fails closed. A
  working-form record still fails closed on a missing role; a Warrant
  manifest naming a working-form profile is refused outright.
- RQ-070: the CLI works file-native and offline for drafts. Every ticket
  command reads only files under the repository root.
- RQ-074: `war check` is deterministic and agent-free; its ticket rules are
  structural.
- §66: the append-only journal. Tickets reuse `journal_cmd::record` and the
  §66.3 envelope unchanged.
- §86 (`atomic.rs`): checklist and intent writes go through `write_if` with
  the prestate read, retrying when another writer moved the file.
- CONTEXT.md: Completion ("the declared work is done; it may remain
  unverified") is what a done ticket is; Verified is not awarded here.

## Prerequisites

- OW-WAR-0140 (profile registry), authorized.
- OW-WAR-0141 (intake): `war create --draft` asks the same `[plan]
  drafter_argv` through `plan::request` and `plan::run_drafter`.

## Unknowns

- U-001 (escalated, non-blocking for the ticket path): whether a working
  form needs a SAS clause of its own. §16.3 names two profiles and their
  roles; a working form adds no core profile and no Warrant is ever
  composed against one, but the SAS does not yet say a record of fewer
  roles may exist beside the corpus. Options: (A) a §16.5 clause in the
  next SAS revision proposed by the owner; (B) keep tickets outside the
  SAS as tool behaviour. Recommendation: A.
- U-002: `profiles/ticket.toml` says `approved = false` until the owner
  signs this Warrant; nothing on the ticket path reads the flag.

## Assumptions

- A-001: hard links are available where claims live (Linux, macOS). Where
  they are not, the claim falls back to `O_EXCL` create-then-write; a
  reader in that instant sees a held lock naming nobody, never a free one.
- A-002: a claim's actor name (`--as`, `OPENWARRANT_ACTOR`, else
  `[project] performer`) is coordination, not identity; nothing in a
  ticket authorizes anything, so no self-check rests on it.

## Residual risks

- R-001: two stealers of one stale claim and a third plain claimant inside
  microseconds of each other can leave the third holding it while the first
  believes it does (documented in `claim.rs`). The journal records all three.
- R-002: claims live in one working tree's `.openwarrant/state/claims/`;
  agents in separate worktrees see each other's claims only when
  `[tickets] claims_dir` points at a shared path.
