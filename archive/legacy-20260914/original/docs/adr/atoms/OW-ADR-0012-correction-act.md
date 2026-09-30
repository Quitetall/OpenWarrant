---
schema: oh.war/atom/v1
adr_uuid: 2e497de8-3ffd-489c-88ad-b2e34fd98ee0
local_alias: OW-ADR-0012
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a06a12-0aa2-7503-b589-67cf75905be4"
---

# ADR OW-0012: A delivered artifact of a resolved Warrant moves only through a correction

## Status

Proposed by the performer under OW-WAR-0064; adopted when the owner accepts it.

## Context

`deliverables.toml` pins the bytes of every delivered file. A resolution binds
`sha256(deliverables.toml)` into its own record (`artifact_manifest_digest`).
Once a Warrant is resolved, its pins therefore have no authorized way to change:
`war check` raises `deliverable.digest-drift`, and the remedy that rule names —
regenerate the record — would move the resolution's digest and stale the
resolution.

Three defects hit that wall in one day, all found by other projects reading our
seams, none fixable: the Dispatch compiler drops continuation lines of wrapped
instructions (OW-WAR-0056 D-002), `war status` falls back to a literal `"OW"`
namespace (OW-WAR-0055/0057), and neither `war sas accept` nor `war authorize`
validates `effective_time` (OW-WAR-0058). A fourth followed: `plant.sh` itself,
pinned by OW-WAR-0063, could accept no new plant.

The gate was right to refuse. A delivered artifact must not move because the
performer noticed something afterwards. What was missing was the act that lets
it move for a reason. Without one, known-wrong artifacts accumulate until
somebody decides the digest gate is an obstacle rather than a control — and that
decision, once taken, is taken for every artifact.

SAS §34.4 already answers this question for a *requirement* that turns out to be
wrong: open an ADR, propose a controlled revision, supersede or amend affected
Warrants, and preserve the original requirement and its evidence history.
Supersede rather than erase. Nothing said what those four steps are for a
*deliverable*. This ADR says.

## Decision

§34.4's four steps, applied to a delivered artifact of a resolved Warrant:

1. **The act is written down before it is used.** This ADR is that record; the
   mechanism is OW-WAR-0064's deliverable, not the other way round.
2. **No SAS text changes.** §37's pin is untouched: `deliverables.toml` keeps
   the digest it recorded, and the resolution keeps binding that file. A
   correction is *added evidence*, not a revised record.
3. **Supersede through a record beside the manifest, never by regenerating it.**
   Each correction is one append-only file,
   `docs/warrants/<alias>/corrections/<deliverable-id>-<n>.toml`
   (`oh.war/correction/v1`), carrying the deliverable, the digest it supersedes,
   the digest the file has now, the reason, the kind of change
   (`behaviour-change` or `added-refusal`), the human who authorized it, the
   role exercised, and the effective time. Corrections chain: the first
   supersedes the recorded digest, each later one supersedes its predecessor's
   `new_digest`, numbered 1..n with no gaps. One function, `chain_head`, decides
   the digest the file must have now, and `war check`, `war correct`, `war sign`
   and `war show` all use it.
4. **The original stays honest history.** The recorded digest remains in
   `deliverables.toml`, in every correction's `superseded_digest`, and in what
   `war show` prints under a `## Corrections` trailer. A correction that erases
   what it replaced has done the thing this act exists to prevent.

The act is the fifth two-half seam. An agent may emit the request
(`war correct <alias> <deliverable-id>`), which names the recorded digest, the
chain head, the file's digest now, and who may sign; only a human's response,
ingested through the authority register, writes a record. An agent is refused by
kind, as everywhere else (§27.2). The journal witnesses each record's digest as
written, so a correction edited afterwards — or a second one applied as an edit
to the first — is caught as `correction.edited`, and a record no one ingested is
`correction.unjournalled`.

`war check` suppresses `deliverable.digest-drift` in exactly one case: the
file's bytes equal the chain head. Every other divergence is a named refusal —
`correction.new-digest-mismatch` (the correction corrects nothing),
`correction.superseded-never-delivered` (it supersedes nothing that happened),
`correction.sequence-gap`, `correction.malformed`, `correction.agent`.

The `kind` field is recorded and not enforced. A behaviour change alters what
already-accepted records meant; an added refusal cannot invalidate an accepted
record, only stop the next bad one. A later revision may admit the second more
readily than the first; this ADR gives it the data and does not pretend to have
decided.

## Consequences

- A resolved Warrant can be repaired without unmaking the judgment that
  resolved it. The resolution stands; the artifact moves; both are on record.
- The conformance battery, the Dispatch compiler, `war status` and the two
  ingest seams named above can now be fixed, each through a correction signed
  by the owner — the first users of the act are not its author.
- A correction is only as good as the reason written in it. Nothing here tests
  whether a reason is honest; the record makes it attributable, which is a
  smaller claim, and the one this system makes everywhere.
- Signing a correction without reading it converts this control into paperwork.
  `war sign` shows the superseded and current digests and refuses to draft
  without a kind and a reason; it cannot make anyone read them.
