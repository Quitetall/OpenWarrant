# W68 skill transport evaluation

Current status: the corrected loop and measured skill baseline are implemented,
unverified. Hosted run 40088204346 at `64603ffda27907bd90fb30856a54b25ca1986b73`
passed the unchanged original twelve-task baseline and all five skill cases.
Every performing case retained a real Dispatch, positive task-context account,
and selected glossary; combined context stayed within the stage budget. The
byte-identical observed skill result is now the separate regression baseline.
Eight public CLI controls and Clippy pass. Review/map examples, the owner's
first-run review and independent qualification remain separate pending work.

Earlier false-positive hosted scores and local observation timeouts remain
preserved. They were not promoted into this baseline. No signature or legacy
resolution is claimed by these fixture observations.

The eval interface now accepts explicit repository-relative context sources.
It assembles exact UTF-8 bodies and SHA-256 hashes into a packet for the drafter
and performer. Source hashes appear in the result notes. Context delivery is
included in each relevant input estimate. The frozen proposal, milestones and
report envelope formats are unchanged.

The skill fixture consumes the selected skill, shared workflow and glossary.
The specification case also receives drafting guidance. These are deterministic
transport fixtures, not language models. They check method/source availability
and produce known structured proposals. They cannot establish model judgment,
owner review, independent Warrant verification or human acceptance.

The cases cover:

| Case | Required observation |
| --- | --- |
| grill | Q-001 is retained in the request and exported note |
| spec | one bounded retry policy retains the settled requirements and refusal case |
| tickets | milestone M2 depends on M1; API.md exists while CLIENT.md is absent; result remains partial |
| grill-unanswered | questions are recorded without allocating a Warrant; result needs input |
| grill-12k-budget | the complete context delivery exceeds this budget and remains over budget |

The initial fixture emitted JSON-style YAML; the current milestone reader did
not find a stage. That failed run is preserved. The fixture now emits the
supported block form. The next run measured 14,652 to 16,974 estimated input
tokens and exceeded the exploratory 12,000-token allowance for all three cases.
Those observations are preserved. Full-context cases now declare 20,000 tokens;
the original limit remains an explicit budget control. The repository's
60,000-token review cap is unchanged. This is not evidence of context-size
optimization.

The unanswered case exposed a separate eval defect: the planner saved questions
and returned success, while eval called the missing Warrant an error. Eval now
recognizes the structured intake result as `needs_input`. It does not fabricate
an alias or report the work complete.

The original twelve-task baseline remains unchanged. The skill baseline is
separate. All observed results remain fixture observations. W68 remains open
for the remaining review/map evidence, the owner's first-run review and
independent qualification.

## Earlier validation checkpoint

The first full public-CLI control run passed five tests. Final Clippy passed
after correcting the nested conditional and tightening waiting-input accounting.
The final source test run and sealed-tool provenance capture are still active
at this checkpoint. No final skill baseline has been adopted yet. An incomplete
executable copy caused the separate record attempt to exit 126; it is not a
fixture result. Its partial output was removed after that attempt stopped.

