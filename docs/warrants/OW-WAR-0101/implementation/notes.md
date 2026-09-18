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
