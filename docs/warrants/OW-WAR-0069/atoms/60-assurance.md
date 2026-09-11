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

Assurance level is proposed as `basic` and deliberately not raised. Raising it to
`controlled` would require an adequacy this Warrant cannot yet claim, and the
corpus already warns that the available verification is role separation by one
person rather than organizational independence (§27.4).

### OBL-001 — a lock file participates in the contract digest

- **scope:** Warrants in `conformance/` fixtures that carry a
  `basis.lock.toml`. No claim about Warrants that do not.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** one fixture Warrant compiled twice, differing only in one pinned
  revision, producing two different contract digests.

### OBL-002 — the domain is registered and domain-separated

- **scope:** `DigestDomain::ALL` and its conformance test.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the count assertion passing at its new value, the URI
  `oh.war/dependency-lock/v1` present in the uniqueness set, and a preimage
  showing the `digest_domain` field per §65.2.

### OBL-003 — every malformed pin is refused before it is recorded

- **scope:** the four refusals named in the work order. No claim about malformed
  input shapes none of the plants exercise.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `conformance/plants.d/69-*.sh` output showing each refusal with
  its named rule, and raw battery output.

### OBL-004 — no existing contract digest moves

- **scope:** every Warrant in `docs/warrants/` on this branch at the time of
  execution. No claim about Warrants on other branches or in other repositories.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the whole corpus compiled before and after the change, with every
  contract digest diffed and zero differences. The lock field must be `Option`
  with `skip_serializing_if = "Option::is_none"` so a Warrant carrying no
  `basis.lock.toml` serialises byte-identically into the FormatBasis preimage.

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

OBL-004 exists because the failure it guards is silent and corpus-wide: a
non-optional field would move all 63 contract digests at once and stale every
authorization, and no single-Warrant test would notice.

**Executed attacks:** none. Execution is pending.
