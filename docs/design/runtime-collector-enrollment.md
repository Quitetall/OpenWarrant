# Runtime collector enrollment — candidate contract

An agent can draft a collector configuration. That draft cannot grant trust to
its own executable, key or provider. A human administrator signs the exact
configuration against the repository's current protected authority. A host checks
the signature before using it, and checks authority and runtime inputs again
when it uses the authenticated configuration.

This is proposed OW-WAR-0149 infrastructure, not an accepted standard or deployed
security claim. Prompt-only work and ordinary unverified capture do not require
this enrollment. Native execution eligibility must use a trusted provider adapter;
human acceptance, spend enforcement and sandbox enforcement remain separate.

## What is bound

- Repository identity and the exact current authority revision digest.
- Provider kind, identity and interface version.
- The SHA-256 identity of the native verifier's executable bytes.
- A principal holding the `runtime-collector` role in that same authority.
- Explicit Warrant UUIDs. An empty scope or wildcard grants nothing.

Signers must be current `human` principals with `authority-admin`. A collector
cannot sign its own enrollment merely because it holds the collector role. Every
supplied signature must pass; an extra invalid signature is not silently ignored.
If current policy requires presence, missing evidence is UNKNOWN and observed
absence refuses. A TTY or key label is not presence evidence.

An authority transition invalidates both the signed enrollment and an already
authenticated SDK object. It must be deliberately rebound and signed against
the new authority. Changing provider, executable, collector or Warrant scope
also requires a new signed configuration.

## SDK and host responsibilities

The pure [SDK module](../../crates/openwarrant-core/src/runtime_collector.rs)
provides canonical encoding, configuration identity, signature-policy checks and
exact-use eligibility. The authenticated object has no public constructor or
wire decoder. Input JSON cannot supply its own positive verdict.

The host must authenticate and protect the current authority, provide a real
signature verifier, enforce any authenticator-presence policy, measure executable
bytes, and keep the approved inputs protected through launch and capture. The
SDK does none of that filesystem/process work. A successful SDK result does not
establish human review, private-key custody, rollback protection or isolation.

The reference CLI adapter uses `/usr/bin/ssh-keygen` with a fixed namespace,
cleared environment, bounded input/output and a deadline. It verifies against
the exact current public key supplied by the SDK. It reads authenticator flags
only after cryptographic verification succeeds. An ordinary SSH key supplies
no presence evidence. Verification scratch contains public keys, signatures
and payloads only; it is private and removed after the call.

This adapter is a callable transport component. It does not activate a collector
or make an unsigned repository configuration trusted. A trusted host must own
the scratch root; same-account filesystem writers are not isolated by it.

Remaining work: protected-store loading,
enrollment/activation transport, collector launch fencing, real provider capture
and independently observed operator deployment. The CLI currently has no trusted
native collector configured by this module.

## Wire details and observations

Schema: `oh.war/runtime-collector-enrollment/v1-draft.1`. OpenSSH signing namespace:
`openwarrant-runtime-collector-v1`. Sign the exact JCS enrollment bytes. The
enrollment identity is SHA-256 over `openwarrant-runtime-collector-enrollment-v1`
plus a NUL byte plus those bytes; this does not replace any native receipt hash
or signature domain.

The envelope contains only `enrollment` and a principal-to-signature map. The
decoder requires exact canonical bytes and refuses duplicate/unknown fields.
Wire input is at most 64 KiB; at most 32 signatures, 16 KiB each; at most 64
explicit Warrant UUIDs. Provider support remains a separate runtime check.

The first test run failed because the module was absent. The first implementation
compile found an unsupported hexadecimal formatting trait, then used explicit
lowercase digest bytes. Current controls use a **synthetic** signature adapter;
they do not establish OpenSSH cryptography or actual human presence. Eight scoped controls pass; the core all-target/all-feature run passes 705
tests, and scoped Clippy with warnings denied passes. The earlier default-profile
compile timed out before a test verdict. Lean reruns preserved assertions and
test scope. Full repository and transport qualification remain open. See
[retained observations](../warrants/OW-WAR-0149/implementation/collector-enrollment-20261010/README.md).
