---
schema: oh.war/atom/v1
adr_uuid: 01a0e507-38bd-7ea8-8105-f79353bea9ad
local_alias: OW-ADR-0026
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0d04c-5f55-7171-906f-45d79a408ce3"
---

# ADR OW-0026: A deliverable a native system holds is a reference, never a repository file

## Status

Proposed by the performer under OW-WAR-0127. **Not decided.** That
Warrant's blocking question Q-001 (U-001, rationale A-001) has no recorded
answer from the owner: `judgment_ref` is empty and `war answers` lists no
answer. Until the owner answers it and accepts this ADR, it records options
and a recommendation, and it governs nothing. No code classifies a native
reference yet, and OW-WAR-0127's work order says the performer starts the
code only after Q-001 is answered.

When the owner answers, the Decision section is rewritten to the answer and
the options not taken stay below as the record of the choice. If the answer
is not (a), the plant `conformance/plants.d/61-native-authority.sh`, which
writes its native reference in form (a), is rewritten to the chosen form in
the same change. The plant counts nothing while this ADR is not `accepted`:
it prints each case as UNKNOWN, with what it observed.

## Context

RQ-065: native systems retain artifact authority. §11 names them: CAD,
datasets, instruments, invoices, payments, QMS records. §13 gives `external`
as a Source Holder kind, and §13.2 says a bound fact held elsewhere is not
edited inside the Warrant. §37.1 shows `target_ref` as a URI (`git://...`).

Every reader of a deliverable today joins `target_ref` to the repository
root: `resolve.rs` (§56.1 requirements 2 and 3), `check.rs` (drift),
`pins.rs` (refresh), `correct.rs`, `bundle.rs`, `status.rs`. Declared as
`external://dataset-registry/ds-42` in a scratch program, the reference is
read as the path `<root>/external:/dataset-registry/ds-42`:

- absent there, `war check` warns `deliverable.target-unreadable` ("No such
  file or directory"). A question nobody could ask here reads as a failure.
- present there, `war pins --refresh` rewrites the recorded digest from
  those bytes, `war correct` reports them as the current digest, `war status`
  calls the digest `drift`, §56.1 requirement 2 is met, and the verification
  bundle carries the file's text as the deliverable. OpenWarrant has taken
  over an artifact it does not hold.

`crates/openwarrant-core/src/seam.rs` already states the rule for seams: the
other system owns its own facts, and OpenWarrant records references to them.
This ADR applies it to deliverables.

## Q-001 — the form of a native reference

### (a) `external://<system>/<external_id>` — recommended

One scheme. `<system>` names the holder; `<external_id>` is whatever the
holder calls the artifact, opaque to OpenWarrant. It mirrors Knowledge
Fabric's `content.external_locator` (`system`, `external_id`) at commit
`3d3c871e`, and §13's `external` Source Holder kind.

- One classifier: a `target_ref` whose scheme is `external` is native;
  every other value parses as it does today. OBL-006's "every existing
  deliverable classifies as a repository path" then holds by construction,
  because none of the 238 existing deliverables starts with `external://`.
- The holder is in the reference and in provenance, so the tool can see
  both. Whether it refuses a mismatch between them is a further rule the
  work order does not list; if the owner wants it, it is an amendment.

### (b) One scheme per kind (`dataset://…`, `cad://…`) — not recommended

The holder is named only in provenance. The set of schemes is open, so the
classifier needs a registry of them, or must treat any `scheme://` as native.
The first is U-003's list in another form; the second makes a typo or a
future `git://` (the form §37.1 shows) native by accident. A kind is also
not a holder: two dataset registries would share a scheme.

### (c) `kf://` references only — not recommended

A native artifact must first be registered in Knowledge Fabric. That makes
RQ-065 depend on a component a repository may not run, and moves the
question to OW-WAR-0126 (RQ-060), which OW-WAR-0127 lists as a non-goal. A
`kf://` reference can still be expressed later as a holder under (a).

**Recommendation: (a).** Without an answer, nothing is parsed.

## What every form shares (the work order, whatever the answer)

These follow from OW-WAR-0127's work order and do not depend on Q-001:

- **Recognized in one place.** `deliverable.rs` classifies a `target_ref`
  as a repository path or a native reference. No other file parses one.
- **A native reference carries its holder.** Validation refuses, each as a
  named `DeliverableError`: no provenance or `provenance_required = false`;
  a Source Holder that is empty or `git`; `content_addressed = true` with no
  content digest.
- **Nothing reads, rewrites or corrects it.** No command joins a native
  reference to the root. `pins --refresh` leaves its recorded digest as it
  is; `war pins` and `war status` say "held by <system>"; `war correct`
  refuses it by name and writes nothing; drift checking skips it and says so
  once per Warrant.
- **Unknown is not failure and not pass (Law 15).** §56.1 requirements 2 and
  3 are booleans and stay so (a frozen surface). A required native
  deliverable leaves them unmet, with an UNKNOWN diagnostic naming the
  holder, never "does not exist".
- **The verifier is told what it cannot see.** The bundle carries the
  reference, holder and recorded digest, marked not read, and no bytes.

## A-002 — a native reference is in the signed set

A native deliverable belongs in the deliverable set the authorizer signs
(`deliverable_set_digest`), so the authorizer sees what the Warrant claims
to produce. §37.5's ownership of a *path* does not apply to it: there is no
path, and OW-ADR-0021's `owned` set names repository paths only. Proposed:
a native reference is listed in the authorization's set and never in
`owned`. Confidence: medium; the owner may choose to keep it out of the set.

## R-003 — a repository file declared native

A performer could declare a repository file as native to escape pinning.
The holder must be a non-`git` system, so a record that says a repository
file is held elsewhere is a false record, not a loophole in the rule. The
refusal is limited to what the tool can see: it cannot tell that
`external://dataset-registry/ds-42` is in fact a file someone committed.

## Open, not decided here

- **Q-002 (U-002):** how a native artifact's presence and digest can ever be
  established here, so a Warrant with a required native deliverable can
  resolve: (a) a verifier-run gate whose receipt records what it fetched
  (§44.6); (b) the holder's own receipt, attached as evidence; (c) not yet.
  Until answered, such a Warrant stays unresolvable, as OW-WAR-0127 leaves
  it. Recorded for the owner; no recommendation is needed for this ADR.
- **U-003:** whether `openwarrant.toml` must list the native systems a
  repository may name. Default proposed here: no list. Under (a) a list
  would be a check on `<system>`; under (b) it is required.

## Decision

*Pending Q-001.* If the recommendation is taken: form (a),
`external://<system>/<external_id>`, with the shared rules above, A-002 as
proposed, and Q-002 and U-003 open.

## Consequences (if the recommendation is taken)

- No existing record changes. Every deliverable today is a repository path,
  parses as one, and pins and owns as before (OBL-006).
- A Warrant with a required native deliverable cannot resolve until Q-002 is
  answered. That is the honest state: nothing here can observe the bytes.
- Rolling back is removing the classifier and its refusals; a Warrant
  drafted with a native reference meanwhile is then reported by `war check`
  as `deliverable.target-unreadable`, by name: a warning today, not the
  refusal the work order's Rollback section expects.
