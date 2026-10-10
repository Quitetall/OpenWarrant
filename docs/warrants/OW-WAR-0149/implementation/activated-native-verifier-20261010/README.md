# Activated native verifier path

Prototype implementation under OW-WAR-0149. No independent disposition, human
signature, participant agreement or release qualification is supplied.

Both `KatanaProcessVerifier::for_activated_collector` and
`BlutProcessVerifier::for_activated_collector` first load the retained Dispatch
through the existing recorded constructor. The Warrant UUID and configured
provider must match the operator-selected, signed collector enrollment. The host
must independently supply the expected repository and authenticated collector
identity; choosing those strings is not caller authentication.

`ActivatedVerifier` requires an observed activation, checks fresh protected
current authority and exact scope, and acquires the enrollment's SHA-256 executable
as a sealed image. The configured executable path is never used to launch this
protected path after acquisition. Each invocation checks current authority and
activation before execution and again before returning any result, including
failed execution. Both adapters use their existing bounded native response decoder
and receipt mapping. Invalid responses cannot grant assurance or invented cost.

The authentication-only `LoadedEnrollment::load` cannot satisfy this path.
`for_recorded_dispatch` remains available for explicitly caller-trusted prototype
use. Unsupported sealing primitives remain unavailable; there is no fallback to
an unprotected executable. This protection currently requires Linux sealing.

The host still protects native log/head, public key, plan, job and producer inputs,
checks current compilation basis, authenticates the caller, and supplies atomic
launch/revocation fencing. Checks around a process do not prevent a revocation
between the last check and process launch, nor establish child/fork confinement,
a protected host deployment, human presence or metered spend. OW-WAR-0149 remains
incomplete.

Validation on the pinned Rust 1.97.1 toolchain:

- Strict CLI all-target/all-feature Clippy: PASS (`clippy.log`).
- CLI unit suite: PASS, 445 tests, 191.90 seconds.
- Integration suite: initially 212 PASS, one projection-drift failure, four
  explicitly ignored tests. After source compilation, the one failing projection
  test passed on its explicit rerun. This is not an all-at-once green suite claim.
- Explicit separate-UID namespace roundtrip: PASS (`namespace.log`), including
  actual activated sealed execution; refusal of authentication-only enrollment,
  repository, collector, Warrant scope and provider mismatches; refusal of a
  writable executable; changed activation and signed authority revocation.
- `war check --generated`: zero errors and unknown results; 625 warnings retained.
  The initial two projection-drift errors were repaired by compilation. A short
  compile/check observation timed out; no verdict was inferred from that timeout.
- Initial compile rejected a duplicated constructor introduced during editing;
  it was fixed before the successful Clippy and runtime checks.

The native Katana/BLUT external-producer integration fixtures remain explicitly
ignored in this run. This experiment tests activated execution with software
fixture programs and tests existing native response protocols separately. It does
not establish a protected end-to-end native provider deployment or post-run
revocation timing under a real harness. The namespace fixture uses synthetic
software keys held in memory and real separate namespace UIDs, not production
authority or human acts.
