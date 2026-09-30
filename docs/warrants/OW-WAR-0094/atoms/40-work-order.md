---
schema: oh.war/atom/v1
warrant_uuid: 01a0abaf-7830-72b3-bb5e-0d8856c16813
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables
- Extend apps/openwarrant-web with an opt-in configured execution controller and browser actions.
- Update docs/releases/remaining-build-scope.md and skill drafting reference with local/cloud routing.
- Preserve implementation notes, public-seam tests and generated Warrant projections.

## Execution contract
Owner-controlled configuration binds each eligible draft UUID to exact source digest and
Git base commit, required check commands, start requirements and trusted harness command.
Browser requests cannot supply shell commands, policy, actor identity or qualification.
Each Warrant gets one isolated Git worktree; one active writer per Warrant. The harness
provides sandbox isolation, including restricting writes to its worktree and protecting
control/evidence storage. Command launch is direct argv, never a shell interpolation.
Unknown-cost paid execution refuses with a hard spend cap (default $10). Null cap explicitly
permits unknown-cost execution; display unknown cost. No claim of enforced paid caps.
Verified-start requirements refuse in this unverified slice. Dependencies must have an
observed completed result for their exact configured source digest.
Completion requires a valid harness result, a clean committed worktree, and all configured
checks passing on that exact commit. Reports remain performer evidence, not qualification.
Failed/blocked/timeout states do not emit completion. Server restart with unfinished attempts
reports execution unknown and blocks replacement; a PID or timeout is not fencing proof.
Preserve bounded stdout/stderr and all attempt records; require explicit start for retry,
maximum three repair cycles by default and configurable lower/higher finite limit.

## Acceptance seams
Use real local HTTP, subprocess and Git worktrees: successful edit/commit/check, invalid
result, changed source, unknown-cost cap, verified-start, duplicate start, failing checks,
timeout and restart refusal. Synthetic harness fixtures prove protocol, not real-agent quality.
User's requested API/execution workflow defines these public seams; no internal-only tests.

## Autonomy
Prompt-authorized unverified implementation. No paid calls. Preserve all legacy records;
no signatures, dispositions or completion marks for unfinished scopes. One worktree/writer.
