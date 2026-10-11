# Hand-written schemas

Everything under `schemas/format/` is written by hand and is normative: for
OpenWarrant Format 1.0 ([`docs/format/FORMAT-1.0.md`](../../docs/format/FORMAT-1.0.md),
OW-ADR-0032) and for the two frozen records `war schemas` cannot generate.
**Where a schema and the prose that governs it disagree, the prose wins** and
the schema is the defect; each schema says so in its `description`.

The rest of `schemas/` is generated, never edited by hand:

| path | what | made by |
|---|---|---|
| `schemas/oh.war/<record>/v<n>.json`, `schemas/pack.json` | the active schema pack, from the Rust record types | `war schemas` (feature `schema`); `war schemas --check` refuses drift |
| `schemas/typescript/` | structural TypeScript for the pack | `war schemas` |
| `schemas/native-draft/` | an experimental pack of draft formats | `war schemas`; `conformance/native_schema_pack.py` |
| `schemas/format/` | this directory | by hand |

## What is here

| file | describes | `id` in the registry |
|---|---|---|
| `1.0/common.json` | shared lexical shapes (ids, names, digests, relation kinds) | `openwarrant/1.0#common` |
| `1.0/frontmatter.json` | a document's frontmatter, as the JSON object read from the subset (sections 3.2, 3.3, 7) | `openwarrant/1.0#frontmatter` |
| `1.0/ticket.json` | a ticket-profile document's frontmatter (sections 2, 4.1) | `openwarrant/1.0#ticket` |
| `1.0/manifest.json` | the optional `manifest.toml` beside `warrant.md` (section 4.2) | `openwarrant/1.0#manifest` |
| `1.0/record.json` | a record's heading, attribute block and body (sections 3.4 to 3.6) | `openwarrant/1.0#record` |
| `1.0/obligation.json` | an obligation record (section 3.7) | `openwarrant/1.0#obligation` |
| `1.0/item.json` | a checklist item line (section 3.8) | `openwarrant/1.0#item` |
| `1.0/profile.json` | a profile file, `profiles/*.toml` and a pack's profiles | `oh.war/profile/v1` |
| `1.0/type-pack.json` | a type pack's `pack.toml` | `oh.war/pack/v1` |
| `1.0/model.json` | the canonical JSON model (section 8) | `oh.war/model/v1` |
| `1.0/context.json` | `war context --json` (section 10.3) | `oh.war/context/v1` |
| `records/sas-revision.v1.json` | a SAS revision, `docs/sas/revisions/*.toml` | `oh.war/sas-revision/v1` |
| `records/amendment.v1.json` | a section 31 amendment, `docs/warrants/*/amendments/*.yaml` | `oh.war/amendment/v1` |

The authored surfaces have no wire id of their own (a document names its
`format`, not a schema), so the registry names them `openwarrant/1.0#<surface>`.
Each hand-written schema's `$id` is under `https://openwarrant.dev/schemas/format/`,
so none collides with a generated `$id`; `x-oh-war-schema` names the wire id
it describes. Two schemas describe `oh.war/model/v1`: the generated one is what
`war model` emits today, and `1.0/model.json` is what section 8 requires (the
implementation owes the additional members). The `1.0/` schemas `$ref`
`common.json` relatively, so take the directory, not one file.

`registry.json` lists every schema under `schemas/oh.war/` and `schemas/format/`:
its `id`, its file, its `$id`, and whether it is `generated` or `normative`.

## The check

```sh
python3 conformance/schemas/format_schemas.py              # jsonschema 4.26.0
python3 conformance/schemas/format_schemas.py --legacy-report
```

It runs in the gate as the plant `conformance/plants.d/177-format-schemas.sh`.
It fails by name when a frozen record of `docs/COMPATIBILITY.md` (or of
`FROZEN` in `crates/openwarrant-cli/src/compat.rs`) has no registered schema,
when a schema file's `$id` is not in the registry, or when a registry entry's
file is gone, and shows itself refusing each of those (planted in memory). It
validates every hand-written schema against the repository's own records, the
examples of `FORMAT-1.0.md`, the fixtures under `conformance/schemas/fixtures/`
and doctored copies of real records, and requires each schema to be seen both
accepting and refusing. An invalid fixture names the error it must be refused
with.

A schema checks shape. It does not check sort orders, digest values, byte
limits on non-ASCII text, cross-file facts (a profile's name is its file stem,
a predecessor revision exists) or anything Appendix A says about bytes; each
schema's `description` says what it leaves out.
