# Fresh-clone index performance

The earlier 181-second report was not reproduced. Current source: `84b28918`;
measurement binary sha256 is retained in index-refresh-comparison.json.
No coding-agent/provider calls were made.

A fresh clone had 1,233 tracked files with modification times in the same
second as its index. Two unrefreshed reads took 4.570 and 4.567 seconds; their
five Git diffs consumed 0.598 and 0.597 seconds. Both read the untracked list
once. After an explicit refresh, next took 4.075 seconds, with 0.032 seconds
in diffs. Every stdout digest matched, and war never changed the index bytes.
Raw traces and measurements are retained. Repeating without refresh rules out
a first-read-only cache effect on this sample; no broad machine benchmark claim.

The patch prepares the index only in plant-isolated.sh's disposable clone;
the ordinary CLI source and read-only behavior are unchanged. Preparation
failure refuses before executing the battery and removes the clone. A control
runs the actual initializer with real Git and a tracing proxy. Its initial
red run found no clone preparation; a second red run found failed preparation
was ignored. Both corrected controls pass, with the parent index byte-identical.
The controls are part of 48-isolated-battery.sh.

Running the actual modified initializer with an external measurement battery
prepared the index before its first read: next took 3.949 and 3.920 seconds,
with 0.032 and 0.033 seconds in the five Git diffs. Output sha256 matched all
earlier runs. This command tested committed corpus f6924ff5 through the modified
working-tree initializer; it is not an exact-commit whole gate.

Full plant/integration checks remain pending. No Warrant verification,
authorization, resolution or assurance mark is claimed.

## Scoped runtime result

The complete 48-isolated-battery plant at 032630aa passed its eleven scoped
checks, including the new preparation/refusal control, caller preservation,
cleanup, HEAD-only execution and nested-run refusal. Its initial corpus
freshness check failed, for overall exit 1. The raw output is retained.
The compiler then regenerated all five affected projections on this branch;
`war check --generated` has 2,001 passes, 365 warnings and zero errors.

`debug-probe/` preserves the one-off measurement scripts with their original
absolute checkout/output paths. They are evidence, not supported CLI tools.
The retained controls are the portable regression seam.

The final complete 48-isolated-battery run at 50dd3c55 finished **12 passed,
zero failed**, exit 0. The measured performance fix and both preparation/refusal
controls are complete. The full integration gate and Warrant qualification
remain separate. Raw output: runtime-isolated-battery-final.log.
