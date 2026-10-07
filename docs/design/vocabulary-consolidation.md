# One vocabulary for RC.2

Status: discussion, 2026-09-14. Written against `feat/battery-split` at `f22ef2f`
with the uncommitted RC.2 source set in the working tree. This proposes a single
canonical term set and says what adopting it costs. It changes no accepted record,
and nothing here is a decision until the owner adopts it through §18's process.

## Release designation, first, because two documents disagree

The owner has redesignated the releases. Recorded here so no later reader has to
reconstruct it:

- There is **no SAS 1.1.0**. That version line does not exist.
- The target is **SAS 1.0.0 Stable**.
- What is accepted today is **1.0.0-rc.1**. Its acceptance record literally says
  `1.0.0` and binds `sha256:b7105f52…`; those bytes and that signature stand
  unchanged. The rc.1 designation describes the release, not the record.
- The current candidate is **1.0.0-rc.2**, unaccepted.

Three authorized Warrants name the version line that no longer exists:
**OW-WAR-0071** ("SAS 1.1.0: the batch act, and the dashboard as a rendering"),
**OW-WAR-0072** (`war sign --batch`), **OW-WAR-0073** (`war tui`), together with
`OW-ADR-0019` and `OW-ADR-0020`. All three carry the owner's signature from
2026-09-13. RC.2 §18 independently records ADR-0019 as "a proposed 1.1
batch/rendering change, not the release target".

An authorized Warrant is not edited to fix this. The honest routes are: an
amendment re-pinning each to the RC.2 line and re-authorization, or supersession
by successor Warrants written against RC.2. Both are human acts. Until one
happens, the corpus contains signed contracts naming a release that was
withdrawn — which is a true fact about the history and should stay visible, not
be quietly retitled.

## The problem this document solves

Two complete vocabularies are live in one repository:

- **Legacy (rc.1).** `CONTEXT.md`'s glossary (27 terms), `docs/DEFINITIONS.md`,
  `AGENTS.md`, the OpenWarrant skill, 40 `war` subcommands, 305 plants, and every
  signed record: *atom, dispatch, obligation, gate, receipt, pin, the wall,
  correction, resolution, frontier, battery, plant, drafter, proposal.*
- **RC.2.** SAS §3's term table and the format contract F1–F9: *source document,
  source unit, snapshot, contract, master context, projection/packet, pointer,
  dependency, conflict, blob, manifest, entry, brief, binding/background, record,
  evidence, verification, acceptance, assurance mark, workflow.*

They are not two dialects of one language. Some pairs are the same idea renamed,
some are different ideas wearing similar names, and at least one word — 
*projection* — means two different things in the two sets. A reader who does not
know which edition a sentence belongs to cannot tell which.

## The rule for deciding which name survives

1. **A word that a signed record binds keeps its meaning.** Renaming `atom`,
   `receipt` or `resolution` inside existing records would restate history. Those
   words stay, scoped to the legacy adapter and the records that use them.
2. **A word the standard owns takes the RC.2 name in all new text.** RC.2 §2 says
   OpenWarrant owns structure and field meanings; those are the words that travel
   to other tools, so they win in the specification, the format contract, new
   code, and new documents.
3. **A word for something only the reference tool does stays a tool word,**
   marked as such. `frontier`, `battery`, `plant`, `drafter` describe this
   repository's workflow, not the standard. RC.2 §2 leaves workflows to tools.
4. **No term means two things.** Where the sets collide, one of them is renamed,
   and the loser's new name is stated here rather than left to context.
5. **A merge must be argued, not assumed.** Two words that look like synonyms and
   are not (below) keep both names and get an explicit sentence about the
   difference.

## Canonical terms

`legacy-only` means: correct when reading rc.1 records or the legacy adapter,
never in new RC.2 text. `both` means the word is unchanged across editions.

