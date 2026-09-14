---
schema: oh.war/atom/v1
adr_uuid: 5e0c7b3a-2d91-4f46-b8a7-6c1d9e2f4a53
local_alias: OW-ADR-0016
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a021a2-b570-7f57-85b2-0f8189873d9e"
---

# ADR OW-0016: SAS 1.0.0 — one architecture-changing revision, accepted with the release

## Status

Proposed by the performer under the 1.0 plan (slice B4); adopted when the
owner accepts revision `1.0.0` with `war sign 1.0.0 --ssh-sign`, the last act
before the 1.0 tag. §101.3 requires this ADR because the revision adds rows to
§106.

## Context

The accepted revision is `0.1.0-draft.3`. Since it was accepted the tool grew
things the document does not name: a token estimate and budget inside every
Dispatch (§33.7 asked for a budget; §47.1 showed none), two measures §94 never
listed, a correction act for a resolved Warrant's deliverable (§34.4 named the
steps, no requirement row held them), three work kinds each with a gate, an
attestation per ssh-signed act, and a scaffold that writes a program's SAS.
§82.2 also named a command (`liminal-compiler`) that no repository ships, in a
document whose own §75.2 says the command is the adapter's.

Every Warrant resolved so far is pinned to `0.1.0-draft.3`, and the pin is
inside the contract digest (§14). A revision therefore cannot move a resolved
Warrant, and an unresolved one moves only through an amendment that names the
new revision and a re-authorization.

## Decision

1. **One revision, `1.0.0`**, folding every SAS edit the 1.0 plan produced:
   §6.10 (the `war init --program` template note), §47.1 (`tokens`), §47.2
   (over-budget is a refusal), §82.2 (no command named), §94 (two measures),
   §106 (four rows: RQ-036 correction, RQ-046 tokens, RQ-077 work kinds,
   RQ-085 attestation). No row is removed or retitled.
2. **Architecture-changing** under §101.3, by §106's diff (4 added), and so
   carrying this ADR.
3. **Accepted with the release**, by the owner, through the same ssh-signed
   act as every other acceptance; its acceptance is the first attested SAS act
   (OW-ADR-0015).
4. **The circular pin.** An amendment may carry a top-level `sas_revision`
   (and `predecessor_sas_revision`). The tool reads the latest amendment's
   pin ahead of the authorization's when computing the Warrant's Basis, so
   the contract digest moves and the next authorization revision appears in
   `war sign --list`; ingesting that authorization persists the new pin. A
   re-pin whose revision lacks a requirement the Warrant `implements` is an
   ERROR (`sas.repin-unknown-requirement`); a re-pin on a resolved Warrant
   surfaces as the existing `resolution.stale`, never as a moved resolution.
   Warrants that do not re-pin stay honestly pinned to `0.1.0-draft.3`.

## Consequences

- `war sas propose 1.0.0` records the proposal; `war sas accept 1.0.0` puts
  it to a human. Until accepted, `sas.pin-superseded` stays a WARNING and the
  latest *accepted* revision governs.
- `TELEMETRY_MEASURES` goes 18 → 20; the baseline is re-taken and says the
  two new measures are not yet takeable in this corpus (no compiled Dispatch
  is journalled on the committed tree).
- The amendment record's struct is pinned by a resolved Warrant (OW-WAR-0010);
  the pin is read from the amendment file by the CLI, not added to the struct,
  so no correction is needed for this change.

## Alternatives considered

- A revision per slice (`draft.4`, `draft.5`, …): rejected — each would be a
  re-pin opportunity nobody takes, and §101.2 makes every accepted revision
  immutable anyway.
- Leaving §106 unchanged and describing the additions in prose: rejected —
  the rows are what Warrants `implements`, and four deliveries would have
  nothing to trace to.
