---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32db-7240-9a9d-cef5e5b41a38
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS 1.1.0 §76.6 (a rendering holds no authority) and §27.2 (human-only
  acts) — delivered by OW-WAR-0071, **not yet accepted**.
- `OW-ADR-0020` (the ratatui dependency, and why a TUI is not an async
  program), amending nothing in `OW-ADR-0014`'s async refusal: ratatui and
  crossterm are synchronous.
- `docs/PROJECTION_CONTRACT.md`: the fields a view may show and what each
  means; the TUI reads the same `CORPUS_STATUS.json`, `CORPUS_TIMELINE.json`
  and `CORPUS_PENDING.json` contracts.
- `crates/openwarrant-cli/src/console.rs`, `frontier.rs`, `questions.rs`,
  `status.rs` as the behaviour to carry over.

## Assumptions carried in

- ratatui and crossterm are MIT, already inside `deny.toml`'s allowlist, so
  the dependency adds no licence family. Verified by reading `deny.toml`
  before this Warrant was drafted; re-verified by `cargo deny` on the gate.
- A TUI needs no async runtime: crossterm's event read with a poll timeout is
  synchronous, so `OW-ADR-0014`'s "no async outside `mcp/`" holds unchanged.
- Live refresh reuses `war watch`'s `notify` watcher rather than polling the
  corpus, and `notify` is already a dependency.
- The batch act (OW-WAR-0072) lands first or the TUI signs one act at a time;
  either way the TUI does not implement signing itself.
