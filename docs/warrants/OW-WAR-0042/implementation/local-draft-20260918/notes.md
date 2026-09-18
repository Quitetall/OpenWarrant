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

Adapters are retained as `.py.txt` evidence, not installed scripts. The invalid
proposal is `proposal.stdout.txt`, preserving exact output without presenting it
as a parseable document. Renaming changed no evidence bytes or recorded digest.

Attempt 3 used explicit brevity guidance and suppressed inventory repetition.
It returned valid JSON (578 completion tokens), but the model converted
role/ordinal/path shorthand into nested paths and sequential ordinals. The real
CLI refused the first path before any application. Raw response and exact refusal
are retained. No fourth call was made; task-owned endpoint was stopped.

Next adapter work should supply explicit field mappings and a constrained JSON
schema, then still run semantic validation. Do not normalize this observation
and present repaired output as what the model returned. These failed observations
do not satisfy the end-to-end applied-draft requirement.
