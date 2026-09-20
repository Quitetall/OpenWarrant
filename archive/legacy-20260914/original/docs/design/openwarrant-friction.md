# OpenWarrant: less friction, safer AI implementation

> **2026-09-14 update:** The owner directed completion of SAS **1.0.0-rc.2**
> and testable Phase 1 scope before building. The [RC.2 source set](../sas/drafts/1.0.0-rc.2/README.md)
> now consolidates these decisions, chooses the candidate format and assurance
> contracts, and provides examples and a build matrix. This document retains the
> earlier interview/research history; statements below that a mechanism remained
> open describe that earlier stage. Use RC.2 for the current candidate design.
> RC.2 is not yet accepted or implemented.

Status: discussion draft, 2026-09-13. This document records the product interview
and candidates for a future revision of the existing program SAS. It is not an
accepted SAS, an authorization, or a completed implementation plan. Existing
governed records remain authoritative until changed through their defined process.

The [consolidated product specification](openwarrant-product-spec.md) summarizes
the confirmed decisions for shared-understanding review. This file retains the
answer ledger and investigation evidence.

## Product goal

OpenWarrant defines a parsable document standard for agentic work. Q36 requests
supporting authoring, semantic compilation, context projection, and work-tracking
tools; the later ownership clarification separates that tooling from ownership
of the standard. Its primary benefit is less human coordination while making
AI-assisted changes safer in real development. The proposed API, native TUI, and
web tracker expose the same work and decisions.

The owner reports that dogfooding has significantly stalled recent OpenWarrant
work. That is a product problem to investigate. Successful adoption must reduce
the effort needed to finish an accepted change, including OpenWarrant's own
administration. Q13 sets an administrative budget of at most 60 seconds per
routine Warrant; the real-work baseline and measurement procedure remain open.

## Product decisions confirmed before Round 1

| ID | Decision | Reason |
| --- | --- | --- |
| C01 | Provide an API, native TUI, and web tracker over the Warrant system. | Other applications and developers need a shared view of progress. |
| C02 | Show Warrants as cards or rows with a title and description; make details available for review. | People need understandable work they can choose. |
| C03 | Select multiple Warrants, then use one Approve & Start action. Eligible agents start immediately after authorization. | Reduce repeated human actions while approving a specific set of work. |
| C04 | Warrants can carry an implementation specification, parsable structure, and required test fixtures. | Agents should receive explicit work and executable checks. |
| C05 | The compiler assembles information and delegates context management and optimization, assisting orchestration. | Humans should not repeatedly rebuild prompts and context. |
| C06 | Every executing agent has a hotline to other agents and authorized humans. | Questions need to reach somebody who can answer while work is underway. |
| C07 | Architecture decisions go to a human with sufficient authority for the affected scope by default; Q9 permits explicit policy delegation. | Technical advice alone does not grant permission to change the architecture. |
| C08 | A material architecture change creates a new revision of the same Warrant, requiring approval under the authority rules in Q9. | Preserve what was originally approved and explicitly authorize the change. |
| C09 | Review the proposed revision within the hotline, with another question round when needed, then approve it and resume affected work. Q9 defines explicit automation exceptions to the default human route. | Reuse nearby facts, observations, and context for equal rigor with less repeated discovery. |

The precise distinction between clarification and material amendment, authority
scope, interruption behavior, and propagation to other agents still needs design.
Suggestions in earlier assistant replies are not decisions unless confirmed here
or by a subsequent interview answer.

## Current evidence relevant to friction

Observed checkout: `feat/battery-split`, `f22ef2f7282e8b5c72f2c4323b300f3b5e16102c`.
This is an investigation snapshot; inspect live state before using its counts.

- `war next --json` reports 19 pending human acts. Earlier review established
  these are resolutions with unestablished obligations, so their existence is
  not evidence that the work should be accepted as satisfied.
- [AGENTS.md](../../AGENTS.md) requires human acts for authorization, resolution,
  SAS acceptance, and corrections to resolved deliverables. The approved local
  skills appendix currently conflicts with whole-file template parity tests.
- [The correction design](../adr/atoms/OW-ADR-0012-correction-act.md) preserves
  original manifests but expects the current delivered file to match the latest
  correction. Normal later development can therefore create work against past
  deliveries. Accepted SAS requirement RQ-036 makes this a specification question,
  not merely an incidental UI bug.
- [The pin guard](../../.claude/hooks/guard-pins.sh) refuses editing a resolved
  delivery even under a newer Warrant's authority, while correction ingestion
  requires changed bytes. Multiple older Warrants can pin the same pathname,
  multiplying correction acts for one implementation change. The design needs
  an authorized transition before the edit, with preserved historical artifacts.
- [The hotline](../../crates/openwarrant-cli/src/questions.rs) currently refuses
  agent answers. Human attribution on an answer is not authenticated authority;
  the answer grants no authorization. C06-C09 require an extension of this model.
- [The configured performer](../../openwarrant.toml) is a fixture that produces no
  artifacts. No verifier command is configured. These gaps require a real runner
  and verification path before claiming an unattended implementation workflow.
- `OW-WAR-0071` has no dispatchable stages. `external_dependency` in 0072/0073's
  assumption TOML is not a field in the parsed assumption type; their first stages
  currently appear open despite their written prerequisite. A visible queue must
  not imply executable permission when the governing prerequisite remains unmet.
- `next`, `frontier`, and `perform` do not use one eligibility evaluation. The
  inspected execution paths do not automatically pause on blocking hotline
  questions. The proposed question-and-resume experience needs execution support
  in addition to records and a user interface.
- [Resolution assessment](../../crates/openwarrant-cli/src/resolve.rs) derives
  independence from admissible verification records. The repository-wide false
  independence flags alone are not proof that an external verification is
  inadmissible. Isolation and protection still have to exist in the actual run.
- [Policy-service resolution](../../crates/openwarrant-cli/src/resolution_cmd.rs)
  has a partial code path under §27.3, but currently treats obligations as all
  mechanical only when there are no declared obligations. It is not a ready
  mechanism for automatically closing normal declared work.
- [System acceptance and success metrics](../sas/WAR_Software_Architecture_Specification.md)
  already require low basic-work overhead, less human time, and less manual
  synchronization (§99.25, §100). [Fixture evaluations](../EVAL.md) exercise
  mechanics; they do not establish human effort saved on real development.
  [The recorded telemetry baseline](../../artifacts/telemetry-baseline.json)
  marks human authoring time and human control time per accepted Warrant as
  unmeasured. Q13 and Q29 select friction targets; Q31 selects the first real
  workflow. Its baseline and measurement procedure still need to be established.

