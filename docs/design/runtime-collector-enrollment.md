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

This signature adapter is a callable transport component. Activation is a separate
operator command described below; signature verification alone does not activate
a collector or make an unsigned repository configuration trusted. A trusted host
must own the scratch root; same-account filesystem writers are not isolated by it.

The read-only execution loader checks ownership, modes and actual writability
against its effective UID before reading authority. It requires the store
execution UID to match, refuses test stores, rechecks authority after verification
and reloads it at every use. Signed source reads are bounded, nonblocking and
reject symlinks or non-regular files. The trusted host must select the authority
path independently of agent input and authenticate the acting collector.

A disposable Linux namespace fixture observed a valid enrollment load under UID
1, denied an authority write, refused a mismatched configured UID and refused use
after a software-signed authority transition revoked the collector. This is a
fixture observation, not human approval or host-account/operator qualification.
The loader itself neither activates configuration nor atomically fences launch.

## Implemented reference integration

The following paths exist in the current source. They are prototype tooling under
OW-WAR-0149, not an accepted document-standard extension or operator qualification.

1. An agent may draft an enrollment. Current human-admin authority must sign its
   exact canonical payload; no collector can grant itself trust.
2. The operator runs `war authority activate-collector --store <protected-store>
   --enrollment <signed-envelope>`. The command validates the signed envelope
   against current authority under the operator lock, atomically selects it,
   retains prior signed objects and records selection observations. Exact replay
   leaves state unchanged. It does not sign or infer human presence.
3. `LoadedEnrollment::load_active` reads the selected object from protected state
   and checks signatures, the actual execution UID, authority and selection. The
   older `load` API authenticates an envelope only; it does not establish selection.
4. `load_host` reads operator-protected `runtime-host.json`, checks the canonical
   repository path and actual execution account, and uses its repository and
   collector identities to load the active enrollment. An agent's principal label
   cannot replace this host configuration.
5. `NativeAssessment::load` reads protected `runtime-native.json`. Policy entries
   bind explicit dispatch digests to provider interfaces, approved verifier images,
   protected native inputs, capabilities, confinement/spend requirements, deadlines
   and response budgets. It selects current retained attempts and freshly assesses
   captures. Missing explicit native policy is never inferred from a receipt.
6. `HostEnrollment::acquire_verifier` acquires the approved executable. On Linux,
   `ProtectedExecutable` runs a sealed copy of its measured ELF bytes. Activation,
   authority and host configuration are checked around execution; the adapter
   rejects changed inputs. This does not protect dynamic libraries, authenticate
   the caller, or atomically fence authority revocation.
7. The explicit `war sign native-resolve` path accepts a prepared response and a
   normal protected native store. It requires requested human SSH signing and fresh
   checks at ingestion; it has no TTY-approval or agent-signing fallback. Ordinary
   unverified capture and prototype work do not require this assurance path.

Native configuration v1 retains its fixed 128 MiB input budget. V2 requires an
explicit positive budget no larger than 2 GiB. This is an input-storage bound,
not a spend cap, execution sandbox or evidence that a provider meets those policies.

Implemented transports have disposable Linux fixture observations. See
[activation observations](../warrants/OW-WAR-0149/implementation/collector-activation-20261010/README.md),
[activated verifier observations](../warrants/OW-WAR-0149/implementation/activated-native-verifier-20261010/README.md)
and [native command inventory](../warrants/OW-WAR-0149/implementation/native-command-inventory-20261011/README.md).
They do not establish an independently operated deployment or human acceptance.

## Remaining qualification and implementation boundaries

Host roots must be selected independently of agent input. The host must authenticate
callers, protect dynamic dependencies and provider inputs, enforce spend and
confinement policy, and fence revocation through launch/capture where required.
Before/after checks and a sealed executable alone do not establish those properties.
Actual provider work, joint participant agreement, independent review and operator
custody observations remain required within the shared Warrant's scope. macOS
sealed execution is unavailable on the current reference path. Do not claim all
collector work is complete from the implemented transport or fixture checks.

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
lowercase digest bytes. Those original controls used a **synthetic** signature adapter;
they do not establish OpenSSH cryptography or actual human presence. That historical run passed eight scoped controls; the core all-target/all-feature run passed 705
tests, and scoped Clippy with warnings denied passes. The earlier default-profile
compile timed out before a test verdict. Lean reruns preserved assertions and
test scope. Full repository and transport qualification remain open. See
[retained observations](../warrants/OW-WAR-0149/implementation/collector-enrollment-20261010/README.md).

The separate [capture attribution interface](capture-attribution-authentication.md)
binds the complete canonical capture request and original receipt bytes to one
current recorded dispatch. Its explicit `runtime import-attributed-inline` command
uses protected active enrollment and a separately purposed capture signature.
Ordinary unsigned capture imports do not enter this path. Historical retained
attribution is not a fresh native verdict or an assurance mark.
