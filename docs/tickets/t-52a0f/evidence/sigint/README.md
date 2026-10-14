# Ctrl-C during the initial durable event write

Synthetic preparation controls; no participant or independent qualification.

Actual SIGINT during the first fsync reproduces an escaping KeyboardInterrupt and no session output on c1d04cde. The corrected regression also fails against that committed baseline. Both SIGTERM and SIGINT now request interruption without unwinding a journal write; the original handlers are restored. All 18 study tests pass on this candidate. Five-second input/test bounds, durable fsync and exact ledger digest checks remain.

The first refactored regression helper mistakenly looked up its signal after resetting argv; its two setup errors are retained in recorder-sigint-tests.log and are not attributed to product behavior. The corrected helper captures the signal before resetting argv. The final log and separate committed-baseline regression provide the actual comparison. Storage stalls remain a separate open item.