## Round 1: work, history, completion, and verification

Later owner clarification C10 qualifies earlier workflow decisions: autonomous
prototyping is allowed under human-granted permissions, while an optional mark
and signed verified/complete Warrant require human review and acceptance.
Read the following historical choices with C10; automatic acceptance does not
by itself earn that mark. Unresolved implications are recorded in Round 10.

Q1-Q4 were answered by the owner on 2026-09-13. Round 1 is complete.
Selected product directions still require the applicable governed changes before
they become effective repository policy.

| ID | Decision | Selected direction or recommendation | Status / alternative |
| --- | --- | --- | --- |
| Q1 | What is the default unit of a Warrant? | One independently reviewable outcome, with stages for its implementation steps. | Confirmed. The owner selected approving password reset once, with API, email, UI, and tests handled as stages within its bounds. Separate approval per implementation task was not selected. |
| Q2 | How should new approved work change files delivered by resolved Warrants? | Preserve the historical delivered version and evidence; let the new Warrant govern the new version, with explicit lineage. | Confirmed. The owner selected immutable old artifacts and evidence, with normal later development authorized as new work. Correction acts against every older delivery were not selected for this case. |
| Q3 | How should completion be accepted? | Manual acceptance is one easy action, and an automatic acceptance mode is available. Automate preparation and checks in both modes. | Confirmed by the owner's clarification after Round 1. Q8-Q10 define eligibility and authority; Q16 defines activation. Initial authorization and material-amendment approval remain required, with Q9 defining delegation for governing acts. |
| Q4 | What is the default independent verification for ordinary AI code? | A separate agent context and isolated workspace, rerunning protected checks and evaluating the implementation against the approved work. | Confirmed. The owner selected an independent agent and protected checks. Detailed human code review of every implementation is not the selected default; final acceptance follows the mode selected under Q3. |

Q2's selected direction needs a controlled clarification or change to RQ-036
and artifact semantics. Q3 adds an automatic mode whose authority and eligibility
must be defined; §27.3 currently permits only a conditional mechanical-work
exception. These choices do not change current rules merely by appearing in this
draft. Q4 defines a
bounded verification procedure;
it does not promise defect-free code or organizational independence.

## Round 2: everyday behavior and automatic acceptance

Q5-Q8 and the document-bundling clarification Q5a were answered by the owner on
2026-09-13. Round 2 is complete.

| ID | Decision | Selected direction or recommendation | Status / alternative |
| --- | --- | --- | --- |
| Q5 | What is binding in the implementation specification by default? | Outcomes and explicit architectural constraints are binding. Implementation steps are guidance unless marked as constraints; agents record in-bounds choices without another approval. | Confirmed. The owner selected binding required behavior, architecture boundaries, permissions, and checks, with implementation steps adaptable inside those bounds. |
| Q5a | Can a Warrant include more detailed governing and supporting documents? | A Warrant can bundle ADRs and other supported documents to make its implementation plan more detailed. | Confirmed by the owner's additional clarification. This complements Q5 rather than replacing its autonomy boundary. |
| Q6 | When must executable acceptance fixtures exist? | Core acceptance fixtures exist and are protected before implementation begins. The performer may add tests; changing mandatory expectations requires an approved revision. | Confirmed. The owner selected pre-existing protected acceptance fixtures, with additive implementation tests allowed. |
| Q7 | What stops when an architecture question needs a human? | Pause the affected stages and their dependents, preserve progress, and let independent work continue. | Confirmed. The owner selected pausing affected work and its dependents, with saved progress and independent work continuing. |
| Q8 | What work may automatic acceptance cover? | Human-defined policy determines eligible work. The platform must support categories ranging from routine features and maintenance to SAS-changing work. | Confirmed. The owner explicitly rejected a hardcoded low-risk-only ceiling, while describing automatic acceptance of SAS changes as very much not recommended. Governing-act delegation and policy-change authority need further definition. |

Q5 still requires an approved revision for changes to agreed scope, architecture,
authority, or mandatory acceptance conditions. Q5a means those constraints may
come from bundled governing documents as well as the Warrant's main presentation.
The compiler must preserve source identity and the approved document revisions,
and distinguish governing constraints from supporting explanations and suggested
steps. The supported formats, attachment/reference mechanism, conflict handling,
and context selection rules remain implementation-design questions.

Q8 extends past §27.3's present mechanical-work exception and requires an explicit
governed policy change. It does not itself define which actors can exercise each
delegated act. In particular, automatically accepting completed work that changes
a SAS and automatically accepting the resulting SAS as governing authority are
distinct acts. Q9 confirms that an explicit policy may delegate both. Current
human-only rules remain effective until their governing changes are accepted.

## Round 3: policy authority and first-use experience

Q9-Q13 were answered by the owner on 2026-09-13. Round 3 is complete.

| ID | Decision | Selected direction or recommendation | Status / alternative |
| --- | --- | --- | --- |
| Q9 | Can an explicit human policy delegate governing approvals as well as Warrant completion? | Support explicit delegation of both, including architecture-amendment and SAS acceptance acts; default those governing acts to human approval. | Confirmed. The owner selected policy-controlled governing acts, retaining direct human approval as their default. |
| Q10 | Who can change the policy granting automated authority? | An authorized human approves effective automation-policy changes; automation may propose edits. | Confirmed. The owner selected human approval for changes to effective automation policy. |
| Q11 | What should acceptance do to a verified code change? | Merge when the Warrant and repository policy explicitly permit it; otherwise present the accepted change ready to merge. Deployment requires its own permission. | Confirmed. The owner selected combining acceptance and merging when explicitly permitted, with deployment separately authorized. |
| Q12 | Where should the first usable version run? | A local background service on the developer's machine, shared by TUI, browser UI, API clients, and agent adapters. | Confirmed. The owner selected the developer's machine first, with interfaces designed for later shared hosting. |
| Q13 | What administrative effort is acceptable for a routine Warrant? | No manual record editing or required shell commands after setup; at most 60 seconds of administration per Warrant. Measure substantive review and decisions separately and include them in total human effort. | Confirmed. The owner selected zero manual records and at most one minute of administration per routine Warrant. |

