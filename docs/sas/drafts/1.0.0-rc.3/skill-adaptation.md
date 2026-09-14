# War skills: adapted methods, standard artifacts

Normative companion to SAS RC.3 §16. OpenWarrant maintains its own workflow-adapted
suite based on Matt Pocock's skill library. This adopts and adapts the methods;
it does not transfer Matt Pocock's authorship or claim his endorsement.

## Existing source and intended scope

The repository already records adaptations in [docs/SKILLS.md](../../../SKILLS.md)
and `.claude/skills/war-*`. Their declared upstream is
[mattpocock/skills at 3cca18b](https://github.com/mattpocock/skills/tree/3cca18b).
The upstream [MIT license](https://github.com/mattpocock/skills/blob/3cca18b/LICENSE)
names Copyright (c) 2026 Matt Pocock and requires retaining its notice with copies
or substantial portions. Preserve that attribution/license in distributed adapted
material. Keep OpenWarrant changes attributable separately. This document does not
claim the entire upstream library has already been imported or tested.

Each adopted skill SHALL record its upstream path and revision, local adaptation
revision, license/notice location and changed output contract. Existing short SHA
provenance is historical; future updates should pin a full immutable revision and
review differences before adoption. No automatic upstream update changes local
workflow authority or artifact meaning. Verify distribution notices during release
qualification rather than treating a source comment as a packaging audit.

## Invocation and output contracts

| Invocation/method | OpenWarrant result | Observable completion |
| --- | --- | --- |
| `war-grill` / grilling | Durable questions, answers, unresolved decisions, then a Warrant draft/proposal when outcome is bounded | Settled decisions retained; remaining blockers explicit; next artifact passes selected SDK validation |
| `war-spec` / spec or PRD synthesis | Warrant document/proposal, linked ADRs for material decisions and fixture references | Scope/outcome/constraints and evidence plan are present; no invented answer or approval |
| `war-tickets` / decomposition | Warrant stage/milestone plan; child Warrants only for independently reviewable outcomes | Dependencies explicit; no unnecessary split into one Warrant per implementation file |
| `war-map` / wayfinding | Decision-oriented Warrant with linked questions, options and resolution criteria | Every known decision branch accounted for; unresolved meaning remains visible |
| `war-review` / code review | Separate standards findings and independent obligation findings with evidence/subject bindings | Findings preserve scope, exact candidate, UNKNOWN/FAIL and reviewer independence |
| Domain modeling / ADR method | Context definitions and supported ADR documents linked to the affected Warrant | Terms agree with selected edition; proposed architecture does not claim acceptance |
| TDD / handoff method | Reviewed check fixtures and exact task/context references or provider packet | Refusal/control case exists; handoff identifies required outputs, evidence and limits |

The first five war names exist today under the legacy contract. Rows describing
methods are adaptation targets, not invented installed commands. Invocation syntax
(`/war-grill`, harness skill invocation, or future CLI routing) is transport-specific.
The portable contract is the task and artifact result, not a universal slash parser.

For ungoverned discovery, a skill may first collect structured questions and make
a minimal draft. It SHALL NOT force a fabricated Warrant outcome before the user
has defined one. When a Warrant is the intended outcome, it persists that artifact
rather than ending only in chat, a generic plan, an issue or an HTML report. Supported
ADR/context documents remain their own kinds, linked to the work; not every artifact
is renamed Warrant. Tracker items and HTML may accompany the canonical record as
views. Adapters preserve identity and version links back to the source.

## Predictable process and economical context

1. Resolve the applicable standard/profile, current record, user intent and
   effective policy. Read only the governing sources required for this branch.
   Completion: exact basis and task boundary are explicit.
2. Reuse settled session decisions and inspect facts in the repository. Ask only
   unresolved decisions that materially affect the result. Completion: no invented
   answer, implicit architecture change or unnecessary repeated approval.
3. Produce the declared typed artifact through SDK/CLI helpers. Completion: local
   validation reports actual validity and remaining readiness/authority limitations.
4. Report artifact location/identity and what was performed versus proposed.
   Completion: another consumer can read it without reconstructing chat history.

Keep common steps in SKILL.md. Put branch-specific templates, grammar and examples
behind explicit pointers saying when they must be read. Required rules and their
definitions/exceptions travel together. SDK/profile definitions are a single source
of truth; skills link to them instead of maintaining independent validators.
Use rich trigger descriptions only when autonomous invocation is needed. A router
may help users discover manual skills. Harness-specific invocation flags must be
tested in the supported harness; they are not OpenWarrant standard metadata.

## Integration with existing context documents

OpenWarrant integrations adopting this profile SHALL work with the repository's
existing `AGENTS.md`, `CLAUDE.md`, `CONTEXT.md` and other configured context sources.
Those files retain their native purpose, ownership and directory scope. They need
not become OpenWarrant documents, contain its metadata or move to its store.
An OpenWarrant-compatible Warrant remains valid without installing a harness hook
or using one of those filenames.

The host harness owns instruction discovery and precedence. An integration SHALL
preserve that hierarchy, including nested directory rules; OpenWarrant supplies
work contracts and evidence, not a competing instruction-precedence system.
Retrieving a document, following a context pointer or naming a file `AGENTS.md`
does not by itself establish permission, policy or human acceptance. Conflicting
applicable requirements remain visible; the responsible decision-maker settles
the affected scope. An adapter cannot silently pick whichever rule is convenient.

Keep one source for shared OpenWarrant guidance. Integrations SHALL add a small
conditional pointer or a supported include where the host loads instructions.
The pointer names both the target and the work that requires it. Preserve unrelated
content and user changes. Repeated setup SHALL avoid duplicate entries; removal
SHALL affect only the unchanged entry owned by that integration. If the entry was
edited, report the conflict and leave the file intact. Preserve symlinks and their
targets; a link to a shared/global instructions file is not permission to change
that file. Resolve and show the actual write target before any requested edit.

For example, a `CLAUDE.md` in this repository can reference its shared guidance:

```markdown
For OpenWarrant work, read [AGENTS.md](AGENTS.md), then follow its task-specific
context pointers. Keep applying the other instructions in this file.
```

This example is an entry pointer, not a request to create a duplicate file or
rewrite an existing one. Adopters choose the target holding their shared guidance;
they need not adopt this repository's file layout. Merely pointing two files at
each other without a source containing the required guidance is not integration.

Context providers SHALL represent native documents as source references with exact
revision/content identity, applicable scope, and provenance. The caller/host supplies
their instruction role and effective scope; the compiler does not infer authority
from prose or filenames. Required applicable rules and their dependencies remain
exact in the agent packet. Background can be summarized under the existing context
contract. Already supplied context may be reused only when its exact identity and
coverage are established; a path alone does not prove delivery. Missing required
content remains a coverage gap and blocks affected dispatch, not independent work.

The SDK represents and validates supplied references and records. Providers resolve
and project native sources; harness adapters load context and apply host rules.
No SDK filesystem scan, database, model call or new authority follows from this
integration. Work-change rules still govern a changed required instruction during
execution; see the [stop contract](work-stop-contract.md).

Phase 1 S08/SDK-07 SHALL include fixed context-entry edit proposals: retain existing
host text, choose an explicit shared source, reuse the same pointer on repeat, and
retain nested scope/source identity. Refusal cases include a modified owned entry,
an unapproved symlink target, a missing required source and conflicting applicable
instructions. Test byte preservation for unchanged content. These are direct
artifact/adapter fixtures; Phase 3 observes actual harness loading and installation.
The current legacy `war agents-md --force` replaces a whole file and does not
implement this additive contract. Its template stays a legacy workflow reference
until separately scoped migration; current repository guidance may link that
reference beside project-specific and successor-design instructions.

## Authority and evaluation

For prompt-only work without an explicit Warrant action gate, skills SHALL proceed
without requiring prior OpenWarrant authorization, start approval or signing.
They report the actual work as unverified and retain unresolved qualification
conditions for later review. A completion signature is optional. Explicitly chosen
professional workflows may run pre-work checks. At release, skills can assemble
the evidence and exact review manifest for one human/agent review effort and
human signing ceremony. Every signature retains its actual actor identity.
Qualification always requires SAS §12 conditions, evidence and secure human
acceptance. A skill cannot satisfy its own independent gate or expand its policy.

Explicit Warrant gates can require verified preparation or signoff before a named
action. Skills must honor those gates rather than treating unverified mode as a
bypass. Ordinary work without a gate retains the prompt-only path.

Every completed work stop returns the exact configured safeword on its first line,
then concise pointers to the deterministic progress overview, implementation notes,
generated document trail and next steps. Scope distinguishes a completed feature
from a completed Warrant. Configurable brevity reduces chat tokens; it must not
replace recorded tracker facts with an agent-written status estimate. The full
overview is not pasted by default. Drafting/planning only signals completion of
its own declared unit, never completion of unperformed implementation. A harness
interruption is not a work-stop completion. A subsequent prompt starts the next
work subject to its explicit gates, without inventing a universal signing step.
Existing legacy skill text saying "a skill never signs" remains current runtime
guidance until a scoped SDK/workflow migration is implemented. This draft does not
silently loosen installed hooks or human-key handling.

Test each adopted invocation on fixed positive, incomplete and adversarial cases:
correct artifact kind, required fields/context, settled-answer reuse, refusal of
fabricated authority and truthful completion. Measure dropped constraints, wrong
acts, extra questions, tokens and human effort against the prior adaptation.
Include a prototype with missing OpenWarrant approvals: the skill must not ask for
a signature before working, and must not award a mark after its own completion.
Calling methods well-tuned is an intent; measured results determine quality claims.
