# Example 4 — a contractor Work Order: terms beside the technical core

> **NON-BINDING.** This example and its fixture are technical demonstrations.
> Nothing here is a contract or a legal instrument, and nothing here is legal,
> financial or quality advice. The contractor profile is not approved
> (`approved = false`, SAS §22.3): SAS §98 requires separate legal, finance and
> QMS decisions before it is delivered, and none exists in this repository
> (OW-WAR-0140 U-001). Every party, term, invoice and payment the fixture cites
> is invented and does not exist.

OW-WAR-0140 adds profiles as data. `profiles/contractor.toml` extends
`delivery` and requires four namespaced roles. Nothing in the technical core
learns their names: they are composition entries like any other atom, and the
core modules the work order freezes did not change when the profile was added.

The fixture is `conformance/fixtures/contractor/work-order.sh`; the plant that
walks it is `conformance/plants.d/56-contractor.sh`. Every step below runs
there, in a scratch program.

## 1. A program adopts the profile

A profile is a file in the program's `profiles/` directory. The repository
reads every `profiles/*.toml` when it opens:

```bash
mkdir -p profiles && cp <openwarrant>/profiles/contractor.toml profiles/
```

The registry refuses a definition that redefines `delivery` or `decision`,
that extends anything but a core profile, that does not say whether it is
approved, or that requires a role outside its own namespace (`contractor.*`).
A refused definition refuses the repository: every command names the file.

## 2. `war new --profile contractor`

```bash
war new --profile contractor "Fixture: a contractor Work Order (non-binding)"
```

writes the `feature` preset's five delivery atoms and one stub per contractor
role, each stub taken from the definition's own text:

| ordinal | role | carries |
|---|---|---|
| 71 | `contractor.parties` | the parties, by reference |
| 72 | `contractor.terms` | scope, compensation, schedule, confidentiality and legal terms, by reference |
| 73 | `contractor.acceptance` | the acceptance authority (in the header) and criteria |
| 74 | `contractor.commercial` | invoice and payment relations, by reference |

The manifest says `profile = "contractor"` and declares all nine atoms
required. Without `profiles/contractor.toml` the same manifest is refused
`unknown profile "contractor"`; with it, a manifest that drops
`contractor.acceptance` is refused `profile contractor requires a
contractor.acceptance atom`. A `delivery` Warrant that declares a required
`contractor.terms` is still refused as an unknown required role: the registry
widens exactly the roles one definition names, for Warrants of that profile.

## 3. What `war check` says about it

```text
PASS    profile.registered            profile contractor extends delivery and is defined at sha256:…
WARN    profile.unapproved            profile contractor is not approved (`approved = false`) …
PASS    profile.acceptance-authority  acceptance authority "…" is a human holding resolver …
UNKNOWN profile.reference             contractor.terms cites kf://work-orders/FIXTURE-WO-0001#scope; no Knowledge Fabric is reachable …
```

- **Acceptance** is the existing resolution act. The acceptance atom's header
  names an actor (`acceptance_authority: <actor>`). The check reads the
  existing register, `docs/authority/roles.toml`. It refuses an agent by
  kind, an actor without `resolver` by name, and the performer (§27.2). No
  actor role was added: whoever accepts does it with `war resolve`, which
  refuses an agent whatever the atom says.
- **References, not copies.** `contractor.terms` and `contractor.commercial`
  must each cite at least one record. A `kf://` reference reads UNKNOWN,
  because no Knowledge Fabric is reachable here (U-004). That is not a pass
  and not a failure. A `war://` reference to a Warrant in this repository
  resolves.

## 4. The technical core did not move

The plant compiles the fixture twice: as `contractor`, then as `delivery` with
the four contractor atoms removed from the manifest. The two IRs differ only
in:

- `identity.profile` and `format_basis.profile_schema_id`;
- the four `contractor.*` entries in `source_and_composition.atoms`;
- the manifest digest, and the composition and workspace digests that cover
  those entries.

No `FIXTURE-*` party, work-order, invoice or
payment token appears anywhere in either IR: the terms stay in their atoms.
The plant also diffs the frozen core modules from the commit that introduced
the registry to HEAD. The diff is empty. A planted `invoice` field on a core
struct fails that check.

## What this example does not show

The Phase 10 Exit. The fixture shows the technical half: it compiles, its IR
matches `delivery`, and the core is unchanged. The Exit also requires the
legal, finance and QMS decisions §98 names. Until those exist as records a
Warrant can cite, no contractor Work Order here is more than a fixture.
