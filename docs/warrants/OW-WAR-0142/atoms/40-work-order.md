---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e89-7990-8b2c-5a43577b5ce5
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Scope

M1 decides. Under **A**, M2–M4 build the standing authorization and M5 is
the owner's. Under **B**, M2–M4 are cancelled by amendment. The Warrant
then delivers OW-ADR-0029 with B's decision and a *Routine work* section
in `docs/SIGNING.md`, and D-003 to D-014 leave the deliverable set by that
amendment. Under **C**, M2–M4 are cancelled the same way and a new
Warrant proposes the §27.2 revision. This Warrant does not build C.

## Deliverables

M1, the decision:

1. `docs/adr/atoms/OW-ADR-0029-standing-authorization.md`:
   - options A, B and C;
   - the human cost of each per routine change;
   - the recommendation and why;
   - rewritten to the owner's answer to **Q-001** (blocking, STAGE-001).
2. Under A, the SAS 1.2.0 text:
   - §28.8 *Standing authorization*;
   - one clause in §37.5;
   - one §106 row;
   - proposed with `war sas propose 1.2.0 --adr OW-ADR-0029`.

   Accepting it is the owner's act.

M2, the class and the check (A only):

3. `crates/openwarrant-core/src/standing.rs`:
   - `StandingAuthorization`, its closed terms, and `NEVER_COVERABLE` (the
     compiled-in set in OW-ADR-0029);
   - `covers(&Contract) -> Result<(), Vec<Refusal>>`, one refusal per term
     broken;
   - a class whose globs match any never-coverable path is refused when
     parsed.
4. `crates/openwarrant-core/src/lib.rs`: the module.
5. `schemas/oh.war/standing-authorization/v1.json`, and `schemas/pack.json`
   moved once.
6. `crates/openwarrant-cli/src/standing_cmd.rs`:
   - `war standing propose <file>` (the request a human signs);
   - `war standing show [<id>]` (terms, globs, what each matches today,
     count used, expiry);
   - `war standing apply <alias>` (the coverage check; writes
     `authorization.toml` or refuses and writes nothing);
   - `--dry-run` and `--json` on each.
7. `crates/openwarrant-cli/src/lib.rs`: the command.
8. `crates/openwarrant-cli/src/check.rs`. Covered authorizations are
   re-derived, never trusted. The rules:
   - `standing.outside-class` (error);
   - `standing.never-coverable` (error);
   - `standing.unsigned` (error: the class signature does not verify);
   - `standing.expired` or `standing.exhausted` (error, for a record
     stamped after either);
   - `standing.revoked` (note: earlier records stand, per §31).

M3, the human acts and ownership (A only):

9. The signing and batch paths, `sign.rs` and `batch_cmd.rs`:
   - new acts `Pending::AcceptStanding` and `Pending::RevokeStanding`;
   - both human-only by kind, with a dry run;
   - the screen prints every glob and its current matches under "Covers:";
   - revoking batches like any other act.
10. `crates/openwarrant-cli/src/ownership.rs`. A covered Warrant may not
    declare a path whose current owner is an authorized, unresolved
    Warrant (`standing.in-flight-owner`), subject to Q-003.
11. `crates/openwarrant-cli/src/resolve.rs`. A covered Warrant is never
    resolvable by the §27.3 policy-service path
    (`resolve.standing-needs-human`).
12. `conformance/plants.d/69-standing.sh`, the plants named in OBL-002 to
    OBL-007.
13. `docs/SIGNING.md`: *Routine work under a standing authorization*.
14. `CONTEXT.md`: the term *standing authorization*.

## Frozen Surfaces

- `oh.war/authorization/v1`: a covered record uses its existing fields.
- The response schemas of every existing act.
- §27.2's list.
- `roles.toml`'s refusal by kind.
- The blind verifier's admissibility (`verify.rs`).
- Every signed Warrant's manifest.

## Premade Instructions

- The coverage check is a pure function of the compiled contract and the
  signed class. If it needs judgment, the term is wrong: block and
  propose (§30.4).
- A term the class does not state is refused, not defaulted.
- The never-coverable set is a constant in `standing.rs`, and no file
  extends or shrinks it. The plant fails if it is read from anywhere
  else.
- Nothing an agent can call writes a class, a revocation or a resolution.
  `war mcp --describe` lists `standing apply` and `standing show`, and
  refuses `standing propose --ingest`.

## Autonomy and Escalation

Tier T2. Q-001 is the owner's. Escalate rather than decide:

- any term a class needs that OW-ADR-0029 does not name;
- any path proposed for addition to, or removal from, the never-coverable
  set;
- any reading under which a covered Warrant's record would name anyone
  but the human who signed the class.

## Rollback

- Remove the `standing` command and module, and revoke every class.
- Records already written stay. They are authorizations the owner granted
  through a class and are historical under §31.
- `war check` still reads them after rollback. A covered record with no
  `standing.rs` to re-derive it reports `UNKNOWN`, never `PASS`.
