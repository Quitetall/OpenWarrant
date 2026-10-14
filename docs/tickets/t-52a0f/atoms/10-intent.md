# Retain study recorder interruptions during durable journal writes

OW119 full conformance at a4fc3a75 produced 1662 PASS and one failed study-recorder plant. Actual stderr shows two child timeouts and SIGTERM interrupting the started-event fsync before Recorder assignment. The signal race is reproduced separately by injecting a real SIGTERM at the first fsync. Synthetic controls are not participant evidence. Preserve the failed receipt; storage cause remains unknown.

## Notes

- **2026-10-14 16:14 UTC, agent://codex:** Correction to i-06bb completion note: the repetition job was still running when I wrote 20 repetitions pass. Its actual terminal result is 18 PASS and 2 ERROR from five-second child timeouts. The signal regression and full 17-test single run and scoped plant did pass, but no successful 20-run soak is established. The storage timeout item stays open. This correction is by the same performer, not independent evidence.
- **2026-10-14 16:17 UTC, agent://codex:** Actual strace follow-up: full 17-test suite yields 15 PASS and 2 child timeout errors. Both timed-out child PIDs are inside fsync(3) before cleanup kills them; other fsync calls take 0.52-1.60 seconds. ext4 on /dev/nvme0n1; observed I/O pressure full avg10=54.90. This establishes the blocking syscall, not the underlying storage/workload cause. Preserve strict deadlines and fsync. i-dc7f stays open; no foreign host workloads changed.
