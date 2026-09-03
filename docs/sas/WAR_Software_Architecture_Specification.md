# WAR Software Architecture Specification

## OpenWarrant: Work Authorization Records for machine-parsable, agent-executable, evidence-backed work

| Field | Value |
|---|---|
| Document class | Software Architecture Specification |
| Short name | WAR SAS |
| Status | Draft for adoption |
| Version | `0.1.0-draft.3` |
| Date | 2026-08-19 |
| Enterprise identifier | Unallocated — this file name is not an official Identifier Registry allocation |
| System name | **OpenWarrant** |
| Record name | **Warrant** |
| Formal expansion | **WAR — Work Authorization Record** |
| Repository name | `OpenWarrant` |
| CLI binary | `war` |
| Canonical protocol family | `oh.war/*` |
| Initial optimization target | Software engineering and agentic programming |
| Eventual scope | Any bounded institutional work |
| Institutional authority | OpenHuman Knowledge Fabric |
| Document and context substrate | Liminal |
| Agent runtime | Katana |
| Typed computational runtime | BLUT |
| Gate authority | Knowledge Fabric Gate Registry |
| Canonical portable representation | RFC 8785 canonical JSON |
| Normal human representation | Generated Markdown |
| Source model | Ordered, typed, independently authoritative atoms |
| Parent editing rule | Generated parent documents are never directly edited |

> **Normative summary.** A WAR is one logical, machine-readable document that authorizes a bounded undertaking, compiles the exact context and authority required to perform it, tracks milestones and execution, separates empirical evidence from inference and judgment, and closes through an attributable resolution. The human reads one Warrant. The system preserves the distinct authority of every atom, external record, artifact, runtime receipt, and gate observation from which that Warrant is compiled.

---

# Part I — Constitutional architecture

## 1. Purpose

OpenHuman needs a work primitive that is simultaneously:

- understandable as one document;
- authored as small, independently editable sources of truth;
- fully machine parsable;
- suitable for a stateless agent;
- usable by a human, contractor, service, laboratory, or orchestration engine;
- capable of recording decisions, work instructions, milestones, attempts, blockers, evidence, judgments, and resolution;
- federated across repositories and subsystems without creating competing authorities;
- rigorous enough for high-assurance work but cheap enough for ordinary engineering.

Existing Architecture Decision Records solve only one portion of this problem. Work orders solve another. Issue trackers, model transcripts, CI results, experiment ledgers, contractor acceptance, and generated documentation solve still others. Putting all of those meanings into one mutable Markdown file produces ambiguity: a progress note can silently amend a contract; a performer-authored report can masquerade as evidence; a declared command can look like a passing gate without ever having executed; and a generated status can become indistinguishable from authored truth.

The WAR architecture resolves this by making the **Warrant one logical document and one compiled product**, while preserving multiple independently governed source atoms and receipts underneath it.

## 2. Scope

### 2.1 Initial scope

Version 1 is optimized for:

- software implementation;
- architecture and engineering decisions;
- agentic coding;
- repository-scoped execution;
- reproducible tests and builds;
- typed computational work executed through BLUT;
- context compilation and agent execution through Liminal and Katana;
- Knowledge Fabric registration, authorization, and resolution.

### 2.2 Eventual scope

The architecture SHALL support additional profiles for:

- hardware engineering;
- laboratory testing;
- scientific experiments;
- investigations;
- reviews;
- remediation;
- controlled operations;
- contractor work;
- quality and regulatory evidence;
- other bounded institutional work.

The initial implementation MAY omit these profiles. The core identity, composition, authority, evidence, and resolution model SHALL NOT require redesign when they are added.

### 2.3 Out of scope

OpenWarrant is not:

- a replacement for Knowledge Fabric;
- a second organizational database;
- an agent harness;
- a model provider;
- a generic workflow engine;
- a replacement for Git, CAD, issue trackers, laboratory systems, or financial systems;
- a replacement for Liminal document semantics;
- a replacement for Katana runtime logging and capability enforcement;
- a replacement for BLUT typed pipeline execution;
- a legal contract engine in version 1;
- a claim that executable gates can prove arbitrary semantic correctness.

## 3. Normative language

The terms **MUST**, **MUST NOT**, **SHALL**, and **SHALL NOT** state requirements.

The terms **SHOULD** and **SHOULD NOT** state strong recommendations. A deviation requires an attributable reason.

The term **MAY** states a permitted option.

A conforming implementation may deliver capabilities incrementally, but it SHALL preserve the ownership boundaries, identifier semantics, immutable revision rules, evidence distinctions, and compilation contracts defined here.

## 4. Naming

### 4.1 Formal and human names

The formal record type is:

> **WAR — Work Authorization Record**

The normal human noun is:

> **Warrant**

Examples:

- “Open Warrant 42.”
- “This Warrant implements SAS requirement 31.”
- “Compile the Warrant for Katana.”
- “WAR-0042 is blocked.”
- “The evidence warrants resolution as satisfied.”

The dual meaning is intentional: a Warrant both **authorizes action** and accumulates the basis that may later **warrant a conclusion**.

### 4.2 System names

| Concept | Name |
|---|---|
| Repository and subsystem | `OpenWarrant` |
| CLI | `war` |
| Rust crate prefix | `openwarrant-*` |
| Machine kind | `work_authorization_record` |
| API family | `oh.war/*` |
| Human record | Warrant |
| Formal acronym | WAR |

### 4.3 ADR is preserved

**ADR** continues to mean **Architecture Decision Record**.

OpenWarrant SHALL NOT redefine ADR. Every normative decision represented in the WAR system SHALL be a first-class ADR with its own identity, lifecycle, source atom, and global or provisional registry entry.

A Warrant may transclude zero or more ADRs into its generated Decision section. ADRs also compile into a major generated ADR Overview for auditability.

### 4.4 Work Order is preserved

A Work Order is the implementation projection of a Warrant.

In version 1, a technical Work Order generated from a WAR is not automatically the authoritative contractor, financial, or legal Work Order in Knowledge Fabric. The Warrant links to those records when they exist.

The WAR structure SHALL be designed so a future contractor-work profile can use the same technical composition without redefining the core protocol.

## 5. Architectural thesis

> **A Warrant is one semantic work object compiled from ordered atoms and authoritative bindings into multiple role-specific projections.**

The full human Warrant is one parent document. Its underlying atoms are the editable sources of truth. Bound records are edited only through their owning systems. Generated sections are projections and are never directly edited.

The system therefore behaves like a compiler:

```text
human request / project requirement
               │
               ▼
      authored and bound atoms
               │
               ▼
      immutable compilation basis
               │
               ▼
         canonical WAR IR
     ┌─────────┼───────────┬──────────────┐
     ▼         ▼           ▼              ▼
 Full WAR   Work Order   ADR section   Assurance Case
     │         │           │              │
     └─────────┴─────┬─────┴──────────────┘
                     ▼
               Stage Dispatch
                     │
                     ▼
            actor / Katana / BLUT
                     │
                     ▼
       artifacts + receipts + evidence
                     │
                     ▼
                 Resolution
```

The parent document is a view. The semantic graph and its governed source atoms are the meaning.

## 6. System hierarchy

WAR sits inside a larger hierarchy of intent and realization.

```text
Product or system vision
          │
          ├───────────────┐
          ▼               ▼
Software Architecture   Production Roadmap
Specification (SAS)     sequence and priority
          │               │
          └───────┬───────┘
                  ▼
             WAR / Warrant
                  │
             Milestones
                  │
                Stages
                  │
              Dispatches
                  │
       Artifacts and Evidence
                  │
              Resolution
```

### 6.1 Product or system vision

The vision states why the system should exist and the durable product thesis.

### 6.2 Software Architecture Specification

The **SAS** states what the software is and shall become:

- architecture;
- invariants;
- capabilities;
- boundaries;
- interfaces;
- requirements;
- non-goals;
- correctness conditions.

SAS requirements SHALL have stable identifiers so WARs can reference them.

### 6.3 Production Roadmap

The Production Roadmap states sequencing, dependency, priority, and phase.

The Roadmap is separate from the SAS. Scheduling changes SHALL NOT require architecture changes unless the underlying architecture itself changes.

### 6.4 WAR

A WAR authorizes one bounded intervention that realizes, validates, modifies, or investigates the architecture and roadmap.

Every WAR SHOULD reference:

- the SAS requirements it realizes or affects;
- the Roadmap item or phase that motivates it;
- governing ADRs;
- parent or superseding WARs;
- target repository or subsystem.

### 6.5 Milestone

A milestone is a meaningful acceptance checkpoint inside a WAR.

A milestone may require multiple stages. A milestone is not necessarily independently dispatchable.

### 6.6 Stage

A stage is the smallest independently dispatchable execution node.

Each stage has:

- typed inputs;
- typed outputs;
- context;
- capabilities;
- resources;
- stop conditions;
- submission contract;
- associated obligations.

### 6.7 Dispatch

A Dispatch is the exact stage-specific packet given to one actor or runtime.

### 6.8 Artifact and evidence

Artifacts are the subjects produced or changed.

Evidence is the immutable basis used to observe or evaluate those subjects.

### 6.9 Resolution

Resolution is the attributable organizational conclusion about the Warrant under one exact contract and assurance basis.

### 6.10 The levels, and the one rule about SAS and Warrant

Every level above is a named kind of object. This subsection fixes what each
one IS, so that no reader has to infer it from the diagram, and states the one
rule that has been misread in practice.

| Level | Object | What it is | Written by | Governed by | Read by |
|---|---|---|---|---|---|
| Vision | product or system vision | why the system should exist; not a record this system compiles | a person | nothing here | everyone |
| **Release** | an **accepted SAS revision** (`docs/sas/revisions/<version>.toml`) | the contract for a WHOLE PROGRAM: what the software is and shall become, with stable requirement ids (§106) and phased Objectives (§98) | a person or agent proposes; a human accepts (§101.2) | §101 | `war sas`, the corpus projection's Release axis |
| **Objective** | a §98 phase, `roadmap://<PREFIX>-PHASE-<N>` | a stage of the Roadmap with an Exit sentence; achieved when its `exit`-slugged Warrant resolves satisfied | the SAS | §98 | the corpus projection's Objective axis |
| **Requirement** | a §106 row, `sas://<PREFIX>-SAS-RQ-<NNN>` | one stable, append-only architectural requirement; a Warrant implements it partially or completely | the SAS | §34, §101 | `war check`, the requirement ladder |
| **Warrant** | a WAR (`docs/warrants/<alias>/`) | the contract for ONE BOUNDED INTERVENTION inside a program: intent, basis, work order, milestones, obligations; authorized, executed, verified, resolved | a person or agent drafts; a human authorizes (§28.4) and resolves (§56) | §16–§56 | `war` |
| Milestone | `M<n>` in a Warrant's milestones atom | an acceptance checkpoint inside one Warrant, reached when its obligations are established | the Warrant's author | §23 | `war status <alias>` |
| Stage | `STAGE-<nnn>` | the smallest independently dispatchable execution node | the Warrant's author | §47 | `war dispatch` |
| Dispatch | a compiled §47.1 packet | the exact stage-specific instruction given to one actor, digest-bound | `war dispatch` | §47 | the executor |
| Artifact / Evidence | deliverables, receipts, verifications | what was produced; the immutable basis for judging it | performers; gates; independent verifiers | §37, §40, §44, §46 | `war resolve` |
| Resolution | `resolution.toml` | the attributable organizational conclusion about one Warrant under one exact contract | a human resolver | §56 | the ladder |

**The rule.** A SAS and a Warrant are the **same class of artifact at two
levels of importance**: each is a controlled contract with an intent, a
basis, deliverables, acceptance obligations, gates, and immutable revisions.
They differ in scope and in what traces to them, not in kind.

- The **SAS** is the contract for a program. A program has **exactly one**
  SAS, and every Warrant in that program traces to it through `[[implements]]`
  (§34). Its requirements are the program's obligations; its phases are the
  program's milestones; its accepted revisions are the program's authorized
  contract revisions (§101 is §28 at program scale).
- A **Warrant** is the contract for one bounded intervention. It exists only
  inside a program, and it is not a small SAS: it names the SAS requirements
  it realizes and the phase that motivates it, and it can be resolved.

So:

- **Starting a program?** Write its SAS. Do not write a Warrant "in the
  style of the SAS" and call it the program's specification: a Warrant with
  no SAS to trace to has no requirement ids to implement, no Objective to
  discharge, and no Release to belong to — the projection reports it under
  `unassigned` and it can never read `satisfied` on any requirement.
- **Doing work inside a program?** Write a Warrant against that program's
  SAS. Do not write a second SAS for a piece of work: a program with two
  SASs has two Release axes and no single answer to "how far along are we".
- **Which one is this document?** The OpenWarrant SAS (this file) is the SAS
  of the OpenWarrant program. Another program — a codec, a server, a
  laboratory workflow — gets its own SAS in its own repository, with its own
  prefix, and its own Warrants trace to that one, not to this one.

The correspondence, level for level:

| In a Warrant | In a SAS |
|---|---|
| intent atom (§16) | §1 Purpose, §5 Architectural thesis |
| basis atom (§14, §16) | §6 System hierarchy, §7 Design laws, §10 Implementation basis |
| work order deliverables (§37) | §98 phase deliverables |
| milestones (§23) | §98 phases (Objectives) |
| acceptance obligations (§38) | §106 requirements |
| gate citations (§43) | §99 System acceptance criteria |
| contract revision, amendment (§28, §31) | SAS revision, ADR-carrying revision (§101) |
| authorization (§28.4) | acceptance (§101.2) |
| resolution (§56) | Release fulfilled: every requirement satisfied |

## 7. Design laws

### Law 1 — Atoms are authored; parents are projections

A generated WAR parent SHALL never be directly edited.

Every controlled edit targets:

- an authored atom;
- a bound authoritative record through its owning system;
- or a typed action that creates a generated record.

### Law 2 — One logical document does not mean one mutable blob

A Warrant is one document to the reader and one canonical export to the machine. Internally it is an ordered semantic composition whose members retain separate identity, history, Jurisdiction, and authority.

### Law 3 — One authority per fact

No fact SHALL have two competing canonical authorities.

OpenWarrant compiles facts. It does not silently take authority from Git, Knowledge Fabric, Liminal, Katana, BLUT, a Gate Registry, an instrument, or another native system.

### Law 4 — No controlled fact exists only in prose

Lifecycle, scope, deliverables, milestones, gates, capabilities, blockers, deviations, obligations, evidence provenance, judgments, and resolution SHALL have typed machine representations.

Prose MAY explain them.

### Law 5 — Authorized contract revisions are immutable

Draft atoms may be edited. Once a contract revision is authorized, later semantic change creates a new immutable revision.

### Law 6 — Progress cannot amend the contract

A progress note, model response, commit message, or milestone event SHALL NOT silently change authorized scope, gates, deliverables, or acceptance claims.

### Law 7 — All normative decisions are ADRs

Any choice represented as a normative decision in the WAR system SHALL be a first-class ADR.

A local implementation choice already permitted by an authorized autonomy envelope is an execution choice, not a new normative decision.

### Law 8 — Decision overview is generated

OpenWarrant SHALL compile all ADR atoms into a major ADR Overview document. The overview is never manually maintained.

### Law 9 — Parent WARs own the originating rationale for child WARs

A child WAR derives purpose from an exact parent WAR revision. The child may add local context, but it SHALL NOT rewrite or become the retroactive source of the parent’s original rationale.

### Law 10 — Child and superseding WARs are different

A child WAR decomposes or implements its parent. A superseding WAR replaces a prior authorization, rationale, or approach.

### Law 11 — Superseded WARs remain historical truth

A superseded WAR remains immutable and readable. Its canonical currency becomes `superseded`; human views mark it deprecated for new execution and point to the replacement.

### Law 12 — Performer artifacts and verifier evidence are different

A performer produces artifacts and claims. A verifier or qualified system produces independent observations. A performer-authored report SHALL NOT satisfy a gate that requires independent evidence.

### Law 13 — Evidence, observation, inference, judgment, and resolution are different

The schema SHALL preserve these epistemic classes.

A measurement is not a judgment. A judgment is not a measurement. A gate verdict is not an organizational resolution.

### Law 14 — Claims are bounded by their evidence

A corpus pass establishes the declared result for that corpus. It does not establish universal correctness unless an independent argument proves the corpus exhausts the claimed domain.

### Law 15 — Unknown is not failure and not pass

A gate that cannot be asked, selects nothing, crashes, times out, or lacks required fixtures returns an unknown result. Required unknown results block resolution.

### Law 16 — Gate logic is separately governed

Reusable gate definitions, qualifications, protected fixtures, and pass predicates are governed through the Knowledge Fabric Gate Registry.

### Law 17 — Preflight exercises the real actor path

Readiness SHALL test the environment, network route, workspace, identity, capabilities, and tools that the actual performer or verifier will use.

