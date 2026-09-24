---
schema: oh.war/atom/v1
warrant_uuid: 01a0cc13-dd90-71b1-9fc5-9989932b3bae
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — currency is derived, a written currency is refused, and 0073's signature stands
- **scope:** `relations.rs`, `ownership.rs`, `warrant_overview.rs`, `docs/warrants/OW-WAR-0073/manifest.toml`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** on a scratch program, a Warrant that another authorized Warrant `supersedes` reads `relations.currency` PASS with no field in its manifest; a manifest carrying `currency = "superseded"` is refused `relations.currency-authored`; a `supersedes` cycle is refused `relations.currency-cycle`; on this corpus, `war sign --all --dry-run` reports no `authorize.no-amendment` for OW-WAR-0073 and `war authorize OW-WAR-0073` prints contract digest `691f51ce…`.

### OBL-002 — the master document is current by construction and the history holds the rest
- **scope:** `current.rs`, `history.rs`, `compile.rs`, `docs/generated/CURRENT.md`, `docs/generated/HISTORY.md`, the four pointing documents.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** on this corpus, `CURRENT.md` names OW-WAR-0073 exactly once, as `OW-WAR-0073 → OW-WAR-0112` under *Replaced*, and contains none of 0073's atom text; `HISTORY.md` contains 0073's intent verbatim; every atom of every current Warrant appears verbatim in `CURRENT.md` (a plant hashes each atom's body and finds it); a hand edit to `CURRENT.md` fails `war check --generated` with `generated.drift`; `[generated] history = false` on a scratch program produces no `HISTORY.md` and no drift error; `README.md`, `QUICKSTART.md`, `AGENTS.md` and `docs/TUI.md` each name `docs/generated/CURRENT.md` as the first thing to read.

### OBL-003 — no signing command is handed to a human unjudged
- **scope:** `next.rs`, the app's Help pane.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war next --json` carries `judged` on every action whose command begins `war sign`, valued `would_record` or `would_refuse` with the refusing rule; on a scratch program with one refusable act, that action sorts after the recordable ones and the human rendering shows the rule beside the command; `war next` writes nothing and touches no key (a `git status` diff and a transcript grep, as `98-ownership.sh` does).

### OBL-004 — presets type the authored atoms without answering for the author
- **scope:** `templates/presets/`, `new.rs`, `check.rs`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war new "x"` without `--preset` writes the profile's default preset and names `feature`, `fix`, `decision` (AM-002), and never the `TODO` skeleton; `war new "x" --preset feature` writes one atom per required role of the profile, each with its headings and questions; `war check` on that draft reports `atom.preset-unanswered` as a warning naming the heading, and as an error once `war authorize` has been run for it; a preset atom whose optional heading was deleted passes; `war check` refuses an atom whose role no projection renders (`atom.role-unprojected`).

## Gate Adequacy

Required at `basic`. The load-bearing check is OBL-002's "names 0073 exactly
once": a master document that quietly includes replaced text is the four-file
problem again with one more file.

**Adversarial question:** could `CURRENT.md` claim currency for a subject
that is not current? Only if the derivation disagrees with the relations, and
the derivation is the same function `relations.currency` checks — one
computation, read by both. A hand edit is drift.

**Second adversarial question:** could a preset make an author look finished?
Only by answering; presets ask, and an unanswered required heading is named.
