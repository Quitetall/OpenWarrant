---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6056-70f1-a94b-56ec20257a39
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

M1 — the assignment record, signed with the authorization:

1. `crates/openwarrant-core/src/assignment.rs` (new).
   - `oh.war/assignment/v1`: per act kind (`verify`, `resolve`), one
     or more actors.
   - Pure validation against an `AuthorityRegister`: the actor exists and
     holds the role; an agent is refused for `resolve` by kind; the
     performer is refused as its own verifier.
   - The set digest, computed like the deliverable-set digest.
2. `crates/openwarrant-cli/src/authorize.rs`.
   - The request lists the assignment and its digest.
   - Ingest refuses a response whose echoed digest differs
     (`authorize.stale-assignment`), before any signature is verified.
   - The recorded authorization carries the set.

M2 — assignment narrows eligibility; a queue per person:

3. `crates/openwarrant-cli/src/sign.rs`.
   - `eligible` intersects with the assignment when one is recorded.
   - `choose_actor` picks the single assigned actor without `--as`.
   - `--list --as <actor>` shows only that actor's acts, assigned first.
   - Refusals: `sign.not-assigned` and `assignment.role-revoked`.
4. `crates/openwarrant-cli/src/inbox/mod.rs`: `war inbox --as <actor>`,
   through the same function.

M3 — the verifier is the assigned one:

5. `crates/openwarrant-cli/src/verify.rs`. On a Warrant with a recorded
   assignment:
   - a verdict from an unassigned verifier is refused
     (`verify.not-assigned`) and not written;
   - a human verifier without `verifier` in `roles.toml` is refused
     (`verify.role-missing`).

   Unassigned Warrants are unchanged.

M4 — documents and plants:

6. `docs/TEAMS.md` (new): assigning, the personal queue, what assignment
   does not do.
7. `docs/THREAT_MODEL.md`: a row for assignment edited after signing and
   for a verdict under another person's name. Its residual points at
   OW-WAR-0138.
8. `conformance/plants.d/57-teams.sh` (new), the refusals in the assurance
   atom.

## Frozen Surfaces

- `oh.war/report/v1`.
- The response and authorization schemas, except for the one added
  assignment field.
- `--ssh-sign` and `authority_check`.
- §27.2's refusals by kind.

## Premade Instructions

- An assignment only removes signers. If code ever adds an actor to
  `eligible`, that is a defect.
- Plant every refusal before writing its positive: the refused case must be
  seen refusing.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:

- U-001, before M1 starts;
- the schema-pack move (U-002);
- any change to `roles.toml`'s format.

## Rollback

Remove the assignment files. Every path reads "no assignment" as today's
behavior, so reverting the code leaves no record unreadable. A signed
authorization that carries an assignment set stays valid. Its set is then
unread, and it narrowed nothing that a revert widens without the record
saying so.
