---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32db-7240-9a9d-cef5e5b41a38
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the dependency adds no licence family and no async runtime
- **scope:** `Cargo.toml`, `Cargo.lock`, `deny.toml`, `OW-ADR-0020`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `cargo deny check licenses` passes unchanged; a `cargo tree` diff shows no `tokio`, `async-std` or executor crate entering the graph; the unit test that only `tui/` may `use ratatui|crossterm` passes; the ADR states both facts.

### OBL-002 — the rendering holds no authority and leaves no wreckage
- **scope:** `crates/openwarrant-cli/src/tui/`, `conformance/plants.d/97-tui.sh`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a source grep finds no `ssh-keygen`, no `SSH_AUTH_SOCK` and no signature construction under `tui/`; every act shells to `war sign --ssh-sign`; a fixture that panics mid-render leaves the terminal restored (the plant asserts the alternate screen is off and raw mode cleared); `war tui` with stdout not a TTY exits 2 naming the reason.

### OBL-003 — the acting panes are the same acts, not new ones
- **scope:** the queue, questions and frontier panes.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the queue's rows equal `war sign --list` (compared in the plant); a question answered in the TUI writes the same record `war answer` writes, refusing an agent actor by the same rule; starting a stage calls `war perform`/`war run` and records the same submission.

### OBL-004 — the reading panes agree with the record, and say when they cannot
- **scope:** the corpus, obligations, evidence and journal panes.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each pane's numbers are read through `status::build` / `resolve::assess` / the receipts, never recomputed; an obligation with an inadmissible verification renders `inadmissible` with the refusal's words, matching what `war resolve --dry-run` reports; an unreadable record renders as named-unreadable rather than blank (planted by corrupting one record).

## Gate Adequacy

Required at `basic`. The load-bearing check is OBL-002: a terminal application
one keypress from a signing command is exactly where an authority-holding view
would be built by accident.

**Adversarial question:** could the TUI sign without the human? Only by holding
a key or driving the agent socket, both of which the plant greps for and
neither of which exists; it shells out, and `ssh-add -c` puts the dialog in
front of the human as before. §76.6 is the rule that makes such a change a
defect rather than a feature.

**Second adversarial question:** could a pane show a comfortable number that
the records do not support? Every pane reads the functions `war check` and
`war resolve` read, and OBL-004's plant corrupts a record to prove the pane
reports the corruption instead of drawing an empty cell.
