---
schema: oh.war/atom/v1
warrant_uuid: 01a0ca4a-0c02-7cd3-b49d-786377a1aa06
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

M1 — pins mean ownership (OW-ADR-0021, SAS 1.1.0):

1. `docs/adr/atoms/OW-ADR-0021-pin-ownership.md` and the SAS text: RQ-036 retitled, RQ-037 added, §37.5, one line in §28.4, an optional `locator` in §56.2. `war sas propose 1.1.0`; the owner accepts with `--adr OW-ADR-0021`. `docs/warrants/OW-WAR-0071/amendments/AM-001.yaml` re-targets 0071 to 1.2.0.
2. `crates/openwarrant-cli/src/ownership.rs`: `Ownership::index(repo)` over every authorized Warrant with a recorded, attested set; `current(path)` by `(authorized_at, alias)`; `newer_than(path, alias)`. Legacy authorizations never own.
3. `crates/openwarrant-cli/src/authorize.rs`: the request lists `deliverables[{id, target_ref}]` and `deliverable_set_digest`; the response echoes it; the record stores `owned[]` and the digest top-level; `authorize.stale-deliverables` refuses a moved set; a same-contract re-sign with a moved set routes through the amendment branch; `authorize.time-before-owner` refuses an out-of-order effective time.
4. `crates/openwarrant-cli/src/sign.rs`: "Grants ownership of:" on the signing screen; the set digest as a DSSE subject beside `contract:<alias>`.
5. `crates/openwarrant-cli/src/pins.rs`: `historical` and `governed_by` on every pin; `war pins --history <path>` renders the lineage with "verifies at <commit>" or "UNKNOWN (resolved before OW-ADR-0021)".
6. `crates/openwarrant-cli/src/correct.rs`: `correction.historical` refuses a correction against a pin a later owner governs.
7. `crates/openwarrant-cli/src/resolution_cmd.rs`: a top-level `[locator]` (`commit_sha`, `worktree_clean`, `paths_dirty`) written at ingest.
8. `.claude/hooks/guard-pins.sh`: prefers `./target/debug/war`; denies only pins whose `historical` is false, absent read as false.
9. `crates/openwarrant-cli/src/check.rs`: the drift decision gains `deliverable.superseded-by` (pass, checked before the correction-mismatch branch); `deliverable.digest-drift` names both remedies; `deliverable.undeclared-at-authorization` and `deliverable.declared-then-removed`; `sas.pin-superseded` narrowed to unresolved Warrants. `crates/openwarrant-cli/src/resolve.rs`: requirement 3 unchanged, its reason names a newer owner when one moved the file. Both edited under this Warrant's ownership, after its authorization.
10. `conformance/plants.d/98-ownership.sh` on a scratch program; `00-corpus.sh:737` retargeted to a resolved Warrant with no newer owner; `73-plugin.sh`'s fixture retargeted off `check.rs`. `schemas/oh.war/authorization/v1.json` regenerated; `docs/THREAT_MODEL.md` gains the row "ownership claimed after signing".

M3 — every diagnostic carries its remedy:

11. `crates/openwarrant-cli/src/remedy.rs`: `remedy_for(rule, message, file) -> Option<Remedy{argv, purpose, kind}>`, a rule table first and the backticked `war …` already in a message second; `kind` is `auto`, `human` or `informational`, and `auto` never contains `sign`, `authorize`, `resolve`, `correct`, `accept` or `answer`.
12. `crates/openwarrant-cli/src/diagnostic.rs` gains `remedy: Option<Remedy>` and renders it as a third line; `crates/openwarrant-cli/src/output.rs` carries it in `WireDiagnostic` (additive) and `check::print` ends with a `REMEDIES:` block by `(rule, kind)`; `schemas/oh.war/report/v1.json` and `schemas/pack.json` regenerated with `SCHEMA_PACK_VERSION` unchanged.

M4 — `war init` is a conversation:

