# Separate service attempt evidence

Two actual runs of OW66's `true` service gate in the disposable fixed-stream clone
produced two dispatches but only one receipt. The second run overwrote the first
receipt, reused its run ID and reused stdout/stderr paths. Original receipt bytes
and the red observation are retained here.

Each new service dispatch now reserves a separate `gate-runs/<dispatch-id>/`
directory before execution. A pre-existing attempt directory refuses instead of
being reused. The run ID names that dispatch. Streams, run record and receipt all
share the attempt directory. Existing historical records are not rewritten.
This prevents routine reruns from overwriting earlier attempts; it is not filesystem
isolation from arbitrary processes with write access.

The regression executes two service attempts, checks both receipts and distinct
run IDs/subjects, and proves first receipt/run-record bytes survive unchanged.
Stream bytes and archive retention checks remain. 17 preservation tests and strict
all-target Clippy passed; generated checks report 966 pass, 88 warnings, no errors
or unknown. The existing service plant now discovers nested receipt paths; its
runtime checks and the full new-head gate remain pending at this recording.

Existing service plant group ran in independent clone at feb4af8d: 10 passed,
zero failed. Includes successful service receipt, human-stage refusal, unknown
gate refusal, timeout, self-completion refusal and external submission checks.
This focused group does not replace the complete new-head integration gate.
