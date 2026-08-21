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
