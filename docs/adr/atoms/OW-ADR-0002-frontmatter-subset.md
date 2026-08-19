---
schema: oh.war/atom/v1
adr_uuid: 01a01928-d309-72ee-8550-33779fc3d2a3
local_alias: OW-ADR-0002
role: adr
jurisdiction: authored
order: 30
classification: internal
status: accepted
decided: 2026-08-19
governs:
  - "war://01a018db-19fc-7f34-92db-54b2dca5446d"
---

# ADR OW-0002: Parse atom frontmatter with a restricted reader, not a YAML library

## Status

`Accepted 2026-08-19`

Governs OW-WAR-0002, whose Basis records this as a blocking unknown that must be
resolved by an implementation ADR before the frontmatter half of the work starts.

## Context

SAS §62 specifies "Markdown with YAML frontmatter" for prose-heavy authored
atoms. §80's stack table hedges conspicuously: "`serde_yaml` or **a safer
maintained YAML parser**." §87.2 requires the parser to behave safely on
untrusted sources.

Surveyed 2026-08-19, the hedge is justified — the Rust YAML ecosystem has no
option that is both stable and maintained:

| crate | version | license | last updated |
|---|---|---|---|
| `serde_yaml` | 0.9.34**+deprecated** | MIT OR Apache-2.0 | 2024-03-25 |
| `serde_yaml_ng` | 0.10.0 | MIT | 2024-05-26 |
| `serde_norway` | 0.9.42 | MIT OR Apache-2.0 | 2024-12-21 |
| `saphyr` | **0.0.12** | MIT OR Apache-2.0 | 2026-08-18 |

The serde_yaml lineage is stale by twenty months or more and its origin is
explicitly deprecated. `saphyr` is actively maintained and fully YAML 1.2
compliant, but is pre-1.0 with an unstable API.

Meanwhile, what an atom's frontmatter actually contains is tiny and fully known:

```yaml
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f2a-8e39-69730f255e33
role: intent
jurisdiction: authored
order: 10
classification: internal
```

Flat keys, scalar values, and occasionally a list of strings. Nothing else.

## Decision

Parse frontmatter with a **restricted reader** implemented in
`openwarrant-core`, accepting a documented subset and **refusing everything
outside it**. No YAML library dependency.

The subset is: a leading `---` fence, then `key: value` lines where the value is
a plain scalar or a `- item` block-sequence of plain scalars, then a closing
`---`. Comments (`#`) and blank lines are permitted. Everything else — anchors,
aliases, tags, flow collections, nested mappings, multi-document streams, block
scalars, quoted keys — is a parse **error**, never a silent reinterpretation.

## Rationale

**A full YAML parser is a large attack surface for a six-line header.** YAML 1.2
brings anchors and aliases (the "billion laughs" expansion DoS), merge keys,
arbitrary nesting, tag resolution, and multi-document streams. An atom header
needs none of it, and §87.2 makes accepting untrusted input a stated concern.

**Implicit typing is a correctness hazard, not just a safety one.** YAML's
implicit scalar resolution is where the Norway problem lives: a bare `NO`
becomes boolean `false`, `1.0` becomes a float, and a version string like
`1.10` may not survive a round trip. Frontmatter values here are typed by the
*schema*, not by their spelling. A reader that does not guess types cannot
guess wrong.

**Fail-closed is the whole point.** SAS §62.3 requires unknown required fields
to fail and namespaced optional fields to be preserved. A restricted reader
makes that trivially enforceable, because anything it does not recognise is
already an error. A permissive parser would happily accept a nested mapping
where a scalar was meant and hand us a value the validator then has to
second-guess.

**The dependency choice was between stale and unstable.** Adopting a deprecated
crate means inheriting its unfixed bugs; adopting a 0.0.x crate means an API
that may break on any release. Writing roughly a hundred lines we can test
exhaustively is a smaller liability than either, and it removes a transitive
dependency from a repository whose license graph must stay clean for the
Apache-2.0 path.

## Alternatives Considered

- **`saphyr` 0.0.12** — the honest runner-up: actively maintained, correct, and
  properly licensed. Rejected on API instability at 0.0.x for something on the
  parse path of every atom. This is the option to revisit at 1.0, and revisiting
  is cheap because the reader sits behind one function.
- **`serde_norway` / `serde_yaml_ng`** — rejected. Both are unmaintained forks
  of a deprecated crate; adopting one is choosing which abandoned codebase to
  inherit.
- **`serde_yaml`** — rejected. Its own version string says `+deprecated`.
- **TOML frontmatter instead of YAML** — genuinely tempting, since `toml` is
  already a dependency, well maintained, and has no implicit-typing hazard.
  Rejected because SAS §62 specifies YAML frontmatter for the v1 source adapter,
  and unilaterally changing the source format would put this repository's atoms
  out of conformance with the specification it implements. If the SAS is revised,
  this is the first thing to reconsider.

## Consequences

**Good.** No YAML dependency, no expansion-DoS surface, no implicit-typing
surprises, and a parser small enough to test exhaustively against its own
grammar. Unknown constructs fail closed by construction.

**Bad.** Atoms cannot use the full YAML the SAS nominally permits. An author who
writes a nested mapping in frontmatter gets an error rather than a value — and
that error must say *which* construct was refused and that the subset is
deliberate, or it will read as a bug in the tool. If a future atom role
genuinely needs structured frontmatter, this decision has to be revisited rather
than worked around with string encoding.

**Unchanged.** Atom bodies are Markdown and are untouched by this; exact source
bytes are still preserved per §62.2.

## Validation

Watch for: an author hitting the subset limit more than once, which means the
subset is too narrow for real atoms; any need to encode structure into a scalar
to get around it, which is the workaround this ADR would rather forbid than
permit; and `saphyr` reaching 1.0, which reopens the dependency option on better
terms.
