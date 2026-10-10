# Verifier availability is not signature invalidity

The prior adapter mapped every unsuccessful OpenSSH exit to invalid crypto.
A valid in-memory software fixture signature under namespace UID 2, with no
account entry, reproduced `Ok(SignatureEvidence { valid: false, ... })`.
The initial failing observation is retained.

The bounded process transport now preserves stderr as well as stdout without
changing its existing status/stdout interface for native provider adapters.
OpenSSH success establishes validity. An explicit recognized incorrect-signature
verdict establishes invalidity. Other unsuccessful exits remain unavailable,
including missing accounts, unsupported tools, loader errors and unfamiliar
crypto errors. LC_ALL=C remains fixed; unfamiliar output is never guessed.

Seven collector controls, the explicit namespace roundtrip including missing-account
UNKNOWN and normal activation/replacement/revocation, one classification control,
17 process/capture regressions and strict all-target/all-feature CLI Clippy pass.
Two native provider tests and namespace entry points remain ignored in their
normal suites. The namespace is separately exercised; it is not host deployment,
physical human presence or independent/participant qualification.
