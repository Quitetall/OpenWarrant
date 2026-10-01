---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ee5-7ba2-9dc3-b9c50dcc6ba1
role: work_order
jurisdiction: authored
order: 40
classification: internal
---


# Work Order

## Deliverables

1. `docs/GRAMMAR.md`, the grammar of the SAS 1.1.0 atom format.
   - Definitions: *standard conformance* (what the SAS text requires of
     any tool) and *tool conformance* (what `war` does).
   - One section per construct: the manifest; the atom header; structured
     atoms; Markdown body headings; the obligation block and its bullets;
     the gate citation; roles and extension roles.
   - Each section has a table: construct, SAS clause, standard
     (accept/refuse/unspecified), `war` (accept/refuse, rule id), plant.
   - A divergence list. At least: §62's nested `holder:` example refused
     by `war`; unknown non-namespaced header keys kept; unknown manifest
     keys ignored; obligation grammar unspecified by the SAS; `atom_uuid`
     in §62's example absent from every atom in the corpus.
   - Proposed SAS text for each divergence, marked as a proposal for a
     separate SAS revision.
2. `crates/openwarrant-cli/src/repo.rs`: the `atom.header` rule, at atom
   load, for every `.md` atom whose manifest role is not `adr`:
   - `schema` equals `oh.war/atom/v1`;
   - `warrant_uuid` equals the manifest `uuid`;
   - `role` equals the manifest entry's role;
   - `order` equals the manifest entry's ordinal;
   - a missing key is an error naming the key; a different value is an
     error naming both values.
3. `conformance/plants.d/57-grammar.sh`:
   - header plants on a bound atom, restored after: `role` changed,
     `warrant_uuid` changed, `order` changed, `schema` removed. Each must
     fail `war check` with `atom.header` naming the key.
   - reader plants: an anchor, a tag, a flow collection, a block scalar, a
     duplicate key, and §62's example header verbatim. Each must fail with
     `atom.frontmatter` naming the construct.
   - positive controls: §62's example with `holder` removed passes; a
     namespaced key (`x.note: kept`) passes and survives compilation.
   - a drift control: every rule id `docs/GRAMMAR.md` cites exists in
     `war check`'s output for some plant above.

## Frozen Surfaces

- The frontmatter and structured readers' accepted sets (OW-ADR-0002,
  OW-ADR-0003).
- Every record schema and the schema pack version.
- Every bound atom in the corpus.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any existing atom that `atom.header` reports (A-001 false);
- any wish to widen or narrow a reader;
- any SAS wording beyond a proposal in `docs/GRAMMAR.md`.

## Rollback

Remove the `atom.header` rule and the plant; delete `docs/GRAMMAR.md`.
No record or atom changes, so nothing else moves.
