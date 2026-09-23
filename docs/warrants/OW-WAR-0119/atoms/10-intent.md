---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e99-7663-90dd-3063ec23be35
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

RQ-001 says every WAR has an immutable UUIDv7 identity. RQ-002 says a local
alias does not substitute for it. Much of this exists already:

- `openwarrant-core/src/identity.rs` mints UUIDv7 (`WarUuid::mint`) and
  refuses any other version. `war check` reports a v4 manifest UUID as
  `manifest.invalid`.
- The contract digest covers `identity`, so changing an authorized
  Warrant's UUID makes its authorization stale.
- `journal.wrong-warrant` fires when the manifest UUID no longer matches
  the journal's `draft.created` event.

Probes on a scratch program (2026-09-23) found what does not exist:

- **Two Warrants can share one UUID.** A Warrant directory copied under a
  new alias, UUID unchanged, passes `war check` with 0 errors. Nothing
  checks UUID uniqueness across the corpus.
- **An atom can name another Warrant.** An atom whose `warrant_uuid`
  differs from its manifest passes `war check`. Only the legacy importer
  (`document/legacy.rs`) compares them.
- **An alias passes for an identity in a `war://` reference.**
  `[[supersedes]] ref = "war://ID-WAR-0002"` is accepted with no finding.
  A `[[parents]]` ref in alias form is reported `UNKNOWN` as "not in this
  repository", not as an alias. Six ADR `governs` entries in this corpus
  use alias form (OW-ADR-0003 to OW-ADR-0008).
- **`war blut` writes an alias as a stage identity:**
  `stage_identity: format!("war://{alias}")` in `blut.rs`.
- **ADR identities are not checked at all.** SAS §12.1 requires a UUIDv7
  for every WAR *and ADR*. 13 of the 23 ADR atoms here carry a v4
  `adr_uuid`, and nothing reports it.

## Desired Outcome

`war check` enforces identity across the corpus, by name:

- `identity.duplicate-uuid` (error): two Warrants, or two ADRs, with one
  UUID. It names both.
- `identity.atom-mismatch` (error): an atom's `warrant_uuid` is not its
  manifest's UUID.
- `identity.changed` (error): a manifest's UUID differs from the one
  committed at HEAD for the same path. This holds §12.2 directly, not only
  through the journal.
- `identity.alias-ref`: a `war://` reference whose body is not a UUID.
  It is an error in a manifest relation (`parents`, `supersedes`) and a
  warning in an ADR's `governs`. When the alias resolves in this
  repository, the finding names the UUID to write instead.
- `identity.adr-not-v7` (warning): an ADR whose `adr_uuid` is not a
  UUIDv7, listed one by one.
- `war blut` writes `war://<uuid>` as the stage identity.

## Non-goals

- Re-minting any existing identity. §12.2 says a UUID never changes, so the
  13 v4 ADR UUIDs stay as they are and are reported. Whether to go further
  is the owner's question (20-basis U-001).
- Editing the six alias-form ADR `governs` entries. They are reported;
  correcting them is an ADR amendment, not this Warrant.
- Changing record schemas. `authorization.toml`, `judgments.toml` and
  the response files name a Warrant by alias. They bind to the UUID through
  the contract digest. Adding `warrant_uuid` to those schemas is a schema
  change for a later Warrant.
- Enterprise identity and federation (RQ-003 to RQ-005). Knowledge Fabric
  allocates those (OW-PHASE-4).