### Law 18 — Agents operate inside explicit authority

An agent may draft, propose, execute, report, review, and recommend. It may not authorize its own material amendment, accept organizational risk, authorize the contract, or resolve its own work.

### Law 19 — Rigor is proportional to consequence

The architecture is universal. Ceremony is selected through assurance level and executor responsibility tier.

### Law 20 — Reproducibility is role-specific

Exploratory execution may be context-complete and policy-controlled. Final verification should be hermetic or controlled where technically meaningful.

### Law 21 — Retries have explicit semantics

Replay, repair, and restart are different attempt types with different bases and authority.

### Law 22 — Amendments do not reinterpret history

Every attempt, dispatch, gate run, and artifact remains tied to the exact contract revision under which it occurred.

### Law 23 — Federation does not erase local sovereignty

Repositories and subsystems may own WAR source holders and local aliases. Knowledge Fabric allocates global identity, authority, lifecycle, and cross-system relations.

### Law 24 — OpenWarrant is not another kernel

OpenWarrant SHALL reuse Liminal for document semantics, Knowledge Fabric for institutional authority, Katana for agent runtime, BLUT for typed computational execution, and native systems for native artifacts.

### Law 25 — Architecture-complete, capability-incremental

The final ownership seams and protocol identities are defined now. Individual capabilities may be delivered in phases without creating temporary competing authorities.

## 8. Goals

OpenWarrant SHALL make it possible to:

1. create a WAR from a short natural-language request;
2. ask only the unresolved interview questions necessary to make the request executable;
3. compile repository, SAS, Roadmap, ADR, policy, and artifact context;
4. create typed authored atoms rather than arbitrary model-written files;
5. generate one complete human Warrant;
6. generate a Work Order projection;
7. include ADRs on demand without making every WAR ceremonial;
8. define milestones and dispatchable stages;
9. track execution without editing generated checkboxes;
10. separate empirical evidence and judgment;
11. dispatch one stage to a stateless agent;
12. receive typed artifacts, blockers, deviations, and decision proposals;
13. verify outputs through qualified gates;
14. federate repository-local WARs into Knowledge Fabric;
15. preserve one canonical portable WAR document;
16. migrate existing atom/parent documentation and ADR corpora;
17. support future contractor Work Orders without redesigning the technical core.

## 9. Non-goals

Version 1 SHALL NOT require:

- a production-ready Liminal compiler;
- a Knowledge Fabric server for local drafting;
- a database in OpenWarrant;
- a web UI;
- cryptographic signatures;
- a complete contractor or payment profile;
- every eventual work profile;
- autonomous agent authorization;
- automatic proof that gates entail arbitrary claims;
- replacement of current repository documentation before measured migration.

## 10. Implementation basis

This SAS is designed against the following current repository boundaries:

| Repository | Revision inspected | Relevant basis |
|---|---|---|
| `Quitetall/liminal` | `2b7fc3f448b171e9a4aa2439a32b7a46f9509871` | Nodes/Relations, Jurisdiction, Holders, immutable Workspace Basis, RepairPlan, ILRP, human/AI projections, execution-grade work orders |
| `Quitetall/katana` | `651ba435296c37d91be25d458a6e485d35ac516e` | tamper-evident runtime log, pure PromptIR compiler, capability broker, confinement, version-anchored edits, runtime receipts |
| `Quitetall/blut` | `f5403b9d45585544f6d2f5d1a34e048915b2545d` | typed Stage/Plan DAGs, artifacts, resource admission, cache, status, lineage |
| `Quitetall/openhuman-knowledge-fabric` | `9e0e75550f96a851c126e59e696d71245d51382d` | typed actions, authority, idempotency, audit, object identity, Liminal document compilation, evidence preservation |
| `Quitetall/LamQuant` | `5369da813578df355ea1c8c17bf20d85e426681a` | atom/parent documentation, ADR lifecycle, generated views, measured gate-execution failures |

These revisions are implementation evidence, not permanent protocol pins. A future WAR implementing this SAS SHALL pin the exact revisions it uses.

---

# Part II — Authority, federation, and composition

## 11. Component ownership

| Component | Canonical ownership |
|---|---|
| Knowledge Fabric | global WAR and ADR identity; authorization; lifecycle; role authority; judgments; resolution; cross-repository relations; institutional Gate Registry; preservation |
| OpenWarrant | WAR schemas; canonical WAR IR; validation; file-native authoring; CLI; compilation orchestration; projections; protocol adapters |
| Liminal | atom/source semantics; Nodes and Relations; Jurisdiction; Holders; Workspace Basis; source maps; semantic graph; human and AI compilation |
| Katana | agent event log; PromptIR; model and tool calls; capability realization; confinement; runtime receipts |
| BLUT | typed computational DAG; stage execution; resources; cache; status stream; lineage |
| Git | repository source and commits |
| Gate runners | verifier-controlled execution and raw gate observations |
| Object storage | immutable artifact and evidence bytes |
| Native systems | CAD, datasets, instruments, invoices, payments, QMS records, and other domain-native facts |

### 11.1 Knowledge Fabric is the federation plane

Knowledge Fabric SHALL make WARs globally discoverable and relate them across repositories and subsystems.

A repository may remain the Source Holder of authored WAR atoms. Registering a WAR in Knowledge Fabric does not transfer source authority unless a typed Holder-transfer action explicitly does so.

### 11.2 OpenWarrant is the protocol and compiler surface

OpenWarrant SHALL NOT own a second institutional database.

It may maintain disposable indexes and local draft journals. When a WAR is registered, Knowledge Fabric owns authoritative lifecycle and controlled actions.

### 11.3 Liminal is the eventual semantic substrate

The final production compiler SHALL use a versioned Liminal profile for WAR source, composition, provenance, context, and projections.

Until Liminal is qualified, OpenWarrant MAY ship a constrained Markdown/frontmatter adapter that lowers into the same canonical WAR IR.

The compatibility adapter SHALL eventually become an importer or differential oracle, not a second permanent definition of WAR semantics.

### 11.4 Katana is the agent runtime

OpenWarrant may ask Katana to draft a proposal or execute a Dispatch. It SHALL NOT implement a second agent loop, capability broker, sandbox, or PromptIR.

### 11.5 BLUT is the typed computational runtime

OpenWarrant may lower compatible stage graphs to BLUT `PlanSpec` or a successor protocol. It SHALL NOT implement a second typed ML or computational DAG engine.

## 12. Identity and federation

### 12.1 Identity layers

Every WAR and ADR SHALL have:

1. an internal UUIDv7 identity;
2. zero or one repository-local alias;
3. zero or one globally allocated enterprise identifier;
4. zero or more external aliases.

Example:

```yaml
identity:
  uuid: "019c8f2d-7b4d-7c41-9cb7-2636e5f582ea"
  local_alias: "LIM-WAR-0042"
  enterprise_id: "OH-WAR-000042"
  external_aliases: []
```

### 12.2 UUID is immediate

The UUIDv7 is created at draft creation and never changes.

### 12.3 Local alias

A repository or subsystem MAY allocate a readable alias such as:

```text
LIM-WAR-0042
KAT-WAR-0018
BLUT-WAR-0091
KF-WAR-0077
```

The alias namespace SHALL be declared in repository configuration.

### 12.4 Enterprise identifier

Knowledge Fabric eventually allocates the official identifier under the OpenHuman Identifier Registry.

The enterprise identifier SHALL NOT be fabricated from a filename or local sequence.

A WAR may remain valid as a local draft before allocation. It may not claim globally authorized or effective state until registered through Knowledge Fabric.

### 12.5 Federation record

Knowledge Fabric SHALL map:

```text
UUID
repository/subsystem
local alias
enterprise identifier
Source Holder
classification
current contract revision
lifecycle projection
relations
```

### 12.6 Offline creation

Offline creation SHALL use UUID identity.

A provisional local alias MAY be created if repository policy can allocate it without collision. Branch-unsafe sequential allocation SHALL NOT be treated as globally unique.

### 12.7 Stable references

Machine references SHOULD use UUID or enterprise identity. Human views may display the local alias.

## 13. Source Holders and Jurisdiction

Every atom or bound record SHALL declare its Source Holder.

Initial holder kinds:

```text
git
fabric_native
external
generated_projection
runtime_receipt
```

The final Liminal-backed implementation SHALL map these into Liminal Jurisdiction and Holder semantics.

### 13.1 Authored atom

An authored atom is directly maintained under its declared Holder.

Examples:

- intent;
- local context;
- work-order instructions;
- milestone definitions;
- acceptance obligations;
- rollback;
- an ADR body.

### 13.2 Bound atom

A bound atom renders an exact authoritative fact held elsewhere.

Examples:

- SAS requirement;
- Roadmap item;
- Knowledge Fabric Work Order;
- requirement record;
- risk record;
- Git commit;
- dataset version;
- gate definition;
- budget;
- contractor authorization.

The rendered bound atom is not edited inside the Warrant.

### 13.3 Generated atom

A generated atom is derived.

Examples:

- current state;
- milestone status;
- attempt timeline;
- blocker list;
- gate results;
- evidence summary;
- artifact manifest;
- ADR index;
- resolution view.

Generated atoms are never directly edited.

### 13.4 Jurisdiction law

Every section of a Warrant SHALL answer:

> Who may change this fact, through which operation, against which prestate, at which Basis?

If the answer is ambiguous, the Warrant is not ready.

## 14. Workspace Basis

Every WAR compilation SHALL run against one immutable Workspace Basis.

The Basis identifies all inputs required to reproduce the semantic compilation, including:

- manifest revision;
- authored atom revisions;
- bound object revisions;
- source commits;
- ADR revisions;
- SAS revision;
- Roadmap revision or snapshot;
- schema pack;
- vocabulary pack;
- policy;
- compiler;
- target projections;
- relevant runtime receipts.

A compilation may finish on its captured Basis or restart on a newer one. It SHALL NOT silently mix independently changing inputs.

## 15. One logical document

### 15.1 Canonical meaning

The canonical meaning of a Warrant is the canonical WAR IR produced from one valid Compilation Basis.

### 15.2 Human parent

The normal human document is a generated Markdown parent containing all applicable sections in deterministic order.

### 15.3 Machine parent

The normal portable machine document is one RFC 8785 canonical JSON file.

### 15.4 Source atoms

Source atoms remain individually addressable, reviewable, and editable.

### 15.5 Concatenation versus composition

The human renderer may appear to concatenate atoms. Internally, composition SHALL be semantic:

```yaml
inputs:
  - ordinal: 10
    role: intent
    ref: "atom://...@revision"
  - ordinal: 20
    role: basis
    ref: "atom://...@revision"
  - ordinal: 30
    role: adr
    ref: "adr://...@revision"
  - ordinal: 40
    role: work_order
    ref: "atom://...@revision"
  - ordinal: 50
    role: execution
    ref: "projection://..."
```

The compiler knows each input’s role, Holder, revision, classification, provenance, and mutability.


## 16. WAR composition grammar

A WAR is composed from typed atom roles. Roles may repeat where explicitly permitted.

### 16.1 Core role order

| Order | Role | Typical authority | Required |
|---:|---|---|---|
| 00 | control | generated/bound | yes |
| 10 | intent | authored | yes |
| 20 | basis | authored + bound | yes |
| 30 | decisions | bound ADR atoms | conditional |
| 40 | work_order | authored | yes for delivery |
| 45 | milestones | authored definitions + generated state | yes for delivery |
| 50 | execution | generated from actions and receipts | after execution begins |
| 60 | assurance | authored obligations + generated proof | yes |
| 70 | resolution | generated/bound | after resolution |
| 80 | validation | authored/bound | optional |
| 90 | relations_and_integrity | generated | yes |

The human parent SHALL omit inapplicable optional roles rather than render empty ceremonial headings.

### 16.2 Multiple atoms per role

A role MAY contain multiple atoms:

```yaml
- role: basis
  ref: atom://technical-context
- role: basis
  ref: atom://security-constraints
- role: adr
  ref: adr://OH-ADR-000137
- role: adr
  ref: adr://OH-ADR-000142
```

Order SHALL be explicit and deterministic.

### 16.3 Required roles by profile

Initial profiles:

#### Delivery

```text
control
intent
basis
work_order
milestones
assurance
relations_and_integrity
```

#### Decision

```text
control
intent
basis
one or more ADRs
assurance
relations_and_integrity
```

A delivery WAR may include ADRs.

### 16.4 Extension roles

Future registered profile extensions MAY add:

- hypothesis;
- experimental design;
- investigation protocol;
- review criteria;
- remediation plan;
- contractor terms;
- physical test basis.

Unknown required roles SHALL fail closed. Unknown optional namespaced roles SHALL be preserved in the canonical export and omitted only when the target projection declares that behavior.

## 17. Parent document rules

### 17.1 Generated header

Every generated parent SHALL begin with a machine-readable and human-visible warning:

```markdown
<!--
GENERATED BY OPENWARRANT. DO NOT EDIT.
WAR: LIM-WAR-0042
Compilation basis: sha256:...
Contract revision: 3
Source manifest: docs/warrants/LIM-WAR-0042/manifest.toml
-->
```

### 17.2 Parent authority

The parent document is never authoritative merely because it is committed to Git.

It is a reproducible projection of the Compilation Basis.

### 17.3 Generated drift

`war check --generated` SHALL fail if committed generated views differ from fresh compilation.

### 17.4 Parent editing

An editor integration that receives an edit against a generated parent SHALL:

1. map the edit through source maps to an authored atom;
2. propose the corresponding atom edit;
3. reject ambiguous or generated-region edits;
4. never silently write the parent.

The minimal v1 CLI MAY simply refuse direct parent edits.

### 17.5 Read projections

The compiler SHALL support at least:

```text
full_warrant
work_order
adr_section
adr_overview
stage_dispatch
assurance_case
status
audit
canonical_json
```

## 18. WAR internal sections

The full Warrant SHALL be capable of presenting the following sections.

### 18.1 Control

- identity;
- profile;
- assurance;
- state projection;
- Source Holder;
- contract revision;
- classification;
- owner;
- relations;
- compilation digests.

### 18.2 Intent

- problem;
- desired outcome;
- completion summary;
- scope;
- non-goals;
- affected SAS requirements;
- Roadmap basis.

### 18.3 Basis

- governing sources;
- exact context;
- prerequisites;
- assumptions;
- constraints;
- existing evidence;
- environment;
- precedence and conflict policy.

### 18.4 Decisions

- transcluded ADRs;
- their statuses;
- affected scope;
- relations;
- consequences.

The Decisions section appears only when ADR references exist.

### 18.5 Work Order

- deliverables;
- frozen surfaces;
- premade instructions;
- algorithms or implementation constraints;
- stages;
- resources;
- capabilities;
- autonomy;
- rollback or compensation.

### 18.6 Milestones

- milestone definitions;
- stage dependencies;
- acceptance obligations;
- derived status;
- target dates where applicable.

### 18.7 Execution

- dispatches;
- attempts;
- runtime receipts;
- artifacts;
- blockers;
- amendments;
- deviations;
- discovered gaps;
- progress timeline.

### 18.8 Assurance

- acceptance obligations;
- adequacy review;
- gate bindings;
- evidence;
- observations;
- inferences;
- judgments;
- residual risk;
- obligation dispositions.

### 18.9 Resolution

- outcome;
- artifact manifest;
- proof snapshot;
- resolver;
- meaning;
- standing;
- supersession, dispute, or annulment.

### 18.10 Ongoing validation

- post-resolution monitors;
- invalidation triggers;
- superseding-WAR triggers.

## 19. ADR architecture

### 19.1 Every normative decision is first class

A WAR SHALL NOT store a normative decision only as inline prose.

When a choice changes:

- architecture;
- contract meaning;
- scope;
- interface;
- invariant;
- gate;
- threshold;
- security boundary;
- accepted risk;
- future constraints;
- or another durable normative fact,

the choice SHALL be recorded as an ADR.

### 19.2 What is not a new ADR

The following do not create a new ADR when already authorized:

- a private variable name;
- a local refactor preserving all declared behavior;
- selection among equivalent permitted tools;
- a mechanical application of a governing ADR;
- an auto-authorized amendment class whose governing policy already made the normative decision;
- a factual correction with no semantic choice.

A useful test is:

> Would a future executor need to know why one alternative was chosen over another, or would this choice constrain future work?

If yes, create an ADR.

### 19.3 ADR Source Holder

Each ADR is an authored atom under a declared Source Holder. It has its own stable identity and immutable accepted revisions.

### 19.4 ADR relation to WAR

An ADR MAY:

- originate from a WAR;
- govern one or more WARs;
- be implemented by one or more WARs;
- supersede another ADR;
- amend the SAS;
- authorize an amendment class;
- record a phase-gate outcome.

A WAR MAY reference multiple ADRs.

### 19.5 ADR creation during execution

When an executor encounters a decision outside its autonomy envelope:

