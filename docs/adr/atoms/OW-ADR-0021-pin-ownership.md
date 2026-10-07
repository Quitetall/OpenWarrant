---
schema: oh.war/atom/v1
adr_uuid: 01a0ca4b-0458-79f2-b7f1-77302bb98192
local_alias: OW-ADR-0021
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0ca4a-0c02-7cd3-b49d-786377a1aa06"
---

# ADR OW-0021: A delivered path is governed by the latest authorized Warrant that declares it

## Status

Proposed by the performer under OW-WAR-0112. Adopted when the owner accepts
SAS revision 1.1.0: this revision retitles a §106 row, which §101.3 makes
architecture-changing, and this ADR is the record that revision carries.

## Context

RQ-036 says a delivered artifact of a resolved WAR changes only through a
recorded correction. Applied to a living codebase it produced, by 2026-09-22,
52 resolved pins over 47 paths, 27 corrections already signed, 22 more
waiting, four files pinned by two Warrants each, and a rule under which the
oldest code is the most expensive to touch: every later Warrant that
legitimately edits a file has to buy a human-signed correction from every
earlier Warrant that once delivered it. A one-line fix to `resolve.rs` costs a
signature. A relicense of seventeen headers cost twenty-one.

OW-ADR-0012 gave the correction act for repair after the fact. It never
claimed to be the mechanism for authorized new work, and it is not one: a
correction attests that an artifact moved for a reason, after the fact, under
the old Warrant's name. New work is authorized under a new Warrant's name,
before the fact. Using the first to account for the second is why the wall
grows with the repository's age.

The owner answered this on 2026-09-13, recorded in
`docs/design/openwarrant-friction.md` as Q2: *preserve the historical
delivered version and its evidence; let the new Warrant govern the new
version, with explicit lineage.* Corrections against every older delivery
were not selected. That answer needs a controlled change to RQ-036; this ADR
is it.

Two facts shape the mechanism.

Deliverables are not inside the contract digest: eight of §28.5's seventeen
elements are covered today and `deliverables` is not among them, so
`deliverables.toml` can be edited after signing. Ownership therefore cannot
be read from that file alone; something the human signed has to name the
paths.

And `war pins --refresh` rewrites content digests during ordinary work — that
is its job while a Warrant is open. So the thing an authorizer grants is the
**set of declared paths**, not their bytes. The bytes are what the resolution
will bind, later, as it does today.

## Decision

1. **Ownership is granted at authorization.** The authorization request lists
   every declared deliverable as `(id, target_ref)` and a digest of that set
   (RFC 8785 over the sorted pairs). The response the human signs echoes the
   set digest. The record stores the set. Ingestion refuses a response whose
   set digest no longer matches the manifest (`authorize.stale-deliverables`):
   what was drafted is what was signed. Widening the set after authorization
   is a material amendment under §31 and a re-authorization, or a new Warrant.
   The signing screen prints the paths under "Grants ownership of:", so a
   signer sees what they grant.

2. **The current owner of a path is the most recently authorized Warrant
   whose recorded set names it** — ordered by authorization effective time,
   then by alias on a tie; ignoring Warrants whose currency is `superseded`
   and resolutions whose standing is annulled. An authorization recorded
   before this ADR carries no set and owns nothing; it gains ownership only
   through a fresh signature over a request that lists the paths. The rule is
   not retroactive.

3. **Drift is an error only for a path no authorized Warrant currently
   governs.** For a resolved Warrant W and a declared path P whose bytes no
   longer match W's chain head: if a later authorized owner O exists, W's pin
   is *historical* and `war check` passes it as `deliverable.superseded-by`,
   naming O; if none exists, the change is unaccounted and
   `deliverable.digest-drift` stands, with two remedies named — declare the
   path in a Warrant and have that Warrant authorized, or record why it moved
   with `war correct`. A pin on an unresolved Warrant is a note, not a
   promise, exactly as today (`deliverable.pin-stale`).

4. **Historical means recorded and verifiable, not deleted.** The earlier
   Warrant's `deliverables.toml` and correction chain are untouched; its
   resolution keeps binding them; `war pins --history <path>` renders the
   lineage oldest to newest. A resolution records a commit locator at ingest
   so the delivered bytes can be re-hashed from history; resolutions recorded
   before this ADR report `UNKNOWN` for that question rather than a guess.

5. **OW-ADR-0012 stands, narrowed.** The correction act remains the only way
   to move a path that no later authorized Warrant declares, and the only way
   to repair a resolved delivery in place. A correction against a historical
   pin is refused (`correction.historical`): there is nothing to correct.

6. **RQ-036 is retitled, not removed:** *A delivered artifact changes only
   under a later authorized Warrant that declares it, or through a recorded
   correction.* One row is added, RQ-037: *Ownership of a delivered path is
   recorded at authorization and belongs to the latest such authorization.*
   §37 gains a subsection stating the rule; §28.4 records that an
   authorization covers the declared set; §56.2's record gains an optional
   locator. No row is removed and none is renumbered (§34.1, §34.4).

## Why not the alternatives

- **Put deliverables inside the contract digest.** Every authorized digest in
  the corpus moves and every existing authorization reads as stale — roughly
  a hundred re-signatures for a fact no signer disputed.
- **A separate ownership act.** A third signature for what authorization
  already means: "this Warrant may do this work on these files."
- **Keep corrections as the mechanism.** Twenty-two pending today, one more
  per touched file per earlier Warrant, forever. The owner refused this for
  Q2, and the arithmetic refuses it on its own.
- **Ownership on resolution rather than authorization.** Recreates the wall
  for the whole life of the work: every edit during an authorized Warrant
  would still need a correction from the earlier one.

## Consequences

- The first authorized Warrant that declares `check.rs`, `diagnostic.rs` and
  `resolve.rs` is OW-WAR-0112. The pins of OW-WAR-0005 and OW-WAR-0046 on
  those paths become historical the moment it is authorized, and their two
  pending corrections are no longer offered. The twenty remaining relicense
  corrections are edits under no Warrant; they still drift and are still
  signed as corrections. The rule does not launder the past.
- Old code is touched by declaring it. A Warrant that lists a path in its
  deliverables and is signed may edit that path with no further act, however
  many earlier Warrants once delivered it.
- The pin guard denies edits to current resolved pins only. A historical pin
  is editable under the owning Warrant's authority.
- An authorizer now signs over which paths a Warrant may govern. A request
  that hides a path is caught by the set digest; a path added to the manifest
  after signing is reported as declared-but-unowned.
- Ownership is ordered by a locally stamped effective time (§67.2 calls it
  provisional). Two authorizations signed out of order would flip an owner;
  ingestion refuses an authorization whose effective time precedes an
  existing owner of the same path.
- A pseudo-terminal defeats the terminal gate that the guided `war init`
  (OW-WAR-0112 M4) uses before writing the authority files, exactly as it
  defeats `war sign`'s plain path — THREAT_MODEL entry 2's residual, accepted
  once more in the same words.
