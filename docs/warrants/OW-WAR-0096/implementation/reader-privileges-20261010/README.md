# Permission-changing execution privileges

Unverified implementation under OW-WAR-0096 and the protected reader path used
by OW-WAR-0149. Parent source: `5abb28ae4750f0fd1f128a8b1ee7a023647c439c`.
No human act, independent disposition or production qualification is recorded.

## Reproduction and repair

The explicit disposable UID/mount namespace control gives UID 1 only CAP_FOWNER.
EACCESS denies direct write to an operator-owned readonly probe. chmod and an
actual write succeed, and the unchanged active enrollment loader accepts this
account (`red.log`, 0.33 seconds). Ownership and the exact capability mask are
asserted; neither a same-owner fixture nor mixed DAC privilege explains the result.

Non-owner normal Linux authority readers now observe current kernel effective
and permitted capability sets through the pinned rustix capget API. This
filesystem protection profile requires both sets to be empty. Privileged or
unavailable observations fail closed as unqualified/unavailable, never as an
inferred secure deployment. Existing direct-write refusals still precede this
check. Operator-owner inspection remains an explicit trusted-host assumption.
No policy file, signed authority history, digest domain or wire schema changes.

The repaired fixture observes both the collector loader and general repository
reader refusing. It also drops effective CAP_FOWNER, retains it as permitted,
observes refusal, then re-enables that permitted capability and performs the
mutation. Genuine owner and zero-capability reader observations remain accepted;
existing activation, native-image, scope and revocation controls remain intact.

## Bounds

This supports a conservative unprivileged Linux read profile. It does not prove
sandboxing, authenticate the caller or bootstrap, establish human presence, or
qualify privileged deployments behind stronger containment. Sudo, setuid/file-cap
executables, inherited privilege transitions, other OS mechanisms and changes
after a check remain host responsibilities. OW96 and OW149 remain in progress.

## Validation

Rust 1.97.1. Explicit repaired namespace control PASS, 0.36 seconds
(`namespace.log`). Full CLI integration suite PASS, 213 tests and four explicitly
ignored fixtures, 40.79 seconds (`integration.log`). Format check and strict CLI
all-target/all-feature Clippy PASS (`clippy.log`, 30.31 seconds).

Initial direct Cargo invocation selected Rust 1.83; subsequent workspace build
cache observations contained incompatible Rust 1.99 artifacts. Those attempts did
not reach the control. The authoritative loop explicitly reused this project's
known 1.97 build directory and disabled the wrapper for that invocation. Clippy
required its build options after the subcommand; no shared cache or global config
was deleted or changed. The first repaired namespace run reached and refused the
collector correctly, then failed a new fixture's repository creation in the
operator directory; the fixture was moved to executor-owned scratch. Raw failed
observations are retained separately, not presented as passes.

Focused store units PASS, three tests (`store-units.log`, 0.33 seconds).
Own freshly built CLI compilation and final generated-record checks are recorded
in `record-check-summary.json`; these checks assess record integrity, not runtime
qualification.
Hosted full gate remains separate. Software fixture keys remain in memory;
namespace accounts are not production operator identity or human acceptance.
