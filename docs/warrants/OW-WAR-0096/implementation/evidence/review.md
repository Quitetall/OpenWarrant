# Independent development reviews

Spec and Standards reviews inspected the candidate independently of implementation.
Spec found inherited-PATH verifier substitution. Fixed to system OpenSSH with a
cleared verification environment and hostile-PATH regression. Standards requested
threat-model update and exact unsigned/replay refusal assertions; both fixed.
Both reviewers reported no additional confirmed defects in final read-only review.

Actual LAMU source commit review: PASS WITH NITS (free local Qwen fallback).
Checked findings: write_new syncs file before rename and persist syncs directory
afterward; first load checks protection before lock creation, second load checks
current state under lock; record byte bounds are intentional and consistent.
No changes justified by those nits. No human assurance or deployed isolation claim.
