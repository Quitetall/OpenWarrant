# Work stops, agent stops and changes

Normative companion to SAS RC.3 §20. These requirements consolidate the approved
stop model. State names below define meanings, not additions to legacy wire enums.
SDKs represent supplied facts; workflow apps and harnesses enforce execution rules.
The work-stop output contract is SAS §14.1. This companion owns event, change and
recovery semantics. Required behavior below SHALL be preserved by conforming profiles.

## Two stop classes

| Class | Meaning | Report |
| --- | --- | --- |
| Work stop | A declared unit of work meets its finish conditions. The agent records completion and returns its result; verification is separate. | Every work-stop scope: exact configured safeword first, then concise pointers to generated progress overview, notes, document trail and next steps. |
| Harness/agent stop | Execution or an agent turn stops for a reason other than completing the declared work unit. | When possible: partial progress, cause, unfinished work, live operations and next permitted step. Do not imply completion. |

Use **agent stop** as the short label for the second class. A normal turn ending,
human interruption or token limit belongs here when work remains incomplete.
An agent stop can interrupt healthy work; it is not automatically a failure.

A **tool boundary** is the point before a new tool action starts. Harness updates
apply here. It need not stop the agent or produce a user report. This replaces
"tool stop" as a peer of feature completion.

A **checkpoint** is saved recovery information. Either stop class can have one.
It does not prove completion or prove that all writers stopped.

## Work-stop scope

Keep two classes. Describe the completed scope separately:

| Scope | Finish condition |
| --- | --- |
| Feature | A Warrant-defined feature or bounded action meets its checks, fits its contracts and works independently as required by that Warrant. |
| Integration | A declared combination of components or repository versions passes its shared contract checks. Local success alone is insufficient. |
| Warrant | Requested Warrant work is complete. Unverified work reaches the same complete state; verification and acceptance remain separate. |
| Complete | All required work in a named SAS revision and explicit Warrant inventory or selection rule meets its declared finish conditions. |

Feature and complete preserve the owner's earlier definitions. Integration and
Warrant identify additional bounded completion scopes. Stages and milestones can use these scopes;
they need no additional stop classes. Declare finish conditions before execution.
An agent must not invent a smaller unit to claim completion.

One result may complete a feature, its Warrant and the program: use one report.
Unrelated historical Warrants retain their actual states. New required work creates
a revised program scope rather than changing an earlier completion record.

Work-stop reporting makes review easy. It does not create mandatory human
acceptance tasks for every prototype or grant permission to merge. A Warrant can
complete unverified and counts as completed work in the tracker; its acceptance
and qualification remain separate. It needs no later human act to become fully
complete. If the selected program outcome explicitly includes qualification,
report that separate goal as pending without hiding completed work. The
common assurance mark still requires independent evidence and secure human acceptance.

## Keep five facts separate

Profiles SHALL represent these facts separately; their new wire encoding needs fixtures.

| Fact | Values or examples | Distinction |
| --- | --- | --- |
| Stop class | Work stop; agent stop | Describes the event, not whether every process stopped. |
| Work state | In progress; completed; blocked; failed; cancelled; unknown | Applies to a named work unit and attempt. |
| Cause | Unit finished; turn ended; human interrupted; decision needed; limit reached; tool error; connection lost | Causes do not establish outcomes. |
| Execution state | Running; stop requested; stopped; unknown | Applies to named workers and jobs at an observed time. |
| Review and assurance state | Pending; findings open; accepted; qualified; not requested, according to the applicable profile | Completion is not a signature or assurance mark. |

**Blocked**: a named requirement for continuing is missing. **Failed**: an observed
required condition was not met and this attempt ends without meeting it.
**Cancelled**: permission or intent to pursue the attempt was withdrawn.
**Unknown**: available evidence cannot establish the relevant fact.

A failed check is a check result. It can lead to repair under the same Warrant;
it does not by itself mean the Warrant failed or execution stopped. A tool error
can also be recoverable. An unavailable check reports UNKNOWN, not failure or pass.

Examples: "Agent stop; work blocked; decision needed; execution stopped" and
"Agent stop; work in progress; connection lost; execution unknown." When peers
continue, record their running state separately from the stopped worker.

## Two change classes

Classify a change by its effect, not its filename. A draft proposal is not an
effective rule change.

| Change | Examples | Application rule |
| --- | --- | --- |
| Work change | SAS, architecture, Warrant outcome, scope, required behavior, binding constraint, contract or required check | The user chooses to stop affected work now or retain the current basis until its next declared work stop, then change direction. |
| Harness change | Tool routing, response format, logging, retry implementation or execution configuration within existing permissions and work constraints | The harness applies the admitted update before the next tool action starts. No work stop is required. |

A harness change that also changes required behavior, removes a required rule,
weakens a required check or expands permission is not harness-only. Its work or
authority effects follow the applicable process. Routine tool changes need no
extra approval when existing permissions already cover them.

### Work changes

1. Identify the exact proposed revision, affected work and dependents. Keep current
   required rules in context. Independent work can continue.