Q9 is about future platform policy, not permission for this assistant to sign or
resolve current records. Q11 still needs exact-result, stale-check, and merge-conflict
behavior. Q12 sets the initial delivery target rather than the platform's ultimate
deployment limit. Q13 is a target to test, not a measured claim.

## Round 4: execution integration and user interaction

Q14-Q17 were answered by the owner on 2026-09-13. Round 4 is complete.
Q14 includes the owner's correction immediately after the original selection.

The owner requested inspection of LAMU, Katana, BLUT, and Liminal before choosing
the execution approach. [The adapter investigation](../research/implementation-adapters.md)
records current source interfaces and missing contracts. The owner revised the
initial BLUT-first choice: the first release requires OpenWarrant alone, with
adapter boundaries kept in mind for later integrations. LAMU, Katana, BLUT, and
Liminal are not required dependencies for that release.

| ID | Decision | Selected direction or recommendation | Status / alternative |
| --- | --- | --- | --- |
| Q14 | What integration scope should the first usable workflow require? | Release OpenWarrant on its own; preserve adapter boundaries for later LAMU, Katana, BLUT, and Liminal integrations. | Confirmed by the owner's correction, superseding the initial BLUT-first selection. None of these other projects is required for the first release. |
| Q15 | What human approval gesture should ordinary local use require? | Approval within an authenticated, unlocked application session; explicit policy can require stronger user-presence confirmation for sensitive acts. | Confirmed. The owner selected session-based approval with policy-controlled stronger confirmation. |
| Q16 | How should automatic acceptance be activated? | An authorized human enables a named repository policy. Each Warrant shows its effective acceptance mode before starting; manual review can be selected for particular work. | Confirmed. The owner selected a human-enabled repository policy with per-Warrant manual review. |
| Q17 | Which screen should prove the new complete workflow first? | Native TUI over the shared local API, then the browser UI over the same behavior. | Confirmed. The owner selected the native TUI and shared local API first, with the browser UI following. |

Q14 removes mandatory sibling-project integration from the first release. It
does not remove the approved-work, agent-execution, hotline, verification, and
acceptance product goals. Q18 selects both connected agents using an OpenWarrant
skill and commands launched by OpenWarrant. It does not implicitly select a new agent loop,
model manager, document-semantic engine, or computational DAG engine inside
OpenWarrant. Any departure from current SAS ownership needs an explicit governed
change. Q7 still requires affected work and dependents to pause while independent
work continues. Detailed BLUT and Liminal adapter choices can follow later.

Q14 must preserve compiler/runtime ownership and demonstrate real isolation rather
than trusting an adapter's label. Q15 must authenticate the human and bind the
exact approved work; an agent credential or an attributed name is insufficient.
The current SSH confirmation model remains in force until any changed approval
model is specified, tested, and adopted. Q16 cannot let an agent grant itself a
more permissive mode. Q17 concerns delivery order, not removal of either client
from the product goal.

## Round 5: standalone execution and preparation

Q18-Q20 were answered by the owner on 2026-09-13. Round 5 is complete.
These choices refine the first-release workflow under Q14.

| ID | Decision | Selected direction | Status |
| --- | --- | --- | --- |
| Q18 | How should approved work reach a coding agent in the first release? | Support both an already-connected agent using an OpenWarrant-specific skill and an agent command launched by OpenWarrant. Both use the same approved-work protocol. | Confirmed. The owner explicitly selected both execution routes. No sibling project is required. |
| Q19 | Where should implementation agents edit code? | One isolated Git worktree per Warrant, with serialized writers within that worktree; independent Warrants can run concurrently. Verification uses its own isolated workspace under Q4. | Confirmed. The owner selected one worktree per Warrant with serialized writers. |
| Q20 | Who should prepare mandatory acceptance fixtures? | A separate preparation agent reuses or drafts fixtures alongside the Warrant; approval covers the plan and protected expectations. | Confirmed. The owner selected separate preparation and joint approval of fixtures and Warrant. |

Q18 concerns process ownership, not a new built-in agent loop. Both connections
must bind exact approved work, expose durable progress/questions, and enforce
the required execution bounds. Q19 selects the initial concurrency boundary;
worktree separation alone is not a sandbox or protection for fixtures. Q20
refines authorship under Q6 without weakening the requirement that mandatory
fixtures be prepared and protected before implementation starts.

The existing [OpenWarrant skill](../../.claude/skills/openwarrant/SKILL.md) and
[`performer_argv` runner](../../crates/openwarrant-cli/src/perform.rs) provide
starting points, not the completed dual-route protocol. The current
[frontier](../../crates/openwarrant-cli/src/frontier.rs) infers a claim from a
`dispatch.compiled` event; that event is not proof of a live exclusive writer.
Both routes need common claim ownership, exact revision binding, and safe
handoff. A skill guides an agent's use of that protocol; enforced controls must
live in the service and execution environment. A stale or disconnected worker
must not keep writing concurrently with a replacement.

## Round 6: availability, recovery, and integration

Q21-Q24 were answered by the owner on 2026-09-13. Round 6 is complete.

| ID | Decision | Selected direction | Status |
| --- | --- | --- | --- |
| Q21 | What happens when work is approved but no eligible agent is available? | Queue it visibly as waiting for an agent; start automatically when an eligible agent connects or can be launched. | Confirmed. The owner selected visible waiting and automatic start when an eligible agent becomes available. |
| Q22 | What happens when a worker disappears mid-Warrant? | Preserve progress and automatically resume or replace it within configured retry/spend limits, only after establishing that the previous writer cannot keep writing. Escalate uncertain ownership or exhausted limits. | Confirmed. The owner selected bounded automatic recovery with escalation of uncertainty. |
| Q23 | What happens when the target branch changes before this Warrant is accepted or merged? | Automatically update the Warrant branch within its approved scope and rerun affected protected checks and independent verification on the exact resulting candidate. Escalate changes to binding constraints or unresolved conflicts. | Confirmed. The owner selected in-scope updates, renewed checks and independent verification, and escalation of unresolved conflicts. |
| Q24 | What happens when independent verification finds a fixable implementation defect? | Return evidence to the implementation agent for repair under the same Warrant, bounded by configured retry/spend limits; independently verify the new result. | Confirmed. The owner selected bounded automatic repair followed by independent verification, keeping mandatory fixtures protected. |

Q21 distinguishes approval, queued availability, and actual execution. Q22 cannot
infer that a writer has stopped solely from a missed heartbeat. Q23 invalidates
evidence whose basis changed and does not make policy edits, architecture changes,
or final merge automatically authorized. Q24 does not permit weakening fixtures or
turning an unavailable check into a failed implementation claim. Numeric budgets,
evidence reuse rules, and handling verifier disagreement remain follow-up choices.

