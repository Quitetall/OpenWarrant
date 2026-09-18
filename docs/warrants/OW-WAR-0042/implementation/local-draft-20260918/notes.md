# Local drafting observation

Input: `add a changelog`, preserved verbatim in the canonical request.
A separate Python adapter called Qwen3.5-4B through an isolated CPU-only
llama.cpp endpoint. Model, executable, adapter and input digests are retained.
No cloud calls, shared GPU changes, proposal application or approval occurred.

Attempt 1 timed out at 240 seconds. Server reported request cancellation and
returned to idle. No complete proposal was received. Attempt 2 used a shorter
1000-token output budget with the same time cap. The call returned but hit the
length limit; its output is invalid truncated JSON. No valid proposal was applied.
The task-owned server was terminated after both attempts; shared services were
not stopped.

This exercises the current v2 proposal interface. Legacy OW42's frozen v1
contract is not rewritten or declared satisfied. Human review before apply,
full filesystem audit, durable-choice-to-ADR observation and final independent
qualification remain outstanding. API-only text generation does not by itself
establish a filesystem audit over the model process.
