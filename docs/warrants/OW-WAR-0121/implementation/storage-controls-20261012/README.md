# OW121 storage controls — performer observations

On Linux, the registered `53-storage.sh` controls passed against source basis
decd0c7188c95a3e50afff9ab424ba7875b2f639: 21 feature controls plus the positive
corpus check, zero failures. The disposable Git corpus and throwaway signing
agent/key were isolated from the real authority store. Actual binary identity
is in observation.json; this is a debug build, not a clean release qualification.

Observed: authorization and generated-file preservation after the debug crash
hook; named stray temporary files; unchanged generated JSON; moved-prestate
refusal paired with successful unchanged-prestate writes; signing/compilation
symlink refusals with external bytes retained; torn-tail versus malformed-line
classification, no automatic truncation, named manual repair retaining parsed
events; six-writer inventory paired with a planted direct write.

The release-build fault-hook absence was UNKNOWN and not counted as a pass.
The writer inventory is a static source check, not a separate crash execution
for each of the six writers. No kernel rename crash, physical power-loss,
macOS/Windows, multi-file transaction or independent/human acceptance is claimed.
Parent closure remains 6 PASS, 5 WARN, 8 UNKNOWN, 0 ERROR.

## Clean release supplement

The clean source commit 43a33fb2f4d2e1ffa846fef289df4fdaf9589bed built
successfully offline with Rust 1.97.1 in 425 seconds. Its actual release version
and SHA-256 are in release-observation.json. With that exact artifact available,
the registered plant passed 22 feature controls plus the positive corpus check:
23 PASS, zero failures, including absence of OPENWARRANT_FAULT in the release
binary. The earlier UNKNOWN remains in the original log as actual history.
The private clone's atomic.rs bytes matched source and its metadata was copied
from source for the plant's freshness predicate. No binary timestamp was altered.
This proves the bounded release-hook property, not full release qualification.
