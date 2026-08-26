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
already knows what a person is permitted to see:
`core.set_access_context(p_organization uuid, p_max_classification text)` sets the
access context and the row-level policies on 76 tables do the rest.

So the master document has an invariant, not a specification. It has three
layers, because permission, entitlement and relevance are different kinds of
thing and conflating any two of them is what makes this claim slippery.

**Relevance is a graph property.** `org.person.id` references `core.object(id)`,
so a person is a node. `core.relation` is typed, stateful and time-bounded. What
concerns you is what the graph connects to your node.

**Permission is a lattice property.** `core.set_access_context` takes
`(p_organization uuid, p_max_classification text)` and sets `kf.organization` and
`kf.max_classification`. No policy in `row_security.sql` reads an actor or a
person. RLS admits by tenant and ceiling, not by identity.

    permission(O, C)  = the set RLS admits at organization O, ceiling C
    exclusions(P, T)  = reasoned, authorized, time-bounded withholdings
    permitted(P, T)   = permission(O, C) ∖ exclusions(P, T)
    relevance(P, T)   = ⋃ over anchors a of P: closure(target(a), propagation[a])

    masterRecord(P, O, C, T) == permitted(P, T)

**Membership is permission minus reasoned exclusions. Relevance sections it.
They are never the same mechanism.**

RLS alone cannot express "yours" — it admits by tenant and ceiling, so colleagues
at one ceiling would otherwise hold identical records. `permitted(P, T)` is the
person-level layer that makes a record genuinely someone's, and it sits **above**
RLS rather than inside it: no per-identity policy across 76 tables, and none of
the scaling problems per-identity row security brings.

**Entitlement is subtractive, never granted.** This is the load-bearing choice. A
grant-based layer fails by omission — forget to grant and the record is simply
invisible, with nothing anywhere recording why, which is precisely the untestable
bug this design exists to prevent. A subtractive layer cannot fail that way:
absence always has a row behind it carrying a reason, an authorizer and a time.
Default-open, and every subtraction explains itself.

`core.retention_hold` is the shape to copy — `object_id`, `reason` not null,
`placed_by`, `placed_at`, `released_at`, append-only — extended with the subject
it withholds from, and a reason class so withholding can be reported differently
depending on why: holds enumerated per item, third-party material as a bare
count.

That is the sentence at the top of this atom, read literally: *or haven't been
given permission*. The boundary of the file is what you are permitted to see —
not what someone judged to concern you. So the record contains everything
`permitted(P, T)` admits, and `relevance(P, T)` decides where in the record each
thing appears, and what a derived subset may be scoped to.

An earlier draft made membership the intersection `relevance ∩ permission`. That
was wrong twice over. It contradicted itself two paragraphs later by describing
material outside the intersection as something a rendering could index, which is
impossible if it was never in the record. And more importantly it handed the
relevance policy the power to make records disappear.

Separating them buys the property this whole design exists for. **No absence is
ever silent.** A mistake in the relevance closure can only put a record in the
wrong section, where it is still present, still searchable, still there to be
found — it cannot cause a disappearance. And the two things that genuinely can
remove a record both account for themselves: an exclusion writes a row carrying
its reason and appears in the withheld ledger, and clearance is the single
remaining cause, inspectable and reproducible.

So the untestable bug — somebody cannot find a document they were plainly
implicated in, and nobody can say why — has nowhere left to live. Every "why is
this not here" resolves to a named section, a named exclusion, or a ceiling.

Note what this fixes. Under RLS alone, two people in one organization at one
ceiling would hold identical records, and only the sectioning would differ —
which makes "your record" a courtesy rather than a fact. The entitlement layer is
what makes the difference real, and it does so subtractively, so the fact costs
nothing in findability.

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

## One record, sections the ontology names

The four views first sketched here — about, authored, addressed, visible — are
not four of a kind. Three are relation reachability; the fourth is permission
scope. That mismatch is what earlier tempted this design into filtering
membership by relevance. The resolution is not to split the record but to stop
asking one mechanism to do both jobs.

**The master record is a personal database, not a document.** It holds everything
`permitted(P,T)` admits, complete, at full content, with no ceiling and no
catalogue. Legibility is the job of tooling over it, not of leaving things out of
it.

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

So the sections follow relevance while membership follows entitlement, and the
record splits into two first-class surfaces:

**Your record** — `relevance(P,T) ∩ permitted(P,T)`. What the graph says concerns
you, at full content.

**The org view** — `permitted(P,T) ∖ relevance(P,T)`. Everything you may see that
has no relation to you: the organization's shared and public material from where
you stand. This is a **view in its own right**, not a leftover. It is browsable
and searchable, and because every member of it is already permitted to you,
**opening anything in it needs no further authorization** — the decision was made
when it entered `permitted`, not when you clicked.

That property is worth stating because it is what makes the org view usable
rather than a wall of locked titles. A rendering may present it as an index that
resolves to full content on demand; there is no second gate behind it.

The record being a database is what makes "export in any format" coherent: the
formats are renderings of one complete thing, not divergent extracts.

### Four states, and only one of them is invisible

For any object in the corpus, from a given person's standpoint:

    in your record       permitted and relevant           full content
    in the org view      permitted, not relevant          browsable, opens freely
    in the withheld      excluded, with a reason          named or counted by class
      ledger
    absent               permission never admitted it     one cause: tenant/ceiling

Three of the four are **visible**. Only the last is not, and it has exactly one
explanation, which is inspectable and reproducible. That is a stronger guarantee
than the earlier draft reached: it keeps records genuinely personal while leaving
nothing silently missing, because a record that is withheld says so and a record
that is merely not-about-you is still right there in the org view.

The section grouping is declared by `registry.relation_type`, not by the
compiler. A grouping written in code is a hand-maintained mirror of an
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

Both choices above are maximal. Since membership is permission, relevance can no
longer make anything disappear — so maximalism here is not what keeps records
present. It is what keeps them **findable**, and that is worth being precise
about, because the two arguments are easy to confuse.

Under-disclosure is already structurally impossible: absence has exactly one
cause, clearance, by construction. What a narrow relevance closure costs instead
is that records land in the organization's library rather than in your record —
present, searchable, but filed as though they had nothing to do with you. For
someone looking for the document they were plainly implicated in, "it is in the
file, under a heading suggesting it is not yours" is a real failure, just a
recoverable one rather than an invisible one.

So relevance is tuned maximal to make the sectioning generous: if there is a
defensible reading under which a record concerns you, it appears under your
record rather than the library. The cost is that your sections are large, and
large sections are a tooling problem. The benefit is that the answer to "why is
this filed as not mine" is always inspectable — you can walk the relation path
and see it, or see that none exists.

This is also why the ceiling in OBL-008 governs *rendering* and never membership.
A ceiling that dropped records would put absence back in the hands of policy,
after all this to keep it in the hands of clearance alone.

## Two consequences that are easy to miss

**Staleness is incorrectness, not age.** The invariant is stated at time `T`. A
document that no longer equals the permission set is *wrong* — it asserts a
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
