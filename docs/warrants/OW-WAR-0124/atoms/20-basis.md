---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f0f-7e31-bed2-efca03dd781e
role: basis
jurisdiction: authored
order: 20
classification: internal
---


# Basis

## Governing sources

- SAS §71.1 (`war init`) and §60 (repository configuration).
- SAS §95: changes with no WAR relation become untracked-work candidates;
  the signal "SHALL not fabricate a relationship after the fact without
  review".
- SAS §96 (existing ADR migration; §96.3, no fabricated proof) and §97.1
  ("adopt, do not replace").
- SAS §37.5 and RQ-036/RQ-037: ownership of a delivered path is recorded
  at authorization. A path no authorization declared is owned by no
  Warrant.
- `docs/design/openwarrant-product-spec.md`, "History and adoption":
  import legacy work in its actual state; unrelated closure debt does not
  block new work.
- OW-WAR-0112: `war init` as a conversation (`init/guided.rs`); its
  authority policy is frozen here.
- OW-WAR-0114's gap table: "Brownfield adoption: a guided path for a
  repository with no Warrants", placed at `roadmap://OW-PHASE-3/adoption`.
- OW-ADR-0021: this Warrant declares `init/mod.rs` and `init/guided.rs`
  (OW-WAR-0112), `telemetry.rs` (OW-WAR-0039 and 0041), `lib.rs`
  (OW-WAR-0116) and `config.rs`. On authorization it becomes their owner.

## What the code does today (read 2026-09-23)

- `crates/openwarrant-cli/src/init/mod.rs`: `run` writes
  `openwarrant.toml` and refuses an existing one; `run_program` scaffolds
  the SAS template, authority examples, the `war check` gate and the
  "Adopt OpenWarrant" Warrant from `templates/adopt/`. Neither reads Git.
- `crates/openwarrant-cli/src/init/guided.rs`: `Facts::read` looks at
  `openwarrant.toml`, the authority files, SAS revisions and the first
  Warrant. It does not look at history.
- `crates/openwarrant-cli/src/telemetry.rs`: `untracked_candidates` runs
  `git log --format=%h %s` over all history, unbounded on purpose, and
  matches `OW-WAR-` or `war://` in the subject.
- `crates/openwarrant-core/src/config.rs`: `RepositoryConfig` has no
  adoption field. Its optional tables use `#[serde(default)]`.
- `conformance/plants.d/99-init.sh` and `75-init-program.sh` hold the
  scripted scaffold's output on a repository with no commits.

## Assumptions

- A-001: `openwarrant.toml` is the right home for the baseline. It is
  repository configuration, not an authority record, and it is read by
  every command already. Confidence: medium; the alternative, a record
  under `docs/`, adds a schema for one value.
- A-002: this repository has no `[adoption]` table, so its telemetry
  baseline (`artifacts/telemetry-baseline.json`) is unchanged: it reads
  all history as today, and its namespace is `OW`. Confidence: high.
- A-003: an ADR directory is recognised by the file names `war migrate`
  already accepts (`NNNN-*.md`), in three conventional places. A corpus
  elsewhere is not found, and the documentation says how to point
  `war migrate` at it. Confidence: high.

## Unknowns

- U-001 (non-blocking): whether moving the baseline after adoption should
  be refused. This Warrant writes it once and does not add a rule; a hand
  edit moves it. See R-001.

## Residual risks

- R-001: an edited baseline hides commits from untracked-work detection.
  `war telemetry` prints the baseline it used beside the count, so a moved
  baseline is visible in the output that relies on it.
- R-002: a very large history makes the one-time commit count slow. The
  count is `git rev-list --count HEAD`, read once, at init.
