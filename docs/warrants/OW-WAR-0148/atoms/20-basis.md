---
schema: oh.war/atom/v1
warrant_uuid: 01a0feb6-ab80-73a5-abcb-7ade9cfaaeaa
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Requirements

- **RQ-013** (composition is typed, ordered and deterministic): a type now
  declares its records, relations and capabilities as data. The composition
  stays deterministic.
- **RQ-032** (state decomposed into phase, condition, outcome, currency,
  standing): the fixed kernel states map onto that decomposition. Declared
  states refine it and never replace it.
- **RQ-022** (WARs trace to SAS requirements and roadmap): typed relations
  generalize the trace, and `war impact` walks it.
- **RQ-025** (supersession preserves the old WAR and marks it non-current):
  `supersedes` is a core relation kind, unchanged in meaning.
- **RQ-015** (a missing required atom fails closed): kept. A missing required
  record or relation fails closed too.
- **RQ-074** (`war check` is deterministic and agent-free): every new rule is
  structural.
- **RQ-075** (generated views are drift-checked): the compiled model is a
  generated view, drift-checked like the others.

## Decisions relied on

- **OW-ADR-0021** (ownership by the latest authorized declaration): this
  Warrant's set governs the files it declares from its authorization.
- **OW-ADR-0022** (current by relation): supersession becomes one core
  relation kind. Its derivation is unchanged.
- **OW-ADR-0023** (the roadmap is a record): the roadmap is the first type
  that is accepted but not assured.
- **OW-WAR-0140's profile registry and OW-WAR-0147's working form**: the
  seams this Warrant generalizes.
- **OW-ADR-0031**, drafted with this Warrant and `proposed`: it records the
  model, and becomes binding only when the owner accepts it.

## Assumptions

- **A-001** (high confidence): every per-kind behaviour can be expressed as a
  capability from the closed set. The hard-coded branches were enumerated on
  2026-10-02. If one cannot be, it is named, and it escalates rather than
  gaining a new capability.
- **A-002** (high confidence): the compiled model can sit on the existing
  loaders (`load_warrant`, `lower`, `relations::currencies`,
  `resolve::assess`) without changing how any `CompilationBasis` byte is
  formed. So no contract digest moves. A byte-identical differential over
  eight commands on two corpora checks this.
- **A-003** (medium confidence): recording `profile_digest` in a new
  manifest is enough to bind the type into the signature, because manifest
  bytes are already in the contract digest. Old manifests have no field and
  keep their digests.
- **Q-001 is a blocking unknown**, recorded with `war ask`. The owner answers
  it at the signing sitting.
  - **The conflict:** SAS **RQ-061** says "Liminal owns document semantics
    and Basis", and **RQ-064** says "OpenWarrant does not duplicate those
    kernels". A standalone compiler that owns typed document semantics, as
    the owner directed, contradicts both as written.
  - **(A)** A SAS revision, proposed with OW-ADR-0031, moves document
    semantics for OpenWarrant's own records into its compiler and keeps
    Liminal's role for the documents Liminal holds.
  - **(B)** This lands as a projection layer beside Liminal, and RQ-061 and
    RQ-064 are unchanged.
  - **Recommendation: (A).** The SAS revision is the owner's to accept. This
    Warrant changes no SAS byte.

## Constraints

- **Frozen:** `WarIr`, the contract digest's inputs, `SCHEMA_PACK_VERSION`
  0.2.0, the `oh.war/report/v1` envelope, and every signed record's bytes.
- **Profile file compatibility:** `oh.war/profile/v1` files keep parsing.
  `form = "working"` maps to the working-form capability default.
- **Ticket file compatibility:** ticket files on disk stay byte-compatible.
- **Codex's branch:** `codex/reviewed-subject-binding` is not touched.
- **Other owners:** files governed by other authorized Warrants are declared
  here. By OW-ADR-0021 this Warrant governs them from its authorization, and
  their earlier pins become historical.

## Residual risks

- A rule family left on the "always on" path by mistake. Noticed by: each
  capability's plant runs a kind without that capability and observes the
  family not applied.
- The model drifting from a client's own reading where a client keeps one.
  Noticed by: the M1 differential and the model's drift check.