## Round 7: repair limits, disputed findings, and questions

Q25-Q27 were answered by the owner on 2026-09-13. Round 7 is complete.
Q25 includes the owner's clarification that three is a fallback when no repair
count is chosen.

| ID | Decision | Selected direction or recommendation | Status / alternative |
| --- | --- | --- | --- |
| Q25 | What should be the default limit on automatic repair cycles? | Offer the user a choice of repair count. If none is chosen, default to three repair cycles after the initial implementation. Repository time/spend limits can stop work earlier; effective limits remain subject to human-controlled policy. | Confirmed and clarified. Three is the fallback value, not a mandatory count; choosing another count is optional. |
| Q26 | What happens when the performer disputes an independent verifier's finding? | Allow an evidence-backed rebuttal and independent re-evaluation, preserving both judgments; unresolved disagreement escalates to an authorized human. | Confirmed. The owner selected independent rechecking of evidence-backed rebuttals, with a human settling unresolved disagreement. |
| Q27 | How should the hotline route technical questions by default? | In-scope technical questions go to an AI adviser first; governing decisions go to the authorized decision-maker under policy. The asking agent can request direct human escalation. | Confirmed. The owner selected AI-first in-scope advice and authorized responders for governing decisions, preserving direct human escalation. |

Q25 counts repair cycles separately from interrupted-worker recovery. A provider
with no enforceable spend accounting cannot be advertised as satisfying a hard
spend cap. Numeric time/spend defaults and runtime capability handling remain
open. Q26 never lets a performer's rebuttal clear its own gate; changing mandatory
expectations still requires the approved revision specified by Q6. Q27 separates
technical advice from governing authority and preserves direct escalation when
needed; an adviser answer cannot silently authorize a scope or policy change.

## Round 8: adoption, retention, and first real proof

Q28-Q31 were answered by the owner on 2026-09-13. Round 8 is complete.

| ID | Decision | Selected direction or recommendation | Status / alternative |
| --- | --- | --- | --- |
| Q28 | Does a new repository need an accepted SAS before ordinary implementation starts? | Generate a minimal SAS from the repository and declared goals; an authorized human approves it once during setup. Reuse existing governing documents and expand the SAS as architecture needs grow. | Confirmed. The owner selected an automatically generated minimal SAS, approved once during setup and expanded as needed. |
| Q29 | What setup effort is acceptable for a new repository? | Most users should finish setup in five to ten minutes or less. Advanced setup may include longer substantive discussion, staying below several hours. Measure administration, substantive review/discussion, and installation/download waits separately, alongside total onboarding time. | Confirmed by the owner's clarification. Support a short ordinary setup and optional deeper advanced setup; no exact numeric hour limit was specified. |
| Q30 | What accepted history should the first release retain by default? | Preserve accepted code versions, approved fixtures, decisions, and verification evidence for the project's lifetime; temporary working data can be pruned under policy without removing referenced evidence. | Confirmed. The owner selected project-lifetime retention for accepted history. |
| Q31 | Which real repository should prove the first complete workflow after it is ready? | OpenWarrant itself, using a small meaningful maintenance change and measuring the complete prepare/approve/execute/questions/verify/accept experience. | Confirmed. The owner selected OpenWarrant itself and a small meaningful maintenance change after the new workflow is ready. |

Q28 concerns initial adoption rather than silently waiving governing requirements
for an existing project. The current [program initializer](../../crates/openwarrant-cli/src/init.rs)
already generates a [minimal SAS](../../crates/openwarrant-cli/templates/PROGRAM_SAS.md.tmpl)
and a draft delivery Warrant. [Quickstart](../../QUICKSTART.md) then places SAS
acceptance before Warrant authorization. This is not a requirement to write a
full architecture book: the SAS supplies no defined completeness threshold.
Current [record checking](../../crates/openwarrant-cli/src/check.rs) warns on a
missing SAS revision, and [authorization](../../crates/openwarrant-cli/src/authorize.rs)
permits an absent SAS pin; these paths do not establish an accepted-SAS gate.
Q28 selects an accepted minimal SAS as the first-implementation prerequisite,
consistent with the governing SAS's one-SAS-per-program model (§6.10). The new
workflow must enforce that prerequisite rather than inherit permissive code.
Later SAS amendments follow Q9's authority policy; routine Warrants do not
require repeatedly accepting an unchanged SAS.

Q29 sets an ordinary-setup target of five to ten minutes or less, with a separate
advanced path for deeper discussion. Measure administration, substantive decisions,
and installation waits separately so a longer design conversation is visible
rather than misreported as administrative overhead. Q13 still governs recurring
administration per Warrant. Q30 requires recoverable
bytes, not merely a log claiming that an artifact once existed; precise local
storage, retention, and export mechanics still need design. Q31 does not require
unfinished new tooling to govern its own construction. The selected real workflow
starts after that tooling is ready and must respect effective repository authority.

## Round 9: platforms, execution protections, and migration

Q32-Q35 were answered by the owner on 2026-09-13. Round 9 is complete.

| ID | Decision | Selected direction or recommendation | Status / alternative |
| --- | --- | --- | --- |
| Q32 | Which operating systems must the first release support? | Linux and macOS first, for the local service and native TUI. | Confirmed. The owner selected Linux and macOS as first-release platform targets. |
| Q33 | Who supplies execution isolation in the standalone first release? | The coding-agent harness supplies sandbox enforcement; OpenWarrant manages approved work, worktrees, protected evidence admission, and checks that required protections are available. | Confirmed. The owner selected harness-provided sandboxing with OpenWarrant checking required protections. |
| Q34 | May an agent backend run without spend accounting? | Yes when policy permits: show cost as unknown, enforce available required limits, and refuse the run if policy requires an unavailable hard spend cap. | Confirmed. The owner selected policy-permitted unmetered backends, explicit unknown cost, and refusal when a mandatory hard spend cap cannot be enforced. |
| Q35 | Must legacy Warrant backlog be closed before using the new workflow? | Preserve and import the actual legacy states, including unknowns; block new work only for relevant prerequisites or integrity/authority problems. Unrelated closure debt does not block adoption. | Confirmed. The owner selected preservation of actual legacy states and blocking only for relevant prerequisites or integrity/authority problems. |

