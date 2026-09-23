---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6068-7151-8462-b9ce53fa3227
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

M1 — the profile seam (U-002 option A):

1. `crates/openwarrant-core/src/role.rs`.
   - `Profile` resolves through a registry of pinned definitions.
   - `delivery` and `decision` are defined with exactly today's required
     roles.
   - An unknown profile is still `UnknownProfile`.
   - A profile may require namespaced roles.
2. `crates/openwarrant-core/src/manifest.rs`: a required namespaced role
   is satisfied when the profile requires it. Everywhere else a required
   unknown role still fails closed (§16.4).
3. `profiles/delivery.toml`, `profiles/decision.toml` (new): the two
   existing profiles as data.

M2 — the contractor profile:

4. `profiles/contractor.toml` (new): extends `delivery` and requires
   `contractor.parties`, `contractor.terms`, `contractor.acceptance`
   and `contractor.commercial`.
5. `crates/openwarrant-cli/src/new.rs`: `war new --profile contractor`
   writes the delivery atoms plus the four contractor stubs.
6. `docs/EXAMPLES/04-contractor.md` (new) and its fixture under
   `conformance/fixtures/contractor/` (new). A contractor Work Order
   whose terms are references and are marked "not a legal instrument".

M3 — acceptance and references:

7. `contractor.acceptance` names acceptance authority by actor. A check
   verifies the actor holds `resolver` and is human-kind, using the
   existing register and adding no role. Acceptance is the existing
   resolution act.
8. References in `contractor.commercial` and `contractor.terms` resolve
   or report UNKNOWN (U-004).

M4 — the Exit, shown, and documents:

9. `conformance/plants.d/56-contractor.sh` (new):
   - the IR comparison against `delivery`;
   - the frozen-module diff;
   - the refusals in the assurance atom.
10. `docs/PROFILES.md` (new): what a profile is, how one is added, and
    what a profile may not change.

## Frozen Surfaces

The technical core, as files. The plant diffs them:

- `crates/openwarrant-core/src/`: `lifecycle.rs`, `state.rs`,
  `contract.rs`, `obligation.rs`, `verification.rs`,
  `independence.rs`, `resolution.rs`, `authority.rs`,
  `deliverable.rs`, `gate.rs`, `gate_run.rs`;
- `crates/openwarrant-compiler/src/`: `ir.rs`, `canonical.rs`,
  `digest.rs`.

M1 may change only `role.rs` and `manifest.rs` in the core. After M1,
nothing in the core changes.

## Premade Instructions

- Money, parties and legal text live only in `contractor.*` atoms. A core
  struct that gains a contractor field is a defect.
- References, not copies (§22.3).
- The fixture must never read as a real contract. It says "not a legal
  instrument" in every term atom.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:

- U-001 and U-002, before M1;
- the schema-pack move (U-003);
- any new actor role;
- any change to the resolution act.

## Rollback

Delete `profiles/contractor.toml` and the fixture. The registry keeps
`delivery` and `decision` with today's roles, so every existing Warrant
compiles to the same digests. If U-003 moved the pack version, rollback is a
further pack version, never a re-use of the old one.
