# OW95 measurement preparation

This offline checker evaluates redacted timing records for three real consenting
developers. It cannot prove consent, participant identity, classification honesty,
actual outcome quality or independent review. It never grants qualification.
Test fixtures are synthetic and are not study results.

Run `python3 conformance/study/evaluate.py P1.json P2.json P3.json`.
Exit 0 means all three selected records report completed outcomes within numeric
thresholds, without reported failures or assistance. Exit 1 means missing,
unknown, unmet or review-required observations. Exit 2 means invalid input.
Every output retains `qualification_established: false`.

Session shape is provisional, local study tooling; it is not a new standard wire
contract. Use schema `oh.war/study-session-draft/v1`, participant P1/P2/P3,
`build_commit` (full 40-character commit), `inputs_sha256` (64 hex characters),
`host_os`, `scenario`, `measurement_method`, `consent_confirmed` (true only after
actual private consent), `assisted` (boolean), `failures` (redacted string list),
`outcome` (completed/failed/unknown), `elapsed_seconds`, and `intervals`.

Intervals contain numeric monotonic `start`/`end` offsets from session start and
one category: setup, administration, review, implementation, waiting or excluded.
Use one primary category per interval. Cover total elapsed time exactly, in order;
no overlap or hidden gaps. Explicit exclusions need a nonempty text `reason`.
Explain concurrent activity separately in operator notes; never count the same
wall-clock interval twice in this primary timeline. Missing timing is explicit
`elapsed_seconds: null`, not measured zero. Start/end observations must be retained
by the operator; no invented intervals to make totals fit.

Count commands, metadata fixes and manual configuration as setup/administration.
Thresholds: setup ≤600 seconds and administration ≤60 seconds for every participant.
Retain review, implementation, waiting and excluded totals separately. Exclusion
reasons and assistance require independent review; passing arithmetic proves no
performance claim on its own.

Freeze selected session/build before review. Preserve every failed or interrupted
attempt and link retests to originals in the operator's history. This tool evaluates
one selected record per participant; it does not establish completeness of that
history. Do not use a selected retest to hide earlier failures. Keep identities,
private consent details, secrets and repository contents outside public records.

Checks: `python3 -m unittest discover -s conformance/study -v`.
