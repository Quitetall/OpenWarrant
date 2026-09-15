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

- **Known gap.** OBL-004's remaining six, quoted from
  `docs/sas/WAR_Software_Architecture_Specification.md` §91.4 and §91.5 rather
  than summarised, and each with what it actually needs:

  - **24. "A local choice inside autonomy does not require a new ADR."** This is
    a PERMISSION, not a prohibition. §92's plant shape asks for a violation
    rejected by its intended control, and a permission has no violation to plant
    — the honest form is a positive control asserting nothing fires, like the
    prose-lineage control in OW-WAR-0047. *Resolution requirement:* a decision
    about what a permission-plant asserts, then the control.
  - **30. "Parent source is unchanged when child state changes."** Also a
    property rather than a prohibition. Needs a rule that compares a parent's
    authored-atom digests across a child-state change, which nothing computes.
  - **32. "Child cannot silently replace parent rationale."** Needs a comparison
    between a child's rationale and its parent's, and §35's rationale records
    exist but are not related across the parent edge.
  - **33. "Superseding WAR makes old currency `superseded`."** `Currency` and its
    `Superseded` variant exist in `lifecycle.rs` and are RENDERED — OW-WAR-0001's
    view shows OW-WAR-0002 as `(current)`. What is missing is the transition:
    nothing sets a currency to superseded, because no Warrant supersedes another
    yet.
  - **34. "Superseded WAR remains exportable."** Blocked behind 33 — there is no
    superseded Warrant to export.
  - **35. "Adopted unresolved children are explicit."** Needs an adoption
    relation; the manifest has `[[parents]]` and no adoption.

  Recorded at this length because the previous attempt at a group like this
  (OW-ADR-0007) collapsed four tests into one phrase and got three of them wrong.

  Each of the six above states a CODE FACT — that nothing computes a digest
  across a child-state change, that nothing sets a currency to superseded. Those
  are true on 2026-08-22 and will stop being true as the code moves, at which
  point this entry is stale in the direction that matters: it would claim a gap
  that has closed. Re-read it against the code before citing it, rather than
  citing it because it is written down.

## Constraints and Invariants

- **The import artifact is a GOLDEN file.** `--verify` compares byte-for-byte
  against `artifacts/lamquant-adr-import.json`, so any intentional change to the
  importer's output format makes the committed artifact stale and the check fail
  — correctly. The response is to re-run without `--verify`, inspect the diff,
  and commit the new artifact as a deliberate re-blessing. It is not a one-way
  door; it is a golden, and a golden that can never be regenerated would be the
  defect.

- **Bytes are preserved** (§96.1). Migration adds structure alongside
  the original and never replaces it.
- **No fabricated proof** (§96.3). This is the constraint under the most
  pressure, because a corpus of historical claims looks worse than a corpus of
  resolutions and nobody is watching.
- **An unmapped element is reported, never guessed.**
