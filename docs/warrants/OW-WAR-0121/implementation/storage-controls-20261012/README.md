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
