---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32db-7240-9a9d-cef5e5b41a38
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Workspace dependencies `ratatui` and `crossterm` (MIT, already allowed by `deny.toml`), plus `OW-ADR-0020` recording the choice and stating that neither pulls an async runtime, so `OW-ADR-0014`'s refusal is untouched. A unit test asserts only `tui/` may `use ratatui|crossterm`.
2. `crates/openwarrant-cli/src/tui/mod.rs` — `war tui`: terminal setup and teardown that restores the terminal on panic (a hook), a synchronous event loop (crossterm poll, no async), and a `Model` built from the same functions the CLI uses: `sign::pending`, `questions::list`, `frontier::run`, `status::build`, `resolve::assess`, `commit::message`.
3. Panes, each its own module under `tui/`: **queue** (the numbered checklist, presets, optional description, batch or per-act signing), **questions** (blocking first, Enter takes the recommendation), **frontier** (state per stage, blocked edges named, start with `war perform`/`war run`), **corpus** (rungs, Objectives, the thirteen per Warrant), **obligations** (per obligation: disposition, verifier, admissibility, and the refusal's words when inadmissible), **evidence** (gate runs by class, receipts), **journal** (a timeline, filterable by day and event kind).
4. Navigation: pane switching, a filter line, `?` for keys, and a status bar naming the repository, the SAS revision in force, and the count of acts awaiting a signature. Every pane shows the exact command behind the row it highlights.
5. Live refresh through `war watch`'s `notify` watcher, debounced, so an act signed in another terminal updates the queue without a keypress.
6. `--json` on `war tui` refuses with a message pointing at `war console --json` and `war status --json`: a terminal application has no envelope, and pretending otherwise would be a second projection contract.
7. `conformance/plants.d/97-tui.sh`: the binary holds no key (a source grep: no `ssh-keygen`, no `SSH_AUTH_SOCK`, no signing call outside the existing seams); `war tui` in a non-TTY exits 2 by name rather than scribbling escape codes; the model's queue equals `war sign --list`; a panic restores the terminal (a fixture that panics on purpose).
8. `docs/TUI.md` and a line in `README.md`: what each pane shows, every key, and the §76.6 rule that this is a rendering.

## Frozen Surfaces

Every projection contract, the signing seam, `oh.war/report/v1`. The TUI adds no record type and no schema.

## Premade Instructions

- Read through the same functions the CLI uses. A pane that computes a fact
  itself is a second answer to a question already answered.
- No key, no socket, no signing inside the TUI: shell out to the same
  `war sign --ssh-sign` a hand would run, so the confirm dialog is the act.
- Restore the terminal on every exit path, panic included.
- A pane that cannot answer says so: "not established" is a state to render,
  not a blank.

## Autonomy and Escalation

Tier T2. Escalate rather than decide: whether the TUI becomes the default of
`war` with no subcommand (draft says no — a CLI that opens a full-screen app
when given no arguments surprises scripts); whether `war console` is retired
once the TUI lands (draft says it stays, as the line-based fallback over ssh
and inside the battery).