| Legacy term | RC.2 canonical | Status | Note |
| --- | --- | --- | --- |
| Warrant | Warrant | both | Unchanged. One reviewable outcome (RC.2 §5). |
| SAS | SAS | both | RC.2 makes it a document `kind`, not a separate object class. |
| ADR | ADR | both | Same, now a `kind` with required `context`/`decision`/`consequences` units. |
| Atom | — | legacy-only | **Not a rename of unit.** An atom is a whole authored file; a unit is a marked span inside one file. Legacy import may map atoms to units (RC.2 §18), which is a conversion, not a synonym. |
| — | Source unit | new | The addressable span, `binding` or `background` (F2). |
| Contract Revision | Contract | renamed | RC.2 §5 and F9: the exact outcome/scope/constraints/expectations payload that approval binds. "Revision" now names the author's `revision` integer, so keeping both would collide. |
| Pin | Snapshot | renamed | Captured bytes + revision + locator + digest (RC.2 §3). A snapshot records what was there; it does not freeze the path. |
| The wall | — | retired | RC.2 §13 removes path freezing. Successor work changes a delivered file; the old bytes are preserved by lineage. Keep the word only when describing why rc.1 behaved as it did. |
| Correction | Correction | narrowed | RC.2 §13 keeps it for **correcting an assertion in a record**, an attributable act that does not overwrite old evidence. It is no longer how a delivered file changes. |
| — | Successor work | new | The new version of a delivered file under a new Warrant with explicit lineage (RC.2 §13, RQ-036). This is what replaces most of rc.1's corrections. |
| Authorization | Permission record / approval of a contract | split | RC.2 §11 separates execution permission, policy disposition, governance adoption, human acceptance, merge, deployment. "Authorization" bundled several of these. |
| Resolution | Human acceptance (+ qualification) | split | rc.1's single signed resolution is RC.2's `human-acceptance` record, with the assurance mark as a separate `qualification` record (F8). A resolution therefore has no single RC.2 successor; say which act is meant. |
| Obligation | Obligation | both | F8 verification payload keeps `obligations` with dispositions established / not-established / refuted. |
| §106 requirement | Requirement | both | Still the SAS's word, distinct from obligation. |
| — | Expectation | new | F9's normalized `{id, scope, check_digest}` list inside the contract payload: what approval binds and what a qualifying verification must cover. Neighbour of obligation, not a synonym: an expectation is the frozen check identity, an obligation is the acceptance condition judged against it. |
| Gate | Check definition | renamed | F8 observations carry `check_id`/`check_digest`. `gate://` URIs remain legacy-only. |
| Receipt | Observation record | renamed | F8 `observation` with `execution` and `verdict`. Existing signed receipts keep their name and bytes (RQ-085). |
| Verification | Verification | both | Same meaning; RC.2 adds `isolation_refs` and the explicit performer ≠ verifier field. |
| Blind verifier | Independent verifier | renamed | RC.2 §12(4) states the property (different context and workspace) instead of the metaphor. |
| Dispatch | Packet | renamed | Both are the compiled context for one unit of work. RC.2's packet is a directory with `ENTRY.md`, `packet.json`, `manifest.json`, blobs, and is readable offline; the rc.1 Dispatch is a JSON record that assumed the repository. Legacy Dispatch stays under §18's adapter. |
| Submission | — | legacy-only | RC.2 leaves the return trip to Phase 3 workflow; no RC.2 term yet. Do not coin one here. |
| Projection (generated file) | Generated view | renamed | **This is the collision.** RC.2 uses *projection* for a role/stage view of context (§3). The drift-checked corpus files become "generated views", which is already RC.2's own wording in RQ-012/RQ-075. |
| — | Projection (task view) | new | The role/stage selection delivered as a packet. |
| Attestation | Attestation | both | RQ-085 preserves existing signed acts and their subjects. |
| Blocking unknown | Blocker / unresolved required input | renamed | RC.2 §5 and F6 `readiness-blocked` speak of blockers and stopping conditions. |
| Frontier | — | tool word | This repository's queue; not a standard term. |
| Battery, Plant | — | tool words | Conformance apparatus. RC.2's equivalent is the T-case matrix, which is a plan, not a rename. |
| Drafter, Proposal | — | tool words | Retained for the rc.1 `war plan` seam. RC.2 RQ-071/RQ-072 put agent-assisted proposals in Phase 3. |
| Master context | Master context | new | Generated assembly of a captured source set (RC.2 §6). rc.1 had no equivalent object. |
| — | Pointer / dependency / conflict | new | F3's three header tables. rc.1's context selectors are their ancestors, not their equals: a pointer has an ID, a condition and dependency edges. |
| — | Assurance mark | new | The optional qualification (RC.2 §12). rc.1 had no separate mark; resolution carried everything. |

