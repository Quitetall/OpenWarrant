---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b571-794a-acf0-1559844cf662
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the import ran against one named, frozen commit
- **scope:** the LamQuant ADR corpus at exactly one commit.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** LamQuant, by exact commit SHA, recorded before
  the import begins.
- **evidence:** the SHA recorded in the import artifact, and a re-run at that SHA
  producing byte-identical output.

### OBL-002 — every ADR's bytes survive
- **scope:** every ADR in the corpus at the frozen commit — a bounded corpus,
  not a universal claim. Nothing is asserted about ADRs added after the freeze.
- **scope kind:** bounded_corpus
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each imported ADR carries its original body and a
  digest of it, and a digest comparison against the source shows zero
  differences, counted across the whole frozen corpus rather than a subset.

### OBL-003 — no completion claim became a resolution
- **scope:** §96.3, over the whole frozen corpus.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the count of promoted resolutions is ZERO, and a plant
  attempting to promote a `Complete` line with no admissible evidence is refused
  by `LegacyCompletionPromoted` against the shipped binary.

### OBL-004 — the parent/child and ADR conformance tests are planted
- **scope:** §91.4 test 24 and §91.5 tests 30 through 35.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** seven entries in `conformance/`, each rejected by a
  named rule with a named detail string.

## Evidence

§40's records for the import, which ran for real against the LamQuant corpus.

### EV-001 — the import artifact
- **class:** evidence
- **kind:** migration_artifact
- **origin:** performer
- **admissibility:** performer_report_only
- **digest:** sha256:pending-receipt-binding
- **method:** `war migrate --corpus /mnt/4tb/LamQuant/docs/decisions --commit
  0e0e04a7a6cff7901c073c27cc233d9af9f11664`, written to
  `artifacts/lamquant-adr-import.json`
- **occurred at:** 2026-08-22

### EV-002 — the determinism re-run
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** the same command with `--verify`, which compares against the
  committed artifact instead of writing it; exit 0
- **occurred at:** 2026-08-22

### EV-003 — the promotion refusal
- **class:** evidence
- **kind:** negative_control
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** the same command with `--attempt-promotion`, which tries to promote
  every completion line and must always fail
- **occurred at:** 2026-08-22

### OBS-001 — 173 ADRs imported at one frozen commit, zero preservation failures
- **class:** observation
- **evidence:** EV-001
- **method:** counted by the importer across the whole frozen corpus, not a
  sample. The corpus directory holds 175 `.md` files; `README.md` and
  `TEMPLATE.md` are not ADRs and are not matched by the `NNNN-*.md` rule, which
  accounts for the difference exactly — the two were checked by name rather than
  assumed to be the gap.
- **admissibility:** performer_report_only

### OBS-002 — a re-run at the same SHA is byte-identical
- **class:** observation
- **evidence:** EV-002
- **method:** `--verify` exited 0 against the committed artifact
- **admissibility:** controlled_measurement

### OBS-003 — zero completion claims became resolutions, and promotion is refused by name
- **class:** observation
- **evidence:** EV-001, EV-003
- **method:** the artifact records 82 historical claims and 0 promoted
  resolutions; `--attempt-promotion` exits 1 with `LegacyCompletionPromoted`
  naming `0057-training-campaign-plan.md`. The count alone would not be evidence
  — a build that promoted nothing because it tried nothing looks identical — so
  the negative control is what makes the zero mean something.
- **admissibility:** controlled_measurement

### EV-004 — the test 31 plant
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** conformance/plant.sh, executed by cargo xtask gate
- **occurred at:** 2026-08-22

### OBS-004 — §91.5 test 31 has a purpose-built control, and it is planted
- **class:** observation
- **evidence:** EV-004
- **method:** removing the child line from OW-WAR-0001's committed view is
  rejected by `relations.child-listed`. The plant was first written against
  `generated.drift`, which also fires, and the harness refused it — §92 requires
  the intended control, and a rejection for the wrong reason proves nothing
  about the rule it was meant to exercise.
- **admissibility:** controlled_measurement

### JDG-001 — OBL-004 is partly discharged, and the remainder is named
- **class:** judgment
- **kind:** scope_holding
- **actor:** QuiteTall
- **acting role:** author
- **meaning:** OBL-004 asks for seven plants. One exists — §91.5 test 31, above.
  The other six are not narrowed and not silent; each is recorded in the Basis
  with what it needs. Test 24 is a PERMISSION ("a local choice inside autonomy
  does not require a new ADR"), so its control is a positive one rather than a
  rejection, and §92's plant shape does not fit it without a decision about what
  a permission-plant asserts. Tests 30, 32, 33, 34 and 35 need rules that do not
  exist yet, not plants against rules that do.
- **basis:** OBS-004
- **authority:** authorized
- **limitations:** one actor, so this judgment is not independently reviewed —
  §27.4 says role separation by one person is not organizational independence

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could the import launder an unverified claim?** That is
the question §96.3 was written for, and the honest answer is that the type system
now makes the easy version impossible: `HistoricalClaim` is a different type from
`Resolution`, with no conversion that does not require evidence.

The version it cannot stop is social. A corpus of 173 historical claims and zero
resolutions looks like failure to anyone who does not know what those words mean
here, and the pressure to "just mark the obviously-done ones done" will be real
and will come with good arguments. OBL-003 is written as a count precisely so
that bending it requires changing a number someone can check.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

§96.2 maps twelve element names. LamQuant's ADRs use a house template that
has drifted over 173 documents, and sections that fit none of the twelve will be
reported unmapped. The risk is not that they are dropped — the importer reports
them — but that a large unmapped list gets dispositioned in bulk by someone
tired.
