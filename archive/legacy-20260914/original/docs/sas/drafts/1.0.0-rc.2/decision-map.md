# RC.2 decision and migration map

Status: rationale and traceability for the proposed RC.2 source set. The interview
ledger preserves what was said at each point. This map resolves later answers
against earlier ones; it does not claim that the owner selected every engineering
mechanism proposed in RC.2. Those mechanisms are reviewable in the architecture
proposal and become binding through adoption.

Section references below name the [RC.2 SAS](WAR_Software_Architecture_Specification.md).
Fnn and Tnn identify [build-scope](phase-1-build-scope.md) features and cases.
A Phase 3 proof is planned future workflow evidence, not a missing Phase 1 test.

## Owner decisions Q1–Q53

| Decision | RC.2 interpretation | Source / planned proof |
| --- | --- | --- |
| Q1 | One reviewable outcome, stages within it | §5; F02, Phase 3 multi-stage example |
| Q2 | Preserve old bytes; successor Warrant authorizes new version | §13; T51 |
| Q3 | Easy human acceptance; automatic disposition allowed, human mark/completion retained by C10 | §§11–12; T45–T46 |
| Q4 | Independent agent/context/workspace plus protected checks | §12; T44, Phase 3 isolation proof |
| Q5 | Outcomes and explicit constraints bind; implementation advice may change | §5; F02/F06 |
| Q5 supplement | Attach ADRs and supported documents without copying their authority | §§4–5; T11/T21 |
| Q6 | Pre-work fixtures for reviewed path; Q43 permits later baseline qualification | §§12, 14; T47 |
| Q7 | Pause affected work and dependents only | §14; Phase 3 hotline test |
| Q8 | Human policy may delegate even architecture work; no automatic human-backed mark | §11; T45/T48 |
| Q9 | Delegated SAS adoption may change authority under policy; separate from signed Warrant completion | §11; T48 |
| Q10 | Human approves effective automation policy edits, including indirect changes | §11; T48 |
| Q11 | Merge only explicitly permitted; deployment separately permitted | §15; Phase 3 merge/deploy refusals |
| Q12 | Developer machine first; local background service | §§14, 16; Phase 3 |
| Q13 | ≤60 seconds routine administration, no manual records/shell after setup | §16; Phase 3 timed user study |
| Q14 | Standalone first supersedes initial BLUT/Katana/LAMU-first selection | §§2, 16; no-model/no-service Phase 1/2 runs |
| Q15 | Authenticated unlocked-session approval; stronger confirmation optional | §14; Phase 3 stale-session and exact-subject tests |
| Q16 | Named human-enabled repository policy, visible mode, Warrant manual override | §11; Phase 3 policy tests |
| Q17 | Native TUI over API precedes browser, now within Phase 3 | §16; Phase 3 |
| Q18 | Connected agent skill and launched coding-agent command | §14; Phase 3 both adapter paths |
| Q19 | One worktree per Warrant; serialized writers | §14; Phase 3 competing-writer refusal |
| Q20 | Separate preparation agent; fixtures and Warrant reviewed together | §§12, 14; Phase 3 |
| Q21 | Visible queue when no eligible agent; auto-start on availability | §14; Phase 3 |
| Q22 | Preserve/recover within limits; previous writer must be unable to write | §15; Phase 3 stale-writer fault injection |
| Q23 | Update stale branch within scope; rerun affected checks and verification | §15; T46 plus Phase 3 rebase/conflict tests |
| Q24 | Repair fixable defects under same Warrant, then independently reverify | §15; Phase 3 |
| Q25 | User chooses count; three post-implementation repair cycles only as fallback | §15; Phase 3 limit tests |
| Q26 | Independent verifier rechecks rebuttal; human settles unresolved disagreement | §15; Phase 3 |
| Q27 | AI-first technical hotline; governing questions to authorized respondent | §14; Phase 3 |
| Q28 | Minimal SAS setup for reviewed workflow, qualified by Q48 | §§5, 11, 14; T10 plus Phase 3 onboarding |
| Q29 | Ordinary 5–10 minutes setup; advanced optional longer discussion | §16; Phase 3 measured setup |
| Q30 | Project-lifetime recoverable accepted bytes and evidence | §§10, 13; T49–T53 |
| Q31 | OpenWarrant maintenance change proves full workflow later | §16; Phase 3, not compiler blocker |
| Q32 | Linux and macOS | §§2, 17; Phase 1/2 platform runs |
| Q33 | Harness supplies sandbox; OpenWarrant checks required protections | §14; Phase 3 tamper/isolation proof |
| Q34 | Unknown cost allowed by policy; mandatory unenforceable spend cap refuses | §15; Phase 3 |
| Q35 | Preserve legacy states; block relevant prerequisites/integrity only | §13; T50/T52 |
| Q36 | Manual and agent authoring; shared standard; generated master/task context | §§4, 6, 16–17; F06/F11 and Phase 2 CLI |
| Q37 | Master assembled from separately maintained captured sources | §6; T11/T15/T27 |
| Q38 | Exact binding wording; summarize only background | §§7–8; T21/T28/T31 |
| Q39 | Complete inputs compile without AI | §§2, 6; all direct drivers and Phase 2 offline proof |
| Q40 | Shared structure and field meanings; tools own workflows | §2; F01/F02/F09 |
| Q41 | Minimal valid drafts; readiness separate | §§2, 4; T01/T10 |
| Q42 | Markdown with structured metadata | §4 and format F1–F2; T01–T10 |
| Q43 | Later qualification allowed, actual fixture timing retained | §12; T47 |
| Q44 | Assurance covers one result at exact revision and scope | §12; T46 |
| Q45 | OpenWarrant baseline, repositories may strengthen only | §12; T47 and baseline predicate tests |
| Q46 | Human reviews outcome, evidence and risks, code as needed | §12; T45–T46 |
| Q47 | Finished unreviewed prototypes may rest without acceptance queue | §11; Phase 3 state/UI test |
| Q48 | Prototype without SAS acceptance if no existing governing documents | §11; T10 plus Phase 3 permission test |
| Q49 | Mark required for main merge by default; human exceptions remain unmarked | §15; Phase 3 |
| Q50 | TUI/API before browser; interactive CLI is Phase 2 | §§16–17; Phase 2 CLI and Phase 3 client tests |
| Q51 | Structured conditions plus explicit references | §7 and format F3–F4; T16–T20 |
| Q52 | Unknown applicability includes required rule/dependencies with uncertainty | §7; T17–T18 |
| Q53 | Offline package required, resolver optional | §9; T32–T37 |

