---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f34-92db-54b2dca5446d
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

- SAS §13 (Source Holders and Jurisdiction), §16 (composition grammar),
  §59–§62 (layout, configuration, manifest, atom source format),
  §91.2 (atom and composition conformance).
- Parent Warrant OW-WAR-0001, contract revision 1, which established the crate
  this work lands in and froze the crate's responsibility boundary.

## Context

The five hand-authored Warrants in `docs/warrants/` are the acceptance corpus.
They were written before the parser deliberately, so the parser is developed
against documents nobody shaped to be easy to parse.

## Prerequisites

- OW-WAR-0001 resolved.
- `openwarrant-core` exists with `RepositoryConfig` and identity types.

## Assumptions and Unknowns

- **Evidenced premise.** The `toml` crate parses the manifest shape in §61
  without custom lexing. Verified: `openwarrant.toml` already round-trips
  through it in OW-WAR-0001's tests.
- **Blocking unknown.** Whether YAML frontmatter should be parsed by a YAML
  library or by a restricted hand-rolled reader. §80 lists "`serde_yaml` or a
  safer maintained YAML parser", which is an explicit admission that the obvious
  choice is unmaintained. §87.2 requires the parser to be safe against untrusted
  sources. This blocks the frontmatter half of the work and must be resolved by
  an implementation ADR before that half starts; manifest parsing is unaffected
  and proceeds.

## Constraints and Invariants

- **Fail closed on unknown required things.** An unknown required role, a
  missing required atom, and a duplicate ordinal are all errors (§16.4, §91.2
  tests 7–9). An unknown *optional namespaced* role is PRESERVED, not dropped
  (§16.4) — the two behaviours must not be collapsed into one.
- **Exact source preservation.** Atom bytes and their content digest are kept as
  authored (§62.2). The parser reads; it never rewrites a source atom.
- **Every diagnostic names a file and a rule.** A validator that reports "invalid
  manifest" costs the reader the debugging the tool was supposed to do.
