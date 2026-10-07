---
schema: oh.war/atom/v1
warrant_uuid: 01a0cc13-dd90-71b1-9fc5-9989932b3bae
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Requirements

- RQ-010 — authored atoms are directly editable sources: presets keep them
  editable and make them typed (§8).
- RQ-012 — generated atoms and parents are not directly editable: both
  projections are `generated`, drift-checked (§17.3).
- RQ-013 — composition is typed, ordered, deterministic: role order and the
  profile's role→section map are the composition.
- RQ-021 — the ADR Overview is generated: its accepted subset is rendered
  into the master document, its whole into the history.
- RQ-032 — state is decomposed into phase, condition, outcome, currency,
  standing: currency becomes a derived dimension.
- RQ-084 — historical, superseded, disputed and annulled records remain
  available: the history projection, and the one-line lineage in the master
  document, are how.

## Decisions relied on

- OW-ADR-0022 (this Warrant's own, ordinal 30): two projections, currency by
  relation, the atom definition, role-as-relation, presets.
- OW-ADR-0021: the ownership index the master document renders.
- OW-ADR-0019 / OW-ADR-0020: a projection is a rendering and holds no
  authority; the app's Help pane reads `CURRENT.md`.
- OW-ADR-0016: a SAS re-pin is an amendment; `CURRENT.md` shows a Warrant's
  Basis as the revision its authorization (or latest amendment) names.
- SAS §7 (Law 1), §16.1, §16.2, §19.6, §21.2, §21.4, §21.5, §101.

## Assumptions

- A-001: the SAS document remains one hand-written file; the master document
  embeds its normative statements (`NORMATIVE.md`'s content), not its prose.
  Confidence: high; the owner set this scope aside on 2026-09-22.
- A-002: "fully expanded" means every atom of every current subject
  verbatim, and the master document is therefore long. Length is not a
  defect; a reader who wants less reads the app. Confidence: medium — the
  owner may ask for folding, which is a rendering choice and does not
  change the composition.
- A-003: reverting OW-WAR-0073's manifest restores the bytes its revision-1
  signature covers, so no re-authorization is needed. Verified 2026-09-22:
  `git show 443af497^:docs/warrants/OW-WAR-0073/manifest.toml` recompiles
  to contract digest `691f51ce…`, the signed one.

## Constraints

- No new record type and no schema change: two Markdown projections, three
  check rules, one flag on `war new`, one config key. The schema pack does
  not move.
- Files this Warrant declares that other Warrants pin are edited only after
  this Warrant is authorized (OW-ADR-0021); `compile.rs` (OW-WAR-0004),
  `relations.rs` (OW-WAR-0006), `new.rs` (0002/0062, resolved), `CONTEXT.md`
  (0068), `check.rs` and `ownership.rs` (0112) are declared.
- The master document is written by `compile` and by nothing else; a
  plant edits it and `check --generated` must refuse.

## Residual risks

- R-001: a corpus with a cycle in `supersedes` would have no current
  subject at either end. `relations.rs` refuses a cycle by name
  (`relations.currency-cycle`); the check is part of this Warrant.
- R-002: a preset that asks a question a subject has no answer to invites
  a filler answer. The preset says which headings may be deleted; the
  unanswered-heading rule fires only on headings the preset marks required.
