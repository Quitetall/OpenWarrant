---
schema: oh.war/atom/v1
warrant_uuid: 01a03fb3-3dba-7205-8fdf-bbd7354e0aa3
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

Four stages. The first three need no host; the fourth does.

## STAGE-001 — the invariant, before anything it would guard

Build `visibleSet(P, T)` as an enumeration over
`core.set_access_context(uuid, text)`, and the comparator that diffs it against a
compiled document set. Both directions, reported separately, because they are
different failures with different consequences.

The gate must be **demonstrated to fail** in both directions before it is
trusted: plant a record the subject cannot see and require refusal; plant an
omission of a record they can see and require refusal. A gate that has never
failed is not evidence that it works — it is evidence that it ran.

This stage comes first deliberately. Building the compiler first and the check
afterwards produces a compiler whose output defines the invariant, which is
circular and would pass.

## STAGE-002 — the compiler

Four views (`about`, `authored`, `addressed`, `visible`) into one document with a
standard section order and an overview that precedes detail. Reuse `@kf/export`'s
section machinery rather than starting a parallel renderer.

Two outputs, not one: the document, and a **manifest** recording `compiled_at`,
the visible-set digest it matched, what was included, and what was withheld or
withdrawn with the reason for each. The manifest is what makes the document a
claim rather than a file.

Renderers: Markdown and HTML through `@kf/export`; PDF and DOCX through pandoc,
which is already host-qualified across three agreeing versions.

## STAGE-003 — withdrawal

A record that leaves the set reads as **withdrawn**, with time and reason, built
on `secure_object.erasure_tombstone`. Verified by removing a record from a
compiled set and requiring the withdrawal to appear — not by requiring the record
to be absent, which is what a silent drop also looks like.

## STAGE-004 — delivery, and only then

A signed, expiring, revocable, access-logged link served by KF itself, fed by a
consumer that drains `core.outbox` and writes a delivery receipt back. Then the
first real disclosure — my own master document — and then a derived subset
carrying its own runtime warrant.

The first transport opens **no new outbound credential and involves no third
party**. That is a deliberate ordering choice, not timidity: it is the only
transport whose failure is recoverable, and the invariant should be proven
against a recoverable transport before an irreversible one.

# Not authorized by this Warrant

Each of these is wanted, is recorded in `10-intent.md`, and has a named successor.
Listing them here is what keeps the boundary a sequencing decision rather than a
quiet narrowing.

**OW-WAR-0055 — the living external copy.** A work document held in Google Docs
or Drive that updates itself as records appear and are removed, with removals
recorded. This is the most valuable deferred piece and the hardest: it needs
OAuth *write* scope, a reconciliation loop, and a conflict model for a document
someone may have edited. It also crosses the boundary
`federated_source_read_only` was drawn beside. It deserves its own threat-model
pass and its own Warrant, not a milestone at the end of this one.

**OW-WAR-0056 — the web viewer and management platform.** Overviews, navigation,
and seeing who holds which subset. `apps/web` exists and has never completed a
login against a real realm. Its own Warrant once M1-M6 hold, because a viewer
over a document that is not yet provably complete would make the wrong thing
look finished.

**OW-WAR-0057 — email transport.** Separate credential, deliverability and
attachment-size surface, and irreversible on send. Deferred until the invariant
has been exercised against a transport that can be taken back.

# Constraints

**Sequence is load-bearing between stages, not within them.** STAGE-001 must
precede STAGE-002 for the circularity reason above. STAGE-004 must be last.

**No milestone is discharged by a passing test alone.** OBL-001 requires the gate
to have failed on a plant. A green run of a check nobody has ever seen go red is
not evidence, and this Warrant is at `controlled` partly to make that explicit.

**The invariant is not negotiable down to `⊆`.** If the `⊇` direction proves
expensive, that is a finding to record and decide about, not a licence to ship
half the claim while `10-intent.md` still promises both. Narrowing the claim
means amending this Warrant.
