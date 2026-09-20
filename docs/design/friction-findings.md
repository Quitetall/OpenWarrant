# Architecture friction: what landed, and what a signature is holding

Status: record, 2026-09-20, branch `claude/friction`. An architecture review of
this repository produced seven deepening candidates and a list of mechanical
defects. This file records all of them, what was delivered, and — for the rest —
which pin blocks it and which act releases it. It authorizes nothing.

Measured on `claude/friction` at `2557487f`: the CLI crate was 39,635 lines with
no `lib.rs` and no `trait` in it; `Repository::discover(None)` appeared 49 times;
`Pending` was matched at 43 sites in `sign.rs` alone; 333 `Diagnostic`
constructions across 33 files carried 259 distinct free-form rule names.

## Delivered

| # | Commit | What |
|---|---|---|
| 1 | `bf526021` | The plant guard and `restore` read one `PLANT_PATHS` array. They had drifted in both directions: `schemas/` was restored but unguarded, so uncommitted work there was discarded silently; `docs/SKILLS.md` was in neither, and the pins plant's leak reached commits twice. |
| 2 | `03c1f1ce` | `openwarrant-cli` gets `lib.rs`. 61 `pub mod`, `main.rs` becomes the argv adapter. No module file edited, no pinned file touched. |
| 3 | `521de80c` | `--root`, global and lazy. 46 `discover(None)` become `open_repo()`; `doctor`, `dispatch_bundle_cmd` and `preservation` take the root; the SDK argv heuristic learns that `--root` takes a value. |
| 4 | `e97a91a3` | `mcp::self_exec` passes `--root` instead of relying on `current_dir` alone. |
| 5 | `7aad9ba9` | Four private `sha256_hex` copies become the compiler's public one. |
| 6 | `ebbc9792` | `accept.accept.request` becomes `sas.accept.request`. |
| 7 | `b807db90` | `war next` renders `why` instead of Debug-printing `NothingActionable`. |

## Blocked, and by what

Every row is work an agent may draft and may not deliver. A **resolved** pin moves
only through `war correct <alias> <D-id>` signed by a human (AGENTS.md rule 4,
OW-WAR-0064 / OW-ADR-0012). An **open** pin is not violated by an edit, but
refreshing it rewrites a record this work does not own.

### Behind a resolved pin — needs a signature

| Finding | File | Pin |
|---|---|---|
| A `Rule` type replacing the free-form rule string | `diagnostic.rs` | OW-WAR-0005 D-002 |
| …and its 86 call sites | `check.rs` | OW-WAR-0005 D-001 |
| A `Check` trait; `check_one` is 592 lines | `check.rs` | OW-WAR-0005 D-001 |
| `resolution.requirement-unmet` vs `requirements-unmet` — one concept, two spellings one character apart | `resolve.rs` (`:852`), `resolution_cmd.rs` (`:398`) | OW-WAR-0046 D-001 |
| An `Act` trait replacing `Pending`'s four variants | `verify.rs`, `sas.rs`, `journal_cmd.rs` | OW-WAR-0046 D-002, OW-WAR-0058 D-002, OW-WAR-0031 D-002 |
| A `Corpus` aggregate; "is this Warrant resolved?" is re-derived at 12 sites in 3 idioms | `status.rs`, `resolve.rs` | OW-WAR-0055 D-003, OW-WAR-0057 D-002, OW-WAR-0046 D-001 |
| The fifth `sha256_hex` caller | `status.rs` | OW-WAR-0055 D-003 |
| Core vocabulary collisions: two `Admissibility`, two `ActorKind`, two `Independence` | `epistemic.rs`, `execution.rs`, `contract.rs`, `independence.rs` | OW-WAR-0017 D-001, OW-WAR-0024 D-001, OW-WAR-0009 D-001, OW-WAR-0021 D-001 |
| **SPDX headers still `AGPL-3.0-or-later` under an Apache-2.0 workspace** | 17 files | 17 of 17 pinned — see below |

The licence headers are the one row here that is not a refactor.
`CONTRIBUTING.md` treats the relicense as load-bearing, and every one of the 17
files carrying the old identifier is pinned: `diagnostic.rs`, `gate_cmd.rs`,
`migrate.rs`, `new.rs`, `resolve.rs`, `verify.rs`, `warrant_overview.rs`,
`adequacy.rs`, `autonomy.rs`, `context.rs`, `drafting.rs`, `gate.rs`,
`gate_run.rs`, `migration.rs`, `rationale.rs`, `role.rs`, `sas.rs`.

A correction request cannot be emitted ahead of the edit —
`war correct OW-WAR-0046 D-001` today answers `drift = false`, "nothing to
correct", which is correct: a correction attests to a change that has happened.
The sequence for each row is therefore: authorize a Warrant for the change, make
it, then `war correct` and sign.

### Behind an open pin — needs the owning Warrant to refresh

Editing these is permitted and raises `deliverable.pin-stale`, which moves
`CORPUS_PENDING.json` and turns `corpus-pending.drift` from PASS to ERROR.
`war pins --refresh` clears it and rewrites a record belonging to another
Warrant, so these wait for their owners.

| Finding | File | Open pin |
|---|---|---|
| The fifth and sixth `sha256_hex` callers | `compile.rs`, `context_select.rs` | OW-WAR-0004 D-002, OW-WAR-0068 D-003 |
| `kf::ActionEnvelope` renamed so one name stops meaning two things | `kf.rs` | OW-WAR-0028 D-001, OW-WAR-0044 D-001 |

### Not a defect after all

The review called `cli::kf::ActionEnvelope` and `core::seam::ActionEnvelope` a
duplication. They are not. `kf.rs:117` sends the actor, acting role and
organization as `x-kf-*` **headers**, and says so at `kf.rs:45`: attribution
must not be editable in a payload. The core type carries those as body fields
because §67.1 models the envelope, not KF's route. Two shapes on purpose; the
only defect is that they share a name.

§67.3 `check_version` and §67.4 `check_idempotency` having no caller is likewise
not a miswiring. `check_version` needs the current version read back from KF and
`check_idempotency` needs a key-to-payload store; neither exists. That is an
unbuilt capability and wants its own Warrant, not a call.

## Standing hazard, unfixed

`conformance/lib.sh` hardcodes `WAR="./target/debug/war"`. Where
`~/.cargo/config.toml` sets `target-dir`, cargo builds elsewhere and a stale
`./target/debug/war` from an earlier layout is left in place — so `cargo xtask
gate` builds one binary and the battery tests another, silently. It happened
during this work: several battery runs and `war check --generated` invocations
executed a binary predating every commit, and only a behaviour change that
failed to appear revealed it. The battery has no way to notice, because a stale
binary refuses the plants just as well as a current one.

A fix belongs in `lib.sh`: resolve the binary through cargo rather than assuming
the path, or refuse when it is older than the newest source file.
