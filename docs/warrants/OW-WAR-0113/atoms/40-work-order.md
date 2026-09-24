---
schema: oh.war/atom/v1
warrant_uuid: 01a0cc13-dd90-71b1-9fc5-9989932b3bae
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Scope

The compiler's projection layer, the relations check, `war new`, `war
next`, and the four documents that say where to read first. Nothing that
signs, records or verifies changes.

## Deliverables

M1 — currency by relation:

1. `relations.rs`: `currency(S)` derived — *superseded* iff an authorized
   subject declares `supersedes → S`; *annulled* from the resolution
   standing; *deprecated* through a successor's `deprecates` relation, or a
   legacy bare field (reported, tolerated); else *current*. A manifest
   writing `currency = "superseded"` or `"annulled"` is refused
   `relations.currency-authored`; a `supersedes` cycle is refused
   `relations.currency-cycle`. `relations.currency` passes on the derivation.
2. `ownership.rs`, `warrant_overview.rs`, `compile.rs`, `lower.rs`'s
   consumers: read the derived currency, never the raw field.
3. `docs/warrants/OW-WAR-0073/manifest.toml`: the mark reverted to the
   signed bytes (`git show 443af497^:…`); `war sign --all --dry-run` no
   longer reports `authorize.no-amendment` for it.

M2 — the two projections:

4. `crates/openwarrant-compiler/src/current.rs`: `docs/generated/CURRENT.md`.
   Sections, in order: *Read this first* (what this document is, how it is
   made, the date of the tree); *In force* — SAS revision, digest, the ADR it
   carried, and its normative statements (the content `NORMATIVE.md`
   renders); *Decisions* — every accepted ADR, expanded; proposed ADRs one
   line each; *Warrants* — every current subject grouped by roadmap phase,
   each fully expanded atom by atom in role order, verbatim, with rung,
   Basis revision, deliverables and their digest state; *Replaced* — one line
   of lineage per superseded or annulled subject (`A → B`), nothing else;
   *Who governs what* — the ownership index by path; *Who may sign* — the
   register; *Awaiting a human* — the queue, each command annotated by the
   dry run (`would record` / `would refuse: <rule>`). Links resolve to
   atoms and records, never to other projections.
5. `crates/openwarrant-compiler/src/history.rs`: `docs/generated/HISTORY.md`,
   written when `[generated] history = true` (default true here): every
   subject ever, lineage chains, superseded atoms expanded, the state
   timeline (`CORPUS_TIMELINE.json`'s content), every ADR with status.
6. `compile.rs`: writes both; `check --generated` diffs both; a plant edits
   `CURRENT.md` by hand and `check --generated` refuses.
7. The profile's role→section map lives in `current.rs` as one table and is
   the only place a role is tied to a rendering; a namespaced optional
   extension role (SAS §16.4) renders verbatim in an *Extensions* section
   under its role name (AM-002), and `check` refuses an atom whose role is
   neither in a row nor such an extension (`atom.role-unprojected`).

M3 — the dry run in front of every handed-over command:

8. `next.rs`: for each human action whose command begins `war sign`, run
   the act's dry run (`sign::dry_run` on that target) and carry its verdict
   in `Action` (`judged: would_record | would_refuse(rule)`); `war next`
   renders it beside the command, `--json` carries it, the app's Help pane
   shows it. An action the dry run would refuse is listed after the ones it
   would record.

M4 — presets:

9. `crates/openwarrant-cli/templates/presets/{feature,fix,decision}/` — one
   file per role the profile requires, each heading followed by the
   question it answers as an HTML comment, headings marked
   `<!-- required -->` or `<!-- optional: delete if not applicable -->`.
   `feature` and `fix` are `delivery`; `decision` is the ADR profile.
10. `new.rs`: `war new <title> --preset <name>`; without `--preset`, `war
    new` writes the profile's default preset and names the others — the
    `TODO` skeleton is retired (AM-002: refusing a bare `war new` breaks
    every caller that scripts it and adds a step for no safety).
    `war init --program` and the guided init keep the adopt templates for
    the first Warrant.
11. `check.rs`: `atom.preset-unanswered` — a required heading whose body is
    empty or is only the preset's comment: a warning on an unsigned draft
    (a draft is allowed to be unfinished), an error once `war authorize`
    has been run for it.

M5 — where to read:

12. `CONTEXT.md`: the terms *master document*, *history*, *currency (by
    relation)*, *preset*, *atom* as OW-ADR-0022 defines them.
13. `README.md`, `QUICKSTART.md`, `AGENTS.md`, `docs/TUI.md`: "read
    `docs/generated/CURRENT.md` first"; the app's Help documents list it
    first among the shipped ones.
14. `conformance/plants.d/69-current.sh`: a superseded Warrant appears in
    `CURRENT.md` only as a lineage line and appears expanded in
    `HISTORY.md`; a manifest with `currency = "superseded"` is refused; a
    `supersedes` cycle is refused; a hand edit to `CURRENT.md` fails
    `check --generated`; `war new` without a preset writes the default
    and names the others; a preset atom with a required heading left as its comment is
    `atom.preset-unanswered`; `war next --json` carries `judged` on every
    signing action and the refused ones sort last.

## Frozen Surfaces

Every record schema, the schema pack version, the signing seam,
`oh.war/report/v1`, `oh.war/status/v1`. The two projections are Markdown;
they add no contract.

## Premade Instructions

- Currency is computed where relations are checked (`relations.rs`) and
  read everywhere else. A second computation is a second answer.
- The master document contains atoms verbatim. It does not paraphrase, and
  it does not include a superseded subject's text because a current atom
  mentions it — a mention is a link.
- The dry run is the test of every command handed to a human. A command
  `war next` prints is one the ingest has already judged.
- Presets ask; they do not answer. A preset's comment is never left in a
  signed atom.

## Autonomy and Escalation

Tier T2. Escalate rather than decide: whether `HISTORY.md` defaults on for
adopters (draft says yes here, no for `war init --program`); whether the
master document folds atoms under their headings or prints them flat (draft
says flat: "fully expanded" was the owner's word).