Current [release configuration](../../.github/workflows/release.yml) targets
x86_64 Linux GNU and ARM64 macOS. The required CI gate runs on Ubuntu; the
[performer](../../crates/openwarrant-cli/src/perform.rs) explicitly lacks Windows
Job Object termination. This establishes configured coverage, not a fresh runtime
qualification of the proposed clients.

The current [threat model](../THREAT_MODEL.md) trusts the operator-controlled
host and documents incomplete separation between human and agent processes
running as the same user. The performer and
[verifier runner](../../crates/openwarrant-cli/src/bundle.rs) do not install
process isolation. The [plugin hook](../../.claude/hooks/hooks.json) covers selected
editing tools rather than arbitrary shell writes. A skill and worktree therefore
cannot establish protected fixtures, credentials, or authority state by themselves.
Q33 selects the harness as the actual enforcement boundary for those claims,
consistent with SAS §11.4. OpenWarrant must check required protections before
admitting the run; the existing unconstrained runner alone does not establish
them. The first release does not need a second sandbox engine inside OpenWarrant.

Q34 keeps unknown cost distinct from zero cost and does not allow an unavailable
mandatory limit to pass. Q35 never repairs history by inventing acceptance,
evidence, or old artifact provenance. Shared authority integrity and explicit
dependencies can still block new work. Migration must preserve the historical
versions selected by Q2 and the retention commitment selected by Q30.

## Round 10: reopened product fundamentals

The owner requested adjustments before SAS revision planning, then asked for
mutual grilling about what the software is and should do. Earlier decisions
remain recorded and revisable; shared understanding has not been confirmed.

### Q36 — From an initial request to standard artifacts

Scenario: a developer says "Make signup easier" before an implementation plan
exists. The owner described several routes through one standardized artifact
generator:

- Open a browser prompt through a button and work with an agent.
- Ask a configured REPL agent to draft artifacts in the TUI.
- Use the OpenWarrant skill from a harness such as Claude Code or Codex, letting
  that agent invoke the CLI or draft artifacts.
- Author manually in an interactive CLI: select a field, write into it, repeat,
  then compile the fields into a single document. Body, context, and relevant
  locations were examples, not a settled field schema.
- Supply explicit fields through a noninteractive CLI.

The owner identifies OpenWarrant itself as the document standard. The generator
is a semantic compiler producing simple, parsable documents. It creates a large
master context document, then smaller projections for project management and
agents containing what each needs for its problem. Supporting tools should ship
with the first revision so the core can be used across formats and interfaces.
The first official parsable standard is being finalized; the owner does not
consider it stable 1.0 yet. This is product direction, not a change to existing
accepted authority or a request to relabel historical versions.

Q36 confirms manual and agent-assisted authoring as equal entry routes. It does
not confirm the assistant's earlier assumption that repository investigation by
an agent must always precede drafting. It also leaves open how a vague request
becomes precise enough to approve.

Questions exposed by Q36 (updated after Q50 and owner clarifications):

- Q37 settles that the master is a generated assembly of source documents.
  Whether its scope is a project or a work outcome remains open.
- What does a smaller but complete projection guarantee, and how are missing
  constraints or context discovered before execution? Q38 preserves exact
  applicable binding wording; applicability and coverage remain open.
- Q39 settles that complete, valid source inputs can compile without AI calls.
- Q40 selects shared structure and field meanings as the standard's contract.
  Which individual requirements belong in the core, supporting tools, optional
  profiles, or repository policy remains open.
- Q41 permits minimal valid drafts without execution records; the minimum field
  set and additional workflow-readiness requirements still need definition.
- Q42 selects readable Markdown with structured metadata as the primary authored
  source. The precise grammar, document types, and other interchange formats
  remain open.
- Q50 retains TUI/API/CLI first, with browser authoring following through the
  same API. Q36's desired browser interface is not a first-release dependency.

The assembly model (Q37), exact binding wording (Q38), compilation without AI
(Q39), shared structure and meanings (Q40), and validity distinct from readiness
(Q41), and Markdown primary source (Q42) are settled answers; the remaining
questions are open. C10 adds optional assurance and autonomous prototyping.

### Q37 — Master context and source authority

Confirmed: the owner selected a generated assembly of separately maintained SAS,
ADRs, Warrants, and repository sources. Edit each fact at its source, then
regenerate the master context and agent projections from those exact revisions.
The master does not become a competing editable source of truth. This preserves
the authority and traceability of the documents it assembles.

This answer settles source ownership. Assembly scope and which material each
agent must receive remain open; Q38 subsequently settles binding wording.

### Q38 — Binding wording in agent projections

Confirmed: the owner selected exact applicable binding rules and source
revisions, with summarization allowed for background context. An AI paraphrase,
even if checked, cannot replace a binding requirement in the task projection.
The question's example was an ADR requiring email verification before account
activation; the projection must preserve the requirement's exact wording.

This settles how selected requirements are carried. It does not prove that all
applicable rules have been selected, define how applicability is established,
or decide what happens when required context exceeds the agent's context budget.

### Ownership clarification — Standard versus consuming tools

While Q39 was pending, the owner clarified that OpenWarrant owns the barebones
document standard. Generators, compilers, and workflows use that standard, with
only partial involvement from OpenWarrant; OpenWarrant does not control those
tools merely because they use its documents.

This changes the interpretation of earlier workflow decisions: retain them as
design inputs for proposed reference tooling rather than automatically imposing
them on every compatible implementation. Q40 subsequently selects shared structure
and field meanings. Exact core requirements and any optional profiles still need
agreement. The owner
has not explicitly withdrawn Q36's request to include supporting tools with the
first revision. Ownership, conformance, and release packaging are distinct issues.

### Q39 — Compilation without AI

Confirmed: complete, valid OpenWarrant source documents can compile into master
context and task projections without any AI/model call. AI can draft sources,
propose context links, and prepare background summaries; the compiler checks
and assembles those explicit inputs reproducibly. Missing required inputs remain
visible rather than being guessed.

This establishes a supported path without AI, not a ban on AI-assisted tools.
The ownership clarification separates the reference compiler's behavior from
standard conformance; Q40 subsequently defines its shared-structure-and-meaning
boundary, with the exact required field set still open.

### Q40 — What document compatibility guarantees

Confirmed: the owner selected shared document structure and defined field
meanings. For example, compatible tools agree on what scope and constraints
describe and which document revision an approval refers to. Tools choose their
own drafting, compilation, and execution workflows.

