---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f34-92db-54b2dca5446d
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Five Warrants exist on disk as `manifest.toml` files and Markdown atoms, written
by hand. Nothing can read them. They are, at present, a directory of documents
that merely resemble a machine-checkable record.

## Desired Outcome

`openwarrant-core` parses a manifest and its atoms into typed values, and
refuses — with a diagnostic naming the file and the rule — every malformed
composition the SAS says must fail closed.

## Scope

Manifest parsing, atom frontmatter parsing, the typed role vocabulary of §16.1,
the authored/bound/generated jurisdiction tri-state of §13, the per-profile
required-role sets of §16.3, and `war new`.

## Non-goals

- No canonical IR and no digests. OW-WAR-0003.
- No rendering of any projection. OW-WAR-0004.
- No resolution of `ref =` bound atoms to a remote authority. A bound atom
  without an exact revision must FAIL authorization (§91.2 test 13), and
  producing that failure does not require the ability to resolve one.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-010` — authored atoms are directly editable sources. Complete.
- `WAR-SAS-RQ-011` — bound atoms are edited only through their owning authority.
  Partial: the class is represented and refused where it cannot be resolved; the
  owning-authority path arrives with federation.
- `WAR-SAS-RQ-012` — generated atoms and parents are not directly editable.
  Partial: the class is represented; enforcement over generated files is
  OW-WAR-0004.
- `WAR-SAS-RQ-013` — composition is typed, ordered, and deterministic. Complete.
- `WAR-SAS-RQ-015` — required atom omission fails closed. Complete.
