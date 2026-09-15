# Definitions — what each object is

The governing text is SAS §6 (the hierarchy) and §6.10 (this page's rule, in
the controlled document). This page restates it for a reader who has not opened
the SAS, and adds the mistake that made it necessary.

## The one rule

**A SAS and a Warrant are the same class of artifact at two levels of
importance.** Both are controlled contracts: an intent, a basis, deliverables,
acceptance obligations, gates, and immutable revisions that a human authorizes.
They differ in scope and in what traces to them — never in kind.

| | SAS | Warrant |
|---|---|---|
| Scope | a whole program | one bounded intervention inside a program |
| How many | exactly one per program | as many as the program's work needs |
| Traces to | the product vision | its program's SAS (`[[implements]]`, `[[roadmap]]`) |
| Its "obligations" | §106 requirements (stable ids, append-only) | acceptance obligations (OBL-nnn) |
| Its "milestones" | §98 phases — the Objectives | M1..Mn |
| Its revisions | SAS revisions, accepted by a human (§101) | contract revisions, authorized by a human (§28) |
| Its "done" | every requirement satisfied — the Release fulfilled | resolved satisfied (§56) |
| Tool | `war sas propose / accept / diff / status` | `war new / check / compile / authorize / verify / evidence / resolve` |

### Starting a program → write its SAS

A new program (a codec, a service, a lab workflow) gets **its own SAS in its
own repository**, with its own requirement prefix. Do not write a Warrant "in
the style of the OpenWarrant SAS" and treat it as the program's specification.
A Warrant with no SAS to trace to has no requirement ids to implement, no
Objective to discharge, and no Release to belong to. The projection files it
under `unassigned`, and it can never move a requirement to `satisfied`,
because there is no requirement.

### Doing work inside a program → write a Warrant

Every piece of work inside a program is a Warrant that names the SAS
requirements it realizes and the phase that motivates it. Do not write a
second SAS for a piece of work: a program with two SASs has two Release axes
and no single answer to "how far along are we".

### Not sure which?

Ask: *what would trace to this?* If other work will cite it by requirement id,
it is a SAS. If it cites requirement ids, it is a Warrant.

## The objects, one paragraph each

**Vision.** Why the system should exist. A person writes it; nothing here
compiles it.

**Release** — an accepted SAS revision. The program's contract at a named
version and digest (`docs/sas/revisions/<version>.toml`, state `accepted`).
Warrants are authorized against a specific revision and keep that Basis until
an amendment re-authorizes them (§14), so a later revision moves nothing that
was already signed. The Release axis of the corpus projection counts how many
of the revision's requirements are satisfied.

**Objective** — one §98 phase, addressed as `roadmap://<PREFIX>-PHASE-<N>`.
It has an Exit sentence in the SAS and is *achieved* when the Warrant carrying
the `exit` slug for that phase resolves satisfied. The projection says
"blocked by" and names the member Warrants below `would_satisfy`; it never
evaluates the Exit sentence itself.

**Requirement** — one §106 row, `sas://<PREFIX>-SAS-RQ-<NNN>`. Stable and
append-only: a revision may add or retitle a row and may never remove or
renumber one (§34.1, §34.4). A Warrant implements it `partial` or `complete`,
or `supersession`s an earlier claim. Its ladder is §34.3's: unaddressed →
claimed → in_progress → satisfied (a *resolved* Warrant with evidence covers
it) → superseded.

**Warrant (WAR — Work Authorization Record)** — the contract for one bounded
intervention: `docs/warrants/<alias>/` with a manifest, ordered atoms (intent,
basis, work order, milestones, assurance), and the records that accumulate
beside them (authorization, verifications, deliverables, gate-runs, judgments,
rationale, amendments, resolution, journal). Identity is the UUIDv7; the alias
is a label. A human authorizes it (§28.4), independent verifiers establish its
obligations (§46), a human resolves it (§56). An agent may draft and propose
every one of those records and may sign none of them (§27.2).

**Milestone** — `M<n>` in the milestones atom: an acceptance checkpoint inside
one Warrant, listing the stages it needs and the obligations that show it is
reached. Reached is derived from verifications; recorded reaching needs §72.6.

**Stage** — `STAGE-<nnn>`: the smallest independently dispatchable node, with
an executor kind (`agent`, `human`, `katana`, `blut`, …) and a responsibility
tier.

**Dispatch** — the compiled §47.1 packet for one stage: context manifest,
attempt basis, capabilities, normative sources, all digest-bound. `war
dispatch <alias> <stage>` produces it; a stage with no `executor_ref` is
refused.

**Deliverable / Artifact** — what the work produced, declared with a content
digest (`deliverables.toml`). Declaring is a performer statement; the digest is
recomputed, never trusted.

**Evidence** — the immutable basis for judging artifacts: §44.6 gate receipts
minted by a real run and bound to the contract digest (`gate-runs/`),
independent verification records (`verifications/`), observations. A
performer's own report is not evidence (§51.3).

**Gate** — a registered, versioned check (`docs/gates/<id>@<version>.yaml`)
with an argv, a fault model and a qualification. An obligation cites a gate;
requirement 5 of resolution asks whether that gate produced an admissible
result *against this contract*.

**Judgment / Residual risk** — §42 records: a human accepting a declared
residual risk (`judgments.toml` ↔ `rationale.toml`). A blocking unknown is not
a residual risk; it blocks resolution until its stated requirement is met.

**Amendment** — a §31 record (`amendments/AM-<nnn>.yaml`) that every contract
revision after the first must carry: the semantic diff, the reason, the
authorizer. Revision N+1 needs N of them.

**Resolution** — `resolution.toml`: the attributable conclusion about one
Warrant under one exact contract digest and assurance snapshot, signed by a
human resolver. `satisfied` is accepted only when every declared obligation is
established by admissible verification (§38.6). A second resolution is refused;
§56.4 dispute and §56.5 annulment change one.

**Journal** — `journal.jsonl`: one §66.3 event per line, appended only by the
commands that change records, never by hand, checked append-only against the
committed baseline. Phase is *read* from it.

**ADR** — an architecture decision record (`docs/adr/`). A SAS revision that
changes §106 requires one (§101.3); a Warrant cites the ADRs that govern it.

## Where to look

- Live ladder: `docs/warrants/generated/CORPUS_STATUS.md` (and the published
  page).
- Per-Warrant: `war status <alias>`, `war journal <alias>`.
- Program: `war sas status`.
