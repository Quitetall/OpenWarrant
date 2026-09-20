# OW-WAR-0083 scheduling slice

Implemented a pure typed Rust API in `document::schedule`. It evaluates explicit
Warrant/stage dependency edges against caller-established facts. Ready means only
dependency-ready; this does not authenticate records, authorize execution or
issue assurance. Missing facts stay unknown and failed facts stay unmet.

Expected output digests are optional before an output exists. Unpinned edges
require one unambiguous matching result; pinned edges select exact output bytes.
Satisfied edges retain input fact indexes. Exact contract, stage and required
result meaning always participate. No latest-result guess or hidden lookup occurs.
Cycle/duplicate/conflict/malformed/over-limit inputs refuse without a partial report.

Optional difficulty metadata has low/medium/high/unknown, reason, confidence,
estimator identity and estimate revision. It is separate from scheduling inputs.
No new document metadata schema, CLI ingestion or dashboard scheduling is claimed.

Six public tests and Clippy passed on Rust 1.97.1. Independent spec and standards
reviews passed, including long-chain and ambiguity probes. The linked inventory
binds source bytes; its SHA-256 is the progress report revision. Logs are performer
observations, not independent assurance dispositions. Full gate is recorded
separately after commit; do not infer a gate pass from these focused checks.

Remaining OW83 work includes record codecs, trusted-fact evaluation, exact human
acceptance, shared/context/stop records and retained-evidence gaps (S04/T42–T48).
This slice does not complete OW83 or unblock the full OW86 phase exit.

## Supplied records and workflow facts

The remaining Phase 1 slice adds strict F8 records, agent acts, exact-subject
readiness and baseline assurance evaluation. Inputs are bounded and supplied by
the caller. A provenance claim never establishes authentication, execution or
criterion support. Missing evidence remains unknown. Automation cannot authorize
its own effective-policy edits. The helper evaluates eligibility and issues no mark.

The candidate workflow codec supports context views, shared contracts, evidence
availability, scoped stops, changes, tracker overviews, handoffs and batch review
coverage. Completion survives unavailable tracker delivery and missing qualification.
The configured completion word requires an exact acknowledged event and attested
overview URL. Work-change resume requires caller-established fencing and context.

### Evidence map

| Requirement | Public observation |
| --- | --- |
| S04 / T42–T48 | `document_records`: claims versus observed execution, UNKNOWN versus FAIL, independent isolation, exact secure human acceptance, candidate mutation, late review timing and delegated policy limits |
| SDK-08–SDK-11, SDK-16 | `document_records`: action-scoped gates versus qualification, unsigned completion, timing and exact policy delegation |
| SDK-12 | `document_workflow_records`: exact batch manifest/member/profile/evidence coverage; each member still requires separate assurance evaluation |
| SDK-13–SDK-19 | `document_workflow_records`: complete/unverified metadata, acknowledged event, stale URL/revision refusal, replay, minimal/rich output and parent scope preservation |
| SDK-20 | Existing adapter request/response boundary; no new query execution in OW83 |
| SDK-21–SDK-24 | Shared revision mismatch, retained/deleted evidence and history, distinct stop states, work versus harness resume boundaries |

Twenty-one focused public tests pass. Ten actual file-backed codec fixtures pass through
`sdk_probe --scope 83`; this driver does not replace semantic tests. JSON schemas
are structural only. Focused library/examples Clippy with `schema` passes. The
broader `--all-targets --features schema -D warnings` check also exposes an existing
`identity.rs` items-after-test-module lint, outside this change.

Independent spec and standards review findings were checked and repaired: artifact
closure/quota, cached support resolution, exact Warrant binding, explicit criterion
support, URL/qualification binding, scalar quotas and positional-array refusal and valid explicit-null preservation.
These observations are not assurance dispositions or a human signature.

Limits: no cryptographic verification, process control, storage deletion, tracker
I/O or runtime orchestration. Unsupported permission expressions remain unknown.
Phase 3 must prove real adapters and recovery; this slice proves only supplied-record
semantics. Full repository gate and commit review are reported separately.

The first aggregate run passed 13 steps; mutation plants refused uncommitted
documentation to prevent destructive restoration. The clean-commit rerun is required.

### Final supplied-record result

At implementation commit `cb0c93c`, the clean aggregate gate passed all 14 steps,
including 308/308 planted refusal controls. See `records-gate.log`. Independent
specification and standards reviews passed the bounded supplied-record slice.
LAMU `review_commit` returned PASS WITH NITS using the local/free 27B model.
The self-loop concern is not a defect: cyclic evidence intentionally cannot
establish itself. Missing baseline conditions and failed observations retain
different findings; response length checks precede allocation intentionally.

OW-WAR-0083 Phase 1 implementation is complete and unverified. Qualification and
legacy resolution remain separate. No human signature, assurance mark or release
was created. Next implementation scope: OW-WAR-0084 legacy history/successor records.
