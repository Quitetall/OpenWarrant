---
schema: oh.war/atom/v1
adr_uuid: 01a01be9-5fd5-79c3-ad31-7fa722f3d0b1
local_alias: OW-ADR-0004
role: adr
jurisdiction: authored
order: 30
classification: internal
status: accepted
decided: 2026-08-19
governs:
  - "war://OW-WAR-0009"
---

# ADR OW-0004: Represent partial authorization and partial contract coverage explicitly

## Status

`Accepted 2026-08-19`

Governs OW-WAR-0009. Answers its OBL-000, which required the
authorization-without-authority question to be decided in writing rather than
defaulted.

## Context

Contract revisions run into two versions of the same problem: OpenWarrant must
represent something the specification defines fully, while only part of what the
definition needs actually exists yet.

**Authorization.** §28.4 says authorization creates an immutable revision
carrying an authorizer, an acting role, an authorization meaning, an effective
time, and a policy basis. There is one actor on this project. OW-WAR-0009's
Basis called this a blocking unknown and warned that treating the absence of
authority as satisfied would be the failure.

**Contract digest coverage.** §28.5 enumerates seventeen elements the contract
digest SHALL cover: intent, scope, basis requirements, assumptions, constraints,
ADR references, deliverables, milestones, stages, capabilities, autonomy,
resources, gates, obligations, rollback, amendment policy, assurance
requirements. Today's `contract_digest()` covers four IR sections — format basis,
identity, source and composition, relations. Most of §28.5's list does not exist
as a typed field: deliverables arrive with OW-WAR-0015, obligations with
OW-WAR-0016, gates with OW-WAR-0019, capabilities with OW-WAR-0023.

So a digest labelled "contract digest" today covers roughly a quarter of what
§28.5 requires, and nothing says so.

## Decision

Two clauses, one principle: **the record states what it actually is.**

### 1. Local authorization is permitted, and the role exercised is recorded

The answer is in the specification and did not need inventing. §27.2's
prohibition is on an **agent**: "An agent SHALL NOT authorize its own proposed
WAR." §27.4 then addresses humans directly:

> One person may exercise several roles. The system SHALL record the role
> actually exercised. Role separation by one person is not organizational
> independence. Human views SHALL not claim four-eyes review when none occurred.

So a human authorizing a Warrant they drafted is representable. Three
obligations follow, and they are enforced rather than documented:

- the **acting role** is recorded, per §27.4;
- **independence is recorded as none** — never omitted, never inferred. An
  absent independence field would read as unexamined; `none` reads as examined
  and absent;
- an **agent** authorizing a proposal it produced is refused outright (§27.2).

### 2. The contract digest declares its own coverage

The digest preimage carries the list of §28.5 elements it covers. A digest over
four elements and a digest over seventeen are therefore distinguishable by
inspection, and cannot be confused for one another.

This means the digest CHANGES as elements land. That is correct and is the point:
adding deliverables to the contract genuinely changes what was authorized, so a
Warrant authorized before that element existed was authorized over less. Silently
widening coverage while keeping the digest stable would be the actual defect —
it would let a later, broader contract present itself as the one that was signed.

## Rationale

**Both clauses refuse the same shortcut.** The tempting move in each case is to
produce the full-strength object and hope the gap goes unnoticed: an
authorization that does not say who authorized it under what role, and a
"contract digest" that sounds like §28.5's but is not. Each would be a claim the
record cannot support, and each would be discovered later by someone relying on
it.

**Recording absence is not the same as having the thing.** `independence: none`
does not make a self-authorized Warrant independently reviewed. It makes the
absence legible, so a reader — or OW-WAR-0021's verifier-independence check —
can act on it. §27.4's last sentence is explicit that role separation by one
person is not organizational independence, and this encodes that sentence rather
than working around it.

**A self-describing digest is cheap now and impossible to retrofit.** Once
digests exist in the wild without coverage metadata, there is no way to tell
which era a given digest came from.

## Alternatives Considered

- **Forbid authorization until Knowledge Fabric exists.** Rejected: it would
  leave every Warrant permanently in `draft`, which makes the state model
  ornamental and blocks OW-WAR-0022 (resolution) indefinitely. §27.4 explicitly
  contemplates one person holding several roles, so refusing would be stricter
  than the specification.
- **Allow authorization and stay silent about independence.** Rejected — this is
  the failure OW-WAR-0009's Basis named. It is also what §27.4's final sentence
  forbids in so many words.
- **Keep one contract digest and widen it silently as elements land.** Rejected:
  a Warrant authorized under a four-element contract would later appear to have
  been authorized under a seventeen-element one. That is the strongest possible
  version of a false claim, because it is cryptographic.
- **Withhold the contract digest until all seventeen elements exist.** Rejected:
  children already pin parent contract digests and that check works. Removing it
  to wait for completeness would delete a working control for the sake of purity.

## Consequences

**Good.** Warrants can leave `draft`. Every authorization says who acted, in what
role, with independence recorded as absent. Digests cannot be mistaken across
coverage eras.

**Bad.** Contract digests will change several times as §28.5's elements land, and
each change invalidates previously pinned parent digests — which will surface as
`relations.parent-digest` errors that must be re-pinned deliberately. That is
noisy, and it is the honest cost of the digest meaning something. It is also
bounded: it happens once per element-bearing Warrant, and the roadmap names them.

**Unchanged.** §27.2's prohibition on agent self-authorization stands and is now
enforced. The existing parent-digest verification keeps working.

## Validation

Watch for: an authorization recorded with `independence: none` being cited as
though it were reviewed; coverage metadata being dropped "because the digest is
stable now"; and the re-pinning churn tempting someone to freeze coverage
prematurely, which would mean later contract elements are excluded from what was
signed.
