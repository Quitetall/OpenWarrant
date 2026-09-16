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
