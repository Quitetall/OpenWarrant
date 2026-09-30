# Friction feedback: retained changes and current gaps

Review baseline: main `058cdd5da54efebaa59c3aa5e5531db7841defe5`.
Source: owner-supplied practitioner feedback, reviewed against current code and
implementation notes. This is a source-backed planning update, not a new security
qualification, signed SAS adoption, or claim that all ten suggestions are missing.

The historical friction interview points to RC.2; current product candidate is
[RC.3](../sas/drafts/1.0.0-rc.3/README.md). Both candidate status and implemented
behavior must be stated separately. OpenWarrant owns standard/SDK; workflow apps
own sessions, execution and enforcement. No second semantic compiler is implied.

## Disposition of the ten suggestions

| Feedback | Current evidence | Retained change / next proof |
| --- | --- | --- |
| Authority proposal/activation | [OW96 notes](../warrants/OW-WAR-0096/implementation/notes.md), [authority CLI](../cli/authority.md): inert proposals, previous-key signatures, locked activation and history implemented as a reference path. Legacy roles.toml still drives legacy consumers. | Complete protected deployment and consumer cutover. Add trusted activation receipts naming authenticated actor, exact transition digest, prior/new head and observed timestamp. Client timestamps must not prove activation time. Recheck affected pending acts against current authority; refuse stale use without deleting history or indiscriminately cancelling unrelated work. |
| Session approval | [Workbench API](../../apps/openwarrant-web/README.md) has local bearer sessions and origin checks; this is not proof of human presence or a secure approval service. [Legacy signing](../../crates/openwarrant-cli/src/sign.rs) explicitly admits pseudo-TTY bypass of terminal confirmation. | Use one protected approval endpoint for CLI/TUI/browser callers. Bind identity, exact subject, current authority, replay protection and confirmation policy. Session possession alone cannot prove human review. Keep SSH as an authenticated transport where key custody is protected; TTY-only confirmation must not become fallback evidence of secure human approval. `ward` remains a proposed name, not a deployed service. |
| war doctor | No `doctor` command in inspected main CLI. Existing checks expose portions of the desired report. | Add read-only aggregate report: parse errors, authority mode/trust availability, subject drift, signer and performer/verifier configuration, dependencies and actionable remedies. Reuse SDK/admission findings. Distinguish absent optional professional setup from a blocker for an ungated prototype. Never silently fix or sign records. |
| Unified eligibility | Legacy next/frontier/perform and reference execution have separate paths; [execution implementation](../../apps/openwarrant-web/execution.py) has its own admission checks. | Define one typed admission result over exact task, action, actor, authority head, dependencies and runtime facts. CLI views, console and auto-start use its reasons. Re-evaluate at dispatch and each affected privileged action; a cached OPEN label is not permission. Same inputs must produce the same reasons across clients. |
| Guided setup | [Phase plan](../sas/drafts/1.0.0-rc.3/phase-plan.md) explicitly allows prompt-only work. | Provide minimal prototype and optional governed setup profiles. Generate requested SAS/Warrant/fixtures/authority proposals and readiness reports without manual metadata surgery. Do not mandate signed SAS or authority enrollment for every prototype. Reruns preserve existing context documents and records. |
| Batch review | [SDK contract](../sas/drafts/1.0.0-rc.3/sdk-contract.md) already permits exact batch manifests and per-result findings. | Review an explicitly selected frozen set. Preserve each act's identity, digest, eligibility and outcome. Separate signatures or one signed exact manifest may bind the set; do not impose separate signatures when an approved profile supports a batch manifest. Changed/new members refuse. Never expand an unrestricted `--all` at signing time. |
| Execution lifecycle | [Reference execution](../../apps/openwarrant-web/EXECUTION.md): isolated worktrees, cross-store claims, bounded subprocesses, checks and explicit retries exist. Unknown writers deliberately block replacement. | Finish proven fencing, question-driven pause/resume, bounded replacement, availability-triggered starts and capability enforcement. Process termination is not proof that escaped or remote writers stopped. Retain limits, unknown cost and exact attempt identity. Reuse existing implementation instead of rebuilding it. |
| Verification loop | Current reference execution explicitly records performer observations, not independent verification. | Add separately controlled verifier context/workspace, protected checks, bounded repairs, evidence-backed rebuttal/recheck, human escalation and stale-result rejection. Reuse the shared admission result. Never let performer clear its own gate. |
| Portable context | [Dispatch bundles](../cli/dispatch-bundle.md) already carry selected exact context and support references offline with integrity/refusal controls. Complete semantic selection/classification is not established. | Reuse bundles. Qualify provider-produced rule/dependency selection, classification and projection closure separately. A portable transport is not evidence that every applicable rule was selected. |
| Measure friction | [OW95](../warrants/OW-WAR-0095/atoms/40-work-order.md) already scopes a consenting three-developer study, setup and ≤60-second administration targets. | Add workflow event capture and report views for setup, administration, review, questions, retries, waits and unknown cost. Actual consenting sessions establish the target; synthetic harness runs cannot. Retain separate durations and failures instead of hiding review/wait time in one success metric. |

## Delivery order

1. **Close authority trust boundary first (OW96 continuation).** Protected root,
   store, verifier and approval transport; exact activation receipts; authenticated
   consumer routing with no legacy fallback. Prove direct file edits and pseudo-TTY
   possession cannot activate authority or satisfy secure approval. Revocation
   blocks affected next actions and stale pending requests; historical signatures
   retain their original subjects and timestamps.
2. **Shared admission contract, then doctor and session clients.** Establish common
   reasons before separate clients encode divergent rules. A first doctor can wrap
   existing diagnostics read-only while convergence proceeds, but must label any
   unavailable eligibility result UNKNOWN. Secure session work must use the same
   trusted authority boundary, not duplicate it in an app token handler.
3. **Guided setup and exact batch review.** Both reuse admission and approval APIs.
   Prove prototype flow needs no manual records; governed setup produces proposals
   for one concise human review. Reject changed batch subjects and lost-session
   retries without duplicate acts.
4. **Recovery and independent verification.** Complete missing lifecycle mechanisms
   around existing worktree/claim/attempt code; test old-writer fencing before
   replacement, changed-code re-verification, retry exhaustion and unavailable
   agent queues. No UI checkbox may fabricate a stopped writer or a passing gate.
5. **Measure throughout; run OW95 after coherent end-to-end path exists.** Capture
   timing events during development. Execute real participant sessions only with
   consent; publish bounded outcomes and unmet thresholds.

This order refines the proposed authority → sessions → doctor → eligibility
sequence: defining shared admission early prevents three new inconsistent clients.
It does not require a monolithic service or change the four delivery phases.

## Scope and provenance rules

Retain OW96 and OW95 as existing work anchors. Use new or amended unsigned scope
for doctor/admission/session integration and remaining lifecycle slices; do not
rewrite signed legacy Warrant contracts, declare completion from this review, or
allocate duplicate efforts merely because an older interview says “missing.”
The feedback identifies product priorities. Exact wire schemas, service deployment
and release qualification still need their normal implementation evidence.

## Implementation follow-through: OW97

[Doctor](../cli/doctor.md) now has a bounded implementation candidate under
[OW97](../warrants/OW-WAR-0097/implementation/notes.md): read-only existing checks,
configuration observations, stage frontier and diagnostic argv. The table above
records the review baseline; it is not a claim that this candidate was already in
that revision. Shared admission, protected authority and runtime probes remain
explicitly UNKNOWN. Legacy read-error classification is disclosed, not hidden.
