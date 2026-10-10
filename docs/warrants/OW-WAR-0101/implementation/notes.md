# Local adapter preparation

Four synthetic tests pass, including separate-process HTTP transport, redirect
refusal, oversized input/output refusal, duplicate JSON, truncation and exact
atom-field enforcement. Invalid input never contacts the backend. Dedicated CI
runs these controls.

The first constrained real-model run returned structural output in 101.18 seconds.
The four non-applying proposal validation steps passed, but inspection found
invented OW99-specific scope and malformed milestone YAML. See local-model/ for
raw traffic, identities, validation result and inspection. Nothing was applied.

The adapter now supplies fixed valid milestone YAML and clarifies inventory scope.
Tests also refuse replacement of that template. Revised real-model observation and
full gate remain pending. Earlier failures remain under OW42. No completion or
qualification claimed.

## Revised observation

The second constrained run preserved fixed milestone YAML but still misunderstood
requested work. It proposed documenting itself instead of adding a project
changelog. Retained under local-model-2/. No usable autonomous drafting claim.
Full gate and a task-faithful backend observation remain open; no more calls in
this bounded experiment.

## Task/context preparation and request-scope refusal

The narrow adapter previously accepted requests for other profiles or assurance
levels while constraining every response to delivery/basic. It now refuses those
requests before contacting the backend. Omitted fields retain the documented
narrow defaults. Separate-process HTTP controls cover unsupported values, no
backend call on refusal, and unchanged successful proposal output.

The final user message now contains the exact task; preceding JSON retains all
other input fields without truncating constraints or unknowns. This addresses the
observed inventory/task confusion as a prompt-layout experiment, not a proven
model-quality fix. Four adapter tests pass; raw prior model failures remain.

Third bounded local run at c62fd2f completed in 88.38 seconds but failed task
fidelity again. It invented OW-WAR-0100 and offered self-descriptive assurance
instead of deliverable checks. See local-model-3/ for unchanged raw output,
identities, non-applying validation and inspection. Server stopped cleanly; no
paid calls, applied Warrant or qualification. Prompt layout is not a proven fix.

## 27B local observation, 2026-10-09

The unchanged adapter now has a successful task-faithful starting-draft observation
on an explicitly selected local Qwen 27B backend. The exact retained request yielded
CHANGELOG.md and a README link with deliverable-specific positive/refusal checks.
Four transport/refusal tests and the non-applying proposal pipeline pass. A loading
503 and the subsequent complete raw response are both retained; no historical
failure was repaired or deleted. See local-model-4-ready/inspection.md for bounds.

OW101's adapter and bounded observation scope is implementation-complete, unverified.
This does not claim autonomous general planning: the draft still needs review of
its open-ended ongoing-maintenance wording, format choice and fixture quality.
Nothing was applied. No independent dispositions, signatures or marks were written.
Formal verification and acceptance remain separate from this performer report.
