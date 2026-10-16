# Batch recovery input — performer observations

Source basis: d1592eae0369ca3fa0f758cdf6525af7bb42ffa5; Rust 1.97.1.
No independent verdict, human signature, or parent-Warrant completion is claimed.

Two public CLI reproductions returned `batch.recovered` for a truncated marker
and an absolute external root. The malformed marker was incorrectly treated as
an empty completed rollback. Recovery now parses the actual marker bytes and
checks all roots, record paths, backup presence, symlinks and recorded backup
digests before its first destructive operation. Roots are limited to the current
batch record directories. Invalid input retains the marker and current files.
Record schemas and signing semantics are unchanged.

Validation: two CLI regressions pass (malformed/external input, missing or
symlinked later backup with earlier root unchanged). Registered idempotency and
recovery plants: 35 passes, zero failures, including valid SIGKILL recovery,
successful batches, replay/conflict and compatibility cases. CLI Clippy and
format checks pass. The first full CLI run: 475 library and 3 binary tests pass;
integration 271 pass, one generated-projection drift failure, four ignored.
That projection run preceded staging/recompilation; it is retained, not called
an aggregate pass. Final projection and subsequent suite results are recorded
alongside these logs.

Limits: this is path confinement and snapshot consistency, not cryptographic
snapshot authentication, exclusion of a hostile concurrent writer, a power-loss
qualification, or permission to erase records signed after a crashed batch.
The parent closure dry run remains 6 PASS, 4 WARN, 8 UNKNOWN, 0 ERROR.

Final staged suite: 475 library, 3 binary and 272 integration tests pass;
zero failures, four ignored. Final CLI Clippy passes. Generated check:
2029 PASS, 634 WARN, zero UNKNOWN, zero ERROR. The observations do not
clear independent or human assurance gates.