## Pairs that must not be merged

- **Atom / source unit** — file versus span. A legacy Warrant's five atoms become
  many units; a unit ID is not an atom role.
- **Correction / successor work** — an assertion in a record versus a new version
  of a file. Merging them is precisely the rc.1 behaviour RQ-036 withdraws, and it
  is what produced four corrections for one two-line fix on 2026-09-13.
- **Obligation / expectation / requirement** — acceptance condition, frozen check
  identity, program-level rule. Three words, three objects.
- **Resolution / acceptance / qualification** — one rc.1 act, three RC.2 records.
  A sentence that says "resolved" in RC.2 text is ambiguous and should be rejected
  in review.
- **Projection (generated view) / projection (task view)** — after this document,
  only the second meaning is RC.2's.

## Where each vocabulary is authoritative

| Surface | Vocabulary | Why |
| --- | --- | --- |
| RC.2 SAS, format contract, build scope, examples | RC.2 | The candidate standard. |
| New code under Phase 1 (`document.rs`, `condition.rs`, `packet.rs`, …) | RC.2 | New types, no legacy bytes. |
| Existing records, signed acts, `oh.war/*/v1` schemas | legacy | Renaming restates history (RQ-085, §18). |
| Current `war` CLI, plants, skill, `AGENTS.md` | legacy | Until Phase 2 replaces the command surface. |
| `CONTEXT.md`, `docs/DEFINITIONS.md`, `docs/SKILLS.md` | legacy, banner required | They describe rc.1 and are loaded into agent context every run. |

The banner is the cheap half of this: one line at the top of each legacy glossary
saying which edition it defines and pointing at the RC.2 term table. Without it,
an agent reading `CONTEXT.md` in a Phase 1 packet will use rc.1 words for RC.2
objects, which is the failure this document exists to prevent.

## What adopting this costs, exactly

Measured against the working tree, not estimated:

- `CONTEXT.md` — pinned by **OW-WAR-0068 D-001, authorized and unresolved**. The
  pin may be rewritten on the final bytes; no signature needed.
- `docs/SKILLS.md` — pinned by **OW-WAR-0068 D-009**, same situation.
- `docs/DEFINITIONS.md` — pinned by **OW-WAR-0062 D-002, resolved**. Behind the
  wall. Changing it costs a `war correct` with the owner's signature, or it waits
  for the RC.2 migration that retires the wall.
- `AGENTS.md` and `.claude/skills/openwarrant/SKILL.md` — unpinned; free to edit,
  and `AGENTS.md` already has an uncommitted drift that fails the template test.
- The RC.2 source set — adding a normative glossary means a new file plus a
  `source-set.json` entry, which changes the candidate's exact bytes. If RC.2 is
  to carry the glossary, it should land before the owner accepts the source set,
  not after.

## Proposal

1. Fold the "Canonical terms" table into the RC.2 source set as a normative
   glossary unit — either a new `glossary.md` listed in `source-set.json`, or an
   expansion of SAS §3's term table. §3 is the smaller change and keeps one
   document authoritative for meaning.
2. Add the edition banner to `CONTEXT.md`, `docs/SKILLS.md`, `AGENTS.md` and the
   skill. Leave `docs/DEFINITIONS.md` alone until its correction or the migration.
3. Adopt "generated view" for the drift-checked corpus files in all new text, and
   reserve "projection" for task views.
4. Decide OW-WAR-0071/0072/0073: amend and re-authorize against the RC.2 line, or
   supersede them. They are signed; this is the owner's act either way.

## Open questions for the owner

- Does the glossary belong **inside** the RC.2 normative set (binding on any tool
  claiming conformance) or beside it (guidance)? Binding is stronger and makes a
  term collision a conformance failure rather than a style note.
- rc.1 `resolution` has no single RC.2 successor. When the legacy adapter reads a
  resolved Warrant, does it report `human-acceptance`, `qualification`, or both
  with a note that rc.1 did not separate them? The honest answer is the third, and
  it should be written down before F10 is implemented.
- Should `war` keep the legacy command names through Phase 2 (RC.2 build scope
  reserves `war document`/`war context` in a new namespace) or alias them? An
  alias that maps `war compile` onto RC.2 semantics would silently change what an
  existing script means.
