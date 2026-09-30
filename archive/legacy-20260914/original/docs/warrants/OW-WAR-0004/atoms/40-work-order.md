---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-75b4-9586-0aae240f38bc
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. The `full_warrant` Markdown projection, following §103's section order.
2. `WAR.json` — the canonical JSON projection.
3. The §17.1 generated header on every generated file.
4. `war compile <alias>` writing into `generated/`.
5. `war check --generated` comparing committed views against fresh compilation.
6. A planted-drift test: mutate one byte of a committed parent, observe refusal.

## Frozen Surfaces

- The generated header's field set. It is what a reader uses to find the sources
  behind a document, and what a future importer keys on.
- The `generated/` directory layout of §59.

## Premade Instructions

- Compilation must be a pure function of the Basis. No wall-clock time, no
  absolute paths, no environment-dependent ordering in generated output. Any of
  those makes every recompilation look like drift.
- Omit inapplicable optional roles; do not render empty headings (§16.1).
- Refuse direct parent edits rather than attempting to map them back. §17.4
  permits the minimal CLI to refuse, and a wrong mapping silently edits the
  wrong atom.
- The drift check compares bytes, not a rendered diff.

## Resources and Capabilities

Repository-local filesystem read and write, restricted to `generated/` for
writes. No network. No secrets.

## Autonomy and Escalation

Tier T2, except the generated header's field set, which is T1 — it is the
provenance record every downstream reader depends on.

## Rollback

Revert, and delete `generated/`. Source atoms are untouched by anything in this
Warrant; a rollback loses only projections, which are by definition
reproducible.
