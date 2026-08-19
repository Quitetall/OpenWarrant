---
schema: oh.war/atom/v1
adr_uuid: 01a01bde-3277-7196-9d60-40e98711eeb8
local_alias: OW-ADR-0003
role: adr
jurisdiction: authored
order: 30
classification: internal
status: accepted
decided: 2026-08-19
governs:
  - "war://OW-WAR-0007"
---

# ADR OW-0003: Extend the restricted reader for structured atoms rather than adopt a YAML library

## Status

`Accepted 2026-08-19`

Extends OW-ADR-0002 to machine-dense atoms. Required by OW-WAR-0007's OBL-000,
which cannot resolve until this decision exists.

## Context

OW-ADR-0002 rejected every Rust YAML library for atom **frontmatter**, on the
grounds that frontmatter is six flat keys and does not justify YAML 1.2's attack
surface. That reasoning was explicitly scoped to frontmatter.

§62.1 permits machine-dense atoms — milestone graphs in particular — to use YAML
or canonical JSON, and a milestone graph is genuinely not six flat keys. So the
question reopens on different facts, and the earlier ADR does not settle it.

**What the structured atoms actually contain**, measured across the eight
milestone atoms in this repository rather than assumed:

- 10 distinct keys: `schema`, `milestones`, `stages`, `id`, `title`,
  `depends_on`, `stage_refs`, `obligation_refs`, `executor_kind`,
  `responsibility_tier`.
- Maximum indentation 4 columns — two levels: a top-level mapping, whose values
  are block sequences, whose items are mappings of scalars.
- Two container shapes: block sequences of mappings, and flow sequences of
  scalars (`["STAGE-001"]`).
- No anchors, aliases, tags, nested flow collections, block scalars, or
  multi-document streams.

The ecosystem position is unchanged since OW-ADR-0002 and was re-checked today:

| crate | version | licence | last updated |
|---|---|---|---|
| `saphyr` | **0.0.12** | MIT OR Apache-2.0 | 2026-08-18 |
| `serde_norway` | 0.9.42 | MIT OR Apache-2.0 | 2024-12-21 |
| `serde_yaml_ng` | 0.10.0 | MIT | 2024-05-26 |

## Decision

**Extend the restricted reader** to cover a second bounded shape: a top-level
mapping whose values may be block sequences of flat mappings, plus flow
sequences of plain scalars. No YAML dependency.

The subset is defined by what the `oh.war/milestones/v1` schema needs and is
enforced by refusal, exactly as the frontmatter subset is.

## Rationale

**The consistency argument is the strongest one.** If a six-key header does not
justify YAML 1.2's surface, a ten-key two-level document does not either. Adopting
a YAML parser one Warrant after rejecting one would mean OW-ADR-0002's reasoning
was about convenience rather than about attack surface — and the next structured
atom would face the same question with the precedent already broken.

**We own the schema.** `oh.war/milestones/v1` is defined by this project. Unlike
frontmatter, where an author might reasonably reach for YAML they know, a
milestone atom is written against a schema we publish. Keeping it inside a
documented subset costs authors nothing they were going to use.

**The hazards OW-ADR-0002 named all still apply, and one gets worse.** Anchors
and aliases remain an expansion-DoS vector under §87.2. Implicit typing remains a
correctness hazard — and it bites harder here: `responsibility_tier: NO` would
become boolean `false` under YAML's Norway rule, and an identifier like `id: Y`
is a bare `true`. Milestone and stage identifiers are exactly the short
uppercase tokens that implicit typing mangles.

**It is small enough to test exhaustively.** The extension is one additional
container shape over a reader that already parses scalars, quoting, comments, and
block sequences of scalars.

## Alternatives Considered

- **`saphyr` 0.0.12** — the honest runner-up again, and stronger than last time:
  actively maintained, correctly licensed, YAML 1.2 compliant. Rejected on the
  same 0.0.x API-instability ground plus the consistency argument above. Worth
  revisiting at 1.0, and revisiting is cheap because parsing sits behind one
  module.
- **`serde_norway` / `serde_yaml_ng`** — rejected. Both are unmaintained forks of
  a deprecated crate; nothing has changed since OW-ADR-0002.
- **Switch structured atoms to canonical JSON**, which §62.1 explicitly permits
  and for which `serde_json` is already a dependency. Genuinely tempting and the
  cheapest option in code. Rejected because JSON has no comments, and the
  milestone atoms in this repository carry explanatory comments that are part of
  why they are readable. Trading author-facing readability for implementer
  convenience is the wrong direction for a document format.
- **Switch to TOML**, already a dependency with no implicit-typing hazard.
  Rejected: §62.1 permits YAML or canonical JSON, not TOML. Deviating from the
  specification to save implementation effort is precisely what a conformance
  claim must not do.

## Consequences

**Good.** No new dependency. The Apache-2.0 licence graph stays clean. The
expansion-DoS surface stays closed. Implicit typing cannot mangle an identifier.
The reader remains small enough that its whole grammar is testable.

**Bad.** A second bounded shape is more parser to maintain, and every future
structured atom role must fit the subset or force this decision open again. An
author who writes valid YAML outside the subset gets an error, and that error
must say the subset is deliberate — otherwise it reads as a broken parser. This
is a real ongoing tax, accepted knowingly.

**Unchanged.** OW-ADR-0002 stands. Frontmatter keeps its narrower subset; this
adds a second, wider one for `.yaml` atoms, and the two are separate grammars
rather than one permissive union.

## Validation

Watch for: a structured atom role that genuinely needs nesting the subset does
not cover, which reopens this; authors hitting the limit more than once, which
means the subset is too narrow; `saphyr` reaching 1.0, which reopens the
dependency option on better terms; and any temptation to widen the frontmatter
subset to match this one, which would undo OW-ADR-0002 by accident.