13. `crates/openwarrant-cli/src/init/mod.rs` (today's `init.rs`, unchanged in behaviour) and `crates/openwarrant-cli/src/init/guided.rs`: a state machine — Program, Signer, Confirm, KeyLoaded, Sas, SignSas, Authorize, SignAuthorize, Done — with `Machine::from_tree(root)` reading its position from the tree, a line front end, `git config user.name` and `ssh-add -L` as defaults, and the authority files written once, only behind `sign::at_a_terminal()`, only from what the human typed or chose, with a header that records the date, the name, and the human's unverified answer to the `-c` question. `--namespace` optional at a terminal; `--non-interactive`, `--json` and a non-TTY keep today's behaviour byte for byte.
14. `docs/authority/roles.toml.example`, `docs/authority/allowed_signers.example`, `docs/RESOLVING.md`, `QUICKSTART.md`: the rule restated — a tool writes these files only from a human's answers at a terminal, once; no command edits them afterwards. `conformance/plants.d/99-init.sh`.

M2 — `war` is the app (OW-WAR-0073's eight deliverables, adopted verbatim, then extended):

15. Workspace dependencies `ratatui` and `crossterm` (exact versions, no `event-stream`), OW-ADR-0020 marked accepted with this Warrant added to `governs`, `cargo deny check licenses` and a `cargo tree` diff as evidence, and a unit test beside the async one asserting only `tui/` names either crate.
16. `crates/openwarrant-cli/src/tui/`: terminal guard with teardown on every exit path and a panic hook; a synchronous crossterm loop; a `Model` built only from `console::board`, `next::run`, `status::build`, `frontier::run`, `resolve::assess`, `evidence`, `journal_cmd::load`, `check::run`, `doctor::run`; `shell.rs` that suspends the screen and runs `current_exe --root <root> --json <args>` for every act, signing included; live refresh through `watch.rs`'s fingerprint poller (`watched_dirs` and `fingerprint` become `pub(crate)`).
17. Panes: queue, questions, frontier, corpus, obligations, evidence, journal — as 0073 specified them — plus Help and Setup. Navigation, a filter line, `?`, and a status bar naming the repository, the SAS revision in force and the acts awaiting a signature; every pane's bottom line shows the exact command behind the highlighted row.
18. `crates/openwarrant-cli/src/lib.rs`: `command` becomes optional; no subcommand at a terminal opens the app; no subcommand elsewhere prints usage and exits 2; `war --json` with no subcommand and `war tui --json` refuse by name (`tui.json`) pointing at `war console --json` and `war status --json`; `war tui` in a non-TTY exits 2 as `tui.no-tty`. `main.rs`'s `NOT_YET` gains `tui`.
19. Help: a "what next" list — the Setup step if incomplete, then `next::run`'s actions human first, then doctor and check findings deduplicated by rule with counts and their remedies, `x` running an `auto` remedy and never a `human` one — and the documents: `docs/TUI.md`, `QUICKSTART.md`, `README.md`, `docs/DEFINITIONS.md`, `docs/SKILLS.md` and the `AGENTS.md` template embedded at build, with the repository's own `AGENTS.md`, `CONTEXT.md` and `CONTRIBUTING.md` shown first when present, through `tui/md.rs`, an in-house converter with no dependency.
20. Setup: the M4 machine as a pane, shown first whenever it is not `Done`, including when no `openwarrant.toml` exists.
21. `conformance/plants.d/97-tui.sh`: no `ssh-keygen`, no `SSH_AUTH_SOCK`, no signing call under `tui/`; non-TTY exits 2 by name; the queue equals `war sign --list`; a hidden `--panic-after-setup` restores the terminal; every embedded document renders. `docs/TUI.md` and a line in `README.md`; `AGENTS.md` tells agents to call `war status` and `war next`, never bare `war`.

M5 — the 65 warnings become one act:

22. `crates/openwarrant-cli/src/sas_repin.rs` and `SasCommand::Repin { alias | --all, --reason, --dry-run }`: for every authorized, unresolved Warrant whose pinned revision is not the latest, an `amendments/AM-<next>.yaml` in the shape `88-sas-repin.sh` already drives, all or nothing, refusing by name a manifest that implements a row the latest revision lacks; resolved Warrants skipped and counted. `conformance/plants.d/88-sas-repin.sh` extended.

## Frozen Surfaces

`oh.war/report/v1` (additive field only; keys frozen at 1.0), the signing seam and its namespaces, `SCHEMA_PACK_VERSION`, every contract digest recorded before this Warrant, `war console` as the line-based fallback, `core/resolution.rs`, `core/autonomy.rs`, `core/contract.rs`, `core/sas.rs` (none declared, none touched), the seventeen relicense files.

## Premade Instructions

- Nothing under `tui/` computes a fact. A pane that answers a question itself is a second answer to a question the CLI already answers.
- No key, no socket, no signing inside the app: every act shells out to the same `war sign --ssh-sign` a hand would run, so the confirm dialog is the act. `console.rs`'s in-process call is the fallback, not the pattern.
- A pane that cannot answer says so: "not established" is a state to render, not a blank.
- Restore the terminal on every exit path, panic included.
- The first edit to `check.rs`, `diagnostic.rs` or `resolve.rs` happens after this Warrant is authorized and the hook reads the new pin list. Under the old binary that edit reads as drift on an already-red corpus; no gate consults it; the rebuilt binary reads it as `superseded-by`. Do not correct it, do not refresh it, do not wait.
- Every new plant runs on `scratch_corpus`. Battery counts are recorded in each commit message and never decrease.
- `auto` remedies are the closed list in deliverable 11. A remedy that signs is `human`, and the app shows it and stops.

## Autonomy and Escalation

Tier T2. Settled by the owner on 2026-09-22: ownership on authorization; ratatui; Help as both a contextual guide and the documents; SAS 1.1.0 for ownership with 0071 re-targeted to 1.2.0; the authority files written at a terminal. Escalate rather than decide: whether `sas.pin-superseded` should be a note rather than a warning once M5 exists (draft: it stays a warning, now with a remedy); the exact key bindings beyond those named in `docs/TUI.md` (draft: as written there).

## Rollback

M1: `war sas propose` a revision that restores RQ-036's title (the row cannot be removed); the ownership fields are optional and a record without them owns nothing, so a binary built before M1 reads every record as before. M2–M5: delete the modules and the clap variants; the records they wrote stay as history.
