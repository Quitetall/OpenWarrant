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
operator or independent verifier. The explicit attributed import below consumes this proof; native provider
qualification, custody and fencing remain open. The existing
unverified import interfaces retain their behavior and do not require a signer.

## Explicit attributed import

`runtime_capture::attributed::payload` prepares the canonical inline envelope from
one request and its exact receipt bytes. The signed payload includes all request
metadata, including declared provider, time, transport, command and original
receipt reference. Whitespace in input JSON is not a signed claim; canonical
field values and the original receipt bytes are. The collector signs the core
`Proof` over this payload and the checked recorded binding, in the capture namespace.

The reference CLI exposes a separate opt-in path:

```sh
war runtime import-attributed-inline OW-WAR-NNNN \
  --request capture-inline.json --attribution capture-attribution.json \
  --authority-store /operator/protected-store \
  --verifier-executable /operator/protected-verifier --json
```

The operator store supplies authenticated current authority, active enrollment,
collector/repository identity and the execution UID binding. The executable is
observed against the enrolled byte digest and held as a sealed image on supported
Linux hosts. This operation does not execute it. Caller-provided principal names,
workspace authority files and submitted proof fields cannot replace host settings.
The host rechecks authority, activation and configuration around signature checking.
Unsupported protection or unavailable observations remain UNKNOWN; a known mismatch
refuses. These snapshot checks do not provide atomic revocation fencing.

The reusable `attributed::import_inline` takes a trusted `CaptureAuthenticator`.
It passes the current recorded binding, declared provider and complete canonical
payload to that host adapter, before provider checks and again before retention.
An adapter must authenticate current inputs independently. Returning an object
for another binding or payload refuses. No signing or authority activation occurs.

Attributed records retain the signature and proof alongside original source blobs.
`authenticated_on_import` is a historical observation; `trusted_on_read` is false.
A retained observation is not an independently authenticated current verdict.
Existing unsigned commands retain their interface; previously retained records
are not rewritten. Native verification,
current policy, caller/source custody, atomic fencing, independent assurance and
human acceptance remain separate requirements. This does not close OW149.
