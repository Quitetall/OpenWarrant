# OW133 evidence reuse and context conflict scope

**20 current CLI controls and one compiler conflict test passed.** The exact source,
retained debug binary, logs and bounds are in [observation.json](observation.json).
The report records performer observations, with no independent disposition,
new required gate receipt or assurance mark.

| Scope | Observation |
| --- | --- |
| OBL-001: receipt source identity | Clean and dirty source receipts carry the observed tree, delivery/declared-input subjects and fixture digests; seals recompute. |
| OBL-002: moved source is historical evidence | A declared input change makes the receipt stale; an unrelated change stays admissible. The controls name the expected refusal instead of treating every receipt as invalid. |
| OBL-003: missing source binding | A contract-only receipt is reuse-unknown, not an admissible gate result or gate failure. An unsealed edit is refused. The repository's resolved history is not re-evaluated and its resolution bytes stay unchanged during the control. |
| OBL-004: truthful conflict status | The context manifest states which conflict check ran and which remains unchecked; a bare unchecked empty list is refused. |
| OBL-005: required context and conflicting revisions | An insufficient budget emits nothing; the in-budget stage includes all required atoms. Two references at one digest compile; one path at two digests is refused and both versions are named. The compiler unit also exercises this boundary within its test fixture. |

## What remains

The current read-only closure evaluation returned **exit 2**, with **5 PASS,
1 WARN, 9 UNKNOWN and zero ERROR**. [closure.json](closure.json) is the actual
report. Exact authorization, delivery digests, dispositions, required gate results,
unknowns/blockers, judgments, independence and risk authority are not all established.
D-009 (`status.rs`) is reported as moved under later OW148 ownership. Historical
records stay intact; this audit does not repin files or change authority to clear
those checks.

AM-004's four run tasks were replayed with the fixture agent and fixture verifier.
Each reached `would_satisfy`, matches its baseline rung and has no unmet
loop-owned requirement. Each still lists six unmet authority requirements. The
actual results and timings are retained here, named `run-*.json`. These are fixture
observations, not real agent judgment, human review or independent assurance.
Raw `--json` stdout includes initialization text before its final report; the result
files are valid JSON. This transport issue is retained, not silently repaired.

Next: reconcile current authorization and the complete amended delivery history,
obtain admissible exact-subject gate evidence and independent reassessment, then
present human acceptance. The tracker remains **in progress**.
