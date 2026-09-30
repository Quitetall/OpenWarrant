---
schema: oh.war/atom/v1
adr_uuid: c4d1e8f2-3a5b-4c6d-9e7f-1a2b3c4d5e6f
local_alias: OW-ADR-0015
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a021a2-b570-7f57-85b2-0f8189873d9e"
---

# ADR OW-0015: every ssh-signed act is attested as an in-toto Statement in a DSSE envelope

## Status

Proposed by the performer under the 1.0 plan (slice B3); adopted when the owner
accepts it.

## Context

A signed response under `docs/authority/responses/` proves that a human signed
those bytes. It does not, by itself, say which records the act produced, and
a verifier that has never heard of OpenWarrant has to learn this repository's
layout to check anything. Supply-chain tooling already has a shape for "a
signed statement that these artifacts have these digests and this predicate":
in-toto Statements in DSSE envelopes.

## Decision

- After `war sign --ssh-sign` ingests an act, an in-toto Statement v1 is
  written whose subjects are the record the act wrote and the signed response
  (sha256 of each, as on disk at that moment), plus `contract:<alias>` when
  the response names a contract digest; the predicate is the response's own
  fields plus `actor` and `act`; `predicateType` is
  `https://openwarrant.dev/attestation/<act>/v1`.
- The Statement is JCS-canonical (RFC 8785, the same serializer every
  digest here uses), base64'd into a DSSE envelope with
  `payloadType = application/vnd.in-toto+json`, and signed over the DSSE
  pre-authentication encoding with `ssh-keygen -Y sign -n oh.war/dsse` — the
  same key, a different namespace, so a response signature does not verify as
  an attestation and an attestation does not verify as a response.
- `sig` is the raw SSHSIG blob (the armor's base64 body), not a bare ed25519
  signature; `keyid` is the OpenSSH `SHA256:` fingerprint. A foreign
  verifier unwraps SSHSIG or hands the PAE and the re-armored blob to
  `ssh-keygen -Y verify`. This is stated in the module docs because it is in
  neither spec.
- Files: `docs/warrants/<alias>/attestations/<act>-<n>.dsse.json`;
  `docs/sas/revisions/attestations/sas-accept-<version>-<n>.dsse.json`.
  Written with `create_new`; never overwritten. A Warrant act also journals
  `attestation.recorded`.
- Only ssh-signed acts are attested. A TTY signature has no key; acts made
  before this existed are unattested; `war attest` says "tty-signed or
  earlier; git is the witness" rather than pretending.
- `war check` stays deterministic and structural. Signature verification is
  `war attest <target> --verify` / `war attest --all`, and one xtask step.
  Verification takes the principal from the predicate's actor through
  `roles.toml`, never from a flag, and recomputes every subject digest
  against the tree today (`attest.subject-drift`).
- `allowed_signers` must permit the `oh.war/dsse` namespace beside
  `oh.war/response`; that file is human-written, so this is an operator edit
  the example file now shows. An attestation the key may not make is a WARN
  (`attest.not-emitted`) on the act, which stands: the record is the act,
  the attestation is its portable witness.
- No new cryptography crate. base64 is forty lines in core (the crate's
  dependency surface stays serde, thiserror, uuid); sha256 and JCS are the
  ones already in use.

## Consequences

- Every human act from here on has a self-describing, externally checkable
  witness beside its record, and `war attest --all` fails the gate when a
  record moves after its attestation.
- The key custody question (THREAT_MODEL.md entry 1) is unchanged:
  attestations make a signature made without a dialog visible, not
  impossible.
- A second signer on one envelope is refused (`this tool writes and checks
  exactly one`); multi-signer attestation is a later decision, not an
  accidental capability.