2. Record the user's choice: **stop now** or **continue to the next work stop**.
   The continuation choice is the explicit old-basis override. Do not ask the user
   to repeat the same approval merely to create a second record.
3. For stop now, stop admitting new affected actions and request supported
   cancellation of live actions. Report remaining running or unknown operations.
   This is an agent stop unless the declared unit already finished.
4. For continue, retain the old rules and identify the exact next work-stop scope.
   At that boundary, report the result against its old basis. Do not start the
   next affected unit before applying the permitted change.
5. Establish that old workers can no longer mutate the affected scope. Preserve
   history and the old packet. Apply the authorized update/archive transition,
   assess affected work and evidence, deliver the new packet, then resume when
   its requirements, permissions and limits are satisfied.

While an effective work change awaits the user's choice: block new
affected actions, preserve progress and request one decision. Silence does not
grant continuation. Drafting possible changes does not trigger this block.

An intervening agent stop does not count as the selected work stop. After a token
limit, continuation can retain the old basis until the named feature finishes,
provided its recorded permission remains valid. The user can change the choice.
Finishing the old-basis unit does not prove compliance with the new rule; record
required follow-up work explicitly.

Active required rules cannot leave context while affected work continues.
Archiving an old revision does not establish that its requirement ceased to apply.
Replacement rules and required dependencies must be available before resumption.
An override does not expand a hard spend cap or restore revoked permission.
Mandatory precedence: revocation, mandatory limits and explicit emergency cancellation
block new affected actions without waiting for completion.

### Harness changes

1. Record the admitted update and its revision. Before dispatching the next tool
   action, check pending updates and apply them in the declared revision order.
2. Record the effective harness revision for that action. Queued actions that have
   not started are subject to this check. Concurrent dispatch uses the same admission
   control so an action cannot skip an already admitted update.
3. Running actions do not retroactively use the new configuration. If applying an
   update requires cancellation or restart, block affected dispatch, save recovery
   information and establish the actual stop/restart result.
4. If applying the update fails, block affected tool dispatch and report the failure.
   Do not silently run the next action under stale configuration.

If no further tool call occurs, the update can remain pending. Apply it before the
first affected tool call on continuation. Response-format changes should also apply
before the next response the harness can control; already emitted text is unchanged.

## Scenario coverage

These families define required conformance scenarios for the contract. New causes fit the same two classes;
unknown causes stay explicit rather than requiring a new stop type.

| Scenario | Classification and handling |
| --- | --- |
| Feature meets declared finish conditions | Work stop, feature scope. Send notes, evidence, remaining review and tracker link. |
| Tool succeeds; feature is incomplete | Tool boundary only. Continue if permitted. |
| Final feature also finishes Warrant and program | One work-stop event with all demonstrated completed scopes. |
| Work ends; verification or acceptance is pending | Work stop; Warrant complete, unverified. A selected qualification workflow can remain pending without relabeling work as unfinished. |
| Prototype finishes; qualification was not requested | Work stop for permitted scope, unqualified. No fabricated mark or mandatory acceptance task. |
| Agent ends its turn after a progress update | Agent stop if execution pauses. A progress message alone need not pause execution. |
| Human stops response or says "pause" | Agent stop. Preserve progress; inspect live tools and children. Interruption alone does not cancel the Warrant. |
| Human explicitly cancels the assigned attempt | Agent stop; cancelled attempt. Stop affected execution and retain history. |
| Token, context or turn capacity runs out | Agent stop. Save checkpoint if possible; do not claim a feature finished. |
| Context compaction or model handoff occurs without pausing execution | No stop event needed. Preserve exact work basis and required context. |
| Worker replacement or harness restart pauses execution | Agent stop. Establish exclusive writer ownership and supply required context before replacement work begins. |
| Tool action fails; bounded retry or repair is permitted | Record failure; continue within limits. No stop unless execution pauses or the attempt ends. |
| Required check fails; repair cycles are exhausted | Agent stop; failed attempt with observed findings. Preserve evidence and report next steps. |
| Required check cannot run | Check result UNKNOWN. Block dependent work if required; other permitted work can continue. |
| Qualification-only prerequisite is missing | Unverified execution remains available. Refuse the unsupported mark. |
| An explicit Warrant start/signoff gate is unmet | Block its named action even in unverified mode. Report the missing condition; do not invent a global gate for other Warrants. |
| Human answer or prerequisite is missing | Affected work blocked. An actual worker pause is an agent stop; independent work can continue. |
| Performer disputes verifier finding | Keep finding open during independent recheck or escalation. A pause is an agent stop, not acceptance. |
| Hard runtime limit reached, resource access revoked or user withdraws execution instruction | Block new affected actions; request supported cancellation. Report stopped or unknown execution honestly. |
| Provider, connection or host disappears | Surviving system records agent stop if observable. Execution can be unknown; inspect before retrying possible side effects. |
| Tool returns after launching background job | Tool boundary observed; job remains running. Turn end and timeout do not prove it stopped. |
| User chooses work change now | Agent stop within unfinished work. Preserve old basis, stop affected writers, apply permitted change, then resume. |
| User defers work change to next work stop | Continue under recorded old-basis override. Report unit, apply change, then start next affected unit. |
| Harness update arrives during tool action | Apply before next action starts. Current action retains old harness revision. Restart requirements can cause an agent stop. |
| Harness update fails or conflicts with another update | Block affected dispatch and report update failure/conflict. Do not select an unapproved fallback. |
| Harness update also changes architecture, required checks or authority | Separate its effects or use the stricter applicable work/authority process. Filename cannot bypass that process. |
| New packet lacks a required rule | Do not resume affected work. Missing context is a blocker; packet budgets cannot remove requirements. |
| Work finishes, then agent crashes before reporting | Preserve observed completion evidence; report delivery remains pending. Recovery reconciles and sends missing report. Crash is a separate agent-stop event. |
| Repositories pass locally but fail shared check | Preserve local results; integration incomplete. Repair if permitted, otherwise stop with actual failure or blocker. |
| One worker stops; peers continue | Record stopped worker and affected dependents. Do not claim project-wide execution stop. |
| New commits or rules arrive after checks passed | Assess exact changed basis and rerun affected checks/verification. Old evidence retains its original scope and revision. |
| New work added after complete stop | Preserve completion record. Define next scope and obtain applicable permissions before dispatch. |
| Declared work unit completes without verification | Record its completed scope/unverified standing; return exact safeword and configured pointers to generated overview, notes, trail and next steps. |
| Work completes but tracker sync/projection fails | Preserve completed work and report sync/delivery pending. Reconcile using the same event identity; do not fabricate an updated view or duplicate completion. |
| User prompts to start the next Warrant | Start subject to that Warrant's explicit gates. Do not require the prior Warrant's qualification unless a declared prerequisite requires it. |

