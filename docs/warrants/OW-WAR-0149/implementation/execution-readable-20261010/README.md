# Explicit read access to public authority metadata

Candidate implementation; not independent verification or human acceptance.
The normal operator CLI can bootstrap a store with `--execution-readable` and a
separate execution UID. It shares only public state and maintains operator write
ownership across signed activation. Private storage remains the default.

A normal store loader checks state-file ownership and write permissions before
parsing. Unit controls exercise explicit sharing, private defaults, writable
state and writable directories, and reject sharing in unprotected test mode.

The separate-user fixture uses a private user/mount namespace and disposable
chroot so filesystem ancestry is genuinely owned within that namespace. Its
operator runs actual normal-mode bootstrap and activation commands. The worker
must read a cryptographically authenticated enrollment, fail to write authority,
reject a mismatched configured account and reject later signed revocation.
Software fixture keys remain in memory. Namespace identities and a software
signature do not establish a real human, protected host-account deployment,
key custody, collector activation, caller identity or launch fencing.

The first normal-mode attempt refused namespace-unmapped host ancestor ownership
(`authority-store-unsafe-owner-or-mode`). Production guards were preserved.
The private filesystem fixture replaces that invalid setup. Default namespace
entries remain ignored; explicit observation is recorded separately.

Full hosted CI and deployment/independent qualification remain required.

Observed: four authority unit controls, seven collector controls and the explicit
namespace fixture pass. All-target/all-feature CLI Clippy passes with warnings
denied. Authored/projection integrity has zero errors and zero unknowns; existing
historical warnings remain visible. No independent verdict or human act is recorded.
