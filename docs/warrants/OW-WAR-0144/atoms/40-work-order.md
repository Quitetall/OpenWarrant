---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e9a-7621-b9fe-dc0738889647
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/authorize.rs`: before the recording write,
   when `authorization.toml` exists and its bytes differ from the record
   about to be written, copy it to `authorization.<digest8>.toml` beside
   it. An existing file of that name with the same bytes is left alone;
   one with other bytes refuses the act (`authorize.retire-collision`)
   before anything is written. The dry run keeps nothing and writes
   nothing.
2. `conformance/plants.d/49-authorization-retained.sh`, on a scratch
   corpus with a throwaway key in a throwaway agent (as `96-batch.sh`
   does): authorize revision 1, amend, authorize revision 2; then the
   kept file, the verification, and the refusals below.
3. `docs/SIGNING.md`: one paragraph under the batch section saying where a
   superseded authorization is kept and why.

## Frozen Surfaces

Every record schema; the response, batch and attestation formats; what
`war check` accepts as an authorization.

## Autonomy and Escalation

Tier T2. Escalate rather than decide any change to how an authorization is
judged.

## Rollback

Revert `authorize.rs`. Kept files stay; they are exact prior bytes and
harm nothing.