This establishes a document contract with shared semantics. It does not make
every earlier reference-tool workflow choice mandatory for all implementations.
Q41 subsequently permits minimal valid drafts; the precise required fields,
extensions, and any optional profiles still need definition.

### Q41 — Minimal document validity versus execution readiness

Confirmed: a small, honestly labeled draft can be a valid OpenWarrant document
without fixtures, approval, or execution records. The question illustrated a
draft describing title, outcome, scope, and context; that example does not settle
the exact minimum field set.

Document validity and readiness to execute are separate. The chosen workflow
requires additional records before acting. A valid draft does not imply that
the work is authorized, verified, or complete. This keeps simple authoring
available without silently weakening the previously selected execution workflow.

### C10 — Autonomous prototyping and an optional assurance mark

After Q41, the owner clarified that OpenWarrant can be used with humans entirely
out of the execution loop, including workflows with poor development practices.
An agent may carry out Warrants when permitted and configured by a human. Such
permission does not inherently satisfy development standards or confer a mark.

A separate tag or mark requires actual human review and acceptance together with
a set of serious-development standards. "OpenWarrant Verified" was an example;
the owner has not selected its name. Human acceptance is required for a Warrant
to be signed as verified or complete. Agent execution alone cannot establish
that signed status or the mark.

The owner wants low-friction prototyping until it is time to accept work into
main or production. This qualifies earlier per-Warrant start approvals and
universal pre-implementation fixture requirements: they are not requirements
for every permitted prototype. The exact mark criteria and treatment of work
prepared without those controls remain open.

Earlier Q3/Q8-Q9/Q16 automatic-acceptance and delegation choices remain historical
answers. They cannot substitute for C10's required human act or award the mark.
Q47 subsequently defines a resting state of implementation finished, unreviewed,
without mandatory acceptance tasks for prototypes. The exact scope of delegated
governing acts still needs clarification. Q49 subsequently sets the default
mark requirement for the bundled tooling's main-branch merge action, with
human-controlled exceptions and separately permitted deployment. Q48 permits
unmarked prototyping in a new repository
without prior SAS acceptance, subject to human-granted permissions and existing
governing documents. No current governed record or authority changes.

### Q42 — Primary authored document format

Confirmed: the owner selected one readable Markdown file with a small structured
metadata header and precisely defined sections as the source developers normally
edit and commit. This supports manual authoring and deterministic parsing.
It selects the primary authoring format, not every future import/export format.
The exact header encoding, field schema, and section grammar are not yet selected.

### Q43 — Later qualification of prototypes

Scenario: prototype code was written without protected acceptance fixtures. The
team later adds fixtures, independently verifies the result, and obtains human
review and acceptance before merging. May that result earn the assurance mark?

Confirmed: the owner selected later qualification under standards that assess
the final result, preserving the actual history. Any standard requiring fixtures
before implementation remains unmet if they were added afterwards. The prototype
is not permanently excluded merely because it began without those fixtures, but
later review cannot establish that an earlier prescribed process was followed.

The exact standards and evidence requirements still need definition; Q44
subsequently settles the mark's scope. This answer does not itself establish
that any existing work has met a qualification standard or received human
acceptance.

### Q44 — Scope of the assurance mark

Confirmed: the owner selected one accepted Warrant result, tied to its exact
code revision and stated scope. A repository can display those records, but
one marked change does not imply that its entire codebase meets the standards.
The selected unit is the Warrant result, not a whole release or the repository
as an ongoing whole.

Q45 subsequently assigns the qualifying baseline to OpenWarrant. The exact
evidence needed and how a mark is represented or issued remain open. The mark's
scope does not imply assurance for other changes or for later revisions that
have not been assessed.

### Q45 — Authority over the assurance baseline

Confirmed: OpenWarrant defines a versioned minimum baseline for the common
assurance mark. Repositories may strengthen the requirements but cannot weaken
them while retaining the same mark. Teams remain free to use other workflows
without earning that mark.

This gives the common mark shared minimum meaning while preserving freedom to
use the document standard. Q46 subsequently defines baseline human review.
The remaining baseline requirements, evidence, and version-transition rules still
need definition.

### Q46 — Human review required for the baseline mark

Confirmed: the human reviews the outcome, verification findings, and remaining
risks, inspects code where needed, and explicitly accepts the exact result.
Independent verification supplies detailed technical review; repositories may
require human review of the full code diff in addition to this baseline.

This makes the required human act substantive without claiming that every line
was manually reviewed. The actual review and acceptance apply to the exact
result selected in Q44, not a generic approval of future agent work.

### Q47 — Finished prototypes need not create acceptance tasks

Confirmed: prototype Warrants may remain "implementation finished, unreviewed"
without automatically generating mandatory human acceptance tasks. Request review
when the user seeks qualification or repository policy requires it.

This lets experiments stop without requiring signed closure merely to clear a
review queue. The status carries no assurance mark and is not signed Warrant
completion. Actual execution and evidence history remain accurate.

### Q48 — Prototype setup without prior SAS acceptance

Confirmed: in a new repository with no governing documents, unmarked prototyping
may start under human-granted execution permissions without first approving a
minimal SAS. SAS acceptance remains required where the chosen workflow or
repository policy calls for it. Existing governing documents remain binding.

This qualifies Q28's earlier universal prerequisite: its minimal-SAS setup path
still describes the reviewed workflow, but is not mandatory for every unmarked
prototype. It does not permit agents to grant their own execution authority or
discard an existing project's governing documents.

### Q49 — Default main-branch merge policy

Confirmed: the bundled tooling's default repository policy requires an assurance
mark before merging into main. An authorized human may explicitly allow unmarked
merges. This governs the tooling's merge action, not basic document validity.
Deployment still requires separate permission.

A merge exception does not award the assurance mark, weaken its baseline, or
establish signed Warrant completion. This policy does not itself establish
enforcement against Git writes performed outside the bundled tooling.

### Q50 — Delivery order for authoring clients

Confirmed: the owner retained native TUI, shared local API, and CLI first, with
browser authoring following through the same API. This reaffirms Q17 while
resolving the ambiguity introduced when Q36 listed browser prompting among the
desired artifact-generator interfaces.

At Q50, the artifact generator and context compiler remained included with the
first standard revision. C11 subsequently replaces the single first-workflow
milestone with staged feature/library, CLI/Compiler, and Workflow development.
The TUI-before-browser client preference remains recorded under that sequence.

