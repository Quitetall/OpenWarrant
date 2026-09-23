---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ef7-7862-8205-72d16ebcaf9f
role: intent
jurisdiction: authored
order: 10
classification: internal
---


# Intent

## Problem

RQ-023 says a child Warrant cites the exact revision of its parent. §20.2
gives the citation three parts: the parent's ref, a `contract_revision` and
a `contract_digest`. Today the tool checks the digest against the parent's
**current** compiled contract and never checks the revision number.

- `manifest.rs` refuses a `[[parents]]` entry with no
  `contract_revision`, and nothing more. Any number passes.
- `check.rs` (`relations.parent-digest`) compares the cited digest with
  the parent's contract as it compiles now. So a child can never rest on
  an older, exact revision: every amendment of a parent turns all its
  children red until they re-cite.
- That is what happened. OW-WAR-0001 was amended to revision 2
  (`authorization.toml`: revision 2, digest `f29c7d95…`, predecessor
  `ab7e2df7…`). Its four children, OW-WAR-0002 to 0005, were re-cited to
  the new digest and kept `contract_revision = 1`. Each now cites
  "revision 1" at revision 2's digest, and `war check` passes them.
- `war new` has no `--parent`, although §71.2 lists it. A child's
  citation is typed by hand, which is how the number and the digest came
  apart.
- §106 lists RQ-023 as unaddressed: no Warrant implements it.

## Desired Outcome

- **The number and the digest must name the same revision.** For each
  `[[parents]]` entry, `war check` looks up the cited revision in the
  parent's authorization records and compares the cited digest with that
  revision's digest.
  - A revision the parent never had is an error.
  - A digest that belongs to a different revision of the parent is a
    finding that names the revision it does belong to (severity: Basis,
    U-001).
  - A digest that matches no revision of the parent is an error, as today.
- **An exact older citation is sound.** A child citing revision 1 at
  revision 1's digest, under a parent now at revision 2, passes. A
  separate warning, `relations.parent-moved`, says the parent has moved and
  that re-citing is an amendment of the child.
- **An unauthorized edit to the parent is still caught.** When the child
  cites the parent's latest authorized revision and the parent's working
  contract no longer compiles to that digest, `relations.parent-digest`
  reports it as it does today. The existing plant in `00-corpus.sh` keeps
  holding.
- **What cannot be looked up is UNKNOWN.** An earlier revision's digest
  comes from retained history (`contract_history.rs`). In a shallow clone
  or with no Git, the finding is UNKNOWN, never a pass or an error.
- **`war new --parent <alias>` writes the citation.** It writes the
  parent's `war://` ref, its latest authorized revision and that
  revision's digest, from the parent's `authorization.toml`. It refuses a
  parent with no authorized revision.

## Non-goals

- Cross-repository parents. They stay UNKNOWN until federation exists
  (RQ-005).
- `war child` (§71.11) and inherited context selectors (§20.2's
  `inherited_context_selectors`). A later Warrant carries them.
- RQ-024 (a child does not rewrite parent rationale) and the parent's
  child view (§20.4), which exist already (`relations.parent-source`,
  `relations.child-listed`).
- Editing OW-WAR-0002 to 0005. Their manifests are under signature. How
  their citations are reported is the owner's decision (U-001), and any
  change to them is an amendment by the owner.
