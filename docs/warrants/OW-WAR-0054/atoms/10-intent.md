---
schema: oh.war/atom/v1
warrant_uuid: 01a03fb3-3dba-7205-8fdf-bbd7354e0aa3
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Give every person one living master document: everything the Knowledge Fabric
holds that concerns them, compiled from federated material into a single
navigable record they can read, export in any format, and share bounded subsets
of.

The sentence this Warrant exists to make true:

> **If it's not in the file, you don't know it or haven't been given permission.**

That is not a description of a feature. It is a claim about the relationship
between a document and a person's access, and it is either enforced or it is
decoration.

## The document is not a report about access. It IS access.

A conventional export answers "what did we decide to send you". This answers
"what do you have". The difference is that the second is checkable, because KF
already knows what a person can see: `core.set_access_context(uuid, text)` sets
the access context and the row-level policies on 76 tables do the rest.

So the master document has an invariant, not a specification. For person `P` at
time `T`:

    documentSet(P, T) == visibleSet(P, T)

Equality, in both directions, and each direction fails differently:

- **`⊆` — no over-disclosure.** Something in the document that P cannot see is
  another person's record, or privileged material, or something under
  `core.retention_hold`. This is the failure that cannot be taken back once a
  transport has run.
- **`⊇` — no under-disclosure.** Something P can see that is absent from the
  document breaks the promise the sentence makes. A person who is told the file
  is complete, and acts on it, is worse off than one who was told nothing.

An implementation that enforces only `⊆` is an export tool with good manners.
Only the conjunction is the claim.

## Four views, one document

Not a union — a union destroys the reason a reader opens the file. The same
record means something different depending on why it is theirs, so the master
document carries four standard views:

    about       records where they are the subject
    authored    records they created or acted on
    addressed   records directed at them: approvals, notices, assignments
    visible     everything else their access reaches

Standard layout, standard section order, an overview that makes the whole legible
before any detail, and navigation good enough that finding one thing does not
require reading the rest.

## Two consequences that are easy to miss

**Staleness is incorrectness, not age.** The invariant is stated at time `T`. A
document that no longer equals the visible set is *wrong* — it asserts a
completeness it no longer has. So a served document carries `compiled_at` and the
digest of the enumeration it matched, and a reader can tell whether they are
holding a claim or a souvenir.

**Absence must be explained.** When a record leaves the set it appears as
**withdrawn**, with time and reason. Silently vanishing is the worst available
behaviour: a reader who remembers something that is no longer there cannot
distinguish removal from their own error, and will trust the file less than if it
had never claimed completeness. `secure_object.erasure_tombstone` already exists
and is signed; this is a presentation obligation on top of a primitive that is
there.

## Sharing is a warranted act, and it recurses

A contractor's work document is a subset of **their own** master document —
scoped to an engagement so they can start work and see what is current. Deriving
it is a disclosure in its own right, with its own recipient and its own scope, so
it mints its own runtime warrant and receipt.

This is why the Warrant is both layers at once. This document authorizes building
the capability; the capability's obligation is that every act it performs is
itself warranted. Sharing is not a button with an audit log bolted underneath it.

## The whole horizon, recorded here so it survives this Warrant

Everything below is part of the intent and **none of it is authorized by this
Warrant**. It is written here rather than in a planning document because
planning documents are not in the repository, and a successor who reads only the
authorized scope would rebuild a smaller thing than the one that was wanted.

**One master document per person, for every person involved.** Not a compliance
feature reached by request. The normal way anyone — staff, contractor,
collaborator — finds out what they have and what is current. Federated to what
they should know, and complete within that boundary.

**Export in any format the reader needs.** PDF, HTML, Markdown, MS Office. The
document is one thing; the rendering is a choice made at the point of reading,
not a separate artifact that drifts.

**Share in any format the recipient uses.** Google Drive, and whatever else
people actually work in. Meeting people where they are is the difference between
a record system that gets used and one that gets exported from once.

**Living, not a snapshot — including outside KF.** If a person's work document
lives in Google Docs or in KF, it **updates itself** as records are added and
removed. New material appears. Removed material is **recorded as removed**, never
silently dropped. This is the hardest and most valuable part of the vision: a
shared copy that is still true tomorrow, and that tells you what changed.

**A web viewer and management platform.** Strong overviews, real navigation, and
the ability to see and manage what exists and who holds which subset — so the
person who owns records can see the shape of them, not just receive a file.

**Subsets that let someone start work.** A contractor receives their work
document and knows immediately what the job is and what is current, without
being handed either everything or a stale attachment.

Each deferred piece has a named successor in `40-work-order.md`. The boundary is
a sequencing decision about what must be true first, not a narrowing of intent.

## What this Warrant must not become

A general document management product. The temptation is real — a viewer, a
sharing UI, a Drive integration and an email sender are each one step from here
and each is separately reasonable. They are excluded by `40-work-order.md` on
purpose, because the invariant is the thing that makes this worth building and
none of them help establish it.

Build the claim first. Everything else is transport for a document that does not
yet deserve to be trusted.
