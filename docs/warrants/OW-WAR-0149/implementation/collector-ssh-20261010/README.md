# Collector OpenSSH transport observations

This candidate component verifies exact collector enrollment bytes through
`/usr/bin/ssh-keygen`. It does not activate an enrollment, establish protected
operator custody, authenticate the supplied authority store or award assurance.

Rust 1.97.1 observations:

- First build observation ended before a test verdict: UNKNOWN.
- Continued RED run: missing `collector_signature` module, E0432.
- Four controls: PASS, including real cryptographic payload/key/namespace
  refusals, signer-line injection refusal, scratch cleanup, ordinary-key
  presence UNKNOWN and exact SDK enrollment authentication.
- Software security-key-shaped fixtures: authenticated flags observed only
  after cryptographic verification. No physical key or human act is claimed.
- Runtime capture regressions: 17 PASS, 2 native-provider tests explicitly
  ignored because they require separately built producer/verifier binaries.

Private fixture key material stays in memory. Scratch contains only public
keys, signatures and payloads and is removed by the adapter. These observations
use zero debug information and one build job, preserving assertions/test scope.
CLI all-target/all-feature Clippy with warnings denied: PASS. The full
repository gate for this new transport remains unestablished.
Protected loading, activation, launch custody and independent qualification
remain open. `source.json` binds the exact exercised implementation/test bytes.
