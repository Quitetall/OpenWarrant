# Prospective authority activation — bounded observations

New v2 activations require human administrators/recovery signers from current
trusted authority. A proposal cannot promote its own agent signer. Missing actor
kind is UNKNOWN. Current presence policy is checked after cryptographic signature
verification: an ordinary signature leaves presence UNKNOWN; a signed clear flag
refuses. Proposed policy relaxation does not authorize its own adoption.

The initial SDK test compilation failed because the new admission function did
not exist (`red.log`). Nine authority SDK controls pass. The core all-target,
all-feature suite passes 707 tests. The final authority CLI unit run passes three
tests, including existing store protection and two new cryptographic/policy
controls. Both crates pass all-target/all-feature Clippy with warnings denied.
The public CLI refuses agent and policy-service signers before asking a key to sign; an unbound v2 signer returns exit 2, one UNKNOWN and zero errors. No signature is emitted and input authority bytes stay unchanged (`cli-observations.json`). The earlier transport run is retained separately; final source is in `source.json`.

Controls verify real OpenSSH signatures made from software fixture keys held only
in memory. They plant agent/policy-service administration, missing actor binding,
missing/absent presence, tampered proposed policy, and proposed self-promotion.
Refused and unknown activations leave the stored authority snapshot unchanged.
The positive fixture records no human review. Simulated security-key flags do not
establish actual hardware presence, custody, human consent or deployment.

Retained history still uses its original signature and role validation. Legacy
v1 keeps its role-only activation contract and supplies no v2 actor-kind/presence
assurance. This is not retrospective invalidation, formal Warrant verification,
collector enrollment activation, launch fencing, or independent qualification.
Full repository CI, protected collector workflow and operator observations remain
open. No human authority record was created by this work.

Eight authority/collector integration regressions pass; two namespace entries are
explicitly ignored by the default suite. The explicit namespace test initially
failed because the execution UID could not read enrollment in the default scratch
environment (`namespace-default-scratch-failure.log`). The same built test binary,
run with `TMPDIR=/mnt/4tb/tmp`, passes the full two-UID load/revocation roundtrip
(`namespace-provisioned.log`). Neither result is replaced by the other.

The existing authentication plant enabled presence and later attempted to disable
it using an ordinary key. That setup no longer satisfies the current policy. The
presence case now uses its own disposable store, with an added refusal control
for that attempted downgrade. Other revocation cases keep their independent
store; no accepted authority record is reset or rewritten. Syntax checks pass;
the focused plant run is pending at this candidate commit.
