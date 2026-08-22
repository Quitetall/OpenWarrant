---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b571-794a-acf0-1559844cf662
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§96, §97, §19.2, §21, §91.4 test 24, §91.5 tests 30–35.

## Prerequisites

OW-WAR-0038 and OW-WAR-0006 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** The importer is implemented, and §96.3's five
  promotion conditions are each tested by removal.
- **Evidenced premise** *(was a blocking unknown; resolved 2026-08-21).*
  LamQuant's ADR corpus is frozen. The resolution requirement was "a named
  LamQuant commit, agreed and frozen for the duration of the import", and
  LamQuant ADR 0186 clause 5 supplies it:

  | | |
  |---|---|
  | SHA-F | `ba9ed833faa9a52940d5e9d424566466e9066867` |
  | branch | `integration/consolidate` |
  | corpus at SHA-F | **173** ADRs |

  The freeze is mechanically enforced on the LamQuant side by
  `tools/check_corpus_frozen.py`, wired into its `doc-tree` job: after SHA-F the
  only permitted write under `docs/decisions/` is an append to ADR 0186 itself.
  It fails closed — an unknown sha, or one that is not an ancestor of HEAD, is
  an error rather than the empty diff that would otherwise read as a pass.

  **The corpus size stated here was wrong and is corrected.** It read "167 at
  `5369da81`, more today". The measured figures are **172** at `39ab6317` and
  **173** at SHA-F — 173 because ADR 0186 is itself part of the corpus it
  closes. An importer told to expect 167, or 172, and handed 173 either drops a
  record or reports a corpus it did not read, and the record it would drop is
  the record of this migration.

- **Accepted residual risk.** §96.2's table maps twelve element names. An ADR
  section named something else is reported unmapped, and a human must decide.
  *Consequence if false:* an unmapped section is silently dropped, which §96.1
  forbids.
  *Now quantified.* LamQuant's `## Completion` heading exists in **nine distinct
  spellings** across 81 ADRs, and only **7** are the bare literal `Completion`.
  Under exact `==` matching this row maps 7 and reports 74 unmapped — not an
  edge case but the common path, and 74 human dispositions is not a viable
  route. Normalisation, or a §31 amendment to §96.2, is required before the
  import runs.

- **Accepted residual risk — the corpus is a property of one ref.** 184 distinct
  ADR ids exist across LamQuant's 128 refs; SHA-F carries 173. Twelve exist only
  off-branch (`0147`–`0149`, `0169`–`0175`, `0187`, `0188`), and **two of those
  are already frozen** — `0188` `complete`, `0187` `superseded`, with
  `0188 supersedes 0187 supersedes 0173`, on `refs/heads/integration/adr0167`.
  ADR 0186 clause 6 requires each of the twelve to be migrated or recorded as
  deliberately excluded.
  *Consequence if false:* a census of the frozen 173 silently loses 12 records,
  two of them frozen — the failure §96.1 forbids.

- **Accepted residual risk — extractor disagreement is a decision, not a
  detail.** LamQuant's own `adr_model._gate_commands` reports **67** ADRs
  carrying **95 distinct** gate commands (100 occurrences); a raw `gate_cmd:`
  line scan reports **162**, because it matches prose and template mentions the
  real parser filters. The importer must declare which extractor it uses and
  match `adr_model`'s semantics.
  *Consequence if false:* an import whose counts cannot be reconciled with the
  source, which is indistinguishable from an import that lost records.

## Constraints and Invariants

- **Bytes are preserved** (§96.1). Migration adds structure alongside
  the original and never replaces it.
- **No fabricated proof** (§96.3). This is the constraint under the most
  pressure, because a corpus of historical claims looks worse than a corpus of
  resolutions and nobody is watching.
- **An unmapped element is reported, never guessed.**
