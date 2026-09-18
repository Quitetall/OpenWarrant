# Retained local dispatch and submission references

Archive audit now reconnects `dispatch.compiled` and `submission.recorded`
local draft-history events. It resolves records within the journal's exact current
or historical prefix. A current event cannot borrow a record from a historical
snapshot. References with unsafe dispatch identifiers are refused as unavailable.

Dispatch checking recomputes the existing dispatch digest and compares Warrant,
stage, dispatch ID and journal digest. Submission checking compares dispatch,
attempt, contract, stage, requested action, blocker count and expected record path.
Unknown event kinds and provider-authoritative classes keep their prior unavailable
behavior. These checks prove retained references, not execution success, source
contract authorization, complete runtime coverage or independent verification.

A fresh service fixture executes a process, commits its records as fixture history,
then exports both historical and current records. Audit coverage becomes retained;
runtime category remains unavailable. Removing the current dispatch, changing its
digest, or changing the submission attempt makes audit coverage unavailable even
with the historical good records still retained.

17 preservation tests passed. The expanded refusal case passed again after adding
digest tampering. All-target CLI Clippy passed. Full new-head gate remains required.
No historical records, signatures or assurance dispositions were rewritten.