### C11 — Feature/library, CLI/Compiler, and Workflow phases

The owner directed development through three phases:

1. **Feature/library phase.** Build the scoped feature files and library
   operations, then test them. Every scoped feature must be runnable through a
   file or CLI command before moving to the next phase. This does not require
   the finished ergonomic CLI to exist before library features can be exercised.
2. **CLI and Compiler phase.** Build a CLI that exposes the primitives, then
   make it interactive and more ergonomic for future callers. The OpenWarrant
   compiler parses machine-parsable, human-readable OpenWarrant documents and
   performs functions such as projections and agent-context optimization.
3. **Workflow phase.** Applications and workflows call the compiler, receive its
   output, and use that output. Slack, Jira, Huey, and other applications were
   examples. The first integration is Knowledge Fabric Compiler, a larger
   compiler that supplies inputs to OpenWarrant and consumes its output.

This establishes Knowledge Fabric Compiler as the first Workflow-phase consumer,
not as a dependency required to demonstrate isolated libraries or the standalone
compiler. Its actual interface and integration evidence have not been established
by this discussion. No connector to a named example application is implicitly
added to the required first integration.

C11 supersedes treating the complete TUI/API workflow as the first development
milestone. Q50's client preference remains TUI before browser within the later
workflow scope. At C11, exact phase-to-release mapping remained open; C12 below
places v1.0 at the end of Phase 2. Phase 1 feature inventory, Phase 2/3 exit
criteria, and the Knowledge Fabric input/output contract remain open.
Q31's OpenWarrant maintenance use case can supply later workflow evidence; it
does not gate proof of the earlier phases.

### C12 — Official standard and compiler v1.0; SAS consolidation

The owner directed use of Matt Pocock's writing-great-skills and related
agent-writing guidance to review SAS, Warrants, and other artifacts. Discussion
will select improvements to context pointers, document structure, projections,
and the compiler. Consolidate the product changes and agreed improvements into
the next SAS draft, rather than leaving the redesign only in discussion notes.

The owner placed OpenWarrant v1.0, the official standard and compiler for other
tools, at completion of the CLI/Compiler phase. Workflow integrations follow;
they are not required to reach that milestone. Exact release acceptance tests
remain to be agreed. This product milestone does not rewrite an existing
signature. C14 below clarifies that the earlier baseline is designated
1.0.0-rc.1 while the SAS continues toward 1.0.0 Stable.

### Q51 — Structured conditions on context pointers

Confirmed: v1 supports structured conditions plus explicit references. The
compiler evaluates declared conditions without an AI call; AI can help draft
either the references or their conditions. Work stage, declared subsystem, and
file path were examples, not a settled condition schema.

Q51 did not define condition syntax, missing-input behavior, dependency closure,
or how source bytes reach a consuming agent. Q52 below settles missing-input
behavior for an available referenced rule; Q53 requires portable offline context
delivery. The
[artifact and projection review](agent-context-review.md) records source-backed
findings, proposed improvements, and candidate tests for the next discussion.

### Q52 — Unknown pointer applicability

Confirmed: when a referenced rule is available but the compiler lacks a field
needed to evaluate its pointer, include the rule and its dependencies and flag
uncertain applicability. The owner chose conservative inclusion rather than
blocking the task projection merely because this selection condition is unknown.

Keep the condition unresolved in the record and preserve its wording. Inclusion
does not turn a conditional rule into an unconditional requirement or establish
that its condition is true. Missing permissions and unresolved architecture
decisions still block affected work. This choice does not waive access controls,
unavailable required content, source conflicts, or context-budget requirements.

### Q53 — Portable offline context package

Confirmed: v1 must export a portable offline context package; a resolver is
optional. Include the task brief, exact rules, required dependencies, and source
manifest so an agent can read all required context without access to the original
repository or a running OpenWarrant service. Optional references remain clearly
labeled; a reference alone is not a claim that its content was supplied.

The owner selected this delivery requirement over requiring repository or resolver
access. It concerns context delivery, not the code workspace needed to execute
the task. Exact packaging, entry-document layout, and handling of optional
background material remain open. This answer does not mandate an archive format,
a network server, or a bundled execution environment.

### C13 — Integrate context management and agent packet design

The owner directed integration of the reviewed improvements into the SAS.
The [working SAS](../sas/drafts/1.0.0-rc.2/WAR_Software_Architecture_Specification.md)
now contains the context and packet requirements, their compiler interfaces,
conformance scenarios, and requirement-index entries. Its
[integration checklist](../sas/drafts/1.0.0-rc.2/README.md) distinguishes this completed
drafting work from remaining wire-format decisions and wider product consolidation.
Writing these requirements does not establish implementation or acceptance.

### C14 — Refine 1.0.0; designate the old baseline 1.0.0-rc.1

The owner corrected the assistant's proposed 1.1 draft: the SAS remains 1.0.0
and is being refined toward stable. The previous baseline is designated
1.0.0-rc.1, not 1.0.0 Stable. Stable follows completion of the compiler phase
and the agreed conformance requirements.

The assistant's unaccepted 1.1 scaffold was renamed to the 1.0.0 working draft.
The [SAS index](../sas/drafts/README.md) explicitly records the RC designation. Existing
signed records still use their original `1.0.0` identifier and source digest;
they were not rewritten to pretend the original signature named RC1. The live
registry/version migration and acceptance of the refined bytes remain governed
acts. The older proposed 1.1 batch/rendering ADR is historical pending work,
not the version target for this refinement.

## Historical artifact migration evidence

A read-only bounded investigation recovered original manifest-pinned bytes for
two representative source files from existing local Git history:

| Path | Current original-pin owners | Matching commit | SHA-256 |
| --- | --- | --- | --- |
| `crates/openwarrant-cli/src/status.rs` | 0055/D-003, 0057/D-002 | `797706e058bb053ef3a7e214ab5bf8692dd4f9fd` | `7ccbbc7ff519fd7c7994aa3e99badde8bd4f9c5b787510eb672421adc6daaeff` |
| `crates/openwarrant-core/src/execution.rs` | 0024/D-001, 0056/D-001 | `5aa019f4ba9fceba018b412f7cd3d67eeec14f96` | `346119a4523830e46b77b5d6e5938b68838e812952913a15c738d8fcce1d7f42` |

