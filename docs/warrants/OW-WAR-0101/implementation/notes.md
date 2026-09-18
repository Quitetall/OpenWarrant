# Local adapter preparation

Four synthetic tests pass, including real separate-process HTTP transport,
redirect refusal, oversized request/response refusal, duplicate JSON refusal,
truncation and exact atom-field enforcement. Invalid input never contacts the
backend. Dedicated CI runs these controls.

Real constrained-model run and full gate are pending. Existing three failed
observations remain under OW42. No completion or qualification claimed.
