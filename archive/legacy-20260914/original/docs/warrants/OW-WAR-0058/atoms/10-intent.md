---
schema: oh.war/atom/v1
warrant_uuid: 01a06069-882d-7103-a07a-8e2d5c23bd12
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Make "v1.0 of the SAS" a thing that exists as a record, so that "69% of v1.0"
can be a question with an answer rather than a number someone remembers.

## The SAS is not under its own governance

§101 says the SAS becomes a controlled document: accepted revisions are
immutable (101.2), an architecture-changing revision requires an ADR (101.3),
and mirrors state the exact accepted revision and digest (101.6). None of it
is implemented. The digest `aad5256c…` appears in six places in this
repository, all prose; nothing computes it and nothing compares it. §14
lists "SAS revision" as an input to every compilation's Basis, and
`FormatBasis` has no field for it — so no Warrant is bound to *which* SAS it
implements, and the Release axis of the corpus projection reads
"not recorded".

## What this delivers

A revision record: `docs/sas/revisions/<version>.toml`, pinning one version to
the sha256 of the document's bytes, carrying a snapshot of §106 so two
revisions can be diffed from records alone, and moving from `proposed` to
`accepted` exactly once, by a human. `war sas propose` writes one; `war sas
accept` is the same two-half seam as `war authorize` — an agent may propose,
only a human accepts, and ingestion refuses every agent regardless of what
the response says. `war check` compares the document's bytes to the latest
revision's digest and refuses drift. The compilation Basis carries the
revision, so every Warrant's `workspace_basis_digest` moves once, in one
commit, and is bound from then on to a named SAS.

## The rule this Warrant adds to the corpus

Requirement identifiers are append-only. §34.1 calls them stable; §34.3
forbids editing the SAS to tick boxes; §34.4 step 4 preserves a requirement
that turned out to be wrong. A §106 row may be added or retitled and may
never be removed or renumbered — `war sas diff` refuses a candidate that
drops an id. Superseding a requirement is a Warrant's act (§34.2
`supersession`), never an edit to the index.

## What this does not do

It does not accept anything. This Warrant proposes `0.1.0-draft.1` — the
document as it is, at its current digest — and stops. Acceptance is the
owner's, and until it happens the projection says "proposed, not accepted".
