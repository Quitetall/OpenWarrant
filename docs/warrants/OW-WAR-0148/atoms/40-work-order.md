---
schema: oh.war/atom/v1
warrant_uuid: 01a0feb6-ab80-73a5-abcb-7ade9cfaaeaa
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Scope

**Touched:**
- the CLI's corpus loading and every corpus-level builder and client;
- the profile registry and every per-kind behaviour;
- a new record, relation and state layer in core;
- the ticket store;
- the intake module;
- the plants and docs for each of these.

**Left alone:**
- `WarIr` and the contract digest's inputs;
- the schema pack version;
- signed records;
- SAS text (Q-001 decides whether a revision is proposed; this Warrant writes
  none);
- `codex/reviewed-subject-binding`.

## Deliverables

M1, one compiled corpus model:
1. `crates/openwarrant-cli/src/corpus.rs` (new): `Corpus`, built once per
   process. It holds every `Loaded`, each Warrant's `lower` result and contract
   digest, `relations::currencies`, one `resolve::assess` per Warrant, and the
   roadmap, tickets, ADRs and SAS revisions.
2. `_with(&Corpus)` variants, with the plain entry points delegating to them:
   - `status.rs` (`build`, `corpus_status_*`), `frontier.rs`, `sign.rs`
     (`pending`), `console.rs`, `timeline.rs`, `next.rs`;
   - `compile.rs` and `check.rs`, which build the corpus once;
   - one state derivation shared by status and the warrant overview, which
     fixes the disagreement.
3. Clients read the corpus: `webui/mod.rs`, `tui/mod.rs`, `mcp/tools.rs`,
   `mcp/resources.rs`. Each holds one corpus per watch fingerprint.
4. `crates/openwarrant-cli/src/model.rs` (new) and `war model --json` in
   `lib.rs`: `oh.war/model/v1` = `{basis_digest, records[], relations[],
   states[], diagnostics[]}`.
   - The schema is added to `schemas.rs`, `schemas/oh.war/model/v1.json`, the
     TypeScript pack and `pack.json`. Additive; the pack version stays 0.2.0.
5. `crates/openwarrant-cli/src/prepare.rs`: a gate key run once per commit,
   its receipt recorded for each citing Warrant through `war evidence
   record`'s path.

M2, capabilities chosen by type:

6. `crates/openwarrant-core/src/role.rs`:
   - `capabilities` in `oh.war/profile/v1`, from the closed set `structure,
     links, claims, acceptance, evidence, verification, authorization,
     resolution, stages`;
   - prerequisite checks with a `ProfileError` for each refusal;
   - core defaults (delivery: all; decision: all but `stages`; working form:
     `structure, links, claims`);
   - outcome word, falsifiability, standing coverage and promotion target as
     profile data.
7. `profiles/delivery.toml`, `decision.toml`, `contractor.toml`,
   `ticket.toml`: capabilities stated.
8. Every per-kind behaviour reads capabilities:
   - `check.rs` (`check_one` rule families) and `resolve.rs` (§56.1 checks).
     A check whose capability is absent reads "not applicable", named.
   - `next.rs`, `crates/openwarrant-core/src/status.rs` (`WarrantRung`).
   - `authorize.rs` and `verify.rs`, which refuse a kind lacking the
     capability.
   - `roadmap_cmd.rs`, `sign.rs`, `resolution_cmd.rs`,
     `crates/openwarrant-core/src/resolution.rs`,
     `crates/openwarrant-core/src/standing.rs`, `ticket/mod.rs`.
9. The profile pin:
   - `crates/openwarrant-cli/src/new.rs` writes `profile_digest` into a new
     manifest;
   - `crates/openwarrant-core/src/manifest.rs` reads it;
   - `repo.rs` `profile_checks` reports `profile.pin-drift`: a warning while
     unsigned, an error once authorized.
   - A manifest without the field keeps today's behaviour and today's
     digest.

M3, records and typed relations:

10. `crates/openwarrant-core/src/record.rs` (new): `oh.war/records/v1`
    record atoms with `## <ID> · <type>` headings and relation lines, and
    per-record revisions over the record's byte span.
