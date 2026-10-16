# Katana mapping rejection diagnostics

The earlier broad CLI run failed a native mapping rejection assertion but reported neither the invalid field nor the actual provider outcome. The assertion now reports both. Its requirement, one-second deadline, mappings and provider implementation are unchanged.

The instrumented test passes once, then 100 confirmed repetitions pass at parallelism four. The complete integration suite at default concurrency passes 270 tests with four ignored. These runs do not reproduce the original failure, so its cause remains UNKNOWN and no runtime fix is claimed. The original failure remains in OW117's dispatch-budget evidence. Ticket i-f879 stays open; its claim was released without marking it complete.

This is a synthetic protocol fixture, not real Katana execution or provider qualification. No model call, human act or independent disposition was performed. Future failures will report the actual outcome needed to distinguish availability, protocol and mapping failures before a fix is chosen.
