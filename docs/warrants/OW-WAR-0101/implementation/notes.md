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
