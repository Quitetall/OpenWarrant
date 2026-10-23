# Native cancellation and writer handoff

Run `40030825974` passed seven performer unit tests and all 22 physical plant
controls on Linux x86_64 and macOS arm64. Both hosted jobs and a separate local
Linux run used tree `0d38278d974b1afe92b8ac5e008beb489b736eca`.

The existing `60-perform-cancel.sh` plant is unchanged. It checks SIGINT/SIGTERM,
stopped marker growth, no submission, retained Dispatch, a real orphan negative
control, live/unknown/gone writers, controller death across stages and sibling
Warrants, adapter refusals and simultaneous-start serialization. The complete
control count is required; a partial script cannot report native coverage.

The [comparison](comparison.json), CI job record and platform logs retain the
actual source/tree, CLI identity, test output and results. The CI-only `setsid`
helper calls the native POSIX operation and execs the requested command. macOS
uses GNU shell tools to run this existing battery. No product admission rule,
record format, supported-OS set or runtime dependency changed.

Reproduce with a clean checkout, Bash 4 or later, GNU coreutils/sed and Python 3:

```sh
cargo build --locked -p openwarrant-cli --bin war
cargo test --locked -p openwarrant-cli --lib perform::tests::
bash conformance/native/perform.sh
```

The runner uses the existing corpus guard, restoration and positive control. It
performs isolated fixture work; it never invokes a real coding model. The
observations predate attachment of these records and documentation.

This closes the previously unobserved macOS implementation coverage. W131's
scoped implementation is complete and unverified. Legacy delivery history,
registered gate admissibility, independent obligation assessment and actual
human acceptance remain open. Nothing here assigns a disposition or resolves
the Warrant. Escaped groups, remote writers and other limits in `docs/PERFORM.md`
remain outside the claim.
