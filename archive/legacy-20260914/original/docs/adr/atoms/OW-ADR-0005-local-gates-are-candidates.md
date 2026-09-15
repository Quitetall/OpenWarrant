---
schema: oh.war/atom/v1
adr_uuid: 60fb5c27-78ce-440b-8a5e-125a57f38bb2
local_alias: OW-ADR-0005
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
decided: 2026-08-19
governs:
  - "war://OW-WAR-0019"
---

# ADR OW-0005: A local gate is a candidate, and a gate is not a string

## Status

`Accepted 2026-08-19`

Governs OW-WAR-0019. Answers the blocking unknown its Basis raised: whether a
local gate registry is authoritative in the interim, or explicitly provisional.

## Context

Two questions had to be settled before gates could exist here, and only one of
them was the one the Warrant asked.

**The declared unknown.** §43.1 gives Knowledge Fabric the authoritative
institutional Gate Registry and gives OpenWarrant schemas, local candidate
authoring, CLI inspection and binding, and cached projections. Knowledge Fabric
integration is OW-WAR-0028 and does not exist. So gates authored here today are
in a registry that is not the registry.

**The undeclared one, found during implementation.** The first version of the
citation check searched each assurance atom for the substring `gate://`. Run
against the real corpus it immediately flagged OW-WAR-0019's own OBL-003, whose
evidence line reads "a plant citing `gate://does-not-exist`, refused by name" —
a sentence describing a plant, not a citation of a gate.

That was not a tuning problem. OW-WAR-0019's Intent exists because the parent
project declared 94 gates of which 23 named a tool, script, or crate that was not
in the tree. Those were strings in prose that nothing resolved. A checker that
identifies gates by pattern-matching prose is the same object as the defect it
was written to detect.

## Decision

### 1. A local gate is a candidate, and the record says which

§43.1's answer did not need inventing; it is one sentence long:

> Repositories may hold local candidates. A candidate is not a qualified
> institutional gate.

So the local registry is permitted and is **not** authoritative. Provenance is a
required, defaulted field on every Gate Definition — `local_candidate` or
`institutional_projection` — and it is rendered wherever a gate is reported. A
gate authored here reads "local candidate" in `war check` output, always.

`institutional_projection` is representable now, before anything can produce one,
so that the local case is never the only case the schema can express. A schema
that can only say one thing does not distinguish anything.

### 2. A gate is cited through a declared field, never through prose

An obligation cites a gate with a `- **gate:**` bullet, in the same form the
surrounding obligations already use for `- **scope:**` and `- **evidence:**`.
Prose that merely contains a `gate://` URI is prose. §43.5 makes a binding an
object with a subject, a pinned digest, and an evidence policy — not a phrase
someone wrote.

### 3. Qualification requires controls in both directions

§43.4 lists positive controls, negative controls, mutation classes,
environments, detection results, limitations, qualifier, and a digest. All are
required, and two rules are enforced beyond mere presence:

- every fault class in `fault_model` must have a detection result recording
  `detected: true` — a declared fault class with no result is refused, and a
  genuine gap belongs in `known_blind_spots`, stated;
- both control directions must be present. A gate qualified only against bad
  input may flag everything; a gate qualified only against good input may flag
  nothing.

## Rationale

**Recording absence is not the same as having the thing.** This is OW-ADR-0004's
principle applied to a second object. `local candidate` does not make a local
gate institutional. It makes the difference legible, so that OW-WAR-0028 can
promote deliberately rather than discovering that fifteen local gates have been
cited for months as though they were qualified.

**The prose scan is worth naming, not just fixing.** It was written by someone
holding the exact requirement it violated, and it passed its unit test — which
used prose as the fixture, so the test encoded the bug. Only the real corpus
exposed it. That is the third time in this repository a check has looked correct
until it met real data, and it is the argument for running every new rule against
the whole corpus before believing it.

**Both control directions, because this project has been burned by one.** The
parent project shipped a green gate that compared nothing three times. A
qualification record with positive controls only cannot distinguish a working
gate from one that fires on everything; with negative controls only, it cannot
distinguish one from a gate that never fires at all.

## Alternatives Considered

- **Treat the local registry as authoritative until KF exists.** Rejected: it
  contradicts §43.1 in plain words, and the state would have to be un-claimed
  later, after citations had accumulated against it.
- **Refuse to author gates until OW-WAR-0028.** Rejected: it would leave every
  obligation citing prose, which is the status quo this Warrant exists to end,
  and §43.1 explicitly permits local candidates.
- **Keep the prose scan and reword the Warrant that tripped it.** Rejected, and
  it was tempting because it was a two-line edit. It would have made the corpus
  fit the checker instead of the checker fit the specification, and the next
  Warrant to describe a gate in prose would have hit it again.
- **Let a lifecycle of `draft` be bindable, with a warning.** Rejected: §43.4
  says unqualified cannot be bound. A warning that does not block is how an
  unqualified gate ends up cited in a resolution.

## Consequences

**Good.** Gates resolve or they fail by name. Local candidates cannot be mistaken
for institutional ones. A qualification that cannot fail in one direction is
refused.

**Bad.** Authoring a gate is now substantially more work than typing a command
into an obligation, and that is the intended cost. Some real checks will go
unrecorded for a while because nobody wants to write the qualification record;
that pressure is preferable to the alternative, but it is real and should be
watched.

**Unchanged.** §43.1's ownership split stands. When OW-WAR-0028 lands, promotion
is a provenance change on an existing object rather than a migration.

## Validation

Watch for: `local_candidate` being dropped from rendered output "because they are
all local anyway", which is exactly when the field starts to matter; a
qualification record whose negative control is a restatement of its positive one;
`known_blind_spots` left empty on a gate whose `fault_model` is short, which
usually means the blind spots were not looked for; and any reintroduction of
prose scanning as a convenience.
