# Amendment review and publication evidence

## Current status

The owner ran the helper and confirmed both corrections. Sequence-2 records match
current README and xtask bytes; `signed_via = "tty"` records terminal confirmation,
not a new SSH signature. Original responses and original SSH signatures were moved
to digest-tagged paths without byte changes. All nine conflicts are reconciled.
After recompilation, corpus check reports 846 passes, 88 warnings and zero errors.
The requests and instructions below are retained as the review trail; do not rerun
the helper for already-corrected bytes. This does not accept the SAS or qualify work.

## Review trail before owner confirmation

The README, banner and diagrams were reviewed against the RC.3 design. No false
release, implementation or assurance claim was found. The standards review found
an archive verifier bug under Python optimization; explicit runtime checks fixed
it, and eight normal/optimized positive/refusal cases pass. The migration cutover
checklist now explicitly requires validated new OpenWarrant documents and exact
archived-source/section mappings.

Seven legacy digest conflicts under unresolved OW-WAR-0068 have been reconciled
as new candidate deliveries. Its original delivery manifest remains byte-exact
in `attempts/attempt-1/`; attempt 2 records old/new file hashes, the known contract
evaluation basis and source inputs. Original authorization, contract and evidence
remain unchanged. Candidate registration does not satisfy contract obligations,
independent verification, owner review or resolution.

Two conflicts remain in signed correction chains:

| Subject | Change to review | Exact request | Tool-generated signing preview |
| --- | --- | --- | --- |
| OW-WAR-0060/D-002 | xtask's skill-reference test checks linked/present files rather than freezing the reference count at four | [Request](xtask-correction-request.json) | [Preview](xtask-signing-preview.txt) |
| OW-WAR-0062/D-003 | README describes the standard/SDK/provider/workflow design and includes the new banner/diagrams | [Request](readme-correction-request.json) | [Preview](readme-signing-preview.txt) |

Existing `war correct`/`war sign` enforcement and repository legacy guidance
reserve these acts for a human. The agent has prepared requests and has not
signed either one. From a human terminal, this single entry checks that the
reviewed bytes remain unchanged, then presents the two signing prompts:

```sh
bash docs/design/amendment-review/sign-corrections.sh
```

It signs only these two corrections after the human confirms each prompt. It
does not accept the SAS, resolve OW-WAR-0068, qualify new work or publish a release.
Parser implementation proceeds independently under the owner's prompt permission.

The app's LAMU MCP connection returns `Transport closed`. A fresh task-owned MCP
session can call the same `review_commit` tool through local `lamu serve` fallback.
Its environment excludes cloud credentials and uses an empty task-local cloud
catalog. Local model selection remains automatic. A task-local HTTP adapter sets
`enable_thinking=false` so the local model returns review text within the response
budget; it does not alter review prompts or verdicts. No paid inference was used.
This does not reconnect the app's existing transport.

Historical commit `a5e9473113e9a23e00ade292e6071e26bea88dc4` received
**PASS WITH NITS**; the [raw tool response](commit-reviews/a5e9473113e9a23e00ade292e6071e26bea88dc4.json)
identifies the actual local model (the tool's generic heading incorrectly names
MiMo). The findings suggest Markdown scanner refactoring without a demonstrated
defect. The suggested regex would miss language-tagged opening fences, and the
claimed unclosed-fence gap is already checked after the loop. No change was made
on those findings.

Commit `13cd17928c7539a1535e1c4babba3ff37351fe81` received
[PASS](commit-reviews/13cd17928c7539a1535e1c4babba3ff37351fe81.json).
Commit `a186ed542abbddb28c3deaa3cbc1c0e8990c72e5` received
[PASS WITH NITS](commit-reviews/a186ed542abbddb28c3deaa3cbc1c0e8990c72e5.json).
Its two findings were checked against that exact commit: `range(122, 135)`
correctly includes requirements 122 through 134, and the insufficient-budget
case is a future negative expectation, not the positive reference package audited
by `check_examples.py`. Neither demonstrates a defect. No expectation or budget
was weakened. The reviewer's merge recommendation does not clear the outstanding
signed corrections, SAS acceptance or aggregate gate. These three historical
reviews do not cover current uncommitted changes.

## Integrated commit review and clean-tree correction

Commit `81fcd29e` received [PASS WITH NITS](commit-reviews/81fcd29e.json). The initial
response repeated speculative suggestions; retained [raw output](commit-reviews/81fcd29e-initial.json)
shows that limitation. Findings were checked before changes: the UTF-8 span already
adds `valid_up_to()` to its length; BOM rejection is an explicit F1 requirement;
title length is explicitly Unicode scalar values; fence info-string backticks
are forbidden by F2; table-depth accounting is a documented conservative resource
bound. No source expectation was weakened for these findings.

The first clean-tree gate passed 13/14 steps: all 740 Rust tests passed, and the
battery reported 306 passes and one failure. The [full log](clean-gate-before-template-fix.log)
retains the failure. The shell installer test still equated repository-specific
AGENTS.md with the generic template, despite the approved context integration and
existing Rust test already using `docs/agents/legacy-warrant-workflow.md`. The shell
test now uses that same exact reference and additionally checks the root link;
its overwrite-refusal test is unchanged. This test file has no resolved delivery
pin. Final rerun and required GitHub `gate` determine publication readiness.