1. the stage records a decision proposal;
2. the affected work blocks where necessary;
3. OpenWarrant drafts a proposed ADR;
4. the ADR is reviewed and accepted or rejected;
5. the WAR receives an authorized contract revision if needed;
6. preflight runs again.

### 19.6 ADR overview

OpenWarrant SHALL generate one major ADR Overview per configured scope.

The overview SHALL contain:

1. a summary table;
2. status and currency groupings;
3. decision relations;
4. affected SAS requirements;
5. governing and implementing WARs;
6. the complete ADR bodies concatenated in deterministic order;
7. unresolved proposals;
8. supersession chains;
9. gate or evidence debt where applicable.

A separate compact index MAY contain links only. The major audit Overview SHALL remain a single complete document. Repository scopes MAY generate local overviews. Knowledge Fabric SHALL generate a federated OpenHuman overview.

### 19.7 No manual ADR index

A manually maintained ADR index is prohibited once OpenWarrant manages the scope.

## 20. Parent and child WARs

### 20.1 Parent semantics

A parent WAR preserves the originating context, rationale, and authorization from which child work is decomposed.

The parent SHALL identify:

- why the overall work exists;
- broad desired outcome;
- shared constraints;
- high-level acceptance;
- decomposition policy.

### 20.2 Child semantics

A child WAR SHALL reference:

```yaml
parent:
  warrant_ref: "war://..."
  contract_revision: 2
  contract_digest: "sha256:..."
  inherited_context_selectors:
    - "intent"
    - "basis.constraints"
    - "adr_refs"
```

The child adds only the local context necessary for its bounded outcome.

### 20.3 No retroactive parent rationale

A child outcome, later discovery, or implementation detail SHALL NOT become the supposed original rationale of the parent.

If the parent rationale is wrong or materially incomplete, create:

- an ADR that records the changed decision; and
- a superseding WAR when authorization or outcome changes.

### 20.4 Parent generated child view

The parent’s generated Execution or Relations section SHALL list child WARs and their current states. The list is a bound/generated projection, not an edit to the parent’s original contract.

### 20.5 Parent resolution

A parent MAY resolve only when:

- its own obligations are satisfied; and
- every required child WAR has an admissible disposition.

A child may resolve independently.

### 20.6 Split rule

Create a child WAR when a portion of work has an independent:

- authorization boundary;
- acceptance boundary;
- payment boundary;
- classification;
- rollback or compensation boundary;
- owner;
- cancellation decision;
- release decision.

## 21. Supersession and deprecation

### 21.1 Supersession relation

A replacement WAR SHALL declare:

```yaml
supersedes:
  - warrant_ref: "war://..."
    reason: "..."
```

### 21.2 Old WAR state

The replaced WAR’s canonical currency becomes:

```yaml
currency: superseded
```

Its generated view SHALL display:

> **Superseded and deprecated for new execution. See WAR X. Historical artifacts, evidence, and resolution remain authoritative for the period and basis they describe.**

### 21.3 Deprecated without replacement

A WAR retired without a direct replacement uses:

```yaml
currency: deprecated
```

### 21.4 No deletion

Superseded and deprecated WARs SHALL remain available for audit and relation traversal.

### 21.5 Adoption of unresolved work

A superseding WAR SHALL explicitly identify which unresolved child WARs, deliverables, evidence, or obligations it adopts. Nothing is silently carried forward.

## 22. Work Order architecture

### 22.1 Technical Work Order

For software delivery, the Work Order projection SHALL contain:

- scope;
- spec inputs;
- frozen surfaces;
- premade decisions through ADR references;
- data schemas;
- algorithms;
- deliverables;
- milestones;
- stages;
- exact commands or registered gates where appropriate;
- expected observable outputs;
- amendments;
- discovered gaps;
- exit criteria.

### 22.2 Tracking is generated

Milestone and step definitions are authored. Completion state is generated from typed actions and receipts.

The parent MAY render checkboxes:

```markdown
- [x] M1 — Define scalar interface
- [ ] M2 — Pass portability matrix
```

The checkbox is not directly edited.

### 22.3 Contractor future compatibility

The eventual contractor profile may add:

- parties;
- authorization scope;
- compensation;
- schedule;
- deliverable ownership;
- acceptance authority;
- invoice relation;
- payment relation;
- legal terms;
- confidentiality;
- signatures.

Until that profile is approved, a technical WAR SHALL link to, not replace, the Knowledge Fabric contractual Work Order and finance records.

### 22.4 Work Order projection is not a second source

The Work Order is generated from the WAR contract atoms. Editing the generated Work Order is prohibited.

## 23. Milestones and stages

### 23.1 Milestone definition

```yaml
id: "M2"
title: "Reference equivalence is established"
depends_on:
  - "M1"
stage_refs:
  - "STAGE-002"
  - "STAGE-003"
obligation_refs:
  - "OBL-002"
  - "OBL-003"
completion_policy: "all_obligations_established"
```

### 23.2 Milestone state

Derived states:

```text
not_started
ready
in_progress
blocked
verifying
complete
not_completed
cancelled
```

### 23.3 Stage definition

A stage SHALL contain:

- identifier;
- objective;
- executor kind;
- typed input ports;
- typed output ports;
- dependencies;
- context selectors;
- capability authorization;
- resource envelope;
- reproducibility level;
- autonomy envelope;
- failure policy;
- submission schema;
- linked obligations.

### 23.4 Executor kinds

Initial executor kinds:

```text
katana
blut
human
service
laboratory
external
```

### 23.5 Named ports

WAR stage graphs SHALL use named typed ports.

Adapters may lower them to runtime-specific ordering, but the WAR contract SHALL not rely on ambiguous positional semantics.

### 23.6 Stage versus milestone

A milestone expresses meaningful progress or acceptance. A stage expresses dispatchable execution.

One milestone may have many stages. One stage may contribute to several milestones only when the relation is explicit.


---

# Part III — Lifecycle, authority, and contract

## 24. State model

State SHALL be decomposed into independent dimensions.

### 24.1 Phase

```text
draft
proposed
authorized
ready
executing
verifying
resolved
```

### 24.2 Execution condition

```text
clear
blocked
paused
```

### 24.3 Common outcome

```text
none
satisfied
not_satisfied
falsified
rejected
withdrawn
cancelled
inconclusive
```

### 24.4 Currency

```text
current
superseded
deprecated
```

### 24.5 Resolution standing

```text
valid
disputed
annulled
```

### 24.6 Truthful combinations

Completed and later replaced:

```yaml
phase: resolved
condition: clear
outcome: satisfied
currency: superseded
standing: valid
```

Accepted and later challenged:

```yaml
phase: resolved
condition: clear
outcome: satisfied
currency: current
standing: disputed
```

Resolution invalidated:

```yaml
phase: resolved
condition: clear
outcome: satisfied
currency: current
standing: annulled
```

The original outcome remains because it records what was concluded at the time. Standing records whether the organization still permits reliance upon it.

### 24.7 Core transitions

```text
draft → proposed → authorized
authorized --preflight passes→ ready
ready --dispatch→ executing
executing --submission→ verifying
verifying --sufficient assurance→ resolved
```

Blocking overlays the phase:

```text
authorized + blocked
executing + blocked
verifying + blocked
```

Resolving a blocker returns to the same phase and requires re-preflight where the underlying basis changed.

### 24.8 Material amendment transition

Any authorized material contract amendment returns the WAR to:

```text
phase: authorized
condition: clear or blocked
```

Affected stages SHALL be re-preflighted.

### 24.9 Post-resolution events

```text
resolution.disputed
resolution.dispute_resolved
resolution.annulled
warrant.superseded
warrant.deprecated
```

These events do not erase the original resolution.

## 25. Assurance levels

Assurance level governs the strength of proof, independence, custody, review, and authorization.

```text
basic
controlled
high_assurance
```

### 25.1 Basic

Use for routine, low-consequence internal work.

Minimum:

- explicit contract revision;
- typed deliverables;
- fresh verification;
- content-addressed artifacts where applicable;
- no unresolved blockers;
- no unapproved deviations;
- no missing required measurement.

Permitted simplifications:

- context-complete performer basis;
- automated verifier service;
- auto-authorized safe amendments;
- ordinary retention;
- no independent semantic reviewer where policy permits.

### 25.2 Controlled

Use for release-affecting, cross-module, externally important, or expensive-to-reverse work.

Additional requirements:

- structured acceptance argument;
- adversarial gate-adequacy review;
- blind verifier;
- pinned or hermetic final verification;
- exact context and runtime receipts;
- qualified gates or explicit limitations;
- residual-risk judgments.

### 25.3 High assurance

Use for safety-critical, regulated, clinical, major security, product release, legally material, or consequential financial work.

Additional requirements:

- named accountable human authority;
- domain-appropriate independent review;
- qualified gate definitions and fixtures;
- controlled evidence custody;
- explicit retention;
- signatures or checkpoints where policy requires;
- controlled measurement or reproducible build basis;
- explicit acceptance of every residual gap;
- invalidation propagation;
- preservation-grade export.

### 25.4 Risk-derived floor

Knowledge Fabric policy SHALL derive a minimum assurance level from:

- safety impact;
- security impact;
- financial authority;
- irreversibility;
- regulated use;
- external reliance;
- classification;
- blast radius;
- dependency centrality;
- release status.

An actor may raise assurance freely. Lowering below the floor requires an ADR and authorized risk judgment.

## 26. Executor responsibility tiers

Executor tier governs how much responsibility and semantic judgment a step demands.

```text
T1
T2
T3
T4
```

### 26.1 T1 — constitutional or mission-critical

An error may pass ordinary tests while poisoning architecture, evidence, or authority.

Examples:

- accepting a golden;
- changing a public invariant;
- changing gate meaning;
- accepting risk;
- authoring a phase-gate WAR;
- executing irreversible or safety-relevant work.

T1 requires the strongest available executor and an independent second look.

### 26.2 T2 — implementation judgment

The outcome is specified, but engineering judgment is required.

### 26.3 T3 — minor implementation detail

The shape is specified and only bounded local choices remain.

### 26.4 T4 — mechanical

Transcription, file movement, exact command execution, or other fully specified action.

### 26.5 Orthogonality

Assurance level and executor tier are independent.

A basic WAR may contain a T1 step. A high-assurance WAR may contain many T4 steps.

### 26.6 Automatic promotion

Any of the following promotes the moment to T1:

- ambiguity in normative meaning;
- proposed ADR;
- material amendment;
- assertion or gate change;
- golden acceptance;
- unexpected safety or security implication;
- unresolved evidence provenance;
- discovered gap that changes the contract.

## 27. Actor authority

### 27.1 Permitted agent roles

An agent may:

- draft atoms;
- propose a WAR;
- propose an ADR;
- execute stages;
- produce artifacts;
- report observations;
- open blockers;
- propose deviations;
- review artifacts;
- recommend judgments;
- generate projections.

### 27.2 Prohibited self-authority

An agent SHALL NOT:

- authorize its own proposed WAR;
- grant its own material amendment;
- approve its own deviation;
- accept organizational residual risk;
- resolve its own delivery;
- annul a resolution;
- allocate an official enterprise identifier;
- silently transfer a Source Holder;
- change a gate definition it is being judged by.

### 27.3 Basic policy-service resolution

A separately identified policy service MAY resolve a basic mechanical WAR when:

- policy explicitly allows it;
- all obligations are mechanical;
- no residual-risk judgment is required;
- performer and resolver identities are distinct;
- the meaning of resolution is explicit.

### 27.4 Human role multiplicity

One person may exercise several roles. The system SHALL record the role actually exercised.

Role separation by one person is not organizational independence. Human views SHALL not claim four-eyes review when none occurred.

## 28. Contract revisions

### 28.1 Stable Warrant identity

A Warrant identity persists across contract revisions.

### 28.2 Draft revision

Draft atoms may change before proposal.

### 28.3 Proposal snapshot

Submitting creates an immutable proposed Contract Revision.

### 28.4 Authorization

Authorization creates an immutable authorized Contract Revision with:

- contract digest;
- authorizer;
- acting role;
- authorization meaning;
- effective time;
- policy basis;
- exact Compilation Basis.

### 28.5 Contract digest

The contract digest SHALL cover:

- intent;
- scope;
- basis requirements;
- assumptions;
- constraints;
- ADR references;
- deliverables;
- milestones;
- stages;
- capabilities;
- autonomy;
- resources;
- gates;
- obligations;
- rollback;
- amendment policy;
- assurance requirements.

It SHALL exclude later execution, evidence, judgments, and resolution.

### 28.6 Revision ancestry

Every revision SHALL identify its predecessor and structured difference.

### 28.7 No in-place amendment

An authorized contract is never patched.

## 29. Contract content

Every delivery Contract Revision SHALL define:

### 29.1 Objective

```yaml
problem: "What condition requires work."
desired_outcome: "What shall be true afterward."
completion_summary: "Bounded human summary."
non_goals: []
```

The normative completion claim is the set of acceptance obligations, not the summary sentence.

### 29.2 Scope

```yaml
included_subjects: []
excluded_subjects: []
allowed_paths: []
forbidden_paths: []
allowed_interfaces: []
forbidden_interface_changes: []
```

### 29.3 Basis

- exact SAS requirement refs;
- Roadmap refs;
- governing ADRs;
- context items;
- precedence;
- prerequisites;
- assumptions;
- constraints.

### 29.4 Deliverables

Each deliverable declares kind, target, Holder, provenance, required status, and obligation links.

### 29.5 Milestones and stages

Definitions and dependencies.

### 29.6 Autonomy

Permitted local choices, auto-authorized revision classes, and escalation triggers.

### 29.7 Verification

Acceptance obligations, Gate Bindings, adequacy review, independence, and completion policy.

### 29.8 Rollback or compensation

How effects are reversed, contained, or compensated.

### 29.9 Amendment policy

What is local, auto-authorizable, manual, or a new ADR.

## 30. Autonomy envelope

### 30.1 Local choices

A local choice requires no contract revision when it is already authorized.

Examples:

- private module organization;
- private symbol naming;
- equivalent internal algorithm preserving declared invariants;
- additional non-mutating tests;
- use of an already authorized tool;
- diagnostic instrumentation removed before submission.

### 30.2 Auto-authorized contract revision

A narrow policy may pre-authorize:

- adding read-only context;
- adding a stricter gate;
- increasing timeout within a ceiling;
- adding a development-only dependency from an approved source;
- attaching prior failure evidence to a repair attempt;
- clarifying wording without semantic change.

The governing policy or ADR made the normative decision. The instance still creates an immutable revision and audit event.

### 30.3 Manual revision

Manual authorization and usually an ADR are required for:

- completion-claim change;
- scope expansion;
- public interface change;
- security or safety boundary change;
- new production dependency;
- gate weakening;
- pass-threshold change;
- new external side effect;
- material budget change;
- release or regulatory impact;
- accepted residual risk.

### 30.4 Ambiguity behavior

The default is:

```yaml
on_ambiguity: block_and_propose
```

An executor SHALL NOT improvise normative semantics.

## 31. Amendment record

Every revision after authorization SHALL contain:

- structured semantic diff;
- reason;
- governing ADR or policy;
- affected stages;
- affected milestones;
- affected attempts;
- affected Gate Runs;
- artifact admissibility decision;
- restart or repair instruction;
- re-preflight requirement;
- authorizer;
- effective time.

An amendment SHALL NOT retroactively reinterpret prior execution.

## 32. Prerequisites and readiness

A WAR becomes `ready` only through a successful Preflight receipt.

Preflight SHALL validate:

### 32.1 Contract

- schema pack known;
- profile valid;
- contract digest reproducible;
- required atoms present;
- required ADRs accepted or explicitly permitted;
- authorization valid.

### 32.2 Context

- required context resolves;
- normative references are immutable;
- no unresolved conflict;
- no unauthorized omission;
- classification policy satisfied;
- exact Workspace Basis captured.

### 32.3 Graph

- no stage or milestone cycle;
- required stages reachable;
- named ports compatible;
- outputs consumed or delivered;
- supported executor and condition semantics.

### 32.4 Runtime

- target repository or workspace available;
- base revision available;
- actor identity and role valid;
- tools available;
- capabilities realizable;
- provider available;
- actual network path usable from the actor environment;
- resource envelope available;
- output destinations writable;
- required secrets resolvable by reference.

### 32.5 Gates

- Gate Definition exists;
- version and digest match;
- lifecycle permits use;
- qualification meets assurance level;
- fixtures exist;
- performer cannot modify protected gate assets;
- verifier environment can execute the gate;
- selectors are valid and nonempty;
- negative controls remain valid.

### 32.6 Authority

- performer assigned;
- verifier assigned;
- resolver available;
- required independence achievable;
- side-effect authority sufficient.

### 32.7 Preflight meaning

Preflight proves only that the work and its verification can validly be attempted. It does not prove the deliverable correct.


---

# Part IV — Context, rationale, evidence, and proof

## 33. Context model

