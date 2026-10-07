---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eaf-7041-b99b-eaed6ac66f86
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- The owner's question of 2026-09-22, placed at
  `roadmap://OW-PHASE-1/retention` by OW-WAR-0114's gap table.
- SAS §66.1–§66.2: the local journal is append-only draft history and
  shall not become a competing ledger.
- SAS §33.8: compaction shall not launder untrusted influence.
- SAS §88: performance is secondary to semantic correctness; no
  optimization may change semantic output without an ADR and differential
  conformance.
- SAS §106 WAR-SAS-RQ-084: historical superseded, disputed and annulled
  records remain available.
- `docs/design/openwarrant-product-spec.md`, "History and adoption":
  temporary work may be pruned under policy without removing referenced
  evidence; preservation requires recoverable bytes.
- `crates/openwarrant-cli/src/journal_cmd.rs` (`journal.rewritten`,
  `journal.malformed`) and `preservation.rs` (`war archive`).
- `conformance/plants.d/96-batch.sh`: the throwaway key and ssh-agent
  pattern the generator reuses.

## Assumptions

- A-001: a release build is several times faster than the debug build the
  2026-09-23 numbers came from. The budget is stated for a release build,
  and the measurement records which build it used. Confidence: medium.
- A-002: a synthetic corpus built through the real acts costs what a real
  one of the same shape costs. It does not have the real corpus's large
  `implementation/` logs, so the generator adds a log of the real
  corpus's median size per resolved Warrant. Confidence: medium.
- A-003: the synthetic records are fixtures in a scratch directory. The
  throwaway "verifier" in them verifies nothing real, and they are never
  committed or read as evidence about any real Warrant. Confidence: high;
  the plant checks the repository tree is unchanged.

## Unknowns

- U-001 (non-blocking): whether a scripted scratch program can reach
  `war resolve` for hundreds of Warrants in reasonable time: verifier
  distinct from performer, a declared independence, a gate run each. If
  it cannot, OBL-004 is not established, and the record says what share
  was resolved.
- U-002 (non-blocking): the budget numbers. This draft proposes, for a
  release build at 1,000 Warrants with at least 500 resolved: `check`,
  `check --generated`, `next`, `status`, `pins --resolved-only` and
  `sign --list` each at most 10 s median; `compile` at most 30 s. The
  owner accepts or amends them by authorizing.

## Observations

- O-001 (2026-09-23): `war console` answering `1`, `s` on this corpus
  (about 140 Warrants) takes 22 s on a debug build at load average 20, the
  same before and after that day's changes; `war console --json` alone takes
  5.4 s. It walks the board four times. `93-console.sh`'s hang bound was
  raised from 20 s to 60 s for it. The interactive paths belong in this
  Warrant's budget beside the read commands.

## Residual risks

- R-001: one machine's numbers. The record names the CPU, OS and load, and
  the budget is a gate a slower machine may fail. That is a true report
  about that machine.
- R-002: the budget holds today and a later change breaks it. The plant runs
  the gate at a small N only; the 1,000 run is a release-time act, and
  `docs/RETENTION.md` says so.
