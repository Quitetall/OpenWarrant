# Native command inventory correction

Unverified implementation; no human acceptance or independent verdict.

Full CI run 38109369663 failed on head 91c5975c: the command-tree unit test
and help plant found `native-resolve` missing from the member inventory, and a
runtime response test expected the earlier pre-ingestion refusal. The new native
path correctly reports `UNKNOWN` when no protected host is configured. The
correction adds the actual command to the inventory and documentation, preserves
the exact-member and wrong-command refusal controls, and requires the current
protected-host refusal with no resolution written. It does not weaken admission.

All-feature workspace library tests passed (457 CLI unit tests). The initial
integration run passed 233 tests and failed the tracker comparison because the
new unsigned scope changed OW149's contract digest and compliance projection.
A source comparison confirmed those two changed tracker fields. Regeneration
must use `war admin compile`; the test is preserved. The first formatting check
also failed on wrapping; the correction and final check passed. The actual help
plant passed all four controls, and all-target/all-feature CLI Clippy passed.

The original failed CI and local runs remain compressed and hashed beside the
successful observations. Regeneration first reached a 60-second command limit; its incomplete observation
is UNKNOWN and retained. A bounded rerun completed all seven projection writes.
The generated check passed with 2028 PASS, 625 WARN, no errors or unknowns. The
committed integration suite passed: 234 passed, four ignored. Full CI is still
required before merging this checkpoint. Existing native format and real
human/production qualification limits remain; OW95 has no actual participants.
