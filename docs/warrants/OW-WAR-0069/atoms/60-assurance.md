---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-78ce-74b8-b6e8-7b8ea92afa04
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

All obligations are unverified. No disposition, gate receipt, verdict,
authorization, or resolution is asserted by this draft.

Assurance level is proposed as `basic` and deliberately not raised. Raising it
to `controlled` would, under the rule proposed in OW-WAR-0070's sibling
discussion, require citing an end-to-end plant or receipt; that rule does not
exist yet, so claiming the level would assert an adequacy nothing checks.

### OBL-001 — a lock file participates in the contract digest

- **scope:** Warrants in `conformance/` fixtures that carry a
  `basis.lock.toml`. No claim about Warrants that do not.
- **evidence:** one fixture Warrant compiled twice, differing only in one pinned
  revision, producing two different contract digests; and the same Warrant with
  the lock removed reproducing the digest it had before this Warrant.

### OBL-002 — the domain is registered and domain-separated

- **scope:** `DigestDomain::ALL` and its conformance test.
- **evidence:** the count assertion passing at its new value, the URI
  `oh.war/dependency-lock/v1` present in the uniqueness set, and a preimage
  showing the `digest_domain` field per §65.2.

### OBL-003 — every malformed pin is refused before it is recorded

- **scope:** the four refusals named in the work order. No claim about malformed
  input shapes none of the plants exercise.
- **evidence:** `conformance/plants.d/69-*.sh` output showing each refusal with
  its named rule, and raw battery output.

### OBL-004 — independent verification

- **scope:** this Warrant's obligations.
- **evidence:** a `war verify --response` verdict from a verifier that is not
  the performer and did not draft these atoms.

## Gate Adequacy

**Adversarial question: could every obligation pass while the system is wrong?**

Yes, in three ways.

First, the plants establish that malformed pins are refused, not that the set of
malformed shapes is complete. A pin that is 40 hex characters but names a
revision that does not exist upstream passes every check here. This Warrant
records a pin; it does not resolve one, and OBL-003's scope says so.

Second, a digest that moves proves the input is wired in, not that anyone reads
the resulting amendment. A program could carry a correct lock, see its digest
move, and still ship against a stale upstream because no human looked. That is a
gap this Warrant cannot close; it makes the movement visible and no more.

Third, the two-toolchain case that motivated this is not itself planted unless a
fixture carries two upstreams with differing toolchains. If it is not, the
evidence is narrower than the problem, and the claim must narrow with it.

**Executed attacks:** none. Execution is pending.