## Reports and recovery

Work-stop response profile (illustrative placeholders; required output rules are in SAS §14.1):

```text
<configured safeword>
Scope: <completed feature, integration, Warrant or program; exact basis>
Work: <state of that scope; do not complete its parent implicitly>
Qualification: <unverified or established qualification>
Progress: <link to deterministic tracker overview>
Notes and document trail: <links; optional short notes>
Next steps: <brief next step or link; label suggestions>
```

Prefer links over pasting generated state. The overview contains pending work,
progress indicators, statistics and data views, computed from records with scope,
denominator and as-of identity. Its revision acknowledges the completed event/result;
a newer compatible revision can include concurrent changes. Never replace it with
an agent-written estimate. Configurable minimal output can be safeword plus scoped
overview link, with notes/trail/next steps available there. Richer profiles can show
short excerpts without duplicating the whole document in chat.

Safeword is configurable; the reference default is `WORK_DONE`. Output it
exactly on the first line only for completion of the declared work unit. It may be
mentioned in regular conversation; a word alone is not a completion event, signature
or assurance proof. Every feature/integration/Warrant/program work stop uses this
contract. Partial feature stops must not complete their parent. Agent/harness
interruptions use a different report and no completion signal.

An agent-stop report uses `Saved`, `Unfinished`, `Cause`, `Live operations` and
`Resume` instead of a completion claim. Include a tracker link when available.
Crash or forced interruption can prevent that message. The surviving harness
records what it can; the next worker reconciles facts before reporting.

Underlying record: event/attempt identity, timestamp, affected scope, work and
harness revisions, observed results, artifact/checkpoint references, pending change
and user's choice, live operation handles, and resume conditions. Reports are views
of these records. Re-delivery must not create duplicate completion or side effects.

"Stopped" requires evidence that relevant old writers ended or cannot write to the
affected scope. A stopped response is insufficient. Without that evidence, retain
stop requested or unknown state; do not replace the writer or remove required rules.
Recovery inspects actual files and live jobs before replay. Timeout does not prove
an operation never happened.

Uncommitted work can be checkpointed with exact bytes and a base revision. A stop
does not itself require commit, merge or deployment. Resuming preserves prior
attempt evidence and rechecks required context, permission, writer ownership,
pending changes and remaining limits.

## Ownership and implementation proof

OpenWarrant defines portable meanings and SDK record/validation helpers. Context
providers build changed packets. Workflow apps own progress, decisions and delivery.
Harnesses enforce tool admission and report actual cancellation/fencing capabilities.
No SDK record alone can stop a process or prove another harness loaded a rule.

Implementations SHALL define and test update ordering, decision-role policy,
bounded overrides, cancellation/fencing evidence and resume acknowledgements in
their versioned profiles. If updates conflict or required evidence is unavailable,
affected dispatch remains blocked/unknown. No inferred continuation, expanded
permission, stale dispatch or false stopped/completed claim is permitted. Wire and
transport choices can be refined within this boundary; changing the boundary is
a work change. Existing user choices are reused without duplicate approval forms.

Phase 1 SHALL provide supplied-record cases for these distinct states, invalid
combinations, event identity, scopes and update bases. Phase 3 SHALL exercise every
scenario family above with the real tracker/harness, including interrupted live
writers and retry of side effects. Phase 4 adds fault-injection and recovery proof.
Record observed case/build/input identities; a table of expected behavior is not
runtime evidence. Routine stop records and checkpoint references are generated
without requiring the human to fill in forms or report on each tool call.
