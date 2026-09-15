# Prompt-only work and later qualification

Reference examples for SAS RC.3 §§5, 11–14 and SDK-09–SDK-18. Expected behavior,
not executed tests or real authorization records. Profile names below describe
conditions; they do not allocate new schema identifiers.

## Case 1: prompt-only prototype

Input: the user asks an agent to implement a small feature from a Warrant. No
OpenWarrant authorization, human signature or qualification record exists. This
Warrant does not declare an enforced start gate.

Expected: the agent works within available runtime capabilities, records the
Warrant as complete and reports "complete — unverified" with relevant limitations.
It does not require OpenWarrant policy enrollment, start approval or signing.
Human execution has the same unverified path. Optional checks can run without
turning this into a qualification ceremony.

Its normal work-stop response starts with the exact configured safeword and links
to the deterministic tracker overview, next steps, notes and document trail.
It does not paste the full overview in chat. Configuration may reduce inline
wording while preserving those artifacts. The same response kind serves verified
work with different qualification standing. The tracker counts both as completed.

Refusal/control: an attempted Verified claim without required evidence and human
acceptance fails qualification. That refusal does not prohibit the unverified work.
Actual unavailable tools, dependencies or resource access remain ordinary execution
problems, not fabricated missing-signature errors.

## Case 2: prototype qualifies during release review

History: implementation began at T1 and finished at T2 without OpenWarrant start
approval. At T3, the team defines the final candidate and required expectations,
protects them, and obtains independent checks against that exact candidate. At
T4, the human reviews the outcome, evidence and risks and securely accepts it.

Expected: the final-result baseline can qualify the exact result when every
applicable condition is met. Records retain T1–T4 as observed. Authorization of
acceptance happens at T4; it is not represented as permission signed before T1.

Refusal/control: a changed candidate or missing required evidence prevents the
mark. A T4 signature cannot satisfy a condition explicitly requiring approval
before T1. No profile is silently weakened to make that condition pass.

## Case 3: a joint-project professional condition

Contract: qualification under the selected professional profile requires both
project approvals of the same shared Warrant revision before stage execution.
Only one approval exists when the stage starts. In this case the condition is
qualification-only; it is not an enforced Warrant action gate (see Case 7).

Expected: work can still execute on the unverified path. The selected professional
profile remains unmet; the missing historical approval stays visible. An explicitly
selected workflow that promises to enforce this profile waits before dispatch
instead. The standard does not make that workflow universal.

Refusal/control: adding the second approval after implementation does not turn
that attempt into an approvals-before-start pass. A later qualifying attempt must
actually meet the conditions at the required time. Independent-start or mixed
arrangements are other explicit contract choices, not inferred exceptions.

## Case 4: one release signing ceremony, several exact results

Input: several prototype Warrants have finished. An agent assembles a review
manifest containing each selected Warrant revision, exact result and code basis,
profile, required evidence and outstanding findings. Humans and independent agents
perform the required review. The human signs one exact, fully presented manifest
with explicit acceptance of its listed result subjects.

Expected: each member receives its own qualification assessment. One ceremony
can supply human acceptance for all explicitly covered members. Individual marks
still bind individual scopes and exact results. Unselected work remains unverified.

Refusal/control: adding or changing a member after signing invalidates that new
manifest's claimed coverage. An uncovered result, failing member or member missing
required evidence cannot borrow a sibling's pass. A signature over a release name
or a legacy single-result subject is not acceptance of an inferred result set.

## Case 5: tracker synchronization and projection failure

History: work finished and a completion event was saved. The tracker update times
out, or its projection is unavailable. The worker cannot establish whether the
tracker includes the result.

Expected: report work complete with tracker synchronization or response delivery
pending. Do not mark the work unfinished, invent an updated project view or emit
the normal confirmed-completion safeword response. Reconcile the saved event with
the tracker, retry using the same identity, and return the complete response once
the resulting projection is available. Do not repeat the implementation.

Refusal/control: a stale projection missing this completion cannot establish the
handoff; a newer projection may include unrelated concurrent work if it still
acknowledges the exact completion/result. Retrying cannot count completion twice.

## Case 6: start the next Warrant by prompt

History: the first Warrant is complete and unverified, and its response was
delivered. The user says "start Warrant 2."

Expected: if the next Warrant has no explicit blocking gate, the agent starts it
on the ordinary unverified path without
asking for acceptance or a signature for the first. The first remains complete.
Both can later enter a release review; neither loses completed status while
waiting for optional qualification.

Refusal/control: a next-step suggestion or a quoted safeword alone does not
authorize extra work. An unfinished requested implementation cannot be made
complete merely by issuing the word or requesting that work be marked done.

## Case 7: Warrant explicitly requires a verified start

Contract: an action gate requires both project signoffs on an exact preparation
revision before a named stage starts. One signoff is missing.

Expected: the stage waits regardless of unverified mode. The caller receives the
exact unmet condition and the action it blocks. Once the condition passes, the
stage may start. A contract without that gate does not acquire it by default.

Refusal/control: changing a mode label or deleting a local copy of an inherited
condition cannot bypass the gate. Amend the effective contract through its stated
process. A qualification-only condition remains separately classified.

## Case 8: feature stop with its Warrant still in progress

History: a declared feature finishes; further Warrant stages remain.

Expected: emit the configured safeword at the top of a response for that feature,
then link to the generated overview showing feature completion and pending Warrant
work. Notes, document trail and next steps are linked or briefly shown. A later
integration, Warrant or program stop uses the same scoped contract.

Refusal/control: the feature stop cannot mark the whole Warrant complete. Token
exhaustion before the feature finishes is an agent stop with no completion signal.
Mentioning the safeword in ordinary chat does not change any tracked state.

## Case 9: brief and detailed output from the same records

Input: one tracker snapshot, completion event and deterministic renderer version.
Generate a progress overview with pending work, indicators, metrics and document
trail. Render minimal and richer response profiles pointing to that same overview.

Expected: minimal chat can contain the configured safeword and scoped overview
link; richer chat can add short notes and next steps. Required details stay in the
linked package. Repeating generation with identical inputs and configuration needs
no model call and produces the same document content. Metrics identify their scope
and denominator; missing values remain unknown.

Refusal/control: shortening output cannot replace unknown metrics with invented
numbers, hide qualification, drop required linked details or create a link to an
unwritten document. A moved latest-view pointer does not rewrite the preserved
work-stop snapshot.

These examples add behavioral scope. Executable codec fixtures belong to Phase 1;
actual prompt-only execution, professional gating and batch-signing UX belong to
Phase 3. Nothing here claims an installed CLI or harness already has these paths.
