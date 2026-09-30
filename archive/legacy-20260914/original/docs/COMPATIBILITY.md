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

## The crates

`openwarrant-core`, `openwarrant-agent`, `openwarrant-compiler`,
`openwarrant-cli` share one workspace version. Semver applies to the Rust API
as usual; the protocol rules above are stricter and win.

## Not frozen

The human rendering of `war check` and `war show`, the progress platform's
HTML, the skill and plugin files, the conformance battery, gate definitions
under `docs/gates/`, and this repository's own Warrants.
