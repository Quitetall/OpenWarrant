# Actual reader permission check

Unverified implementation under OW-WAR-0096. No human act, independent disposition,
production authority deployment or release qualification is recorded.

## Reproduction

The generic protected authority reader checked effective write permission only
when the caller UID equaled the unsigned `agent_uid` setting. In a disposable
user/mount namespace, the operator (UID 0) bootstrapped normal-mode public metadata.
A separate non-owner process (UID 1), retaining only CAP_DAC_OVERRIDE, could write
operator-owned bytes while real-UID `access(2)` refused WRITE_OK. It changed only
`agent_uid` from 1 to 2. `Repository::open` then accepted that store as governing
policy. The collector loader separately refused this same writable execution
account. The failed public-seam control is retained in `reader-red.log`.

## Repair and observed controls

The generic reader now checks actual effective write access for every non-owner
reader, independently of the claimed execution UID. A known writable boundary
refuses; an unavailable access observation remains unavailable. Existing ownership,
mode, symlink, canonical-state and signature checks remain in force.

`reader-green.log` observes the changed setting refused through `Repository::open`,
the actual store owner's read accepted, and a separate reader without effective
write capability accepted. The same experiment retains the collector loader,
activation, sealed-executable, scope, revocation and unavailable-crypto controls.
The software signing fixture keeps private seed-derived keys in memory. It is
not human presence or production identity.

The store owner's read path remains an explicit operator trust assumption. The
host must select the trusted store independently of agent input, protect operator
identity and credentials, and constrain the execution account. Filesystem checks
are not caller authentication, a sandbox, or proof of secure human acceptance.
Other privilege profiles and actual host deployment remain unqualified.

Validation on Rust 1.97.1:

- CLI unit suite: 445 PASS, 142.24 seconds.
- Full integration suite through Cargo: 213 PASS, four explicit ignored fixtures,
  44.59 seconds. The earlier direct invocation lacked Cargo's benign environment
  marker and failed one environment-isolation precondition (212 PASS). The test
  was not weakened; the configured Cargo invocation passed.
- Strict CLI all-target/all-feature Clippy: PASS (`clippy.log`).
- Namespace control repeated with this checkout's freshly built CLI: PASS
  (`reader-green-own-cli.log`). No real operator key or human presence inferred.

The direct invocation registered 60 temporary test repositories; only those
missing paths belonging to its finished process were removed from the global
registry after preserving the original. Unrelated entries were retained. Initial
short worktree creation timed out and Git removed its partial checkout; the same
owned branch was attached successfully on a bounded retry. Missing CLI artifact
path observations were retained and corrected by building this checkout's CLI.
Full hosted candidate checks and independent qualification remain open.
