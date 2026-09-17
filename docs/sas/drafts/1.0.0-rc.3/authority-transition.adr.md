# Candidate ADR: signed authority transitions

Status: implementation decision under OW-WAR-0096; unsigned candidate. No adopted
ADR identifier or legacy signature subject is replaced.

Use RFC 8785 JCS through the already selected serde_jcs implementation. Candidate
revision schema is `oh.war/authority-revision/1`; proposal schema is
`oh.war/authority-proposal/1`. Revision digest hashes UTF-8 bytes
`openwarrant-authority-revision-v1` followed by NUL and canonical revision JSON.
Proposal digest and signing message use `openwarrant-authority-proposal-v1`, NUL,
and canonical proposal JSON. Digests are lowercase hexadecimal SHA-256 prefixed
`sha256:`. OpenSSH signatures use namespace `openwarrant-authority-v1`.

A revision contains repository identity, sequence and principal/key/role mappings.
A proposal contains operation, previous revision digest and complete next revision.
The first reference profile accepts canonical Ed25519 public keys, exact safe
ASCII names, at most128 principals and32 roles each,64KiB wire records and exact
JSON integer sequences. One prior authority-admin signs an update; one prior
authority-recovery signs a recovery. Both require exact parent and next sequence.
Removing the last administrator is invalid. Recovery replaces future state;
it cannot rewrite historical grants or validate itself using a newly added key.

SDK eligibility consumes explicitly authenticated signer facts. CLI verifies
OpenSSH signatures against previous keys. A protected store owns bootstrap and
current state and atomically retains transitions with the head. The snapshot
schema is implementation-local `oh.war/authority-store/1`, not a portable trust
anchor. Exports carry evidence, not trusted-state installation permission.

Cryptographic authentication does not establish human presence. A deployment
must protect key use and the verifier and must route privileged operations through
trusted current state. Unverified prototyping remains independent. Legacy
records remain byte-preserved and do not acquire new authority retroactively.