## Owner clarifications

| ID | Preserved meaning | RC.2 location |
| --- | --- | --- |
| C01 | API, native TUI, web tracker | §16, Phase 3 |
| C02 | Title/description with detail available | §14 |
| C03 | Select multiple Warrants, one exact-subject Approve & Start action | §§14–15 |
| C04 | Warrant implementation spec and required fixtures | §§5, 12 |
| C05 | Compiler assembles and projects context for orchestration | §§6–10 |
| C06 | Every executing agent has hotline | §14 |
| C07 | Architecture questions need authorized decisions | §§11, 14 |
| C08 | Material architecture change creates new contract revision | §5 |
| C09 | Review revision through hotline and resume affected work | §§5, 14 |
| C10 | Autonomous prototypes allowed; human acceptance for assurance/signed completion | §§11–12 |
| C11 | Feature/library → CLI/Compiler → Workflow; KFC first integration | §§16–17 |
| C12 | Official standard/compiler v1.0 at end of Phase 2 | §17 |
| C13 | Integrate context-management/packet improvements into SAS | §§6–10 |
| C14 | Earlier baseline is rc.1, refining 1.0 rather than 1.1; preserve old signatures | §§1, 18 |
| C15 | Current edition is 1.0.0-rc.2; finish consistent SAS, examples, and Phase 1 scope before building | This source set; implementation remains next work |
| C16 | Consolidate the handoff and one canonical vocabulary before RC.2 acceptance; SAS 1.1.0 is withdrawn, without rewriting its historical references | §§1, 3, 18 |
| C17 | Canonical definitions bind inside RC.2; examples and usage advice remain guidance | §3.1 versus §3.2 |
| C18 | Preserve signed OW-WAR-0071/0072/0073; prepare successors when needed, without performing supersession now | §18; adoption work plan |

## Explicit supersessions

- Q3/Q8/Q9/Q16 automation is not human acceptance or common assurance. RC.2
  distinguishes policy disposition and delegated governance adoption; C10/Q44–Q47
  govern formal signed completion and the mark. No agent can edit its effective
  permission policy by calling the edit a SAS amendment.
- Q6/Q20/Q28 define the reviewed path, not all valid documents or prototypes.
  Q41/Q43/Q48 permit minimal drafts, no-SAS prototypes, and later qualification.
  The baseline checks the final result; fixtures-before-work is a stricter profile.
- Q14's initial mandatory adapter stack was explicitly withdrawn. C11/C12 place
  standalone standard/compiler first and KFC first only in the later workflow phase.
- Q17/Q50's clients remain intended; their release position follows C11.
- Live-path correction fanout in RC.1 RQ-036 is replaced for RC.2 successor work.
  The existing implementation and old records keep their current rules until
  migration; preserving history is not merely deleting a pin check.

