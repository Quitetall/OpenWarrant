# Observe writer startup before cancellation

CI run 38196016065 on 767b5295 failed only the planted-violation step: 1646 controls passed and one failed. The concurrent-start case observed seven valid refusals and one live admitted command, then cancelled that command before its fixture had written a marker. It reported zero writers. This was an observer race, not evidence that admission permitted multiple writers.

A one-second delayed fixture reproduced the same failure. The shipped regression with startup latency also failed before the observer fix. Waiting for both performer and child markers before cancellation made the isolated probe pass. The test still requires exactly one fixture writer, seven named refusals, one Dispatch and zero submissions. Startup failure and multiple writers remain failures.

All 22 selected runtime controls pass after the fix, including the positive corpus check, cancellation, orphan handoff, adapter refusals and the delayed-start regression. Bash syntax passes. No production admission or record format changed. Full CI must run on the final committed candidate. These observations do not establish independent assessment, macOS behavior, the unreproduced spawn-to-index crash window, or human acceptance.