### 33.1 Context item

Every context item SHALL identify:

```yaml
id: "CTX-001"
role: "normative"
required: true
holder:
  kind: "git"
  repository: "Quitetall/example"
  commit_sha: "full-sha"
  path: "docs/spec.md"
content_digest:
  algorithm: "sha256"
  value: "..."
selector:
  sections:
    - "Interface invariants"
classification: "internal"
trust: "authoritative_internal"
taints: []
```

### 33.2 Context roles

```text
governing
normative
input
evidence
historical
informative
negative_control
tool_definition
policy
```

### 33.3 Trust classes

```text
authoritative_internal
authoritative_external
internal_unverified
external_untrusted
performer_generated
model_generated
```

Trust, classification, and authority are separate dimensions.

### 33.4 Precedence

A WAR SHALL declare source precedence.

Recommended default:

```text
law and external obligation
organization policy
security and quality policy
authorized WAR contract
governing ADR
SAS requirement
normative technical source
informative source
performer suggestion
```

Equal-precedence conflicts block readiness unless an explicit resolution exists.

### 33.5 Context completeness

Every required item SHALL resolve before readiness.

Draft requests may use phrases such as “current main.” Authorization SHALL resolve them to exact revisions.

### 33.6 Context manifest

Compilation SHALL emit:

```yaml
context_manifest:
  workspace_basis_ref: "..."
  workspace_basis_digest: "sha256:..."
  included: []
  omitted: []
  unresolved: []
  conflicts: []
  effective_classification: "internal"
  policy_digest: "sha256:..."
  compiler_digest: "sha256:..."
```

A required context item SHALL never be silently dropped to fit an AI context budget.

### 33.7 Context projection

The full WAR may contain more context than a stage needs.

The Dispatch compiler SHALL select the smallest sufficient subgraph and record:

- included sources;
- omitted sources;
- selection reason;
- summaries;
- provenance;
- classification;
- taint;
- budget.

### 33.8 Summary provenance

Summaries inherit the trust, classification, and taint of their source set. Compaction SHALL NOT launder untrusted influence.

## 34. SAS traceability

### 34.1 Requirement references

SAS requirements SHALL have stable identifiers:

```text
WAR-SAS-RQ-001
LIM-SAS-RQ-042
LMQ-SAS-RQ-117
```

### 34.2 WAR implementation relation

A WAR SHOULD declare:

```yaml
implements:
  - requirement_ref: "sas://LIM-SAS-RQ-042"
    intended_contribution: "complete"
```

Other contribution values:

```text
partial
complete
validation
investigation
supersession
```

### 34.3 Requirement status

SAS requirement status is derived from linked WARs and evidence. The SAS source itself SHALL NOT be edited merely to tick completion boxes.

### 34.4 Architecture-change discovery

If a WAR reveals that a SAS requirement is wrong:

1. open an ADR;
2. propose a controlled SAS revision;
3. supersede or amend affected WARs;
4. preserve the original requirement and evidence history.

## 35. Rationale model

Rationale SHALL separate kinds of reasoning.

### 35.1 Node classes

```text
fact
priority
constraint
option
forecast
tradeoff
decision
consequence
```

### 35.2 Facts

A fact cites an authoritative source or empirical observation.

### 35.3 Priority

A priority is a value judgment or policy preference.

Examples:

- preserve compatibility;
- prefer reversibility;
- prioritize safety over throughput;
- minimize long-term maintenance.

A priority SHALL NOT be presented as an empirical fact.

### 35.4 Forecast

A forecast identifies:

- method;
- assumptions;
- uncertainty;
- time horizon;
- source or model.

### 35.5 Alternative

Every considered option SHOULD identify:

- implementation shape;
- expected benefit;
- cost;
- risk;
- affected requirements;
- reason selected or rejected.

### 35.6 Decision

The decision itself lives in an ADR. The WAR rationale section binds the ADR and renders the facts, priorities, and consequences relevant to this Warrant.

### 35.7 Rationale edges

```text
supports
refutes
constrains
depends_on
trades_off_against
causes
qualifies
selected_over
```

## 36. Assumptions and unknowns

Every assumption SHALL use one status.

### 36.1 Evidenced premise

```yaml
epistemic_status: "evidenced_premise"
evidence_refs:
  - "evidence://..."
```

### 36.2 Accepted residual risk

```yaml
epistemic_status: "accepted_residual_risk"
judgment_ref: "judgment://..."
consequence_if_false: "..."
```

### 36.3 Blocking unknown

```yaml
epistemic_status: "blocking_unknown"
resolution_requirement: "..."
```

### 36.4 Circular validation is prohibited

An assumption cannot be validated by a gate whose meaning depends on that assumption.

The claim/evidence graph SHALL be acyclic.

### 36.5 Claim narrowing

When exhaustive evidence is unavailable, narrow the claim rather than overstate it.

Correct:

> Byte-identical for corpus version 2 and generated campaign 3.

Incorrect:

> Correct for every legal input.

unless an independent exhaustive argument exists.

## 37. Deliverables and artifacts

### 37.1 Deliverable definition

```yaml
id: "DEL-001"
title: "Scalar implementation commit"
kind: "git_commit"
target_ref: "git://..."
required: true
content_addressed: true
provenance_required: true
obligation_refs:
  - "OBL-001"
```

### 37.2 Artifact provenance

Registration SHALL record:

- producer;
- producing attempt;
- contract digest;
- input digests;
- tool or runtime identity;
- creation method;
- content digest;
- media type;
- classification;
- retention;
- Source Holder.

### 37.3 Derived report

A report derived from evidence SHALL reference the raw evidence.

A generated report SHALL NOT replace its source observations or bytes.

### 37.4 Performer submission

The submission manifest is normally not a deliverable. It is a claim envelope describing artifacts, blockers, deviations, and requested next action.

## 38. Acceptance argument

### 38.1 Obligations, not one prose claim

A completion summary SHALL be decomposed into bounded acceptance obligations.

Example:

```text
OBL-001 Every declared scalar target compiles.
OBL-002 The conformance corpus decodes byte-identically.
OBL-003 The generated legal-input campaign decodes byte-identically.
OBL-004 Released wire-format constants are unchanged.
OBL-005 The public API is unchanged.
```

### 38.2 Obligation schema

```yaml
id: "OBL-002"
statement: "..."
criticality: "required"
claim_scope:
  kind: "bounded_corpus"
  subject_ref: "artifact://..."
verification_methods:
  - "test"
gate_binding_refs:
  - "GB-002"
known_gaps: []
residual_risk_refs: []
```

### 38.3 Scope kinds

```text
single_instance
enumerated_set
bounded_domain
bounded_corpus
sampled_population
temporal_window
existential
universal
formal_model
```

### 38.4 Universal claims

A universal claim requires a verification argument capable of supporting universal scope. Sampling alone is insufficient.

### 38.5 Dispositions

Every required obligation receives one disposition:

```text
established
refuted
not_established
accepted_with_residual_risk
not_applicable
```

`not_applicable` requires an authorized reason.

### 38.6 Resolution aggregation

A delivery WAR normally resolves `satisfied` only when all required obligations are:

- established; or
- accepted with residual risk under sufficient authority.

## 39. Contract-adequacy review

The authoring process SHALL test whether the gate set meaningfully supports each obligation.

### 39.1 Adversarial question

For every required obligation:

> Construct an artifact that passes every declared gate while violating this obligation.

### 39.2 Outcomes

```text
counterexample_found
no_counterexample_found
obligation_narrowed
gate_added
gate_strengthened
gap_accepted
claim_removed
review_not_required
```

### 39.3 Executed attacks

Where economical, the system SHOULD plant violating artifacts or mutations and run the gates against them.

A violating artifact that passes is empirical evidence of gate inadequacy.

### 39.4 Required level

| Assurance | Adequacy requirement |
|---|---|
| basic | structural checks; semantic review optional by policy |
| controlled | blind adversarial review required |
| high_assurance | independent domain review plus executed negative controls or equivalent |

### 39.5 Honest limitation

No generic compiler can prove that arbitrary gates entail arbitrary natural-language claims. OpenWarrant exposes and records the remaining judgment.

## 40. Epistemic classes

The assurance case SHALL distinguish:

### 40.1 Claim

A proposition requiring support.

### 40.2 Evidence item

Immutable bytes or an authoritative record.

### 40.3 Observation

A method-bound statement about evidence.

Example:

> The verifier selected 142 tests and all 142 passed.

### 40.4 Inference

A reasoning step from premises or observations to a claim.

Kinds:

```text
deductive
statistical
causal
heuristic
formal
```

### 40.5 Judgment

An attributable evaluative or policy choice.

Examples:

- the residual risk is acceptable;
- the gate set is adequate;
- the deviation is immaterial;
- maintainability is sufficient.

### 40.6 Resolution

The organizational adjudication.

### 40.7 Prohibited substitutions

The validator SHALL reject:

```text
performer assertion → independent observation
generated report → raw evidence
test pass → universal coverage claim
model confidence → authorized judgment
gate verdict → resolution
resolution → empirical observation
```

## 41. Evidence model

### 41.1 Origins

```text
performer
verifier
gate_runner
external_authority
instrument
knowledge_fabric
katana
blut
liminal
human_reviewer
```

### 41.2 Admissibility

```text
informative
performer_report_only
independent
authoritative_external
controlled_measurement
formal
inadmissible
```

### 41.3 Evidence record

```yaml
id: "EVD-001"
kind: "gate_output"
origin:
  actor_ref: "service://gate-runner"
  role: "verifier"
subject_refs:
  - "artifact://DEL-001"
collection_method_ref: "gate://..."
runtime_basis_ref: "gate-runtime://..."
content_ref: "object://..."
content_digest:
  algorithm: "sha256"
  value: "..."
classification: "internal"
admissibility: "independent"
occurred_at: "optional-source-time"
recorded_at: "server-assigned"
```

### 41.4 Time authority

`recorded_at` is assigned by the authoritative receiving service.

An actor may provide `occurred_at`.

An authorized action may provide `effective_at`.

### 41.5 Chain of custody

High-assurance evidence SHOULD record:

- collector;
- original digest;
- transfer method;
- storage event;
- instrument or runner identity;
- calibration or qualification;
- transformations;
- access history;
- derivative lineage.

## 42. Judgment model

A judgment SHALL include:

```yaml
id: "JUD-001"
kind: "adequacy"
statement: "..."
actor_ref: "person://..."
acting_role_ref: "role-assignment://..."
basis_refs:
  - "observation://..."
meaning: "What this judgment authorizes or concludes."
scope: "..."
limitations: []
effective_at: "..."
recorded_at: "server-assigned"
```

An approval with no stated meaning is invalid.

An agent may recommend a judgment. It becomes an authorized judgment only through policy and an exercised role.

## 43. Gate Registry

### 43.1 Ownership

Knowledge Fabric owns the authoritative institutional Gate Registry.

OpenWarrant owns:

- gate schemas;
- local gate-candidate authoring;
- CLI inspection and binding;
- cached registry projections.

Repositories may hold local candidates. A candidate is not a qualified institutional gate.

### 43.2 Gate Definition

```yaml
gate_id: "software.codec.byte-identity"
version: "4.0.0"
digest: "sha256:..."
lifecycle: "active"
implementation_ref: "artifact://..."
input_kinds: []
output_schema_ref: "schema://..."
fault_model: []
known_blind_spots: []
qualification_ref: "gate-qualification://..."
```

### 43.3 Lifecycle

```text
draft
qualified
active
deprecated
invalidated
```

Definitions are immutable. Fixes create new versions.

### 43.4 Qualification

Qualification establishes that the gate detects declared fault classes.

It SHALL record:

- positive controls;
- negative controls;
- mutation classes;
- environments;
- detection results;
- limitations;
- qualifier;
- qualification digest.

### 43.5 Gate Binding

A Contract Revision binds an exact gate:

```yaml
id: "GB-002"
gate:
  id: "software.codec.byte-identity"
  version: "4.0.0"
  digest: "sha256:..."
subjects:
  - "deliverable://DEL-001"
fixtures:
  - ref: "artifact://conformance-v2"
    digest: "sha256:..."
parameters: {}
pass_predicate:
  byte_equal: true
evidence_policy:
  producer: "gate_runner"
  performer_authored_report_admissible: false
```

### 43.6 Reusable gates

Reusable gates SHOULD be composed from qualified primitives.

Bespoke gates are permitted, but controlled and high-assurance work require explicit qualification or a limitation judgment.

### 43.7 Subject-owned tests

Tests in the subject repository may contribute evidence. The runner, selector, required count, aggregation, and pass predicate remain verifier-controlled.

## 44. Gate Run semantics

A Gate Run SHALL separate three results.

### 44.1 Askability

```text
askable
not_askable
```

### 44.2 Execution status

```text
not_run
completed
timeout
infrastructure_error
cancelled
invalid
```

### 44.3 Verdict

```text
pass
fail
unknown
```

### 44.4 Examples

Target failed:

```yaml
askability: "askable"
execution_status: "completed"
verdict: "fail"
```

Tool missing:

```yaml
askability: "not_askable"
execution_status: "not_run"
verdict: "unknown"
reason_code: "missing_tool"
```

Zero tests selected:

```yaml
askability: "not_askable"
execution_status: "invalid"
verdict: "unknown"
reason_code: "zero_selected_tests"
```

### 44.5 Required passing result

Only this satisfies a required pass:

```yaml
askability: "askable"
execution_status: "completed"
verdict: "pass"
```

### 44.6 Gate receipt

The receipt SHALL record:

- Gate Definition and Binding digests;
- subject digests;
- fixture digests;
- runner;
- runtime environment;
- exact arguments;
- working directory or physical setup;
- start and completion;
- exit result;
- selected test count and manifest;
- raw evidence refs;
- stdout and stderr refs;
- resource usage;
- verdict;
- receipt digest.

### 44.7 Shell strings

Structured argument vectors are preferred.

A raw shell command is permitted only through a gate that explicitly owns shell parsing and classification.

### 44.8 Mutating gates

A mutating verification action must declare effects, authority, and compensation. It cannot run merely because an old document contains a command string.

## 45. Gate invalidation

When a Gate Definition is invalidated:

1. Knowledge Fabric records invalidation;
2. dependent bindings and resolutions are located;
3. materially dependent resolutions become disputed according to policy;
4. historical gate runs remain preserved;
5. re-verification uses a new gate version;
6. an authorized action resolves the dispute or annuls the resolution.

No historical evidence is rewritten.

## 46. Verifier independence

### 46.1 Independence dimensions

```yaml
performer_transcript_blind: true
performer_rationale_blind: true
separate_writable_workspace: true
cannot_modify_subject_artifacts: true
cannot_modify_gate_definition: true
cannot_modify_gate_fixtures: true
separate_context_compilation: true
distinct_model_required: false
distinct_human_required: false
```

### 46.2 Blind verifier input

A blind verifier receives:

- authorized contract;
- artifacts;
- gates;
- evidence;
- required context.

It does not receive persuasive performer narrative or private reasoning.

### 46.3 Minimums

| Assurance | Independence |
|---|---|
| basic | verifier-controlled execution |
| controlled | blind process or agent review |
| high_assurance | independent accountable person, quality authority, formal verifier, or domain-equivalent control |


---

# Part V — Execution and resolution

## 47. Dispatch model

A Stage Dispatch is the only packet given to a stateless actor.

### 47.1 Dispatch schema

```yaml
api_version: "oh.war/stage-dispatch/v1"
dispatch_id: "uuidv7"
warrant_ref: "war://uuid"
contract_revision: 3
contract_digest: "sha256:..."
milestone_id: "M2"
stage_id: "STAGE-003"
attempt_id: "uuidv7"
attempt_kind: "initial"
attempt_basis_digest: "sha256:..."

objective: "..."
non_goals: []
instructions: []

workspace_basis_ref: "liminal-basis://..."
workspace_basis_digest: "sha256:..."
context_manifest_ref: "artifact://..."
context_manifest_digest: "sha256:..."

input_artifacts: []
required_outputs: []
obligation_refs: []

capability_authorization:
  policy_ref: "policy://..."
  digest: "sha256:..."

resource_envelope:
  wall_time_seconds: 3600
  cpu_cores: 8
  memory_bytes: 17179869184
  gpu_count: 0
  network_policy: "allowlisted"
  spend_limit:
    currency: "USD"
    amount: "2.00"

submission_schema_ref: "schema://oh.war/stage-submission/v1"
```

### 47.2 Dispatch compilation

The compiler SHALL:

- select only stage-relevant context;
- preserve every required normative source;
- record omitted subgraphs;
- preserve provenance;
- enforce classification;
- include prior failure evidence for repair;
- produce deterministic canonical bytes;
- record the Dispatch digest.

### 47.3 Actor-specific projection

The human, Katana, BLUT, laboratory, and service projections may differ in representation. They SHALL preserve the same normative Stage contract.

## 48. Katana integration

