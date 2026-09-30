# Portable legacy Dispatch context

`war dispatch-bundle` captures bytes for an existing legacy Dispatch and reads
that capture without a repository, service, database or model. Existing Dispatch
records and digests remain unchanged. This is an SDK transport adapter, not a
second semantic compiler or an execution harness.

```sh
war dispatch OW-WAR-0023 STAGE-001 --emit dispatch.json --emit-context context.json
war dispatch-bundle create OW-WAR-0023 --dispatch dispatch.json \
  --context context.json --emit bundle.json --json
```

Use a stage actually declared by the selected Warrant. Keep the `bundle_digest`
from the create result through a trusted channel. Transfer only `bundle.json`;
then, from any directory, with no repository present:

```sh
war dispatch-bundle check bundle.json --expected-digest sha256:REPLACE_WITH_DIGEST
war dispatch-bundle check bundle.json --expected-digest sha256:REPLACE_WITH_DIGEST \
  --read atoms/10-intent.md --json
```

`--read` returns a captured context ID or a Dispatch reference, never a filesystem
path lookup. Workspace Basis, context manifest, Warrant contract, capability policy
and submission schema references resolve inside the package. Captured required
atoms retain exact bytes. Selected optional atoms must also match the contract's
source digest. Optional section views include their original source separately
under `provenance://<repository path>` so the reader can check the section against
its source. The section view excludes other text; the provenance source retains
it and has the same disclosure classification as that source.

Missing input-port artifacts or prior failure evidence need explicit
`--attachment 'REFERENCE=FILE'` arguments. The reference must match the Dispatch
and must not contain `=`. A retained SAS can be supplied with
`--attachment 'sas://captured=FILE'`; otherwise the current local SAS is used only
if its bytes match the captured SAS digest. No reference is fetched implicitly.
Unresolved external context or bound atoms refuse capture. The output path must
not already exist, and validation finishes before writing it.

## Capability refusal

The legacy CLI emits `policy://none-declared`, which this transport treats as an
empty allowlist. Every capability request is denied. It does not inherit host,
agent or terminal privileges.

A producer using the existing Dispatch SDK can bind an explicit policy. Supply
its exact bytes with `--policy policy.json`. Candidate policy shape:

```json
{"schema":"oh.war/dispatch-capabilities/1","policy_ref":"policy://example","allow":["fs:read"]}
```

The Dispatch must already name that `policy_ref` and the `sha256:` digest of
those exact policy bytes. Packaging cannot change the Dispatch to grant access.
To test a request, the caller must supply an independently trusted policy digest:

```sh
war dispatch-bundle check bundle.json --expected-digest sha256:BUNDLE_DIGEST \
  --require-capability fs:read --trusted-policy-digest sha256:POLICY_DIGEST --json
```

Capability strings match exactly; no wildcard expansion or inherited permissions.
Multiple `--require-capability` flags must all pass. An unlisted capability fails
with `capability-denied`; a listed capability without the matching trusted policy
fails with `capability-policy-not-trusted`. Empty or duplicate allowlist entries
and wildcard entries are invalid.

This is a pre-action membership decision. It does not authenticate the caller's
trust claim, execute code, provision a sandbox, verify a human signature or
establish a Warrant's execution readiness. A workflow must establish policy
subject, authority, validity and revocation, then call this check before allowing
an action. A harness must enforce its result. Outputs explicitly keep
`execution_authorized: false` and `semantic_closure_established: false`.

## SDK and format limits

Public Rust module: `openwarrant_compiler::dispatch_bundle`. `Bundle::encode`
validates captured bindings and produces canonical JSON. `check(bytes, digest)`
returns a checked reader; `read(reference)` resolves bytes and
`require_capability(name, trusted_policy_digest)` checks membership. These methods
perform no I/O. They compose with existing compilation and external providers.
The current legacy compiler crate houses this adapter; ownership remains the
OpenWarrant SDK, not semantic projection planning.

`oh.war/dispatch-bundle/1` and `oh.war/dispatch-capabilities/1` are implementation
candidates outside the frozen schema pack. They do not change Stable protocol
records or constitute SAS adoption. Packages are canonical JSON, bounded to
64 MiB and 4096 source entries. Typed metadata is bounded before serialization.
Populated IR execution, assurance, resolution or extension sections are currently
unsupported and refuse before recursive processing. Sources are captured through
descriptor-relative regular-file reads on Linux/macOS; symlinks and traversal
refuse. This does not provide a multi-file filesystem snapshot: changed captures
must fail their existing digest bindings.

Integrity proves correspondence to supplied commitments and completeness of the
listed context/required local atoms/support references. It cannot discover an
unstated architecture rule, establish classification policy, or prove semantic
selection closure. A provider owns those judgments. Unknown dependencies must
remain explicit rather than being guessed from prose. Context bytes can be
portable even when execution code, runtime, credentials or approvals are absent.
