# Compatibility from 1.0

What 1.0.0 freezes, what may still move, and how a change is made.

## Frozen

- **Record shapes** `oh.war/<record>/v1` — manifest, atom, milestones,
  deliverables, authorization, judgments, resolution, verification, correction,
  sas-revision, amendment, journal-event, stage-dispatch, stage-submission,
  report, bonsai-evidence, war (the IR root). Their JSON Schemas live under
  `schemas/` (`schemas/pack.json` carries one transitive digest) and are
  drift-checked by `cargo xtask gate`.
- **Digest domains** — every `DigestDomain` string, the RFC 8785 canonical
  form (`serde_jcs`), and the `digest_domain` / `payload` preimage shape (§65).
  A digest minted by 1.0.0 recomputes on every later 1.x.
- **The schema pack identity** `openwarrant-schema-pack` at `0.2.0`. It sits
  inside every contract's `format_basis`, which is inside every contract
  digest; it moves only with a format change, never with a crate release.
- **Exit codes** 0 / 1 / 2 and the `oh.war/report/v1` envelope's fields.
- **Signature namespaces** `oh.war/response` and `oh.war/dsse`, and the
  attestation predicate types `https://openwarrant.dev/attestation/<act>/v1`.

## Additive only

A frozen record may gain a field when the field is `#[serde(default)]` (and
`skip_serializing_if` where absence must not move existing digests), the
schema pack gains the property, and `CHANGELOG.md` names it. A record written
by 1.0.0 must still load, and its digest must not change, on every later 1.x.

## Anything else is a `v2`

A field removed, renamed or retyped; a domain string changed; a canonical
form changed — each is a new `oh.war/<record>/v2` beside the old one, read by
`war migrate`, never a silent edit of `v1`. The pack version moves with it,
and with the pack every contract digest, so a `v2` is a release the owner
re-authorizes into, not a patch.

## Reading backward

The rules above say what a later `war` reads from an earlier one. This is the
other direction: an older `war` meeting a repository, or a record, that a
newer one wrote (OW-WAR-0130).

### A repository says which `war` it needs

`openwarrant.toml` may carry, under `[project]`:

```toml
requires_war = ">=1.2.0"
```

A version requirement in Cargo's comparator syntax: `>=`, `>`, `<=`, `<`,
`=`, `^`, `~` or bare (which is `^`), comma-separated, over a version of one
to three parts, or three with a pre-release (`=1.0.0-alpha.2`). A pre-release
orders below its release, so `>=1.0.0` does not admit `1.0.0-alpha.2`. A
requirement that does not parse is refused when the configuration is read.

It is checked once, when the repository is discovered and before any record
is read. A `war` it does not admit stops there with `compat.war-too-old`,
naming the requirement and its own version, and reads nothing: no Warrant
finding is printed from records it may not understand. No key: every `war`
reads the repository, as before.

The key protects from the first release that reads it. A `war` older than
that does not know the key and ignores it, as it ignores any key it does not
know in `[project]`; for those, the record check below is what remains.

### A record says which major it is

Every frozen record names its schema, `oh.war/<record>/v<major>`, and each
`war` knows the major it reads for each (`crates/openwarrant-cli/src/compat.rs`
lists them, and it moves only with a `v2`). A record naming a newer major is
reported by `war check` as UNKNOWN `compat.newer-record` — never PASS, because
this `war` did not read what it says, and never ERROR, because nothing is known
to be wrong with it. The journal carries its major in each line's `v`.

That is a report, not a translation: whatever else reads the record reads it
as it always did. A repository that relies on a newer record says so with
`requires_war`, which stops an older `war` before it reads anything.

### Unknown optional fields (§69.4)

§69.4 asks that unknown optional namespaced extensions be preserved and that
unknown required ones fail closed. Today they are not preserved: many record
types refuse any field they do not know (`deny_unknown_fields` — the batch,
attestation, drafting-v2 and roadmap records among them), so an optional
extension written by a newer `war` is a parse error to an older one, and the
rest keep only the fields they know when they rewrite a record. Relaxing
that touches every such record type and is its own Warrant (option C of
OW-WAR-0130's U-001); until then, `requires_war` is how a repository keeps an
older `war` away from records it would refuse or trim.

## The crates

`openwarrant-core`, `openwarrant-agent`, `openwarrant-compiler`,
`openwarrant-cli` share one workspace version. Semver applies to the Rust API
as usual; the protocol rules above are stricter and win.

## Not frozen

The human rendering of `war check` and `war show`, the progress platform's
HTML, the skill and plugin files, the conformance battery, gate definitions
under `docs/gates/`, and this repository's own Warrants.