Earlier scratch programs were archived and every regular file was checked
against its archive before the working copy was removed. Durable raw evidence
and active job logs are under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow68-eval-20261024/`.

## Hosted capture

The existing scheduled/manual eval workflow now runs these five cases and
retains their actual result, executable/source identity and scratch archives.
Its existing 30-minute job limit remains unchanged; the skill capture command
has a 600-second guard. The retention helper archives inputs and outputs before
rejecting unexpected scores. Artifact upload runs even on failure. The original
twelve-task baseline and its comparison remain unchanged. This capture must
succeed and be inspected before a measured skill baseline is adopted.

## Hosted artifact review and correction

[Run 40080431918](https://github.com/Quitetall/OpenWarrant/actions/runs/40080431918)
retained the real output and five scratch archives. Its source commit and
executable version match; every downloaded archive matches the uploaded hash.
The first scores are preserved in [the actual result](hosted-40080431918.json).
No baseline was adopted from these green scores.

Inspection found a logical scope problem: both tickets milestones referenced
one obligation covering both files. Preparation needs its own bounded
obligation before the dependent caller can be assessed. This is an observed
artifact defect, not a claim that a runtime scheduler deadlocked. The
[red observation](stage-scope-red.json) preserves the original graph.

The corrected fixture produces one obligation per target and corresponding
milestone and delivery references. Its API.md now states a usable GET /health
contract and the caller's required response handling. The
[interface observation](stage-scope-green.json) records the separate scopes,
prerequisite edge and absent dependent caller. The public CLI regression now
requires these exact relationships. Full corrected execution is pending.

## Refused Dispatch bypass and correction

The earlier hosted result recorded `dispatch.unauthorized` with exit 2, then a
successful performer call and a `would_satisfy` score. No packet had been
emitted, so its Dispatch input estimate was zero. The
[actual result observation](dispatch-bypass-red.json) preserves this defect.
These runs cannot support a measured successful-loop baseline.

The scratch loop now explicitly requests `--prototype` for both packet emission
and service execution. This records unverified authority; no signature is
created. It commits the synthetic source before selection, retains the real
Dispatch and context manifest, requires a typed packet and token account, and
never calls its performer following an unsuccessful dispatch. The additional
context also counts against the stage's unchanged budget before performance.
Unknown observations remain distinct from failures. Early exits retain known
input totals instead of resetting them to zero.

Eight CLI controls pass, including real glossary/packet delivery, malformed
executor refusal without performer output, supplemental-context budget refusal,
and an actual prototype service run with a submission requesting verification.
A context-delivery test now correctly observes an overrun against its unchanged
12,000-token limit. Final Clippy passes. The original twelve-task baseline is
unchanged; compatibility is being checked against actual executions.

The optional Bonsai report job separately refuses W68 because it has no bound
scope sidecar. `scope.toml` contributes to the immutable contract digest, so
adding it just to clear the job would change the signed contract. No sidecar,
authority act or disposition was fabricated. A governed scope revision remains
separate from these implementation tests.

## Service prototype path

The first corrected packet recording observed eight original code/document
cases satisfying their expected rung, while four service cases correctly
refused `dispatch.unauthorized`: their separate `war run` call also needed the
explicit prototype flag. The original results and comparison are preserved.
The harness now requests prototype service execution too. A public regression
requires a successful run and an actual submission requesting verification.
All eight CLI controls pass after both fixes; final Clippy passes. The complete
local recording did not finish before its guard, so full-suite recording and
baseline adoption still await exact-source hosted execution. Original generic
fixture definitions and baseline bytes remain unchanged.

## Corrected measured baseline

[Hosted run 40088204346](https://github.com/Quitetall/OpenWarrant/actions/runs/40088204346)
passed all twelve original tasks against their unchanged baseline and all five
skill cases. Its source commit and executable version match. Every downloaded
archive matches the uploaded hash. Packet inspection confirms actual task
context estimates of 1,999 to 2,222 tokens, plus explicitly counted supplemental
context, all below the unchanged 8,000-token stage budgets.

The actual skill result was copied byte-for-byte to
[the new baseline](../../../../../evals/skills/baseline.json). Its source,
executable and archive hashes are in
[the provenance record](../../../../../evals/skills/baseline-provenance.json).
The original generic baseline is unchanged. The public comparator accepts the
observed result and refuses a mutated failure with `eval.regressed`, exit 2.

Readable actual outputs: [answered export](examples/grill/EXPORT.md),
[retry policy](examples/spec/RETRY_POLICY.md),
[transport contract](examples/tickets/API.md), and
[dependent milestones](examples/tickets/milestones.yaml).
The caller file remains absent and its result partial. The scripted verifier's
verdict counts are fixture data, not independent review. Model quality and
human review have not been measured.
