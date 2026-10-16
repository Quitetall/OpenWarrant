# Local review observations — not qualification

The real local LAMU run used `qwen3.8-27b-heretic-rvn-q4_k_m`, a separate blind conversation per compiler packet, a read-only source mount and an isolated network. The final thinking-off run returned five complete responses. OpenWarrant ingested four `established` claims and one `not_established` result (OBL-004). Those are the model's actual observations, not an assertion here that they establish assurance.

The performer contests the positive claims. In particular, OBL-005 explicitly relies on a prior verifier's judgment rather than visible current execution. Several views contain empty or very small artifact excerpts. OBL-004 correctly identifies missing workflow/signing excerpts. Do not use these observations as a completed independent qualification, a signature, acceptance or resolution.

Next: withhold prior conclusions from new blind views while preserving their archived history, and improve current evidence coverage without deleting required binding rules. Re-review the exact result independently afterward. No disposition is edited by this note.

The actual gate run passed 1,679 controls with zero failures on `cf10013f5f42c732cdb22d65be904775c8f5ddc8`; the software record gate also passed. These executions are distinct from model qualification. The evidence-only source checkpoint is `788bdf0070006490e5894b2fe760765a3b98b4ad`.

The adjacent performer assessment records the actual replies and visible artifact-byte counts. Exact model inputs, provider replies, client hashes, usage, truncated attempts and backend logs are retained under `/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/ow134-local-review-20261016/`. Thinking-enabled attempts were incomplete except one OBL-004 response, which also reported `not_established`; that response and its replacement remain in history. The unexpected effective generation limit is not diagnosed or repaired by the thinking-mode option. All owned inference services were stopped after the run.
