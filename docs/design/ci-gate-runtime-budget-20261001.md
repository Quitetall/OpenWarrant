# Full CI gate runtime budget

The required hosted `gate` for cbe9a8a8 was cancelled at its 20-minute timeout (run 36799088658, job 110169161764). Its log records approximately eight minutes in workspace tests before the planted-violation battery; cancellation occurred during the battery, not after a named assertion failure. This is incomplete evidence, not a pass.

The exact candidate passed all 14 local gate steps and all 1,277 plants. The retained local log at `/mnt/2tb/ow-cbe9a8a8-full-gate.log` was created at 2026-10-01 00:57:36 UTC and last written at 01:37:47 UTC, a roughly 40-minute observation window. These file times are a log-span observation, not per-step profiling or a promise of hosted runtime.

Give the one existing required job a finite 60-minute timeout. Keep `cargo xtask gate`, the post-gate merge-candidate acceptance check, the standard `ubuntu-latest` runner, every test and plant, required status name, concurrency cancellation, and branch protections. No timeout becomes successful evidence. No matrix, paid runner, signing, repinning or release is introduced.

The repository API reports `private: false`, `visibility: public`. [GitHub billing documentation](https://docs.github.com/en/billing/concepts/product-billing/github-actions) states that public repositories using standard GitHub-hosted runners have free runner usage. This observation concerns runner minutes only; it does not claim that all storage, all providers or future private/larger-runner configurations cost zero.

Validation remains the required hosted gate on the new exact candidate. Its result must be observed before merging; the prior local pass alone does not prove it.

Hosted run 36805265520 finished in about 48 minutes, with 1,275 plants passing and two failing. The roadmap schema plant hit the runner's old `jsonschema` relative-reference resolver; the token-budget scratch gate could not find an installed `war`. These are failures, not qualification. CI now installs `jsonschema==4.26.0` in an isolated virtual environment, and the token-budget plant points its scratch gate at the exact binary under test. No assertion, gate deadline, or production gate definition is removed or weakened. The new candidate still needs its full hosted gate.

## Current corpus observation, 2026-10-15

Main commit `c8bc0cb0a4704af8db960694029664fd5c8f09e6`, hosted run
[38668856947](https://github.com/Quitetall/OpenWarrant/actions/runs/38668856947),
job `116207130054`, reports all 14 aggregate gate steps green, with 1,676
plants passing and zero failing. The job started at 02:02:56 UTC and was
cancelled at 03:03:01 UTC. GitHub's annotation states: “The job has exceeded
the maximum execution time of 1h0m0s”. The last gate output at 03:02:57 UTC
says `gate: PASS — 14 step(s) green`; the final merge-candidate acceptance
step did not run. This is a cancelled CI job, not a successful required check.

Set the existing job limit to 75 minutes, giving the observed complete gate
and final candidate check finite headroom. This revises the 60-minute runtime
configuration above using a current observation. Keep every aggregate gate
step, plant, individual command deadline, final candidate check, stable
required check name, standard runner, concurrency rule and branch protection.
The new exact source still needs successful hosted CI before merging.

Raw log and job annotation are retained under
`docs/tickets/t-66d2f/evidence/runtime-20261015/`. The job log is a performer
observation; it does not confer independent verification or human acceptance.