## RC.1 section migration

This table covers every numbered RC.1 section. It is a map, not incorporation of
all old text as RC.2 SHALLs. Historical references always use original source bytes.

| RC.1 sections | RC.2 treatment |
| --- | --- |
| 1–11 | Recast purpose, scope, ownership and laws in §§1–8; remove mandatory sibling runtimes and universal workflow prerequisites |
| 12–17 | Identity/source-holder invariants in §§4/18; single Markdown format F1–F3; atom parent model retained only in explicit legacy adapter |
| 18–23 | Required outcomes, optional detail, ADRs, child/successor and stages in §§4–5/14; no universal five-atom minimum |
| 24–32 | State/authority/contracts in §§5/11/13; validity distinct from readiness; no policy-service substitute for human-mark completion |
| 33 | Exact context, pointers, closure, summaries and budget in §§6–10 and format F3–F7 |
| 34–37 | Source traceability, unknowns, bounded claims, artifact lineage in §§5/8/12–13 |
| 38–46 | Optional assurance and typed record semantics in §§12–13/F8; actual gate execution stays in workflow/harness |
| 47 | Versioned task packet in §9/F5–F7; old Dispatch preserved by legacy adapter |
| 48–50 | Optional runtime adapters, §16; laboratory/contractor extensions outside first software release |
| 51–57 | Claims, retries, independence, limits, resolution standing in §§11–15; execution service deferred |
| 58–65 | New source and packet schemas in §4/F1–F9; old canonicalization and digest domains retained in §18 |
| 66–69 | Preserve original journals/actions/exports and authority; stateful transaction/federation design later; new package F7 |
| 70–76 | Scoped Phase 2 author/check/context CLI; old workflow commands remain legacy and later workflow surfaces |
| 77–89 | Rust/core/compiler/CLI split in §6; explicit I/O, limits, integrity and accounting in §§9–10; optional runtime/storage details deferred |
| 90–100 | Replace eleven-phase/all-integrations gate with §17 and explicit feature/case matrix; workflow friction targets §16 |
| 101–107 | Exact source-set adoption §18; retain and rescope requirement IDs §106; replace old examples with RC.2 reference fixtures |

The prior unaccepted context draft's cases 96–113 are covered by T11–T41 and
Phase 2 deterministic/agent evaluation. They were specifications, not completed
tests; their renumbering does not erase evidence because no execution was claimed.

## Engineering choices made explicit in RC.2

TOML framing, local identity syntax, marker units, limited condition grammar,
consistent-cycle closure, directory packaging, typed record inspection, and exact
Phase 1 inventory are proposed design choices under the owner's instruction to
finish the candidate. They were not earlier multiple-choice interview answers.
The architecture proposal records alternatives and compatibility costs. Accepting
RC.2's exact source set is the later point that makes them governing here.

Still intentionally outside this candidate's Phase 1/2 scope: deployment topology,
service storage engine, signing/session implementation, KFC transport, hosted
permission administration, worker fencing mechanism, and online resolver protocol.
Their Phase 3 contracts must be specified before implementing them. The assurance
mark's public name does not change its defined baseline or postpone parser work.

## Consolidation boundaries, 2026-09-14

The vocabulary discussion proposed treating a legacy resolution as both new
acceptance and qualification. That shortcut is not adopted: Q43–Q46 require
the actual exact-result evidence and the baseline's human review. §18 preserves
legacy resolution facts and permits only evidence-supported derived claims;
F09/F10 still evaluate qualification separately. It also retains legacy CLI
meaning through Phase 2, as the existing additive command plan already specified.

The handoff's description of three authorized Warrants was checked against their
authorization records. Its broader suggestion that both bound ADRs were already
accepted is not treated as proof: ADR-0019 and ADR-0020 retain proposed metadata
and different prose acceptance conditions. §18 records those facts without
changing the sources or manufacturing a disposition.

The handoff's `git log f22ef2f..HEAD` instruction was stale: the root checkout is
still f22ef2f. The cited correction/performer fixes are ancestors of that base,
not five additional commits to merge. Their existence informs migration risk.
The referenced temporary 2026-09-13 handoff file was absent when reread; no claim
depends on having read it during this consolidation.

Cross-program roadmap/requirement-lifecycle requests are tracked in the adoption
work plan. RC.2's standalone format uses explicit captured paths and source
identities; it does not promise an online cross-repository URI resolver, a new
requirement-lifecycle record grammar or an unimplemented in-force annex command.
Those requests must not be reported implemented by projection or by an ignored
legacy metadata field.
