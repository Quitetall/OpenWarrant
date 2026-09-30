# Correct the PR-to-Warrant reporting link

PR #128 originally omitted the machine-readable `Warrant: OW-WAR-0111` line.
The protected-base CI workflow reads that exact line before running Bonsai. Job
105662159919 on run 35364013182 completed successfully but skipped the adapter
build, candidate materialization, Bonsai build, report generation and evidence
upload. That status therefore proves no Warrant scope assessment.

The PR body now names OW-WAR-0111 and describes the actual current implementation,
older bounded full-gate evidence, later focused checks and unfinished qualification.
This commit triggers a fresh pull-request synchronization event so the workflow
receives the corrected body. A rerun of an old event would retain its old payload.

No workflow condition was weakened and no check was waived. The fresh report and
full gate must be inspected before making any scope or release claim. Independent
human/qualification gates remain separate from deterministic Bonsai reporting.
