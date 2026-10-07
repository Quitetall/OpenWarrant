---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ff5-77b0-a389-10e0350f4b79
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- §44.6: a receipt SHALL record subject digests and fixture digests.
- §56.1 requirement 5: every required gate has an admissible result.
- RQ-054: a required unknown result blocks resolution.
- RQ-059: a resolution binds the exact contract and assurance snapshot.
- §33.4: a WAR SHALL declare source precedence; equal-precedence conflicts
  block readiness unless an explicit resolution exists.
- §33.6, §33.7: the context manifest records included, omitted, unresolved
  and conflicts, with a reason for each selection. A required item is never
  dropped.
- RQ-043: a Dispatch contains its exact basis.
- Product spec (`docs/design/openwarrant-product-spec.md`): Q23, and
  "Engineering contracts still to specify".
- Code read for this draft:
  - `crates/openwarrant-cli/src/evidence.rs`: `admissibility`,
    `contract_subject`, `record`;
  - `crates/openwarrant-cli/src/gate_cmd.rs`: receipt minting,
    `fixture_digests: vec![]`;
  - `crates/openwarrant-cli/src/context_select.rs`,
    `crates/openwarrant-compiler/src/dispatch.rs`: `conflicts: vec![]`,
    precedence by item kind;
  - `crates/openwarrant-core/src/context.rs`: the omission and conflict
    rules as types.
- OW-WAR-0059's basis chose contract-digest binding over workspace-basis
  binding, because an edit to a non-contract atom should not invalidate a
  run. This Warrant keeps that for atoms and adds the source bytes the gate
  observed.
- OW-ADR-0021: declaring an existing file makes this Warrant its owner on
  authorization. `gate_cmd.rs` is pinned today by OW-WAR-0020 (resolved)
  and OW-WAR-0059 (authorized); `evidence.rs` by OW-WAR-0059;
  `compiler/src/dispatch.rs` by OW-WAR-0056 (resolved);
  `context_select.rs` by OW-WAR-0068; `docs/RESOLVING.md` by OW-WAR-0112.

## Assumptions

- A-001: the git tree of `HEAD` plus a clean/dirty flag is an honest name
  for "the source a repository gate ran over". A dirty tree has no such
  name, so its receipt is reuse-`UNKNOWN`. Confidence: high.
- A-002: the digest of every declared deliverable's bytes, in `D-` order,
  is a narrower subject that lets a run stay admissible across a commit
  that changes no deliverable. Confidence: medium. A gate that reads files
  outside the deliverable set (the `war-check` gate reads the whole corpus)
  is not covered by it, so the tree is recorded too and Q-001 decides
  which subject governs.
- A-003: receipts minted before this change carry only the contract
  subject. For a Warrant not yet resolved they become reuse-`UNKNOWN` and
  need `war evidence record` again. Confidence: high that this is the
  honest reading. The cost is one re-run per authorized, unresolved
  Warrant.

## Unknowns

- **Blocking unknown — Q-001: which subject decides reuse.** Options:
  - (a) the tree: any commit invalidates every receipt. Simple and strict;
    gates re-run on every unrelated commit.
  - (b) the deliverable bytes, with the tree recorded but advisory.
  - (c) a Gate Definition declares the paths it reads (`inputs` globs) and
    the digest over those decides; a gate with no declared inputs falls
    back to (a).

  Recommendation: (c) with the (a) fallback. It adds a Gate Definition
  field, and definitions are immutable (§43.3), so each gate that declares
  inputs gets a new version.
- **Blocking unknown — Q-002: what a context conflict is.** Options:
  - (a) not decided here; the manifest says `unchecked` rather than `[]`.
  - (b) the author declares conflicts and their resolution in the stage,
    and the compiler refuses a Dispatch with an open equal-precedence one.
  - (c) the compiler detects one mechanical kind: the same source path
    included twice at different digests or revisions.

  Recommendation: (a) now, with (c) as the first detected kind. (b) adds
  authoring work with no detector behind it.
- U-003 (non-blocking): where a Warrant declares source precedence
  (§33.4). The compiler's default order stays until a Warrant can declare
  one. This Warrant does not add a manifest field.

## Residual risks

- R-001: under Q-001 (b), a gate that reads a file outside the deliverable
  set can keep a stale pass. The tree subject is recorded, so the gap shows
  in the receipt. It is not closed.
- R-002: every authorized, unresolved Warrant with receipts moves to
  reuse-`UNKNOWN` at once. That is a true state. The remedy is one command
  per Warrant.
