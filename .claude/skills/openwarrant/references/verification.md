# Verification and closure: what counts

An obligation in `60-assurance.md` is the unit of completion. It needs a
**bounded scope** (§38.4: a claim is bounded by its evidence):

```markdown
### OBL-001 — the parser refuses a duplicate ordinal

- **scope:** manifests exercised by the fixtures in `conformance/`.
- **evidence:** a planted duplicate, and the specific error it produces.
```

"The parser works" is not an obligation. Pair every passes-claim with a
refusal obligation: code that always succeeds satisfies a success claim.

## Independence

`war verify <alias> --performer <you>` emits the request. Hand it to a
separate context — another session, another model, a person — never your own.
`war verify --response` refuses a verdict whose verifier is the performer and
writes nothing; editing `performer` to get past it falsifies a record.

Three dispositions: `established`, `refuted`, `not_established`. `UNKNOWN` from
a gate is neither pass nor fail; it is a gate that could not run, and it blocks.

## Evidence

`war evidence record <alias>` runs the gates the assurance atom cites and mints
§44.6 receipts into `gate-runs/`, bound to the contract digest they ran
against. A receipt from an older contract is inadmissible, not stale-but-fine.

## Closure

`war resolve --dry-run <alias>` lists the thirteen §56.1 requirements and
which are unmet, plus §38.6's "would resolve satisfied" beside them. Two met
honestly beats thirteen claimed. Resolution itself is `war sign` by a human.

## Things that look like progress and are not

- a green check after editing the document the check reads;
- a verification you wrote for your own work;
- a receipt minted against a contract that has since been amended;
- a "resolved" claim in prose, anywhere, that has no `resolution.toml` behind it.
