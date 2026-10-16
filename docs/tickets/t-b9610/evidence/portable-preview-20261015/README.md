# Portable review preview: bounded implementation observation

The opt-in public CLI command writes an experimental compact review view and
an unchanged v2 archive outside the reviewed repository. It does not run a
verifier or ingest a verdict. Proposed ADR0030 defines this experiment; neither
SAS adoption nor assurance is implied.

Observed CLI regression: the flag was refused before implementation. After
implementation, 53 verification-subject CLI tests passed on Rust 1.97.1.
Controls include 2,000 retained workspace bindings, deterministic replay,
collision refusal without overwrite, output-link refusal, refusal of an output
inside the reviewed repository, and retention of exact rules when over budget.
Existing stale-subject, exact-source, old-reader and independence controls also
passed. Package Clippy with all targets and warnings denied passed.

The local-client controls initially failed one positive case with
`local verifier unavailable: timed out`. The exact isolated case passed on
recheck, then all nine controls passed unchanged. No timeout was increased;
the original timeout's cause remains unknown. The new preview-refusal control
observed no health request, inference request, log creation or input mutation.

An exploratory real OW-WAR-0003 emission produced five previews of
47,396-47,663 estimated tokens, within 48,000. Their full archives were
446,559-446,826 estimated tokens. Exact governing sources and fixtures remain
inline; raw gate evidence and full machine bindings remain in the sibling
archive. Selected code remains explicitly excerpted. Fitting is not evidence
sufficiency or verification. This observation used the working candidate based
on 8a547585, not an accepted or released format.

Raw logs, package bytes, hashes and measurement are retained at
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/portable-review-preview-20261015/`.
No production model call, disposition, human signature or Warrant completion
was recorded. Executable compact protocol, response/storage/ingestion binding,
source coverage and production independent reviews remain open.
