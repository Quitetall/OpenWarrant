---
schema: oh.war/atom/v1
adr_uuid: 01a1b785-411b-701e-8caf-e14ef1903c55
local_alias: OW-ADR-0035
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
---

# ADR OW-0035: Format 1.0 revision 4: masters in docs/work, hash ids, one ladder

## Status

Accepted 2026-11-18 by the repository owner, as editor and decider, after the 7-day
review `docs/format/GOVERNANCE.md` requires. The owner approved these amendments on
2026-11-07 as the direction for FORMAT-1.0 revision 4; a performer drafted the text and
this record. FORMAT-1.0 as a whole stays a draft.

## Context

`docs/design/one-format.md` (#407) built one loader for every layout and listed
normative text that waited for the editor:

- §5 had no hash ids, no fixed ids and no rule for slugs or human numbers;
- §4.1 and §4.3 named only `docs/warrants/` and `docs/tickets/`;
- C.3 defines a subject digest, while `authorize` and `resolve` responses carry the
  contract digest, a different value;
- §4.1 had no wording for a document split across files.

Separately, work and documents had no common ranking. `war` already derives a work
order's level (Ticket, Warrant, Order) from two facts, a trigger and a required
signature. Documents (specifications, ADRs, designs, guides) had no equivalent, two
repositories had no named way to coordinate, and nothing said which of two
disagreeing items governs or which a context compiler gives first.

## Decision

Revision 4 of FORMAT-1.0:

1. **§5 ids.** New work orders are `w-<hex>`, new documents `d-<hex>`, the head of the
   SHA-256 of a fresh UUIDv7. `spec` and `roadmap` are the only fixed ids. The slug is
   in the file name only. A human number is a display ordinal from the main-only index
   `docs/generated/numbers.tsv`, never in a file name.
2. **§4.1, §4.3 layout.** Every new item is a master, `docs/work/<id>/<id>-<slug>.md`,
   always a directory, its state beside it. `docs/` otherwise holds views generated
   only on the main line. Older places stay readable and nothing moves.
3. **C.3.** `authorize` and `resolve` responses carry the contract digest, which binds
   every byte of the document; equal contract digests give equal subject digests. A
   reader requires each digest a response carries to match, so a response that carries
   the contract digest alone, as every earlier one does, still verifies.
4. **§4.1 split masters.** A master may keep its sections in further files only when
   joining them gives exactly the bytes a signature covers.
5. **§4.6 the ladder.** Two kinds, shown by the id: work orders and documents. One
   four-tier ladder, derived and never stored: Spec / Roadmap (the singletons), ADR /
   Order (a required signature), Design / Warrant (a trigger), Guide / Ticket (nothing
   extra). Documents declare `type:` (`spec`, `adr`, `design`, `guide`, `interface`),
   checked against the derived tier (`format.type-tier`); a `kind:` that disagrees with
   the id is refused (`format.kind-mismatch`). An interface is a Design-tier document
   and the way two repositories coordinate. Every work order belongs to the Roadmap. A
   signature on the Spec or the Roadmap covers its own text, never the items under it.
   Higher tiers take precedence and are ranked first by the context compiler (§10.3).
   Two built-in profiles, `design` and `guide`, give the lower document tiers a
   profile.
6. **Answers.** Who may answer a question follows the ladder: on a Ticket or a
   Warrant, any actor but the asker; on an Order, or a question that asks for a person,
   a person.
7. **§10.3, §10.4 the context default.** Approved by the owner 2026-11-14 after the
   alpha.7 field report: `war context` filled its 16000 bytes with completed sibling
   phases, rejected options and specification frontmatter. Without `--preset`, a work
   document or an item of one is compiled under `agent.min`: depth 2 over
   `out:implements`, `in:constrains` and `in:evaluates`, and the parent's intent at
   distance 1 (step `parent-intent`, walked no further), budget 16000. Any other
   target keeps `agent`; `--full` is `agent` with no budget.

### Options considered

- **Kind from the profile's `form`.** Rejected: the Roadmap is a work order while the
  `roadmap` profile's form is `document`, and a kind read from two places can disagree.
  The id already carries the kind.
- **A stored tier.** Rejected for the reason `war`'s levels are derived: a stored tier
  can disagree with the fields that make it one.
- **Tier ahead of distance in context order.** Rejected: it would give a distant Spec
  record before the target's own text. Tier breaks ties within a distance.
- **Every document type a profile.** Rejected: `adr` and `spec` already exist, and an
  interface differs from a design in purpose, not in rules.

## Consequences

- Two branches never mint the same id, and renaming a slug moves no digest.
- `war check` refuses a second Spec or Roadmap, a `type` that is not the derived
  tier's, and a `kind` that disagrees with the id. Conformance cases 069-071 pass and
  174-177 refuse, under `war` and the Python reader.
- The repository's own `docs/sas/` and `docs/roadmap/` stay where they are; moving them
  is a migration tool's job, and nothing signed moves.
- Splitting a master (§4.1) is defined but not built: no tool writes or joins parts
  yet, and a reader that meets `<id>.<NN>.md` today ignores it.
- Conformance cases 311 and 312 hold the context default under `war` and
  `context.py`: a work document gets `agent.min` with its parent's intent and no
  sibling or rejected option; a requirement still gets `agent`'s walk.
- The `--needs-human` question flag and the enforcement of who may answer are built
  separately (agent/a8-questions); this revision gives the rule.
