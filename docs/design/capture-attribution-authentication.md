# Capture attribution authentication — candidate SDK interface

A signed capture attribution says that an enrolled collector key endorsed exact
bytes for one exact recorded dispatch. It does not say that the collector's claims
are true or that code is safe, that a human reviewed it, or that work may execute.
This is draft OW149 infrastructure, not an accepted OpenWarrant format extension.

The host supplies `Context`: trusted current authority, an authenticated enrollment,
observed enrollment use, checked `RuntimeBinding`, and actual payload bytes. It
must obtain those independently of fields in the submitted proof. The SDK cannot
make an agent-chosen authority or a claimed executable digest trusted.

Call `Proof::for_context` and `Proof::encode` to prepare the collector's signature
payload. This signs nothing. A signer uses namespace `openwarrant-runtime-capture-v1`.
Decode the returned `SignedAttribution`, then call `authenticate` with the fresh
context and a trusted `SignatureCheck`. Only success constructs the opaque
`AuthenticatedAttribution`. Decoding alone authenticates nothing.

The reference `OpenSshSignatureCheck` supports the enrollment and capture namespaces.
A capture signature cannot enroll a collector or sign a human authority act. The
host still validates current capture structure, provider seals, custody, capability,
spend and confinement observations. The signed local payload digest never replaces
a provider's defined receipt identity. A key holder can sign an inaccurate statement.

Current bounds: 64 KiB canonical wire; 16 KiB signature; 6 MiB nonempty payload.
Exact context mismatch, out-of-scope or stale enrollment, false/malformed signature,
extra/duplicate fields and noncanonical wire refuse. Missing cryptographic observation
or an unsupported schema remains UNKNOWN, distinct from observed refusal.

Public runnable seams:

```sh
cargo test -p openwarrant-core --test capture_attribution --test runtime_collector
cargo test -p openwarrant-cli --test all collector_signature::
```

[Bounded observations](../warrants/OW-WAR-0149/implementation/capture-attribution-20261013/README.md)
include synthetic SDK controls and real OpenSSH checks. They do not qualify a live
operator or independent verifier. CLI imports do not yet consume this new proof;
adding native transport and fresh-use checks is remaining work. The existing
unverified import interfaces retain their behavior and do not require a signer.
