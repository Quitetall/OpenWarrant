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

So the master document has an invariant, not a specification. It has two halves,
because relevance and permission are different kinds of thing and conflating them
is what makes this claim slippery.

**Relevance is a graph property.** `org.person.id` references `core.object(id)`,
so a person is a node. `core.relation` is typed, stateful and time-bounded. What
concerns you is what the graph connects to your node.

**Permission is a lattice property.** `core.set_access_context` takes
`(p_organization uuid, p_max_classification text)` and sets `kf.organization` and
`kf.max_classification`. No policy in `row_security.sql` reads an actor or a
person. RLS admits by tenant and ceiling, not by identity.

    relevance(P, T)   = ⋃ over anchors a of P: closure(target(a), propagation[a])
    permission(O, C)  = the set RLS admits at organization O, ceiling C

    documentSet(P, O, C, T) == relevance(P, T) ∩ permission(O, C)

Both sides are computable, which is the only reason this is a claim and not a
hope. Note what the parameters say: **two people in one organization at one
ceiling have identical permission sets.** What distinguishes their documents is
relevance, and relevance alone.

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

### Where the claim stops

Both `relevance` and `permission` are computed over KF's own tables — the typed
graph and the row-level policies. So the invariant holds for **content
materialized into KF-governed rows** and nowhere else. Federated content resolved
live from an external system is governed by that system's access model, not KF's,
and has no node in the graph to be reachable from.

This is stated here, beside the claim, rather than only in the residual risk at
the end of `60-assurance.md`, because the intent atom is the one most likely to
be read alone. A successor who learned about the boundary three atoms later could
reasonably have built a compiler that includes live federated content, proven the
invariant against KF-native rows, and shipped a document whose completeness claim
silently does not cover part of itself — which is precisely the failure this
Warrant exists to prevent, arriving through the door marked "vision".

So: either federated content is materialized into governed rows before it may
enter a document, or the document states which sections the invariant does not
cover. `45-milestones.yaml` requires that choice to be made before the compiler
is built, not discovered while building it. Note that the first option carries
its own completeness question — a mirror that has not caught up satisfies the
invariant while defeating the intent — and that question is open.

## Two parts, and sections the ontology names

A union of everything destroys the reason a reader opens the file. But the four
views first sketched here — about, authored, addressed, visible — are not four of
a kind. Three are relation reachability. The fourth is permission scope. So the
document splits where the kinds split:

**The master record is a personal database, not a document.** It holds everything
in `relevance(P) ∩ permission(O,C)`, complete, at full content, with no ceiling
and no catalogue. Legibility is the job of tooling over it, not of leaving things
out of it.

That separation is load-bearing. **Membership** is decided by the invariant.
**Presentation** is decided by a rendering. Deciding both at once — "it is too
big, so make that part a catalogue" — silently converts a completeness question
into a layout question, and the completeness claim is the only thing here worth
having.

So renderings are projections over the master record, and each states what it
did: the human-readable document with its overview and navigation; the contractor
subset scoped to an engagement; the PDF. A rendering may inline some things and
reference others, and it says which — but nothing is *absent from the record*
because a renderer found it inconvenient.

Two consequences worth naming. The part of your record with no relation to you —
`permission(O,C) ∖ relevance(P)` — **is** identical to that of every colleague at
your ceiling, because it is the organization's library rather than anything about
you; a rendering may reasonably show it as an index. And the record being a
database is what makes "export in any format" coherent: the formats are
renderings of one complete thing, not four divergent extracts.

The section grouping inside Part I is declared by `registry.relation_type`, not
by the compiler. A grouping written in code is a hand-maintained mirror of an
ontology, and that shape has produced four separate defects in this repository.

    authored / performed   performed_by, proposes, executes, produces,
                           generated_by, was_associated_with
    accountable for        owned_by, authorizes, accepts, governs, released_by
    assigned / addressed   assigned_to, scoped_to, raised_against
    about you              typed rows whose subject is P; evidences toward P

## How far relevance reaches

Depth is not a property of the propagating relation. It is a property of the
**anchor** — your stance toward a record decides how far your interest in it
extends. Five classes, from the 41 registered relation types:

    composition   contains, decomposes_into, baseline_contains
                  DOWN only. Assigned a project ⇒ its parts are yours.
                  Authored one task ⇏ the whole project is yours.

    version       supersedes, amends, extends
                  ALWAYS, both directions. Without it "what is current" is
                  unanswerable and a superseded record vanishes untraceably.

    provenance    derived_from, originated_from, generated_by, used, evidences
                  BACKWARD, from EVERY anchor. If a record is in your file you
                  are entitled to what it was built on.

    lateral       linked_to, depends_on, blocks, affects, conforms_to, bound_to
                  NEVER. These mean "related in the world", not "yours".
                  linked_to is symmetric — one edge leaks relevance both ways.

    authority     governs, authorizes
                  ONE HOP up. You should know the rules you are judged by; the
                  rules governing those rules are the organization's, not yours.

Composition runs the **full subtree, bounded only by clearance**. That is a
deliberate choice and it has a consequence worth stating plainly: with depth
unbounded, `∩ permission(O,C)` is the only thing limiting volume. Classification
correctness therefore governs the SIZE of a person's file, not only its
confidentiality — which makes the clearance model load-bearing in a second,
less obvious way.

Provenance runs backward from every anchor. Also deliberate, and its consequence
is that provenance chains routinely reach material concerning other people, so
withholding is a main path through this system rather than an edge case.

Only six of the 41 types are declared `acyclic` — `contains`, `decomposes_into`,
`supersedes`, `depends_on`, `amends`, `extends`. **Every provenance type is
cyclic-permitting.** A fixpoint over them must be proven to terminate, not
assumed to.

### Why maximal, and why it is a reliability argument

Both choices above are maximal, and the reason is not thoroughness for its own
sake. It is that **a narrow closure creates a second cause of absence that is
indistinguishable from the first.**

With relevance maximal, a record missing from someone's file has exactly one
explanation: their clearance did not admit it. That is a single, inspectable,
testable cause. You can point at it, reproduce it, and answer the question.

Tune the closure — decide that `performed_by` does not propagate composition, or
that provenance stops at one hop — and absence acquires a second cause: the
policy did not traverse that edge. Now "why can't I find the document I was
obviously involved in?" has two possible answers that look identical from the
outside, and distinguishing them means re-running a closure with the policy of
the day against data that has since moved. That failure is silent, it surfaces
long after the decision that caused it, and it is exactly the kind nobody can
debug.

So the maximal closure is chosen to make under-disclosure **structurally
impossible rather than merely unlikely**, and to keep the one remaining cause of
absence in a place that is already governed, already audited, and already
inspectable. The cost is volume, and volume is a tooling problem. The cost of the
alternative is an untestable class of bug in the one property this system exists
to guarantee.

This is also why the ceiling in OBL-008 governs *rendering* and never membership.
A ceiling that dropped records would reintroduce the second cause of absence
through the back door, after all this to keep it out.

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
