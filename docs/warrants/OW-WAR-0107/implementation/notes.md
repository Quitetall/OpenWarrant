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