### 48.1 Runtime seam

OpenWarrant SHALL invoke Katana through a versioned Dispatch protocol or subprocess/API adapter.

### 48.2 PromptIR ownership

Katana compiles the Dispatch and its runtime event history into Katana-owned PromptIR.

OpenWarrant records the PromptIR digest from the Katana receipt. It does not compile or reinterpret Katana’s runtime conversation.

### 48.3 Capabilities

Knowledge Fabric and the WAR contract authorize what may be done.

Katana realizes and enforces the low-level capability set.

### 48.4 Runtime receipt

Katana SHALL return, at minimum:

- Katana session/run identity;
- Dispatch digest;
- PromptIR digest;
- provider/model identity;
- runtime event-log head;
- realized capabilities;
- confinement;
- usage;
- artifact refs;
- terminal runtime status;
- receipt digest.

### 48.5 Taint

Katana taint and influence labels remain Katana-owned runtime facts. Relevant labels are referenced in the WAR assurance case.

## 49. BLUT integration

### 49.1 Lowering

A compatible WAR stage graph may lower into BLUT `PlanSpec` or its successor.

### 49.2 Adapter duties

The adapter SHALL:

- resolve stage names against a pinned registry;
- map named WAR ports to typed BLUT inputs and outputs;
- reject incompatible kinds;
- reject unsupported conditions;
- pin backend and stage identities;
- map resource envelopes;
- record plan provenance;
- return BLUT status, artifact, and lineage receipts.

### 49.3 Authority

BLUT execution lineage remains authoritative in BLUT. The WAR stores exact receipt references and relevant projections.

## 50. Human, service, and laboratory execution

The same Stage contract applies outside software runtimes.

### 50.1 Human execution

A human receives a Work Order or Stage Dispatch view and submits structured results.

### 50.2 Service execution

A service adapter records endpoint identity, request digest, response digest, policy, and side effects.

### 50.3 Laboratory execution

A controlled physical basis SHOULD record:

- test article;
- fixture;
- instrument;
- calibration;
- protocol;
- software;
- operator;
- environmental conditions;
- raw observations;
- deviations.

Computational hermeticity SHALL not be falsely claimed for physical work.

## 51. Stage Submission

### 51.1 Schema

```yaml
api_version: "oh.war/stage-submission/v1"
dispatch_id: "uuid"
attempt_id: "uuid"
contract_digest: "sha256:..."
stage_id: "STAGE-003"

claims:
  - id: "PCL-001"
    statement: "The scalar backend was implemented."

artifact_refs:
  - "artifact://..."

performer_observations:
  - id: "POB-001"
    statement: "The local test exited zero."
    evidence_ref: "artifact://local-log"
    admissibility: "performer_report_only"

blockers: []
deviation_proposals: []
decision_proposals: []
discovered_gaps: []
unresolved_items: []

requested_next_action: "verify"
```

### 51.2 No self-completion

The performer may request:

```text
continue
verify
block
amend
cancel
```

It SHALL NOT set or request authoritative resolution.

### 51.3 Performer claim status

Claims are assertions to be tested. Their structured form does not make them evidence.

## 52. Attempt semantics

### 52.1 Initial

First execution under a Contract Revision.

### 52.2 Replay

A replay uses an identical logical basis.

Use for:

- transient infrastructure failure;
- provider transport failure;
- lost response under idempotent effects;
- explicitly permitted stochastic inference retry.

```yaml
attempt_kind: "replay"
basis_change: "none"
```

### 52.3 Repair

A repair receives prior failure evidence and usually the prior work product.

```yaml
attempt_kind: "repair"
parent_attempt_ref: "attempt://A1"
prior_work_product_ref: "artifact://..."
prior_failure_evidence_refs:
  - "gate-run://..."
```

The attempt basis digest changes.

### 52.4 Restart

A restart abandons the prior approach or baseline.

It may use a new Contract Revision, executor, workspace, or architecture and requires authorization according to policy.

### 52.5 Attempt lineage

Every attempt SHALL have one parent except the initial attempt.

Failure evidence is attached by the runtime or control plane, not selected and rewritten by the performer.

## 53. Blockers, deviations, decisions, and gaps

### 53.1 Blocker

An unmet condition preventing valid progress.

```yaml
condition_ref: "PRE-002"
reason: "Pinned fixture unavailable."
owner_ref: "role://fixture-owner"
required_to_unblock: "Restore or supersede fixture."
```

### 53.2 Deviation

A proposal to execute differently from the authorized contract.

```yaml
affected_contract_path: "/execution/network"
proposed_change:
  policy: "allowlisted"
reason: "Dependency absent from cache."
impact:
  reproducibility: "reduced"
  security: "egress_added"
```

### 53.3 Decision proposal

A choice outside delegated autonomy.

A decision proposal SHALL become a proposed ADR before it becomes normative.

### 53.4 Discovered gap

A discovered gap states that the contract, SAS, ADR, gate, or source under-specified something required for valid execution.

A discovered gap is not silently repaired. It is dispositioned through clarification, amendment, ADR, child WAR, or supersession.

### 53.5 Different remedies

These categories SHALL remain distinct because:

- a blocker needs a condition resolved;
- a deviation needs exception authority;
- a decision needs an ADR;
- a discovered gap may require architecture or authoring correction.

## 54. Reproducibility

### 54.1 Performer levels

```text
open_exploration
context_complete
pinned
hermetic
controlled_measurement
```

### 54.2 Default software pattern

```text
explore dirty
submit source
rebuild and verify clean
accept verifier-controlled observations
```

### 54.3 Final build

Where a released binary is a deliverable, the verifier-controlled process SHOULD rebuild it from submitted source.

### 54.4 Experiment profile

A future experiment profile may require performer-hermetic execution when process history is part of the scientific claim.

## 55. Security

### 55.1 Capability classes

Examples:

```text
filesystem.read
filesystem.write_scoped
process.exec_scoped
network.fetch_allowlisted
git.read
git.commit
artifact.register
blut.plan_execute
laboratory.instrument_operate
external.service_call
secret.use_named
subagent.spawn
```

### 55.2 Default denial

An absent capability is denied.

Headless execution with no permitted approval path fails closed.

### 55.3 Version-anchored writes

File edits SHOULD cite the version observed. Drift produces a structured stale-state error.

### 55.4 Secrets

WAR source, canonical JSON, generated views, Katana logs, BLUT status, and gate output SHALL contain secret references, not values.

### 55.5 Network

```text
deny
allowlisted
provider_only
unrestricted
```

`unrestricted` requires elevated policy.

### 55.6 Classification

Dispatch classification SHALL not exceed actor, runtime, provider, tool, or destination policy.

### 55.7 Side-effect budget

A Stage SHALL declare:

- allowed side effects;
- spend;
- tool calls;
- model turns;
- wall time;
- egress;
- irreversible actions;
- compensation.

Budget exhaustion halts honestly.

## 56. Resolution

### 56.1 Requirements

Resolution SHALL verify:

- exact authorized Contract Revision;
- required deliverables exist;
- artifact digests verify;
- every required obligation is dispositioned;
- every required gate has admissible result;
- no required unknown remains;
- no blocker remains;
- deviations are dispositioned;
- required judgments exist;
- independence requirements are met;
- residual risks have sufficient authority;
- runtime receipts match the basis;
- resolver holds the role.

### 56.2 Record

```yaml
resolution:
  id: "uuidv7"
  common_outcome: "satisfied"
  profile_outcome: "delivered"
  contract_revision: 3
  contract_digest: "sha256:..."
  assurance_case_snapshot_digest: "sha256:..."
  artifact_manifest_digest: "sha256:..."
  gate_run_refs: []
  judgment_refs: []
  residual_risk_refs: []
  resolved_by_ref: "person://..."
  acting_role_ref: "role-assignment://..."
  meaning: "Accept the declared deliverables against the bounded obligations."
  effective_at: "..."
  recorded_at: "server-assigned"
  standing: "valid"
```

### 56.3 Falsification

`falsified` is appropriate only when the profile contains a falsifiable claim, such as an experiment or feasibility hypothesis.

An ordinary failed delivery is normally `not_satisfied`, `cancelled`, or remains blocked.

### 56.4 Dispute

A dispute identifies:

- challenged resolution;
- grounds;
- affected evidence or judgment;
- reliance policy;
- owner;
- required re-verification.

### 56.5 Annulment

Annulment records that the resolution may not be relied upon. The original resolution remains historical.

### 56.6 Supersession

Supersession records replacement, not invalidity.

## 57. Ongoing validation

A Warrant MAY define post-resolution monitors:

```yaml
monitors:
  - metric_ref: "telemetry://..."
    trigger_condition: "..."
    action: "open_superseding_war"
```

A trigger may:

- dispute a resolution;
- open investigation;
- open remediation;
- propose an ADR;
- create a superseding WAR;
- require re-verification.

Monitoring is distinct from the original completion proof unless the contract explicitly includes it.


---

# Part VI — File-native source, canonical IR, and preservation

## 58. Representations

| Representation | Role | Authoritative |
|---|---|---|
| Markdown/Liminal atom source | human authorship | under atom Holder |
| TOML composition manifest | v1 file-native composition source | under manifest Holder |
| Canonical WAR IR JSON | normative compiled machine meaning | derived from exact Basis |
| Generated WAR Markdown | normal human parent | no |
| ADR Overview Markdown | audit projection | no |
| Work Order Markdown | executor projection | no |
| Stage Dispatch JSON | actor contract projection | bound to Contract Revision |
| Stage Submission JSON | performer result envelope | runtime record |
| Assurance Case JSON/Markdown | proof projection | derived from evidence and judgments |
| Preservation package | long-term portable record | yes as preserved institutional package after acceptance |

## 59. Repository layout

The conventional layout is:

```text
openwarrant.toml
.openwarrant/
  cache/
  state/
docs/
  sas/
  roadmap/
  adr/
    atoms/
    generated/
      ADR_OVERVIEW.md
  warrants/
    LIM-WAR-0042/
      manifest.toml
      atoms/
        10-intent.md
        20-basis.md
        40-work-order.md
        45-milestones.yaml
        60-assurance.md
        80-validation.md
      journal/
        local-events.jsonl
      generated/
        WAR.md
        WAR.json
        WORK_ORDER.md
        ASSURANCE_CASE.md
        dispatch/
```

Paths are configurable. Semantics are not inferred from paths alone.

### 59.1 Tracked versus disposable

Tracked:

- configuration;
- manifest;
- authored atoms;
- local draft journal if repository policy permits;
- generated views if repository policy commits them;
- schema pins;
- receipt references.

Disposable:

- search index;
- parsed cache;
- model cache;
- temporary compilation outputs;
- fetched registry cache.

### 59.2 Generated view policy

A repository may choose:

```toml
[generated]
commit = true
verify_drift = true
```

or omit generated files from Git. The authority is unchanged.

## 60. Repository configuration

Example:

```toml
schema = "oh.war/repository-config/v1"

[project]
name = "Liminal"
namespace = "LIM"
knowledge_fabric_project_ref = "project://liminal"

[paths]
sas = "docs/sas"
roadmap = "docs/roadmap"
adrs = "docs/adr"
warrants = "docs/warrants"

[generated]
commit = true
verify_drift = true

[agent]
default_adapter = "katana"

[registry]
mode = "hybrid"
knowledge_fabric_endpoint = "http://localhost:4000"
```

## 61. Manifest

Example:

```toml
schema = "oh.war/manifest/v1"
uuid = "019c8f2d-7b4d-7c41-9cb7-2636e5f582ea"
local_alias = "LIM-WAR-0042"
enterprise_id = ""
title = "Implement crash-safe external-file repair"
profile = "delivery"
assurance_level = "controlled"

[[implements]]
ref = "sas://LIM-SAS-RQ-042"
contribution = "complete"

[[roadmap]]
ref = "roadmap://LIM-PHASE-1/M4"

[[parents]]
ref = "war://019c..."
contract_revision = 2

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"
required = true

[[atoms]]
ordinal = 20
role = "basis"
path = "atoms/20-basis.md"
required = true

[[atoms]]
ordinal = 30
role = "adr"
ref = "adr://019c..."
required = false

[[atoms]]
ordinal = 40
role = "work_order"
path = "atoms/40-work-order.md"
required = true

[[atoms]]
ordinal = 45
role = "milestones"
path = "atoms/45-milestones.yaml"
required = true

[[atoms]]
ordinal = 60
role = "assurance"
path = "atoms/60-assurance.md"
required = true
```

### 61.1 Manifest purpose

The manifest defines composition and relations. It does not duplicate the full semantic content of atoms.

### 61.2 Determinism

Ordinals SHALL be unique within one composition unless an explicit secondary order is defined.

### 61.3 No implicit latest

An authorized compilation SHALL resolve every atom and bound reference to an exact revision and digest.

## 62. Atom source format

The v1 source adapter uses Markdown with YAML frontmatter for prose-heavy authored atoms.

Example:

```markdown
---
schema: oh.war/atom/v1
warrant_uuid: 019c8f2d-7b4d-7c41-9cb7-2636e5f582ea
atom_uuid: 019c8f34-f984-7208-89cb-e31620ad8804
role: intent
jurisdiction: authored
holder:
  kind: git
order: 10
classification: internal
---

# Intent

## Objective

Implement crash-safe external-file repair.

## Non-goals

- Do not build the production parser.
- Do not add synchronization.
```

### 62.1 Structured atoms

Machine-dense atoms such as milestone graphs MAY use YAML or canonical JSON source.

### 62.2 Exact source preservation

The source bytes and content digest SHALL be preserved.

### 62.3 Frontmatter validation

Unknown required fields fail. Namespaced optional fields are preserved.

### 62.4 Source maps

The compiler SHALL retain source maps from canonical IR fields to atom and byte or syntax locations where supported.

## 63. Canonical WAR IR

The canonical IR is a typed semantic object, not a Markdown AST.

Readable top-level shape:

```yaml
api_version: "oh.war/v1"
kind: "work_authorization_record"

format_basis: {}
identity: {}
source_and_composition: {}
relations: {}
state_projection: {}
governance: {}
contract: {}
compilation_receipts: {}
milestones: {}
execution: {}
assurance_case: {}
resolution: {}
integrity: {}
extensions: {}
```

### 63.1 `format_basis`

Pins schema, vocabulary, profiles, relations, and state-machine versions.

### 63.2 `identity`

UUID, aliases, enterprise ID, title, profile, project, Holder, classification, retention.

### 63.3 `source_and_composition`

Workspace Basis, manifest, atoms, source digests, selectors, source maps, conversion loss.

### 63.4 `relations`

SAS, Roadmap, parent, child, ADR, Work Order, requirement, artifact, supersession, and other typed edges.

### 63.5 `state_projection`

Phase, condition, outcome, currency, standing, record version, projection action.

### 63.6 `governance`

Assurance, risk floor, actors, roles, independence, policies.

### 63.7 `contract`

The immutable authorized semantics.

### 63.8 `execution`

Dispatch and runtime receipt references, attempts, blockers, amendments, deviations, artifacts.

### 63.9 `assurance_case`

Obligations, gates, evidence, observations, inferences, judgments, dispositions.

### 63.10 `resolution`

Immutable resolution records, disputes, annulments, and current standing.

### 63.11 `integrity`

All relevant digests and checkpoints.

## 64. Format basis

```yaml
format_basis:
  package_id: "openwarrant-schema-pack"
  version: "1.0.0"
  digest:
    algorithm: "sha256"
    value: "..."
  root_schema_id: "work_authorization_record"
  profile_schema_id: "delivery"
```

The schema package SHALL transitively pin:

- core schema;
- profile schema;
- atom-role vocabulary;
- relation vocabulary;
- lifecycle vocabulary;
- action vocabulary;
- evidence vocabulary;
- gate schemas;
- Dispatch and Submission protocols.

## 65. Digest domains

A conforming implementation SHALL compute at least:

```text
atom_source_digest
manifest_digest
composition_revision_digest
workspace_basis_digest
semantic_graph_digest
contract_digest
context_manifest_digest
dispatch_digest
attempt_basis_digest
artifact_digest
gate_binding_digest
gate_run_digest
assurance_case_snapshot_digest
resolution_digest
war_export_digest
```

### 65.1 Algorithm

Cross-system WAR digests use SHA-256 unless a later protocol revision says otherwise.

Katana may return BLAKE3 identifiers for Katana-owned objects. The algorithm is always explicit.

### 65.2 Canonicalization

Canonical JSON uses RFC 8785.

Hashing SHALL operate on domain-separated preimages:

```json
{
  "digest_domain": "oh.war/contract/v1",
  "payload": { }
}
```

This prevents identical JSON shapes in different semantic domains from being confused.

## 66. Local draft journal

### 66.1 Purpose

File-native operation needs local milestone and drafting provenance before federation.

OpenWarrant MAY maintain an append-only local journal:

```text
journal/local-events.jsonl
```

### 66.2 Authority