11. `crates/openwarrant-core/src/relation.rs` (new): the core kinds
    (`part_of, depends_on, implements, constrains, evaluates, supersedes`,
    plus the rationale edges) and inert namespaced kinds. Profiles declare
    the kinds they allow and require.
12. `crates/openwarrant-cli/src/impact.rs` (new) and `war impact <record>`.

M4, states:

13. `crates/openwarrant-core/src/lifecycle.rs` (new):
    - the fixed states, each `computed` or `authenticated`;
    - `[[states]] name/refines` in profiles;
    - `war state <id> <name>` (`crates/openwarrant-cli/src/state_cmd.rs`,
      new), journaled.
    - A declared state holds only while its parent holds.

M5, tickets on the kernel and ticket features:

14. `crates/openwarrant-core/src/ticket.rs`, `ticket/mod.rs`,
    `ticket/render.rs`:
    - items are records, blockers are `depends_on`, `promoted_to` is a
      relation;
    - type, labels and epics;
    - `war tickets --type/--label/--state/--text`, `--search`;
    - files stay byte-compatible.
15. `crates/openwarrant-cli/src/intake.rs`:
    - `war create --issue <n>` through `[intake] fetch_argv`;
    - `[intake] writeback` (comment and close on done), off unless set, with
      its own argv. The fetch stays a read. A failed write leaves the ticket
      done and reports the issue UNKNOWN.

Across all milestones:

16. Plants: `conformance/plants.d/72-model.sh`, `73-capabilities.sh`,
    `74-records.sh`, `75-states.sh`, `76-ticket-features.sh`,
    `77-typed-demo.sh` (the password-reset demonstration); `45-tickets.sh`
    extended. Tests: `crates/openwarrant-cli/tests/model_cli.rs`.
17. `docs/adr/atoms/OW-ADR-0031-typed-records.md` (proposed),
    `docs/TYPES.md` (new), `docs/PROFILES.md`, `docs/TICKETS.md`,
    `CONTEXT.md`.

## Frozen Surfaces

- `crates/openwarrant-compiler/src/ir.rs` and `lower.rs`: no field and no
  digest input changes.
- `SCHEMA_PACK_VERSION` (0.2.0) and `DigestDomain`.
- `oh.war/report/v1`.
- Every signed record: authorizations, resolutions, responses, batches,
  attestations.
- Ticket files on disk, and `oh.war/profile/v1` files: both keep parsing,
  byte for byte.
- `codex/reviewed-subject-binding` and every file only it changes.

## Premade Instructions

- **The M1 differential.** Before and after M1 and M2, the old and new
  binaries' output of `check`, `check --json`, `check --generated`,
  `status --json`, `next`, `console --json`, `sign --list` and
  `pins --resolved-only` is byte-identical on this corpus and on
  `tools/scale/synth-corpus.sh --n 1000 --resolved 500`.
  - The only difference allowed is the state-disagreement fix, named in the
    commit.
  - `tools/scale/budget.sh` still holds.
- **Batteries.** Every battery runs in a throwaway clone, with
  `CARGO_TARGET_DIR` and `RUSTC_WRAPPER=` set. Never run it where the owner
  signs.
- **Plant rules.** No fixed /tmp paths, and no pipe into a quiet grep. Every
  claim is paired with an observed refusal.
- **A type requires, never supplies.** No profile field may create an act
  kind, loosen independence, or substitute for a signature, an observed run
  or an independent verdict.

## Autonomy and Escalation

T2. The performer builds M1 through M5 unattended after authorization.

Comes back to the owner:
- Q-001;
- any per-kind behaviour that does not fit the closed capability set (A-001);
- any change that would move an existing contract digest;
- anything that writes to GitHub beyond the approved comment and close on
  done.

Verification is opt-in and runs once at the end.

## Rollback

Each milestone is its own commit range.
- Reverting M5 to M3 removes new modules and leaves ticket files readable.
- Reverting M2 restores name-based behaviour. Profiles with `capabilities`
  then fail `deny_unknown_fields`, so revert the profile files with it.
- Reverting M1 restores per-builder loading.

No signed record changes, so a revert needs no correction act.
