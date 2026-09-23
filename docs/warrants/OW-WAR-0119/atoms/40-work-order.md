---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e99-7663-90dd-3063ec23be35
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/identity_check.rs` (new): the corpus
   identity rules, each a named diagnostic:
   - `identity.duplicate-uuid` over Warrant manifests and over ADR atoms;
   - `identity.atom-mismatch` for every Markdown atom's
     `warrant_uuid`;
   - `identity.changed` against `HEAD:<manifest path>`; `UNKNOWN`
     when git cannot answer;
   - `identity.alias-ref` for `war://` bodies that do not parse as a
     UUID: error in `parents` and `supersedes`, warning in ADR
     `governs`, with the resolved UUID in the message when the alias
     exists here;
   - `identity.adr-not-v7` (warning), one per ADR.
   - Unit tests for each rule, with a refusal case and a clean case.
2. `crates/openwarrant-cli/src/check.rs`: call the identity rules once
   per corpus run. No existing rule changes.
3. `crates/openwarrant-cli/src/blut.rs`: `stage_identity` becomes
   `war://<uuid>`; its test asserts the UUID form.
4. `conformance/plants.d/51-identity.sh`, on a scratch program:
   - positive: the clean program and this repository's corpus report none
     of the five rules as an error;
   - a copied Warrant directory with a new alias and the same UUID gives
     `identity.duplicate-uuid` naming both aliases;
   - one atom's `warrant_uuid` changed gives `identity.atom-mismatch`;
   - a committed Warrant's manifest UUID changed (journal left alone) gives
     `identity.changed`;
   - `[[supersedes]] ref = "war://<alias>"` gives an error naming the
     alias's UUID; the same in `[[parents]]` gives the same error;
   - a v4 manifest UUID still gives `manifest.invalid`.

## Frozen Surfaces

Every record schema; `identity.rs`'s types and error messages; the
contract digest; existing diagnostic names. No UUID in the corpus is
changed.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- making `identity.adr-not-v7` or the ADR `governs` finding an error;
- any rule that would fire as an error on an existing Warrant.

## Rollback

Remove `identity_check.rs` and its call in `check.rs`, and restore
`blut.rs`'s stage identity. No record was written, so nothing else
changes.
