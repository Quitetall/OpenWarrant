# Draft checkpoint timeout — bounded observations

Source basis: d96b1ae0. Test-only instrumentation; no product code, timeout,
checkpoint durability, signature, authority or acceptance expectation changed.

The concurrency control previously reported only an elapsed five-second deadline.
It now distinguishes an exited child from a still-live child, reports bounded
stdout/stderr (4 KiB each), observes Linux kernel wait state when available, and
terminates/reaps its owned child on failure. A guard also reaps that writer if a
later assertion fails. A control exercises actual exit 7 with output and an
owned live process that is terminated; no arbitrary process IDs are killed.

The instrumented module reproduced the failure: ten tests passed and the original
concurrency control failed. At the missed checkpoint, try_wait reported the child
still running; Linux wchan reported jbd2_log_wait_commit. After termination the
child was reaped with signal 9; stdout/stderr were empty. A subsequent read-only
process observation also found three owned draft children in rq_qos_wait. These
are actual kernel observations, not a guessed child exit or argument failure.
The transient process listing was not retained as a trace; the journal-wait
observation is retained verbatim in draft-tests.log.gz.

Source document/draft.rs synchronizes a new checkpoint before hard-linking its
published name. The observed journal wait is consistent with a delayed initial
durable write. No runtime syscall trace established the exact blocked syscall;
no underlying storage cause was established. This does not prove every historical
timeout had the same cause. The original diagnostic/cause item remains open.
The five-second readiness expectation and sync_all are unchanged.

Strict all-feature CLI integration Clippy passed. Exact concurrency replay passed
in 0.11 seconds; the unchanged full module replay passed all eleven tests in
0.50 seconds. Failed and passing logs are preserved. These passes do not claim a
storage repair, independent qualification, stable latency or a release gate.