Before KF registration, the journal is repository-local draft history.

After registration, Knowledge Fabric actions are authoritative. The local journal becomes a cache of action requests and receipts and SHALL not become a competing ledger.

### 66.3 Event envelope

```json
{
  "v": 1,
  "id": "uuidv7",
  "warrant_uuid": "uuid",
  "type": "milestone.progress_recorded",
  "actor_ref": "local://git-user",
  "occurred_at": "2026-08-19T12:00:00Z",
  "payload": {},
  "idempotency_key": "..."
}
```

The local clock is not authoritative `recorded_at`.

### 66.4 Material events

Examples:

```text
draft.created
draft.revised
proposal.created
milestone.started
milestone.progress_recorded
blocker.opened
blocker.resolved
stage.submission_recorded
artifact.registered
sync.receipt_attached
```

Detailed Katana or BLUT runtime events remain in those runtimes.

## 67. Knowledge Fabric controlled actions

Initial action vocabulary:

### Contract

```text
create_warrant_draft
revise_warrant_draft
submit_warrant
authorize_warrant_contract
withdraw_warrant_proposal
propose_warrant_amendment
authorize_warrant_amendment
reject_warrant_amendment
```

### Execution

```text
record_warrant_preflight
authorize_warrant_dispatch
attach_warrant_runtime_receipt
register_warrant_submission
open_warrant_blocker
resolve_warrant_blocker
pause_warrant
resume_warrant
propose_warrant_deviation
approve_warrant_deviation
reject_warrant_deviation
record_warrant_discovered_gap
```

### Evidence

```text
register_warrant_artifact
register_warrant_evidence
attach_warrant_gate_run
record_warrant_inference
record_warrant_judgment
request_warrant_resolution
```

### Terminal and administrative

```text
resolve_warrant
dispute_warrant_resolution
resolve_warrant_dispute
annul_warrant_resolution
supersede_warrant
deprecate_warrant
```

### 67.1 Action envelope

Every controlled action includes:

```yaml
action_type: "..."
actor_id: "..."
acting_role_id: "..."
organization_id: "..."
target_ids: []
payload: {}
reason: "..."
idempotency_key: "..."
request_id: "..."
expected_version: 12
effective_at: "..."
max_classification: "..."
```

### 67.2 Server time

Knowledge Fabric assigns `recorded_at`.

### 67.3 Optimistic concurrency

A controlled mutation cites the row or object version it read. Drift fails rather than overwrites.

### 67.4 Idempotency

Equivalent retries replay the first committed result. Conflicting reuse of an idempotency key is rejected.

## 68. Portable preservation document

### 68.1 One-file canonical export

The definitive portable WAR is one canonical JSON document.

It SHALL embed small records and use content-addressed references for large evidence bytes.

### 68.2 Export contents

- complete identity;
- source manifest;
- exact atom revisions and digests;
- Compilation Basis;
- canonical IR;
- contract revisions;
- actions and relevant audit receipts;
- ADR refs and accepted bodies where policy permits;
- runtime receipt refs;
- artifacts;
- assurance case;
- resolution and standing;
- evidence manifest;
- schema and compiler identity;
- optional signatures and checkpoints.

### 68.3 Round trip

Export into an empty compatible Knowledge Fabric instance, reconnect preserved bytes, re-export, and compare SHALL preserve semantic and digest identity.

## 69. Protocol versioning

### 69.1 Semantic versioning

Schema packs use semantic versioning.

### 69.2 Additive evolution

Minor versions MAY add optional fields and namespaced extensions.

### 69.3 Breaking change

A breaking semantic or required-field change requires a major protocol version and an ADR.

### 69.4 Unknown extensions

Unknown optional namespaced extensions are preserved. Unknown required extensions fail closed.


---

# Part VII — OpenWarrant CLI and agent-assisted planning

## 70. CLI thesis

The first useful OpenWarrant product is a simple Rust CLI that turns vague intent into a validated draft WAR and compiles it into one complete document.

The CLI SHALL remain useful without a Knowledge Fabric server.

The primary loop is:

```text
describe work
    ↓
agent proposes structured atoms
    ↓
deterministic validator
    ↓
human or policy reviews diff
    ↓
draft atoms written
    ↓
WAR compiled
```

The agent is a drafter, not the authority.

## 71. Initial commands

### 71.1 `war init`

Initialize repository configuration and directories.

```bash
war init --namespace LIM
```

### 71.2 `war new`

Create a blank or template-driven draft.

```bash
war new "Implement crash-safe repair"
```

### 71.3 `war plan`

Create a draft through an agent-assisted planning workflow.

```bash
war plan \
  "Implement crash-safe external-file repair and prove recovery after every durable boundary"
```

Options:

```text
--agent katana
--profile delivery
--assurance controlled
--against-sas <ref>
--roadmap <ref>
--parent <WAR>
--interview
--apply
--no-write
```

### 71.4 `war interview`

Ask only unresolved, high-information questions.

```bash
war interview LIM-WAR-0042
```

The interview engine SHOULD prioritize questions about:

- desired outcome;
- scope;
- non-goals;
- architecture decision;
- authority;
- evidence;
- rollback;
- ambiguity;
- risk.

### 71.5 `war edit`

Edit one authored atom.

```bash
war edit LIM-WAR-0042 intent
war edit LIM-WAR-0042 work-order
```

Agent-assisted atom edit:

```bash
war edit LIM-WAR-0042 work-order \
  --agent "split the recovery milestone into intent logging and replay"
```

### 71.6 `war adr`

ADR operations:

```bash
war adr new --warrant LIM-WAR-0042
war adr propose --warrant LIM-WAR-0042 "Use one repair interpreter"
war adr check
war adr overview
```

### 71.7 `war check`

Deterministic validation.

```bash
war check LIM-WAR-0042
```

Example:

```text
PASS  format basis
PASS  manifest
PASS  atom composition
PASS  milestone graph
PASS  stage ports
PASS  SAS references
WARN  ASM-002 is accepted residual risk
ERROR OBL-003 references missing Gate Binding GB-003

NOT READY
```

### 71.8 `war compile`

Compile all configured projections.

```bash
war compile LIM-WAR-0042
```

Outputs may include:

```text
generated/WAR.md
generated/WAR.json
generated/WORK_ORDER.md
generated/ASSURANCE_CASE.md
```

### 71.9 `war show`

Render or print a view.

```bash
war show LIM-WAR-0042
war show LIM-WAR-0042 --view work-order
war show LIM-WAR-0042 --view assurance
```

### 71.10 `war diff`

Semantic difference between revisions or Bases.

```bash
war diff LIM-WAR-0042 --from contract:2 --to contract:3
```

### 71.11 `war child`

Create a child WAR inheriting selected parent context.

```bash
war child LIM-WAR-0042 "Implement ILRP replay"
```

### 71.12 `war supersede`

Draft a replacement WAR and relation.

```bash
war supersede LIM-WAR-0042 \
  "Replace the original repair strategy"
```

This command drafts. It does not authorize supersession.

## 72. Federated commands

### 72.1 `war register`

Register a local draft with Knowledge Fabric.

```bash
war register LIM-WAR-0042
```

### 72.2 `war sync`

Synchronize identity, lifecycle, actions, receipts, and generated projections.

```bash
war sync LIM-WAR-0042
```

### 72.3 `war propose`

Submit through the Knowledge Fabric action surface.

```bash
war propose LIM-WAR-0042
```

### 72.4 `war authorize`

Request or perform authorization under the caller’s KF role.

```bash
war authorize LIM-WAR-0042
```

The CLI SHALL not bypass KF authority.

### 72.5 `war status`

Show authoritative lifecycle and local sync state.

```bash
war status LIM-WAR-0042
```

### 72.6 `war milestone`

```bash
war milestone list LIM-WAR-0042
war milestone start LIM-WAR-0042 M2
war milestone progress LIM-WAR-0042 M2 --note "..."
```

Status is derived from actions, obligations, and receipts. A user does not directly set `complete`.

## 73. Execution commands

### 73.1 `war preflight`

```bash
war preflight LIM-WAR-0042
war preflight LIM-WAR-0042 --stage STAGE-002
```

### 73.2 `war dispatch`

```bash
war dispatch LIM-WAR-0042 STAGE-002 --executor katana
war dispatch LIM-WAR-0042 STAGE-005 --executor blut
```

### 73.3 `war submit`

Attach a Stage Submission from a human or external executor.

```bash
war submit LIM-WAR-0042 STAGE-002 submission.json
```

### 73.4 `war verify`

Request Gate Runs and compile the assurance case.

```bash
war verify LIM-WAR-0042
```

### 73.5 `war resolve`

Request resolution.

```bash
war resolve LIM-WAR-0042
```

The command invokes a typed KF action. It does not edit a status line.

### 73.6 `war dispute`

```bash
war dispute LIM-WAR-0042 --resolution RES-001 --reason "Gate version invalidated"
```

### 73.7 `war export`

```bash
war export LIM-WAR-0042 --canonical-json
war export LIM-WAR-0042 --preservation-package
```

## 74. Agent-assisted planning

### 74.1 Agent inputs

`war plan` may provide the drafting agent with:

- user request;
- relevant SAS atoms;
- Roadmap items;
- existing ADRs;
- existing WARs;
- repository map;
- code search results;
- tests;
- configured policy;
- schema;
- risk rubric.

### 74.2 Agent output

The agent SHALL return a structured Draft Proposal, not arbitrary file writes.

```yaml
api_version: "oh.war/draft-proposal/v1"
proposed_identity: {}
atom_operations: []
proposed_adr_drafts: []
proposed_relations: []
assumptions: []
unresolved_questions: []
risk_assessment: {}
adequacy_attacks: []
diagnostics: []
```

### 74.3 Atom operations

```text
create_atom
revise_atom
retire_atom
add_binding
remove_binding
add_relation
propose_adr
```

### 74.4 Validation before application

The CLI SHALL:

1. parse the proposal;
2. validate schema;
3. validate semantic references;
4. run risk and authority checks;
5. show a semantic diff;
6. require review or policy approval;
7. write authored atoms;
8. compile and check.

### 74.5 No direct model mutation

The model does not receive unrestricted filesystem writes through OpenWarrant’s planning mode.

Katana may have its own capability-gated editing tools during an execution stage. Planning proposals still return through the structured WAR protocol.

### 74.6 Interview generation

The agent may propose interview questions. OpenWarrant SHOULD rank them by expected information gain and ask the minimum set needed to remove blockers.

### 74.7 Decision detection

If the planner identifies a choice among durable alternatives, it SHALL produce a proposed ADR draft, not bury the choice in a Work Order atom.

### 74.8 Evidence honesty

The planner SHALL distinguish:

- existing evidence;
- evidence it expects to collect;
- assumptions;
- recommendations;
- unknowns.

It SHALL never fabricate a source or gate result.

## 75. Agent adapter protocol

### 75.1 Preferred adapter

Katana is the preferred agent adapter.

### 75.2 Generic process seam

OpenWarrant MAY support:

```text
war-agent --protocol oh.war/agent-drafter/v1
```

Request: one canonical JSON object on stdin.

Response: one canonical Draft Proposal on stdout.

Stderr: bounded diagnostics.

### 75.3 Trait shape

Illustrative Rust interface:

```rust
pub trait Drafter {
    fn draft(&self, request: DraftRequest) -> Result<DraftProposal, DraftError>;
}
```

The semantic contract is the JSON protocol, not the Rust ABI.

### 75.4 Adapter isolation

An adapter SHALL have no authority to authorize, resolve, allocate enterprise identity, or mutate Knowledge Fabric outside typed actions.

## 76. CLI ergonomics

### 76.1 Fast path

The common workflow SHOULD be:

```bash
war plan "Add scalar portability without changing the wire format"
war check <WAR>
war show <WAR>
git add docs/warrants
```

### 76.2 Explicit diagnostics

Errors SHALL identify:

- file or atom;
- semantic path;
- source location;
- violated requirement;
- likely remediation.

### 76.3 Silence on sound state

`war check --quiet` SHALL emit no output and exit zero when the selected scope is sound.

### 76.4 Machine output

Every command SHOULD support:

```text
--json
```

with a stable versioned result schema.

### 76.5 Noninteractive mode

CI and agents SHALL have a fully noninteractive mode. Missing authority or required clarification fails closed rather than hanging for input.


---

# Part VIII — Implementation architecture

## 77. Language decision

The OpenWarrant core and CLI SHALL be implemented in Rust.

Knowledge Fabric integrations MAY be implemented in TypeScript, but SHALL consume the versioned canonical protocol and generated schemas rather than reimplement WAR semantics independently.

### 77.1 Why Rust

The core requires:

- strong typed enums;
- deterministic compilation;
- graph validation;
- content hashing;
- filesystem safety;
- a durable CLI;
- integration with Liminal, Katana, and BLUT;
- cross-platform binaries;
- low operational dependency.

### 77.2 Why not TypeScript core

A TypeScript core would place the canonical WAR semantics in the Knowledge Fabric application ecosystem while Katana, BLUT, and Liminal remain Rust. A second Rust implementation would then be likely.

The architecture prohibits two semantic owners.

### 77.3 TypeScript role

The KF TypeScript side should own:

- API routes;
- typed actions;
- authorization;
- database materialization;
- generated protocol types;
- process invocation;
- UI projections.

It SHALL not invent a second WAR parser or validator.

## 78. Repository structure

Recommended initial structure:

```text
OpenWarrant/
├── Cargo.toml
├── rust-toolchain.toml
├── openwarrant.toml.example
├── crates/
│   ├── openwarrant-types/
│   ├── openwarrant-schema/
│   ├── openwarrant-source/
│   ├── openwarrant-compiler/
│   ├── openwarrant-protocol/
│   ├── openwarrant-agent/
│   ├── openwarrant-kf/
│   ├── openwarrant-katana/
│   ├── openwarrant-blut/
│   └── openwarrant-cli/
├── schemas/
├── profiles/
├── templates/
├── conformance/
├── examples/
└── docs/
```

The first implementation SHOULD begin with fewer crates and split only at stable authority seams.

Recommended v0:

```text
openwarrant-core
openwarrant-compiler
openwarrant-agent
openwarrant-cli
```

## 79. Crate responsibilities

### 79.1 `openwarrant-core`

Pure domain types and validators:

- identifiers;
- manifests;
- atom roles;
- canonical WAR IR;
- lifecycle;
- milestones;
- stages;
- obligations;
- assurance;
- relations;
- semantic diagnostics.

Avoid I/O where practical.

### 79.2 `openwarrant-compiler`

- resolve source manifest;
- parse source adapter output;
- build composition;
- lower to canonical IR;
- canonicalize;
- hash;
- render views;
- compile Dispatches;
- emit source maps.

### 79.3 `openwarrant-agent`

- agent request/response protocol;
- Katana drafter adapter;
- generic command adapter;
- proposal validation;
- interview orchestration.

No model provider or agent loop.

### 79.4 `openwarrant-cli`

- `clap` command surface;
- repository discovery;
- file-safe writes;
- diagnostics;
- editor launch;
- orchestration;
- process adapters.

### 79.5 Later protocol crates

Stable protocol-only crates MAY be split and permissively licensed if project policy chooses.

## 80. Recommended Rust stack

Nonbinding initial recommendations:

| Need | Candidate |
|---|---|
| CLI | `clap` |
| serialization | `serde`, `serde_json`, `toml` / `toml_edit`, `serde_yaml` or a safer maintained YAML parser |
| diagnostics | `miette`, `thiserror` |
| Markdown | `comrak` or `pulldown-cmark` with explicit frontmatter handling |
| UUIDv7 | `uuid` |
| hashing | `sha2`, `blake3` for runtime adapters |
| canonical JSON | audited RFC 8785 implementation |
| paths | `camino` |
| URLs/refs | `url` |
| graph validation | `petgraph` or small explicit DAG implementation |
| process | `tokio::process` or `std::process` according to async needs |
| JSON Schema | generated schemas plus a conforming validator |
| testing | `proptest`, snapshot tests, mutation tests where useful |

Library selection is an implementation ADR when it becomes binding.

## 81. Canonical compiler interface

Illustrative interface:

```rust
pub trait WarrantCompiler {
    fn compile(
        &self,
        request: CompilationRequest,
    ) -> Result<CompilationResult, CompilationError>;
}
```

### 81.1 Request

Includes:

- protocol;
- Source Holder snapshots;
- manifest;
- supplied source bytes;
- bound records;
- Workspace Basis;
- policies;
- requested targets;
- schema pack.

### 81.2 Result

Includes:

- canonical WAR IR;
- semantic digest;
- dependency digest;
- diagnostics;
- unresolved refs;
- omitted subgraphs;
- source maps;
- conversion loss;
- compiled views.

## 82. Source adapters

### 82.1 Markdown v1 adapter

The first adapter handles the constrained WAR atom profile.

### 82.2 Liminal adapter

