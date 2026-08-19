---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-72ba-87b3-c1bd1aec86a8
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. An implementation ADR selecting the RFC 8785 library, with its license
   checked against the Apache-2.0 path. **This ships first.**
2. `WarIr` — the §63 top-level shape, with `api_version` and `kind` pinned.
3. `FormatBasis` — the §64 schema-pack pin.
4. Canonical serialisation to RFC 8785 JSON.
5. `DigestDomain::preimage(payload)` producing the §65.2 envelope, and the
   digest functions over it.
6. Conformance tests for §91.1 items 1 through 6.

## Frozen Surfaces

- `api_version = "oh.war/v1"` and `kind = "work_authorization_record"`.
- The fifteen digest-domain URIs, already frozen by OW-WAR-0001.
- The preimage envelope's field names and their order under canonicalization.

## Premade Instructions

- The library ADR precedes the code. Not concurrently — before.
- Test canonicalization against RFC 8785's own published vectors, not against
  this implementation's output. An implementation-derived expectation passes
  even when the implementation is wrong, which is the failure mode this whole
  Warrant exists to prevent.
- Distinguish an absent IR section from an empty one, and pin that distinction
  with a test.
- Determinism is asserted across two independent runs, not by comparing a value
  to itself.

## Resources and Capabilities

Repository-local filesystem, network for crate resolution. No secrets.

## Autonomy and Escalation

Tier T1 — this fixes protocol bytes. The library choice, the envelope shape, and
the absent-versus-empty rule all escalate; they are not implementation judgment.

## Rollback

Revert. Because no digest may be published before this Warrant resolves, a
rollback cannot orphan an already-minted digest — which is precisely why the
no-placeholder rule is in the Basis rather than left as a preference.
