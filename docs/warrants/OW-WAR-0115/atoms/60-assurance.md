---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd30-eabb-76a2-b970-42724167ad84
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — projects are remembered on use, never lost silently, and never recorded when opted out
- **scope:** `projects.rs`, `lib.rs`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** with `XDG_CONFIG_HOME` in a temporary directory, `war status` in two scratch repositories makes `war projects --json` list both; a deleted repository is listed as missing, not removed; `war projects --forget` removes exactly one; `OPENWARRANT_NO_PROJECTS=1 war status` leaves the list file absent; an unwritable config directory does not change any command's exit code.

### OBL-002 — the hub opens from anywhere, reads each project through the CLI's own functions, and every command it hands over names its project
- **scope:** `tui/mod.rs`, `docs/TUI.md`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war` outside any repository at a pty opens the Projects pane (the pane title in the captured screen); `war` with no terminal still exits 2 with `tui.no-tty`; a grep of `tui/` finds every `current_exe` child built with `--root`; the Projects row's pending count for a scratch repository equals `war --root <it> sign --list`'s count; the Help pane's first row names the running binary's version and path.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-002's `--root` on every
child. A hub that hands over a command without its project is the 2026-09-23
failure, rebuilt.
