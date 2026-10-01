# Full CI gate runtime budget

The required hosted `gate` for cbe9a8a8 was cancelled at its 20-minute timeout (run 36799088658, job 110169161764). Its log records approximately eight minutes in workspace tests before the planted-violation battery; cancellation occurred during the battery, not after a named assertion failure. This is incomplete evidence, not a pass.

The exact candidate passed all 14 local gate steps and all 1,277 plants. The retained local log at `/mnt/2tb/ow-cbe9a8a8-full-gate.log` was created at 2026-10-01 00:57:36 UTC and last written at 01:37:47 UTC, a roughly 40-minute observation window. These file times are a log-span observation, not per-step profiling or a promise of hosted runtime.

Give the one existing required job a finite 60-minute timeout. Keep `cargo xtask gate`, the post-gate merge-candidate acceptance check, the standard `ubuntu-latest` runner, every test and plant, required status name, concurrency cancellation, and branch protections. No timeout becomes successful evidence. No matrix, paid runner, signing, repinning or release is introduced.

The repository API reports `private: false`, `visibility: public`. [GitHub billing documentation](https://docs.github.com/en/billing/concepts/product-billing/github-actions) states that public repositories using standard GitHub-hosted runners have free runner usage. This observation concerns runner minutes only; it does not claim that all storage, all providers or future private/larger-runner configurations cost zero.

Validation remains the required hosted gate on the new exact candidate. Its result must be observed before merging; the prior local pass alone does not prove it.
