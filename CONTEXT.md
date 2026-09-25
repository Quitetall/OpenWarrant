# OpenWarrant

Work Authorization Records for agents: a program's contract (the SAS) and
the bounded interventions inside it (Warrants), each authorized, executed,
verified and resolved on evidence a tool can check. This glossary is the
shared language; `docs/DEFINITIONS.md` carries the one rule in prose and the
SAS (`docs/sas/`) is the controlled document. Every Dispatch carries this
file, so a term defined here costs one word everywhere else.

Format after mattpocock/skills `domain-modeling/CONTEXT-FORMAT.md` (MIT).

## Language

**Warrant**:
The contract for one bounded intervention inside a program: intent, basis,
work order, milestones, obligations; authorized and resolved by a human.
_Avoid_: WAR (except as the acronym in `oh.war/*`), ticket, task, issue

**SAS**:
The program's contract, a Warrant at program scale: its §106 requirements are
its obligations, its §98 phases its milestones, its accepted revisions its
authorized revisions. One per program.
_Avoid_: spec, PRD, architecture doc

**Atom**:
One authored source file of a Warrant (intent, basis, work order,
milestones, assurance), composed into the generated parent.
_Avoid_: section, chapter, doc

**Contract Revision**:
The compiled, digested form of a Warrant's atoms at one moment; an
authorization binds exactly one and it never changes afterwards.
_Avoid_: version, snapshot

**Authorization**:
A human's signed acceptance of a Contract Revision. Until it exists nothing
may be performed under the Warrant.
_Avoid_: approval, sign-off, LGTM

**Provisional approval**:
A human's recorded decision that is not yet a signature; recorded as a
blocking unknown whose resolution is the signature.
_Avoid_: pre-approval, verbal okay

**Obligation**:
One acceptance condition (`OBL-nnn`) with a scope, a gate and the evidence
that establishes it.
_Avoid_: acceptance criterion, requirement (that word is the SAS's)

**Gate**:
A registered, versioned check (`gate://<id>@<version>`) that runs and mints a
receipt; askable or not, never a string in prose.
_Avoid_: test suite, CI job, check

**Receipt**:
The sealed record of one gate run against one subject digest; the evidence a
resolution reads.
_Avoid_: log, output, result

**Verification**:
An independent actor's disposition of an obligation (established, not
established, refuted) over a bundle, never the performer's own word.
_Avoid_: review, QA, validation

**Blind verifier**:
A verifier that sees only the bundle: no transcript, no rationale, no
workspace of the performer.
_Avoid_: reviewer, second opinion

**Resolution**:
A human's signed outcome for a Warrant against the exact authorized
revision, permitted only when the thirteen requirements hold.
_Avoid_: close, done, merged

**Correction**:
The recorded act by which a resolved Warrant's pinned file moves: reason,
kind, superseded digest, new digest, a human's signature. The only way past
the wall.
_Avoid_: fix, patch, re-pin (that is for unresolved Warrants)

**The wall**:
The set of files pinned by resolved Warrants; edited only through a
Correction.
_Avoid_: frozen files, locked files

**Pin**:
The content digest a Warrant's `deliverables.toml` records for a file.
_Avoid_: hash, checksum

**Stage**:
One dispatchable unit of work inside a milestone, with an executor kind
(human, agent, service) and a context declaration.
_Avoid_: task, step, ticket

**Milestone**:
An acceptance checkpoint grouping stages and the obligations that close it;
`depends_on` gives the blocking edges.
_Avoid_: epic, phase (that word is the SAS's), sprint

**Frontier**:
The stages whose milestone's dependencies are complete and that nobody has
dispatched: what can start now. `war frontier` lists it.
_Avoid_: backlog, todo, ready queue

**Dispatch**:
The compiled context packet for one stage: exact basis, selected context,
token estimate and budget, digest. The unit of agent context and the
handoff document.
_Avoid_: prompt, brief, handoff doc

**Submission**:
What a performed stage hands back: claims, observations, a requested next
action that is never its own resolution.
_Avoid_: PR description, report

**Blocking unknown**:
A recorded assumption whose resolution requirement must be met before the
Warrant can resolve; the fog on a map.
_Avoid_: open question, TBD, risk

**Projection**:
A generated, drift-checked file compiled from records (`CORPUS_STATUS`,
`NORMATIVE`, `WAR.json`); never edited by hand.
_Avoid_: report, dashboard data, export (that word is `war export`'s)

**Attestation**:
The in-toto Statement in a DSSE envelope that every ssh-signed act leaves
beside its record.
_Avoid_: signature (that is the `.sig` sidecar), certificate

**Plant**:
A deliberate violation placed in the tree so the battery can prove the tool
refuses it by name; the red before the green.
_Avoid_: test case, fixture (a fixture is the positive input)

**Battery**:
`conformance/plant.sh` and `plants.d/`: every plant run against the shipped
binary.
_Avoid_: test suite, integration tests

**Drafter**:
The separate process that answers a draft request with a proposal over the
§75.2 seam; writes no file.
_Avoid_: planner agent, model, bot

**Proposal**:
An `oh.war/draft-proposal/v2`: operations with payloads that `war plan
--apply` turns into a Warrant after the §74.4 gauntlet and a human review.
_Avoid_: plan, draft (the draft is the unauthorized Warrant)
