# Real local workhorse — bounded task passed, integration unqualified

A separate local-only LAMU service ran the configured Qwen3.8 27B workhorse
with a private one-model registry, settings, database and runtime. It used an
8,192-token context. The previously declared envelope was 24 GiB host memory,
200% CPU and a 600-second service lifetime. Global routing and configuration
were not changed. No cloud credentials or fallback were provided.

The actual Katana CLI used builder policy, bubblewrap and service tier, with
four turns, four tool calls, a 150-second wall limit and a 60-second provider
limit. Automatic locate-code delegation was disabled. The task permitted only
calc.py: replace subtraction with addition while preserving two unittest cases
and the Makefile. Read, edit and bash calls returned success. Inspection confirmed
exactly the expected function, unchanged test and Makefile bytes; a separate
`make test` run passed both cases. This proves this small task, not general model
quality, all sandbox boundaries or production readiness.

The native result reported 7,878 input and 246 output tokens (8,124 total).
Strict prompt replay and session inspection both exited zero. The earlier CPU
model failure remains [recorded](../real-local-provider-20261024/README.md);
this is a distinct model and resource envelope, not a relabeled retry success.
Cost remains unknown, not zero. No paid provider was called or model review run.

No OpenWarrant dispatch binding was supplied and no provider receipt was imported.
W149 therefore remains in progress. The next positive integration must use an
actual recorded dispatch and retain the native receipt, then exercise current
basis assessment. A successful unbound task cannot clear those requirements or
independent, operator, contributor and human acceptance gates.

The owned service was stopped after verification; MainPID was zero and its socket
was absent. Private state was archived and each regular file hash checked before
removing the run scratch. No foreign service was stopped. Raw session, service
log, argv, replay, private-state archive and cleanup observations are retained at
`/mnt/4tb/artifacts/OpenWarrant/keep/native-workhorse-20261024/`.
