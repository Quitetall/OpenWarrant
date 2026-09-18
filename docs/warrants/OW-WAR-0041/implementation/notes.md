# OW41 current observation and measurement repair

Work remains in progress and unverified. Original signed contract, historical
baseline and judgments remain unchanged.

## Observation at 1d44cc4dadb8bf66f3dc3b2e7e11017242c16753

The CLI emitted 20 measures, eight derived metric placeholders, 253 untracked
history candidates and 16 success metrics marked `no baseline`. The original
receipt is retained in `current-measurement.json`; it is diagnostic evidence of
the old collector, not an accepted baseline. It contains known false explanations:
no resolutions ever occurred, no interviews ever ran, and a constant zero claimed
as measured authorization eligibility. Do not reuse these claims as current facts.

The shipped CLI refused an empty reviewer and accepted a synthetic reviewer in
an in-memory fixture relation. No real attribution, independent review, human
identity or persistent relation is claimed. See `observation.json` for exact
commands, process exits, binary identity and output hashes.

## Repair

Unobserved authorization eligibility now reports `not_measurable_yet` instead of
constant measured zero. Unsupported event metrics describe collector limitations
without asserting those events never happened. A public CLI regression control
checks unknown eligibility, explicit collection limits, real measured zero for
an empty amendment set, and actual Git history detection.

## Remaining scope

Public CLI regression passes on Rust 1.97.1 (see `regression.log`). Full gate is pending. The eight derived metrics still
need defined, observed inputs and calculations. A current measurement cannot
prove the historical before-tuning baseline requirement. Independent review and
any reconciliation of historical obligations remain outstanding; no completion
or assurance disposition is asserted.

## Initial integration failure

The first full gate failed corpus and dependent battery cases because OW39 also
delivers `telemetry.rs` and its unresolved delivery provenance still named old
bytes. Both delivery records now retain the previous source digest as input and
name the actual new content. No signed contract, resolved pin or original
baseline changed. Full failed transcript retained in `initial-gate-failure.log.gz`.
