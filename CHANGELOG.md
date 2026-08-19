# Changelog

Notable changes to OpenWarrant. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/), with the caveat below.

**Pre-1.0, the protocol is not stable.** The canonical JSON shape, digest
domains, and manifest schema may change in any 0.x release. Digests minted by
one 0.x version are not guaranteed to be reproducible by another.

## [Unreleased]

### Added — Phase 1: the file-native compiler

- `war init` — initialize repository configuration and directories (SAS §71.1).
- `war new` — create a draft Warrant, with `O_EXCL` alias allocation so
  concurrent invocations cannot collide (§71.2).
- `war check` — deterministic, agent-free validation with PASS / WARN / UNKNOWN /
  ERROR diagnostics and a readiness verdict (§71.7, RQ-074).
- `war compile` — the full Markdown parent and RFC 8785 canonical JSON, each
  carrying the §17.1 generated header (§71.8).
- `war check --generated` — drift detection against committed projections
  (§17.3, RQ-075).
- Manifest and atom parsing with fail-closed validation for missing required
  atoms, duplicate ordinals, unknown required roles, fabricated enterprise
  identifiers, and composition cycles (§91.2).
- RFC 8785 canonicalization and the fifteen domain-separated digest domains of
  §65, pinned against the official conformance vectors.
- Parent contract-digest verification: a child citing a parent whose contract has
  since changed is reported as resting on a basis it was never authorized
  against (§20.2).
- First-class ADR atoms and a **generated ADR Overview** (§19.6, RQ-021): the
  Appendix A shape with a summary table, lifecycle buckets, and the complete
  decision bodies concatenated as one audit document. Drift-checked like any
  other projection, because §19.7 forbids a manually maintained index.
- `cargo xtask gate` — the aggregate gate of §92: SPDX headers, build, fmt,
  clippy, tests, licenses, and a planted-violation battery that asserts each
  control rejects for its intended reason.

### Decisions

- **OW-ADR-0001** — adopt `serde_jcs` for RFC 8785, chosen by running both
  realistic candidates against the official vectors rather than on reputation.
- **OW-ADR-0002** — parse atom frontmatter with a restricted reader instead of a
  YAML library, because the Rust YAML ecosystem offers only stale or pre-1.0
  options and a six-key header does not justify the attack surface of YAML 1.2.

### Known gaps

Stated rather than left to be discovered:

- Gate execution is not implemented. `war check` validates the record, not the
  work; a Warrant whose acceptance gates are nonsense passes.
- Preflight (§32.7) does not exist, so the verdict says WELL-FORMED rather than
  READY.
- Bound atoms (`ref =`) cannot be resolved offline; federation is Phase 4.
- Two of the nine projections in §17.5 are implemented (`full_warrant`,
  `canonical_json`).
- Contract revisions are not implemented; every compilation is revision 1.
- ADR supersession is not modelled at all — `AdrRecord` has no `supersedes`
  field — so §91.4 test 25 (supersession is acyclic) is not covered. The
  `superseded` status exists as a lifecycle state, but nothing records what
  superseded what.
- Conformance runs on one host, so §91.1 test 1's "two supported hosts" is
  satisfied by two runs rather than two architectures.
