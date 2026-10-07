---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd30-eabb-76a2-b970-42724167ad84
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §76.6 and OW-ADR-0019/0020: the app is a rendering. The hub reads each
  project through the same functions `war status`, `war sign --list` and
  `war questions` use, and holds no key.
- OW-WAR-0112: `war` with no arguments is the app. This Warrant extends where
  it opens, not what it is.
- OW-WAR-0114: the roadmap record the hub's progress column reads when
  present.
- OW-ADR-0021: this Warrant declares `tui/mod.rs`, which OW-WAR-0112 owns.
  On authorization, this Warrant becomes its owner.

## Assumptions

- A-001: a per-user file at `$XDG_CONFIG_HOME/openwarrant/projects.toml`
  (default `~/.config/…`) is an acceptable place for the list. It holds
  paths and last-seen times only; no record content, no authority.
  Confidence: high.
- A-002: recording a repository's path on use is not a surprising write,
  because it is outside the repository and is documented in `docs/TUI.md`.
  `OPENWARRANT_NO_PROJECTS=1` turns it off. Confidence: medium.

## Constraints

- Opening a project in the hub is `Repository::discover(Some(path))`. There
  is no second loader.
- Reading many projects must not stall the hub. Rows are computed lazily,
  with at most one project read per idle tick, and cached until that
  project's `watch` fingerprint moves.
- No key, no socket, no signing call in the hub. `97-tui.sh`'s grep covers
  the new code.

## Residual risks

- R-001: a listed path that now holds a different repository. The row shows
  the program name and namespace read from its `openwarrant.toml`, so a
  swap is visible.
