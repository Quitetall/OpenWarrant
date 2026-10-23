# Public probe observation repair

The earlier local review correctly found that its compact input omitted the
probe stdout retained in the full archive. It also lacked direct parse results.
Those received verdicts remain unchanged in the verification history.

`sdk_probe --scope 75 --fixtures conformance/sdk --observations` now emits
one JSON line per actual case, then a summary. Each case retains the source
hash and byte length, observed parser result, named diagnostic and byte span,
and the expectation comparison. Parsed documents include actual metadata,
unit text, exact unit spans and validation diagnostics. Context resolution,
readiness and qualification are not established by this output.

`observed-cases.jsonl` is the raw executable output, kept separate from the
larger provenance record. `execution.json` records the actual command, binary
hash, source input hashes, exit code and source state. The source was an
uncommitted candidate when captured; the Git baseline alone is not claimed
as its build source. All 22 original cases passed, with unchanged inputs.

The `cases/` files expose individual observed results. They remove duplicate
unit text only after comparing it byte-for-byte with the named source at the
observed spans; `source_ref` identifies that source and its input descriptor.
The exact input record supplies the duplicate filename, length and SHA-256.
Each unit's `text_ref` indexes the exact UTF-8 value in the shared `texts`
dictionary. Together these reconstruct every original observed result.
The complete unmodified stdout retains the original text. These are deterministic projections of
actual observations, not new execution or independent judgments.

Packet inspection still found incomplete delivery: the combined stream exposed
only 2 of 22 cases, and the smaller case files exposed 8 complete cases. A
measured view with all case records whole fits below the same 60,000-token
estimate limit. The allocator therefore remains a separate confirmed repair;
these files do not claim that every required observation reached the model.

The public executable regression check also changes content under a new
filename and verifies the returned text and offsets. Adding invalid trailing
bytes then produces an actual named refusal, a failing case and exit 1.
The default pass/fail output still passes. The first pre-change test refused
the new option; its exact failure is retained. A candidate build failure in
SHA-256 formatting was fixed without changing the expected observations.

The Phase 1 runner includes this regression check. Its inventory retains all
96 case identities and adds the observation check to the document-reader
cases. Native CI now runs for changes to the core or SDK conformance inputs.
This adds coverage; it does not change the protected fixture expectations.

The document integration tests, observation controls and inventory controls
pass on the current host. Strict core Clippy and formatting checks pass.
Raw checks are retained under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow75-probe-observations-20261020/`.
At capture time, the remaining steps were a complete Phase 1 run, fresh review
context carrying the execution output, independent review and the applicable human acts.
No prior result, signature or assurance disposition was rewritten.
