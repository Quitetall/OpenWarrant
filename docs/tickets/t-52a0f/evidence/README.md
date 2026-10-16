# Study recorder SIGTERM retention

Synthetic controls, not real participant evidence or independent qualification.

Full OW119 run at a4fc3a75: 1662 passes and one failed study plant. Exact failed streams and receipt are retained in commit ad8e0eeb. Two children timed out after five seconds; a third received SIGTERM during the initial fsync and escaped without a session summary. Storage cause remains unknown.

The retained reproduction sends a real SIGTERM after the first real fsync. Baseline exits -2 without output. The regression fails before the fix; afterward the child exits 1, retains unknown/signal-interruption and an exact ledger digest, with started and stopped events. Twenty repetitions produced 18 PASS and two five-second child timeout errors. The signal-race fix has passing controlled observations, but a successful 20-run soak is not established. An early completion note incorrectly claimed 20 passes before the job was terminal; the journal records the correction. All 17 study tests and the actual scoped study plant pass.

The handler now requests interruption without raising inside a durable event write. The input loop observes the request with bounded polling; construction checks it before accepting input. Journal fsync, input deadlines, no-overwrite and unknown assurance behavior remain. Synchronous disk durability is not made time-bounded by this fix. Original five-second write-timeout failures remain open.

Run the reproduction with Python and the actual conformance/study source directory as its sole argument. It uses temporary synthetic metadata, cleans its session files, and never contacts participants or a model. An earlier direct plant invocation omitted REPO_ROOT and failed to locate its tests; it is not counted as validation. The retained scoped invocation sets the shipped runner variables and checks PASS/FAIL counters.

A later strace of the full 17-test suite reports 15 PASS and two child timeout errors. Both timed-out child PIDs are inside unfinished fsync(3), then killed by the test cleanup. Other measured fsync calls take 0.52-1.60 seconds. The test directory is ext4 on /dev/nvme0n1. At observation, Linux I/O pressure reports some avg10=58.30 and full avg10=54.90. This localizes the waits to durability I/O, but does not identify the underlying storage or workload cause. No host process or filesystem settings were changed; a final fully passing gate is not established by this trace.