The final adapter invokes a pinned Liminal compiler profile through a versioned process protocol.

Illustrative command:

```text
liminal-compiler --protocol oh.war/liminal-v1
```

### 82.3 Adapter parity

Before cutover, the Markdown compatibility corpus SHALL be compiled by both adapters and compared for declared observable parity.

### 82.4 Cutover

Once Liminal is qualified:

- Liminal becomes production semantic compiler;
- the Markdown adapter becomes importer/test adapter;
- one production definition remains.

## 83. Knowledge Fabric integration

### 83.1 Process boundary

KF SHOULD initially invoke an exact pinned OpenWarrant binary through canonical JSON.

### 83.2 Runtime pin

Production execution SHOULD pin:

- OpenWarrant commit;
- Cargo.lock digest;
- executable digest;
- runtime closure digest;
- protocol version;
- qualification receipt.

### 83.3 Sandbox

The compiler process should run:

- without database credentials;
- without network;
- without ambient source discovery;
- with bounded input/output/diagnostics;
- against supplied authorized bytes and records.

### 83.4 Generated TypeScript types

JSON Schema and OpenAPI generation SHALL produce KF-facing types. Generated types are projections of the Rust-owned protocol.

### 83.5 Database ownership

KF stores normalized query fields and immutable canonical snapshots. It does not store WAR only as a generic JSON blob.

## 84. Database projection

Recommended KF records:

```text
warrant
warrant_contract_revision
warrant_atom_reference
warrant_milestone
warrant_stage
warrant_stage_edge
warrant_acceptance_obligation
warrant_gate_binding
warrant_runtime_receipt_reference
warrant_submission
warrant_blocker
warrant_deviation
warrant_evidence
warrant_observation
warrant_inference
warrant_judgment
warrant_resolution
warrant_resolution_dispute
```

Common query fields:

```text
profile
assurance_level
phase
condition
outcome
currency
standing
owner_ref
scope_ref
classification
authorized_contract_revision
record_version
```

## 85. Runtime receipt protocols

OpenWarrant SHALL define narrow receipt adapters:

```text
oh.war/katana-receipt/v1
oh.war/blut-receipt/v1
oh.war/liminal-compilation-receipt/v1
oh.war/gate-run-receipt/v1
```

Each adapter preserves the native runtime identity and digest rather than reproducing its entire log.

## 86. Transactional behavior

A KF controlled action SHALL:

1. validate idempotency;
2. lock or version-check targets;
3. validate role and preconditions;
4. apply typed state changes;
5. append audit;
6. write outbox;
7. commit atomically.

OpenWarrant file-native commands SHALL use:

- temporary files;
- fsync where required by policy;
- atomic rename;
- prestate digest checks;
- no partial generated parent publication.

Cross-Holder changes SHALL eventually use Liminal RepairPlan and ILRP rather than an OpenWarrant-specific mutation coordinator.

## 87. Security boundary

### 87.1 Untrusted sources

Repository text, external context, model output, and plugin output are untrusted inputs to parsers and agents.

### 87.2 Parser behavior

Parsers SHALL:

- bound input;
- reject malformed encodings;
- avoid implicit code execution;
- preserve unknown extensions;
- emit typed diagnostics;
- avoid path traversal;
- avoid symlink races in controlled writes.

### 87.3 Agent proposal boundary

Agent output is data. It is validated before application.

### 87.4 Generated parent injection

Generated views SHALL escape or safely render source content according to target format. A source atom SHALL not gain execution authority through rendering.

### 87.5 External references

Fetches require explicit network capability, digest verification where pinned, classification policy, and provenance.

## 88. Performance

Performance is secondary to semantic correctness in early versions.

The compiler SHOULD still support:

- incremental atom parsing;
- content-addressed cache;
- dependency-based recompilation;
- deterministic parallel reads;
- no full-repository model context by default;
- compact stage-specific context;
- generated-view cache keyed by Basis and target.

No performance optimization may change semantic output without an ADR and differential conformance.

## 89. Observability

The CLI and KF SHALL emit or derive:

- compile duration;
- context item count;
- included/omitted context;
- model usage;
- human authoring time;
- interview count;
- amendments;
- escalations;
- attempts;
- gate results;
- runtime cost;
- time to usable artifact;
- untracked work;
- post-resolution escapes.

These metrics are evidence about system economics, not proof that a particular WAR is correct.


---

# Part IX — Validation, migration, and delivery

## 90. Conformance philosophy

A capability is complete only when:

1. its positive behavior passes;
2. planted violations fail for the intended reason;
3. unknown and infrastructure failures remain distinguishable;
4. canonical outputs are deterministic;
5. preservation round trips.

## 91. Core conformance suite

### 91.1 Canonicalization

1. Compile the same valid Basis on two supported hosts; assert byte-identical canonical WAR IR.
2. YAML/TOML/Markdown source lowers to stable IR and re-renders deterministically.
3. Generated Markdown changes do not change the contract digest.
4. Unknown required fields fail closed.
5. Optional namespaced extensions survive round trip.
6. Different digest domains produce different preimages.

### 91.2 Atom and composition

7. Missing required atom fails.
8. Duplicate ordinal fails.
9. Unknown required role fails.
10. Generated atom cannot be edited through an authored-source command.
11. Parent edit is refused or maps unambiguously to an authored atom.
12. Composition cycle fails.
13. Bound atom without exact revision fails authorization.
14. Source Holder ambiguity fails.
15. Higher-classification input raises effective classification.
16. Recompilation detects generated drift.

### 91.3 Federation and identity

17. UUID survives local alias and enterprise allocation.
18. Two repositories with the same local alias do not collide.
19. Offline provisional identity registers without renaming the UUID.
20. Official enterprise ID cannot be fabricated locally.
21. Holder registration does not transfer source authority.
22. Cross-repository relation resolves through KF federation.

### 91.4 ADR

23. A normative decision with no ADR fails contract validation.
24. A local choice inside autonomy does not require a new ADR.
25. ADR supersession is acyclic.
26. ADR Overview regenerates deterministically.
27. No manually maintained ADR Overview may drift from source ADRs.
28. A proposed ADR cannot satisfy an accepted-decision prerequisite.

### 91.5 Parent and child

29. Child references exact parent contract revision.
30. Parent source is unchanged when child state changes.
31. Parent generated view lists child.
32. Child cannot silently replace parent rationale.
33. Superseding WAR makes old currency `superseded`.
34. Superseded WAR remains exportable.
35. Adopted unresolved children are explicit.

### 91.6 Lifecycle

36. Illegal phase transition fails.
37. Blocking does not erase phase.
38. Material amendment returns to authorized and requires preflight.
39. Attempt retains old contract digest after amendment.
40. Dispute preserves original resolution.
41. Annulment changes standing, not historical outcome.
42. Supersession does not imply annulment.

### 91.7 Graph and execution

43. Milestone cycle fails.
44. Stage cycle fails.
45. Missing input port fails.
46. Type-incompatible port fails.
47. Unsupported BLUT lowering fails rather than degrades.
48. Dispatch omits irrelevant context but never required context.
49. Dispatch digest changes when prior failure evidence is added.
50. Runtime receipt with wrong Dispatch digest is rejected.
51. Capability realization wider than authorization is rejected.

### 91.8 Agent planning

52. Malformed Draft Proposal fails before file writes.
53. Agent cannot authorize.
54. Agent cannot allocate enterprise ID.
55. Agent-invented source reference is unresolved and blocks.
56. Proposed decision becomes ADR draft.
57. Agent proposal diff is deterministic from response bytes.
58. Noninteractive missing clarification fails closed.

### 91.9 Attempts

59. Replay with changed basis is rejected as replay.
60. Repair includes prior failure evidence.
61. Restart requires authority where policy says so.
62. Idempotent side-effect retry applies once.
63. Infrastructure replay does not duplicate artifact registration.

### 91.10 Gates and adequacy

64. Missing gate version produces unknown.
65. Zero selected tests produces invalid/unknown.
66. Missing tool is not target fail.
67. Timeout is not pass.
68. Infrastructure error is not target fail.
69. Performer report cannot satisfy independent gate.
70. Protected fixture modification is refused.
71. Negative-control violation is detected.
72. A planted artifact that passes while violating an obligation fails adequacy review.
73. A sampled result cannot establish a universal claim.
74. Circular assumption/evidence graph fails.
75. Invalidated gate disputes dependent resolution.

### 91.11 Evidence

76. Evidence digest corruption is detected.
77. Observation with no method fails.
78. Inference with no premises fails.
79. Judgment with no actor, role, meaning, or basis fails.
80. Derived report with no raw lineage fails required evidence policy.
81. Actor-supplied `recorded_at` is ignored or rejected.

### 91.12 Resolution

82. Missing deliverable blocks.
83. Required unknown gate blocks.
84. Open blocker blocks.
85. Unapproved deviation blocks.
86. Unresolved required obligation blocks.
87. Missing risk authority blocks.
88. Performer self-resolution fails under separation policy.
89. Valid basic automated resolution uses a distinct policy identity.
90. Resolution digest verifies after export/import.

### 91.13 Preservation

91. Export-import-export is byte- or semantic-digest stable as defined by protocol.
92. Every referenced large object is included or independently retrievable and digest verified.
93. Schema, compiler, policy, gate, contract, and receipt identities survive round trip.
94. Generated views rebuild from the package.
95. Superseded and annulled history remains traversable.

## 92. Aggregate gate

The repository SHOULD expose:

```bash
just ci
just conformance
```

or:

```bash
cargo xtask gate
```

The final command SHALL exit zero only when every positive fixture passes and every planted violation is rejected by the intended control.

## 93. Dogfooding requirement

OpenWarrant SHALL be built through WARs as soon as the v0 CLI can create and compile them.

At minimum:

- the OpenWarrant SAS implementation;
- canonical IR;
- agent planning;
- KF registration;
- Liminal adapter;
- Katana adapter;
- BLUT adapter;

each receive WARs linked to this SAS.

## 94. Telemetry and unit economics

The system SHALL measure:

- human authoring minutes;
- interview questions;
- clarification count;
- escalation count and class;
- amendments;
- auto-authorizable fraction;
- replay, repair, restart;
- gate failure cause;
- adequacy counterexamples;
- wall time;
- compute and model cost;
- time to first usable artifact;
- reopenings;
- untracked commits or artifacts;
- work completed outside WAR;
- evidence and gate reuse.

Derived metrics:

```text
human control minutes per accepted WAR
amendments per WAR
safe auto-amendment fraction
gate-failure-to-repair success rate
post-resolution escape rate
untracked-work rate
adequacy-review catch rate
gate-library reuse rate
```

Assurance defaults and amendment policy SHOULD be tuned from measured distributions.

## 95. Untracked-work detection

Tracked commits, pull requests, artifacts, and controlled test records SHOULD carry:

```text
WAR UUID or enterprise ID
contract digest
Dispatch ID
runtime run ID
```

Changes to a tracked scope with no WAR relation become candidates for `untracked_work_detected`.

This is a diagnostic and governance signal. It SHALL not fabricate a relationship after the fact without review.

## 96. Existing ADR migration

### 96.1 Preserve bytes

Every existing ADR body remains preserved as an authored source revision.

### 96.2 Map semantics

| Existing element | New meaning |
|---|---|
| status | historical lifecycle evidence |
| Context | facts and context candidates |
| Decision | ADR decision |
| Rationale | rationale graph candidates |
| Alternatives | option candidates |
| Consequences | consequence nodes |
| Implementation Plan | candidate Work Order contract |
| `gate_cmd` | unqualified local gate candidate |
| Progress Log | progress events and performer observations |
| Completion | historical resolution claim |
| Validation | ongoing-validation candidates |
| supersedes/amends/extends | typed ADR relations |

### 96.3 No fabricated proof

A textual gate command becomes:

```text
legacy_declared_unqualified
```

until it is parsed, askable, bound, executed, and supported by a Gate Run receipt.

A legacy `Complete` line with no admissible evidence remains a historical claim, not a newly verified WAR resolution.

### 96.4 Preserve unknown classes

Migration preserves:

```text
malformed
foreign_working_directory
missing_tool
missing_script
missing_crate
mutating
timeout
failed
passed
not_run
```

It SHALL not collapse “could not ask” into “failed.”

## 97. Existing atom/parent migration

### 97.1 Adopt, do not replace

Current authored atoms remain sources.

### 97.2 Add typed manifests

Existing parent lists or composition manifests are translated into WAR/Liminal typed composition.

### 97.3 Mark generated parents

Current parents become explicit generated projections and receive drift gates.

### 97.4 Source maps

Where possible, preserve mapping from generated parent sections to original atoms.

### 97.5 Cutover

The old compiler remains a compatibility oracle during measured parity. After acceptance, one production compiler remains.

## 98. Implementation phases

### Phase 0 — Telemetry shim

Deliver:

- WAR UUID and local alias;
- commit/PR linkage;
- lightweight event logging;
- amendment, escalation, and gate-result classification;
- untracked-work detection.

Exit:

- real distributions for authoring cost, amendment types, and failure causes.

### Phase 1 — File-native WAR compiler

Deliver:

- `war init`;
- `war new`;
- manifest;
- authored atom profile;
- canonical IR;
- `war check`;
- `war compile`;
- full Markdown parent;
- canonical JSON;
- generated drift gate.

Exit:

- OpenWarrant development uses WARs.

### Phase 2 — Agent planner

Deliver:

- `war plan`;
- interview;
- Draft Proposal protocol;
- Katana drafter adapter;
- semantic diff;
- proposed ADR generation.

Exit:

- a vague engineering request produces a reviewable valid draft without direct model file mutation.

### Phase 3 — ADR federation

Deliver:

- first-class ADR atoms;
- local and global identity;
- ADR Overview;
- WAR/ADR relations;
- existing ADR importer.

Exit:

- no managed normative decision exists only inline.

### Phase 4 — Knowledge Fabric registration

Deliver:

- typed KF actions;
- global allocation;
- lifecycle;
- contract revisions;
- synchronization;
- audit;
- preservation.

Exit:

- registered WARs use KF as institutional authority while Git may remain Source Holder.

### Phase 5 — Dispatch and Katana execution

Deliver:

- Preflight;
- Stage Dispatch;
- Katana runtime receipt;
- Stage Submission;
- attempts;
- blockers and deviations.

Exit:

- one WAR stage can be compiled, executed by a stateless Katana agent, and returned without authority confusion.

### Phase 6 — Gate Registry and assurance case

Deliver:

- Gate Definitions;
- qualifications;
- bindings;
- runs;
- evidence;
- observations;
- inferences;
- judgments;
- adequacy review;
- resolution.

Exit:

- a delivery can close only through bounded, provenance-preserving proof.

### Phase 7 — BLUT adapter

Deliver:

- named-port stage graph;
- PlanSpec lowering;
- resources;
- artifacts;
- BLUT lineage receipt.

Exit:

- compatible computational WARs execute without duplicating BLUT.

### Phase 8 — Liminal production compiler

Deliver:

- WAR Liminal profile;
- exact-source CST/HIR/CIR path as available;
- Workspace Basis;
- Jurisdiction;
- source maps;
- human and AI targets;
- adapter parity;
- cutover.

Exit:

- Liminal is the single production document semantic compiler.

### Phase 9 — High-assurance controls

Deliver as required:

- signatures;
- audit checkpoints;
- controlled evidence custody;
- physical test profile;
- independent human workflow;
- invalidation propagation;
- regulatory mapping.

Exit:

- a resolution is signed, its evidence custody is audited, and one gate invalidation propagates to every dependent resolution, with no step performed by the actor who produced the work.

### Phase 10 — Contractor Work Order profile

Deliver only after separate legal, finance, and QMS decisions:

- contractor profile;
- Work Order mapping;
- acceptance;
- invoices and payments;
- signatures and legal terms.

The technical WAR core remains unchanged.

Exit:

- a contractor Work Order compiles through the unchanged technical WAR core, and acceptance, invoicing and legal terms live entirely in the profile.

## 99. System acceptance criteria

The WAR system is acceptable when:

1. a human can create a draft from one sentence;
2. the planner asks only unresolved high-value questions;
3. the agent outputs structured proposals;
4. authored atoms remain the editable sources;
5. the parent is one complete generated document;
6. every generated section has provenance;
7. every normative decision is an ADR;
8. all ADRs compile into the audit overview;
9. WARs reference SAS and Roadmap;
10. child WARs inherit exact parent context without rewriting it;
11. superseded WARs remain honest history;
12. local drafting works offline;
13. KF registration adds global authority without stealing Git source authority;
14. a stateless actor can execute one Dispatch;
15. Katana authority is not duplicated;
16. BLUT authority is not duplicated;
17. Liminal authority is not duplicated;
18. performer claims cannot become independent evidence;
19. unaskable gates cannot pass;
20. the assurance case separates evidence, observation, inference, judgment, and resolution;
21. a material amendment never changes prior attempt basis;
22. resolution requires the exact authorized contract;
23. dispute and annulment preserve history;
24. one canonical JSON export preserves the full Warrant;
25. basic WAR overhead is low enough that bypass is irrational.

