# W68 skill transport evaluation

Current status: implementation in progress, unverified. The first hosted run
passed all five expected transport scores at `dbdfadd3e8eaf728bd1c782f872fc0cd4b764b10`.
Artifact inspection then found both tickets milestones sharing an all-files
obligation. The corrected fixture binds API.md to OBL-001 and CLIENT.md to
OBL-002, with M2 depending on M1. Drafter and performer interface checks pass;
the updated full hosted run and measured baseline are still pending.

The final local five-test run hit its 900-second execution guard after four
tests completed; its full result is UNKNOWN. A separate 600-second capture also
ended before a complete result was written. Both observations remain unchanged.

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
