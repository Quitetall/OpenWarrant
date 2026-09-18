# Local service runtime retention

The experimental archive collector now recognizes declared `service` stages with
`gate://` backends. For each selected source snapshot it reads local dispatches,
checks their existing digest, and connects each dispatch to its own run record,
submission and completed-run receipt. It checks run coherence, run/dispatch
identity, gate identity, submission identity, receipt seal, verdict and exact
dispatch subject. Completed and timed-out attempts must retain their streams.
The existing journal collector also checks event references and receipt evidence.

Failed completed runs retain failure receipts. Timeouts retain unknown runs and
submissions, without completion receipts. Stages with no recorded attempts can
retain their declaration and journal inventory without claiming they executed.
A retained category describes the selected local record set; it does not prove
work completion, permissions, independent verification or source authenticity.

Missing run records, missing receipts, receipt substitution between attempts and
unmapped dispatches remain unavailable. Legacy shared run directories cannot
prove per-attempt linkage through this collector and remain unavailable. No old
record is rewritten. External agent/provider backends still require collectors.
Historical stage-to-contract authority reconciliation remains separate work.

The preservation integration suite exercises real repeated success, failure and
timeout commands. Controls remove run/receipt records and substitute a different
attempt's valid receipt. Existing history and external-backend refusal tests
remain in the suite.
