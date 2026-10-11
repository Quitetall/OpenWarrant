---
schema: oh.war/atom/v1
adr_uuid: 01a12897-783e-71c5-a729-22bc7d21adfa
local_alias: OW-ADR-native-acceptance-7d21adfa
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0f502-4941-70a1-a446-e1eb77dff191"
---

# Version native acceptance separately from legacy resolution

Proposed OW149 implementation; not stable format adoption or human acceptance.

`oh.war/native-resolution-response/v2-draft.1` is a strict TOML response. It binds
one Warrant alias and UUID, reserved resolution UUID, outcomes, resolver and role,
meaning and effective time, contract revision and digest, assurance and artifact
snapshot digests, exact gate/judgment/risk references, explicit stage/capture
selections, protected native policy binding, observed presence requirement and
exact clean Git locator. The human signs the complete original response bytes
under the existing `oh.war/response` SSH namespace. There is no new signature
preimage canonicalization or digest domain. A legacy response cannot attest to
these fields or substitute for this version.

Preparation and ingestion independently obtain fresh protected host observations
and rerun the required native and ordinary resolution checks. Actor binding comes
only from a normal protected authority store at the same authority revision;
working-tree roles and unprotected test stores never grant native acceptance.
Only a granted human resolver distinct from the performer can accept. Enforce
protected presence policy after actual cryptographic verification. Missing
observations remain unknown; known mismatches are refused. No saved verdict or
historical signature-supplement shortcut can authorize native ingestion.

`oh.war/resolution/v2-draft.1` stores the original signed response text alongside
the ordinary derived resolution and locator. Readers must parse that exact
version, match every signed decision field to the derived record and verify the
original response's signature. Schema/extension hybrids and unknown versions
remain unsupported. The actual ingestion timestamp is generated when writing;
it is service metadata, not a human-signed assertion of when the decision was
made, and must not substitute for the signed effective time. The reserved UUID
and all other decision fields must match. Later policy changes preserve this
historical signed decision; new acceptance must use current policy. Current
reliance still depends on applicable standing and assurance checks.

The existing v1 JSON Schema and TypeScript pack stays unchanged. Native response
and record formats are experimental and outside that pack; strict native parsing
defines this draft's reader boundary. `war admin schemas` publishes a separate
experimental `schemas/native-draft/` pack with JSON Schemas, structural TypeScript,
unsigned examples and independent digest manifests. Shape validation does not
establish signature validity, authority, freshness or assurance. Contract
format-basis selection and active v1 publication bytes remain unchanged.
The tested SSH command below remains experimental, not production qualification.

Recheck snapshots, selections and policy around acceptance; use the existing
compare-and-write record prestate. These checks are not an atomic fence over
all source files. An existing resolution is never replaced or supplemented by
this new path. Preserve signed proposals even when fresh ingestion refuses them.

Signing must use the existing secure SSH transport and human key interaction;
there is no new TTY-only approval, agent signing permission, automatic assurance
or stronger hardware claim inferred from software fixtures. Prototype completion
is unchanged. Tests may use explicitly synthetic actors and public fixture keys,
but cannot qualify this repository or stand in for real participant sessions.

## Experimental CLI ingestion

`war resolve <alias> --runtime-selection <relative-file-or-stdin>
--native-store <protected-store> --response <relative-response>` reads bounded
original response bytes and the adjacent `.sig`. Its explicit selection must
match the signed decision exactly. Missing protected host policy does not fall
back to legacy records. `--dry-run` performs the same signature and current
input checks without recording. Legacy signed response dry runs also use the
existing no-write ingestion mode. The legacy runtime review packet remains an
unsigned v1 request, not a native acceptance response.

Software protocol fixtures exercise CLI ingestion under distinct operator and
executor UIDs and remain unverified test evidence; no agent signing permission
is granted.

## Experimental human SSH command

`war sign native-resolve <alias> --response <relative-proposal>
--native-store <protected-store> --ssh-sign` uses one explicit SSH invocation.
Without `--ssh-sign`, it checks and renders the exact proposal without writing
or asking a key. Parent signing options cannot silently replace this decision.
The signing result uses `oh.war/native-signing-result/v1-draft.1`.

Review rendering receives the same in-memory bytes that will be signed. Recheck
the source and required context after review, before asking the key. Refuse an
existing resolution before the key request. Sign a private temporary copy with
the normal protected human principal, then verify the signature over the original
bytes through the native cryptographic reader and rerun current checks. Plain
SSH confirmation is not authenticated hardware presence; enforce configured and
protected presence requirements independently.

A complete, immutable `oh.war/native-signed-proposal/v1-draft.1` JSON packet
retains original response text and signature under
`.openwarrant/state/native-signatures/<packet-sha256>.json`. Descriptor-relative
publication refuses traversal and replacement. Retain this observation if the
proposal, authority or evidence changed during the key request; it grants no
acceptance by itself. A failure after recording has begun requires inspecting
current records before retry, because record publication and journal append do
not form one transaction. Private signing snapshots are cleaned up.

Tests use real OpenSSH with an in-memory public-seed fixture agent. They establish
software protocol behavior, not actual human review, hardware touch, production
key custody, account separation outside the fixture, or release qualification.
