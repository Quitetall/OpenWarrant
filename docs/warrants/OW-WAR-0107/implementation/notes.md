# OW107 implementation notes

Drafted from candidate SAS RC.3 sections 14 and 15 and existing execution contracts.
OW106 integration is a publication prerequisite. Draft check: 15 passes, one
warning, zero errors. Generated corpus: 942 passes, 88 warnings, zero errors.
These observations validate records only. No verification runtime is implemented
by this draft. Planned first seam: configured verifier identity/capabilities and
exact candidate request/result validation, then isolated process/workspace tests.

## Verifier contract candidate

Added bounded request/result validation for controller-owned identities and exact
source bytes, policy digest, candidate commit and check commands. Results bind the
full request digest. Distinct performer/verifier names are required, but naming
alone does not establish independent context, workspace or protected execution.
Those checks belong to the forthcoming runtime boundary. No result accepts an
actor or qualification field. PASS cannot retain unresolved findings; FAIL needs
an observed violation; UNKNOWN stays distinct and cannot instruct automatic repair.
Four refusal-oriented contract tests pass. No harness was launched by these tests.

## Separate workspace and bounded process seam

Verifier workspace is a fresh Git repository populated by exact-commit fetch,
with separate object/ref storage. It excludes performer uncommitted scratch.
Existing, symlink and performer-nested destinations refuse without overwrite.
Required checks run on the exact unchanged candidate before the configured
verifier command. A failed check is retained and skips verifier launch. Candidate
mutation, stale output and timeout remain UNKNOWN, never PASS. Eleven distinct
contract/workspace/process tests pass, including real Git and subprocess refusal
observations. This seam is not yet exposed as a server action and is not a sandbox.

Next: protected configuration/admission, persistent verification attempts and
public HTTP integration, then bounded repair/rebuttal/escalation and browser QA.
No real model calls, independent qualification or human acceptance occurred.

## Verifier policy and advisory admission

Added strict configured performer/verifier identities and commands, finite time
limits and explicit cost policy. Admission matches retained performer command and
current completed candidate policy. Unknown cost refuses a mandatory hard cap.
Configuration does not accept capability flags as proof. Missing authenticated
protection evidence is UNKNOWN; stale, failed or unavailable observations cannot
produce ready. The evaluator takes trusted observations only at an internal caller
boundary; no browser route supplies them. Ready is advisory and grants no dispatch.

Fourteen distinct verifier contract/workspace/process/policy tests pass. Fixture
protection observations exercise evaluation, not real harness authentication.
The next controller must authenticate evidence outside performer/verifier output
before passing it here; that integration and durable jobs remain unfinished.