The investigation examined 22 and 7 path-history commits respectively. Both
matching commits are ancestors of the observed HEAD. Existing declarations store
the path and SHA-256 but no immutable commit/blob locator; their recorded
`contract_digest = "unrecorded"` remains a provenance gap. Finding matching bytes
does not establish missing production authority. This result covers these two
original pins only; it does not establish recovery of every artifact, correction
head, fresh-clone portability, or protected long-term retention.

## Architecture handoff topics

- Work boundaries -> material changes, permitted implementation choices, task
  splitting, shared-file conflicts, multi-Warrant architecture changes.
- Artifact history -> immutable storage, lineage, distinguishing new development
  from correcting false historical claims, evidence invalidation, migration.
- Human authority -> authentication gesture, delegated scope, final acceptance,
  automatic-mode activation and eligibility, merge behavior, refusal and recovery.
- Verification -> protected fixtures, reviewer independence, disagreement, retries,
  evidence reuse, handling unavailable checks and unexpected failures.
- Hotline -> factual advice versus decisions, routing, durable delivery, question
  deduplication, pause/resume, unavailable humans, amended context propagation.
- Runtime and clients -> compiler/runtime ownership, executor adapters, isolation,
  scheduling, API commands/events, TUI/web behavior, cancellation and restart.
- Adoption -> first real workflow, measured friction budget, safety controls that
  must remain effective, sequencing relative to Warrants 0071-0073, rollout.

Rounds 3-5 settle governing-act delegation, policy activation, merge permission,
local deployment, the administrative budget, both agent connection routes,
workspaces, and fixture preparation. Round 6 settles availability and recovery.
Round 7 settles repair limits, disputed findings, and question routing. Round 8
settles adoption, history retention, setup effort, and the first real proof.
Round 9 addresses platform support, execution protections, cost visibility, and
legacy adoption. Remaining technical contracts and measurement procedures will
be made explicit in the shared-understanding review.

Q1-Q50 are answered. C10 records the owner's assurance clarification; C11 records
the three-phase development sequence and Knowledge Fabric-first workflow integration.
The consolidated specification separates recorded product
decisions from unresolved product boundaries and engineering contracts. The owner
has reopened fundamentals; continue mutual grilling before any SAS revision plan.

## Candidate proof scenarios

These scenarios are proposed for designing acceptance criteria, not executed tests.

1. Approve several ready Warrants once; each eligible agent receives the right
   context and starts, while blocked work explains what it is waiting for.
2. Complete a normal code change to a previously delivered file, retaining access
   to the old accepted artifact and its evidence.
3. Ask an implementation question that another agent can answer without a human
   interruption; retain the source and scope of the answer.
4. Discover an architecture change during execution; review an evidence-backed
   revision in the hotline, approve it, and resume from the approved context.
5. Attempt to weaken a required check to get a pass; observe the intended refusal.
6. Lose a worker or reviewer; recover without duplicate writes, lost decisions,
   invented evidence, or treating an unavailable check as passed.
7. Measure human decisions, active administration time, repeated questions, and
   elapsed time on real work. Compare against an explicitly defined baseline.
8. Run equivalent work through a connected agent using the OpenWarrant skill and
   a launched command. Both bind the same approved inputs and obey one exclusive
   writer protocol; racing claims cannot create simultaneous writers.
9. Prepare fixtures separately, approve them with the Warrant, and run both
   implementation and independent verification against those protected versions.

## Document maintenance

Consolidation on 2026-09-13 produced `openwarrant-product-spec.md`. A separate
read-only consistency review compared it with the confirmed ledger; required-limit
enforcement and policy-controlled pruning qualifiers were restored. Local source
links were checked. Record/generated checking reported 720 pass, 88 warnings,
0 unknown, and 0 errors; this was not an end-to-end runtime or full-gate result.
At the shared-understanding review, the owner selected "Adjust product spec before
planning." Q36 then clarified the standard, manual and agent-assisted authoring,
and master-context projections. Q37 selects a generated master assembled from
source documents. Q38 preserves exact binding wording while allowing background
summaries. Q39 supports compilation without AI; the intervening ownership
clarification separates the standard from consuming tools. Q40 defines document
compatibility as shared structure and field meanings, with workflows chosen by
tools. Q41 permits minimal valid drafts and separates validity from execution
readiness. Q42 selects Markdown with structured metadata. C10 separates
autonomous prototyping from a human-backed assurance mark and signed completion;
Q43 allows later prototype qualification under applicable standards while
preserving actual history. Q44 scopes the mark to each accepted Warrant result
and exact code revision. Q45 assigns a versioned baseline to OpenWarrant, with
repositories allowed to strengthen it. Q46 defines human review of outcomes,
verification findings, and remaining risks, with code inspection where needed.
Q47 lets finished prototypes remain unreviewed without mandatory acceptance
tasks until qualification is sought or policy requires review. Q48 permits new
unmarked prototypes without prior SAS acceptance under human-granted permissions,
while preserving existing governing requirements. Q49 requires the mark by
default for the bundled tooling's merges into main, with human-controlled
exceptions and deployment separately permitted. Q50 reaffirms TUI/API/CLI first,
with browser authoring following. The consolidated product draft's release-proof
scenarios now cover unreviewed prototypes, later qualification, human acceptance
for the mark, and explicitly permitted unmarked merges; an older generic
automatic-acceptance scenario was superseded. Both design documents reflect
these answers and the remaining open boundaries. C11 then introduced the three
development phases and Knowledge Fabric Compiler as the first workflow
integration. The product draft now separates direct feature proof, compiler
proof, and later workflow evidence instead of making full application behavior
the first milestone.
Continue mutual product grilling before seeking shared-understanding confirmation
and proceeding to governed SAS revision planning.

Update this document after each interview round. Record selected answers, their
reasons, changed assumptions, and newly unblocked questions. Promote durable
decisions into the repository's governed ADR/SAS process after the owner confirms
shared understanding. Reuse [CONTEXT.md](../../CONTEXT.md) for existing vocabulary;
record new glossary definitions when their meaning and governing scope are settled.

## C15 — Finish current edition 1.0.0-rc.2 before building

On 2026-09-14 the owner directed completion of the consistent SAS 1.0.0-rc.2,
concrete format examples, and testable Phase 1 scope, followed by implementation.
The [candidate index](../sas/drafts/1.0.0-rc.2/README.md) and
[decision map](../sas/drafts/1.0.0-rc.2/decision-map.md) record that consolidation.
This authorizes drafting and example validation, not a fabricated acceptance,
production compiler claim, or a rewritten signature on the prior edition.