## 100. Success metrics

OpenWarrant succeeds when it measurably reduces:

- context lost between humans and agents;
- clarification turns;
- untracked implementation decisions;
- stale gate commands;
- false completion claims;
- manual document synchronization;
- repeated context assembly;
- human time per accepted unit;
- post-resolution surprises.

It also succeeds when it increases:

- stage completion rate;
- evidence reuse;
- gate reuse;
- repair success;
- traceability from SAS to artifact;
- truthful unknown reporting;
- ability to hand work between agents and humans.


---

# Part X — Governance and appendices

## 101. Governance of this SAS

### 101.1 Controlled document

This SAS SHALL become a controlled Knowledge Fabric/Liminal document.

### 101.2 Revisions

Accepted revisions are immutable.

### 101.3 Architecture-changing revision

A revision that changes protocol meaning, ownership, required semantics, state, authority, or compatibility requires an ADR.

### 101.4 Nonsemantic correction

Typographical, formatting, citation, and clearly nonsemantic corrections use controlled document revision history and do not require a new architecture ADR.

### 101.5 Enterprise identity

The official document identifier SHALL be allocated through the OpenHuman Identifier Registry. Until then, this file has no official enterprise ID.

### 101.6 Normative source

After adoption, the accepted controlled revision is normative. Generated copies, repository mirrors, and exports state the exact accepted revision and digest.

## 102. Resolved architecture decisions

The following choices were fixed before this draft.

| # | Decision |
|---:|---|
| 1 | WAR means Work Authorization Record; human name Warrant |
| 2 | Architecture supports all bounded institutional work; v1 optimizes for software and agents |
| 3 | ADR remains Architecture Decision Record |
| 4 | KF federates repository/subsystem WARs and owns global authority; source may remain per-repository |
| 5 | WAR is one logical parent compiled from atoms; parents are not edited |
| 6 | Every atom declares authored, bound, or generated Jurisdiction class |
| 7 | Composition is semantic and ordered, not canonical raw concatenation |
| 8 | Hierarchy is Vision → SAS and Roadmap → WAR → Milestone → Stage/Dispatch → Artifact/Evidence → Resolution |
| 9 | Milestones are acceptance checkpoints; stages are dispatchable execution nodes |
| 10 | Child WARs retain parent context/rationale; superseding WARs replace and deprecate old WARs for new use |
| 11 | Every normative decision is a first-class ADR; ADR Overview is generated |
| 12 | WAR does not yet replace authoritative contractor Work Orders |
| 13 | State uses phase, condition, outcome, currency, and standing |
| 14 | Assurance level and executor tier are orthogonal |
| 15 | Agents may draft, execute, report, and review but may not self-authorize or self-resolve |
| 16 | KF owns the Gate Registry; OpenWarrant owns schemas and CLI support |
| 17 | Amendment classes are local, auto-authorized revision, and manual revision |
| 18 | Liminal, KF, Katana, and BLUT retain their kernels |
| 19 | CLI is hybrid offline Git-native plus KF federation |
| 20 | Define Liminal protocol now; ship constrained Markdown adapter first |
| 21 | UUIDv7 internal identity plus local alias and future official registry ID |
| 22 | Canonical JSON is portable machine document; generated Markdown is human parent |
| 23 | SAS is controlled; architecture changes require ADR |
| 24 | First deliverable is one definitive WAR SAS document |

## 103. Default full Warrant rendering

```markdown
# <WAR ID>: <imperative title>

> Generated by OpenWarrant
> Phase: ...
> Contract revision: ...
> Contract digest: ...
> Compilation Basis: ...

## Current State

## Intent

### Problem
### Desired Outcome
### Scope
### Non-goals
### SAS and Roadmap Traceability

## Basis

### Governing Sources
### Context
### Prerequisites
### Assumptions and Unknowns
### Constraints and Invariants

## Architecture Decisions
<!-- omitted when no ADR refs -->

## Work Order

### Deliverables
### Frozen Surfaces
### Premade Instructions
### Stages
### Resources and Capabilities
### Autonomy and Escalation
### Rollback

## Milestones

## Execution

### Attempts
### Artifacts
### Blockers
### Amendments
### Deviations
### Discovered Gaps
### Timeline

## Assurance Case

### Acceptance Obligations
### Gate Adequacy
### Gate Runs
### Empirical Evidence
### Observations
### Inferences
### Judgments
### Residual Risk
### Obligation Dispositions

## Resolution

## Ongoing Validation

## Relations, Provenance, and Integrity
```

## 104. Minimal draft example

### 104.1 Manifest

```toml
schema = "oh.war/manifest/v1"
uuid = "019c8f2d-7b4d-7c41-9cb7-2636e5f582ea"
local_alias = "OW-WAR-0001"
title = "Implement the first OpenWarrant canonical compiler"
profile = "delivery"
assurance_level = "controlled"

[[implements]]
ref = "sas://WAR-SAS-RQ-001"
contribution = "partial"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"
required = true

[[atoms]]
ordinal = 20
role = "basis"
path = "atoms/20-basis.md"
required = true

[[atoms]]
ordinal = 40
role = "work_order"
path = "atoms/40-work-order.md"
required = true

[[atoms]]
ordinal = 45
role = "milestones"
path = "atoms/45-milestones.yaml"
required = true

[[atoms]]
ordinal = 60
role = "assurance"
path = "atoms/60-assurance.md"
required = true
```

### 104.2 Intent atom

```markdown
---
schema: oh.war/atom/v1
role: intent
jurisdiction: authored
classification: internal
---

# Intent

## Problem

OpenWarrant has no executable canonical compiler.

## Desired Outcome

A Rust library and CLI compile a valid file-native WAR into deterministic
canonical JSON and generated Markdown.

## Non-goals

- No Knowledge Fabric registration.
- No Katana execution.
- No Liminal production integration.
```

### 104.3 Milestones

```yaml
schema: "oh.war/milestones/v1"
milestones:
  - id: "M1"
    title: "Canonical types compile"
    stage_refs: ["STAGE-001"]
    obligation_refs: ["OBL-001"]

  - id: "M2"
    title: "Two-host canonical output matches"
    depends_on: ["M1"]
    stage_refs: ["STAGE-002"]
    obligation_refs: ["OBL-002"]

stages:
  - id: "STAGE-001"
    title: "Implement canonical types"
    executor_kind: "katana"
    responsibility_tier: "T2"

  - id: "STAGE-002"
    title: "Run canonicalization conformance"
    executor_kind: "service"
    responsibility_tier: "T1"
```

## 105. Reference URI forms

Recommended logical forms:

```text
war://<uuid-or-enterprise-id>
adr://<uuid-or-enterprise-id>
sas://<requirement-id>
roadmap://<item-id>
atom://<uuid>@<revision>
artifact://<uuid-or-content-id>
evidence://<uuid>
gate://<gate-id>/<version>
gate-run://<uuid>
dispatch://<uuid>
attempt://<uuid>
resolution://<uuid>
kf://<object-id>@<revision>
git://<repository>@<commit>/<path>
liminal-basis://<uuid-or-digest>
katana-run://<id>
blut-run://<id>
```

Resolvers are adapter-specific. The logical identity SHALL not depend on one storage URL.

## 106. Architecture requirements index

The following requirement IDs provide stable traceability for implementation WARs.

### Identity and federation

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-001 | Every WAR has immutable UUIDv7 identity |
| WAR-SAS-RQ-002 | Local aliases do not substitute for global identity |
| WAR-SAS-RQ-003 | KF allocates official enterprise identity |
| WAR-SAS-RQ-004 | Registration does not silently transfer Source Holder |
| WAR-SAS-RQ-005 | Cross-repository relations resolve through KF federation |

### Composition

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-010 | Authored atoms are directly editable sources |
| WAR-SAS-RQ-011 | Bound atoms are edited only through their owning authority |
| WAR-SAS-RQ-012 | Generated atoms and parents are not directly editable |
| WAR-SAS-RQ-013 | Composition is typed, ordered, and deterministic |
| WAR-SAS-RQ-014 | Full WAR Markdown and canonical JSON compile from one Basis |
| WAR-SAS-RQ-015 | Required atom omission fails closed |

### Decisions and hierarchy

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-020 | Every normative decision is a first-class ADR |
| WAR-SAS-RQ-021 | ADR Overview is generated |
| WAR-SAS-RQ-022 | WARs trace to SAS requirements and Roadmap |
| WAR-SAS-RQ-023 | Child WARs cite exact parent revision |
| WAR-SAS-RQ-024 | Childs do not rewrite parent rationale |
| WAR-SAS-RQ-025 | Supersession preserves old WAR and marks it non-current |

### Contract and lifecycle

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-030 | Authorized Contract Revisions are immutable |
| WAR-SAS-RQ-031 | Progress cannot amend contract |
| WAR-SAS-RQ-032 | State is decomposed into phase, condition, outcome, currency, standing |
| WAR-SAS-RQ-033 | Material amendment creates new revision |
| WAR-SAS-RQ-034 | Prior attempts retain original contract basis |
| WAR-SAS-RQ-035 | Readiness requires Preflight |

### Execution

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-040 | Milestones and stages are distinct |
| WAR-SAS-RQ-041 | Stages use named typed ports |
| WAR-SAS-RQ-042 | Stateless actors receive one Stage Dispatch |
| WAR-SAS-RQ-043 | Dispatch contains exact basis, capabilities, resources, outputs, stop conditions |
| WAR-SAS-RQ-044 | Agent authority is explicit and bounded |
| WAR-SAS-RQ-045 | Replay, repair, and restart are distinct |

### Assurance

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-050 | Completion summary decomposes into obligations |
| WAR-SAS-RQ-051 | Claims declare bounded scope |
| WAR-SAS-RQ-052 | Evidence, observation, inference, judgment, resolution remain distinct |
| WAR-SAS-RQ-053 | Performer reports cannot satisfy independent gates |
| WAR-SAS-RQ-054 | Required unknown gate results block resolution |
| WAR-SAS-RQ-055 | Controlled work requires adequacy review |
| WAR-SAS-RQ-056 | Gate Definitions are separately governed and versioned |
| WAR-SAS-RQ-057 | Gate invalidation propagates to dependent resolutions |
| WAR-SAS-RQ-058 | Residual risk requires sufficient judgment authority |
| WAR-SAS-RQ-059 | Resolution binds exact contract and assurance snapshot |

### System ownership

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-060 | KF owns authority and lifecycle |
| WAR-SAS-RQ-061 | Liminal owns document semantics and Basis |
| WAR-SAS-RQ-062 | Katana owns agent runtime and PromptIR |
| WAR-SAS-RQ-063 | BLUT owns typed computational execution |
| WAR-SAS-RQ-064 | OpenWarrant does not duplicate those kernels |
| WAR-SAS-RQ-065 | Native systems retain artifact authority |

### CLI and planning

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-070 | CLI works file-native and offline for drafts |
| WAR-SAS-RQ-071 | `war plan` returns a structured proposal |
| WAR-SAS-RQ-072 | Agent proposals are validated before writes |
| WAR-SAS-RQ-073 | Planner creates ADR drafts for normative decisions |
| WAR-SAS-RQ-074 | `war check` is deterministic and agent-free |
| WAR-SAS-RQ-075 | Generated views are drift-checked |
| WAR-SAS-RQ-076 | KF commands use typed actions, not direct status edits |

### Preservation

| ID | Requirement |
|---|---|
| WAR-SAS-RQ-080 | Canonical portable WAR is RFC 8785 JSON |
| WAR-SAS-RQ-081 | Cross-system WAR digests use explicit algorithms and domains |
| WAR-SAS-RQ-082 | Export includes contract, sources, receipts, assurance, resolution |
| WAR-SAS-RQ-083 | Export-import-export preserves semantic identity |
| WAR-SAS-RQ-084 | Historical superseded, disputed, and annulled records remain available |

## 107. Final doctrine

The SAS says what the system is meant to become.

The Roadmap says when and in what order it becomes that system.

The Warrant says what bounded work is authorized now.

The Work Order says how the authorized work is to be executed.

The Dispatch gives one actor exactly what it needs.

Artifacts record what the actor produced.

Evidence records what independent methods observed.

Inference connects observations to bounded claims.

Judgment records accountable evaluation and risk acceptance.

Resolution records what the organization concluded.

> **A WAR does not claim that work is correct because an agent stopped working or a command exited zero. It preserves the exact source atoms, Basis, authority, contract, attempts, artifacts, empirical observations, reasoning, judgments, limitations, and resolution that make the conclusion auditable.**

> **The human receives one Warrant. The machine receives one canonical document. The organization retains one authority per fact.**

---

# Appendix A — Initial generated ADR Overview shape

```markdown
# Architecture Decision Record Overview

> Generated by OpenWarrant. Do not edit.

## Summary

| ADR | Title | Status | Currency | Governs | Implemented by |
|---|---|---|---|---|---|

## Proposed

...

## Accepted and Current

...

## Superseded

...

## Rejected, Withdrawn, or Falsified

...

# Full Decision Record

## ADR 0001: ...

...

# Full Decision Record

## ADR 0002: ...

...
```

# Appendix B — Initial Work Order projection shape

```markdown
# Work Order — <WAR ID>

## Authorized Outcome
## Scope and Non-goals
## Governing SAS and ADRs
## Required Context
## Frozen Surfaces
## Deliverables
## Milestones
## Stages
## Resources and Capabilities
## Autonomy and Escalation
## Acceptance Obligations
## Rollback
## Current Blockers
```

# Appendix C — Initial agent-planning interaction

```text
$ war plan "Implement scalar fallback without changing the wire format"

Inspecting configured SAS, Roadmap, ADRs, WARs, repository, and gates...

Proposed Warrant:
  title: Implement scalar fallback without changing the wire format
  profile: delivery
  assurance: controlled
  SAS requirements: 2
  Roadmap items: 1
  proposed ADRs: 1
  milestones: 3
  stages: 4
  acceptance obligations: 5
  blocking unknowns: 2

Questions:
  1. Is the public decoder API frozen?
  2. Which targets are normative for this WAR?

Apply draft atom changes? [y/N]
```

# Appendix D — Initial Rust domain sketch

```rust
pub struct Warrant {
    pub identity: WarrantIdentity,
    pub format_basis: FormatBasis,
    pub source: SourceComposition,
    pub relations: Vec<Relation>,
    pub state: StateProjection,
    pub governance: Governance,
    pub contract: Vec<ContractRevision>,
    pub milestones: Vec<Milestone>,
    pub execution: ExecutionProjection,
    pub assurance: AssuranceCase,
    pub resolutions: Vec<Resolution>,
    pub integrity: Integrity,
}

pub enum AtomJurisdiction {
    Authored,
    Bound,
    Generated,
}

pub enum Phase {
    Draft,
    Proposed,
    Authorized,
    Ready,
    Executing,
    Verifying,
    Resolved,
}

pub enum AttemptKind {
    Initial,
    Replay,
    Repair,
    Restart,
}

pub enum EvidenceClass {
    Informative,
    PerformerReportOnly,
    Independent,
    AuthoritativeExternal,
    ControlledMeasurement,
    Formal,
    Inadmissible,
}
```

# Appendix E — Immediate first WARs

The first OpenWarrant implementation sequence SHOULD include:

1. **WAR: Establish OpenWarrant repository and Rust workspace**
2. **WAR: Implement file-native manifest and atom parser**
3. **WAR: Implement canonical WAR IR and RFC 8785 digesting**
4. **WAR: Implement generated parent and drift checking**
5. **WAR: Implement deterministic `war check`**
6. **WAR: Implement Katana-backed `war plan`**
7. **WAR: Import the WAR SAS as a controlled KF/Liminal document**
8. **WAR: Implement ADR atom and ADR Overview compilation**
9. **WAR: Implement KF registration and global identity**
10. **WAR: Implement Stage Dispatch and Katana runtime receipt**

Each SHALL reference the applicable requirement IDs in §106.

# Appendix F — Repository references used in drafting

- Liminal `README.md`, `ARCHITECTURE.md`, `spec/v4/`, `docs/execution/00-protocol.md`, and `docs/execution/template.md` at commit `2b7fc3f448b171e9a4aa2439a32b7a46f9509871`.
- Katana `README.md`, `docs/spec/v0.3.0.md`, compiler and kernel at commit `651ba435296c37d91be25d458a6e485d35ac516e`.
- BLUT `README.md` and `API.md` at commit `f5403b9d45585544f6d2f5d1a34e048915b2545d`.
- OpenHuman Knowledge Fabric architecture, action kernel, document compiler, and Liminal adapter at commit `9e0e75550f96a851c126e59e696d71245d51382d`.
- LamQuant ADR template, model, generated views, and measured gate executor at commit `5369da813578df355ea1c8c17bf20d85e426681a`.
