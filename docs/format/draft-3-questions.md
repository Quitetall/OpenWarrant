# Questions on FORMAT-1.0 draft 3

Found while writing the conformance corpus (`conformance/format/`), its
independent Python modules and the draft-3 schemas from the text alone. Each
entry gives the section, the sentence, two readings and what the corpus does
until the editor decides. "No case" means that no case depends on the
answer. "Case" means that a case does depend on it: settling the question
the other way changes that case's expected file.

## Rules and profiles

### Q1. `context.span-unknown` has no rule text (13.2)

- **Sentence:** 13.2 lists `context.span-unknown` among the warnings.
- **Problem:** no section says when it is reported.
- **Readings:** (a) it is left over from an earlier draft and should be
  removed; (b) a sentence defining it is missing.
- **Corpus:** no case. `run.py` excuses it from coverage by name
  (`UNCOVERABLE`).

### Q2. Can profile data declare keys and sections in 1.0? (2 against 7.5 and 7.6)

- **Sentences:**
  - 2: "In 1.0 the keys and sections of the built-in profiles are the ones
    this document names (3.3, 4.1); declaring them in profile data is
    reserved for a later minor."
  - 7.5 defines `[frontmatter]` and `[sections]`.
  - 7.6: the built-in profiles are "defined by a profile file shipped with
    this document ..., which is normative."
- **Readings:** (a) 2 governs, and the 7.5 tables are reserved in 1.0;
  (b) 7.5 and 7.6 govern, and 2's sentence is stale.
- **Corpus:** (b). Cases 060, 061, 154 to 156.

### Q3. How does a repository file replace a built-in profile? (7.5)

- **Sentence:** "The built-in profiles of 7.6 need no file; a file of the
  same name replaces one only by `extends`."
- **Question:** must `profiles/ticket.toml` say `extends = "ticket"` (a
  self-reference)? Is a same-named file without `extends` `profile.invalid`,
  or is it ignored?
- **Readings:** (a) the file names itself in `extends`, and any other
  same-named file is `profile.invalid`; (b) such a file is ignored and the
  built-in profile stands.
- **Corpus:** (a). Cases 061 and 404.

### Q4. The field shapes are named but not defined (7.5)

- **Sentence:** 7.5 names the shapes `"text" | "line" | "int" | "id" |
  "ids" | "gate" | "date" | "enum"`.
- **Problem:** none of them is defined.
- **Corpus reading:**

  | shape | read as |
  |---|---|
  | `line` | no continuation |
  | `int` | an optionally signed run of decimal digits |
  | `id` | one target of 3.6 |
  | `ids` | targets joined by `, ` |
  | `gate` | `gate://<id>@<version>`, joined by `, ` |
  | `date` | `YYYY-MM-DD` |

  The other reading is any other definition the editor gives.
- **Corpus:** cases 152 (`int`), 153 (`enum`) and 154 (`int`). None
  depends on the edge cases.

### Q5. Fields of a type that declares none (7.5)

- **Sentence:** "A field neither the core nor the profile declares for that
  type is refused (`format.key-unknown`)."
- **Readings:**
  - (a) A literal reading: every field of a `milestone`, a `term` or a
    `policy` is refused, because nothing declares any.
  - (b) A type for which neither the core nor the profile declares any field
    accepts any field. Only a declared field table closes the set.
- **Corpus:** (b). Existing cases 030 and 120 use `note:` on a requirement
  in a `prd` document, and draft 2 read them that way.

### Q6. Do 4.5's requirement fields bind outside a specification? (4.5)

- **Sentence:** "A `requirement` or `constraint` has a REQUIRED `statement`
  ...".
- **Readings:** (a) this holds in every document, which would refuse
  existing cases 042 and 030 (requirements in `prd` documents without a
  `statement`); (b) it holds under the `spec` profile.
- **Corpus:** (b). The fields live in `schemas/format/1.0/profiles/spec.toml`.

### Q7. Are undotted record types closed? (7.2, 7.5)

- **Sentences:**
  - 7.2: "An undotted name neither this format nor the profile defines is
    refused."
  - 7.5: `[records] types` lists "the record types its documents may hold".
- **Question:** is a record type such as `policy`, `dependency`, `test` or
  `risk`, used by existing cases 052, 016 and 022 under `prd` and `ticket`,
  refused? Is `types` a closed set that narrows the core types?
- **Readings:** (a) refused unless the profile lists the type; (b) not
  refused. A type is closed only where the core says so.
- **Corpus:** (b). No undotted type is refused.

### Q8. The rule id for an unlisted section under `closed = true` (7.5)

- **Sentence:** "a document with a section neither lists is accepted
  unless `closed = true`."
- **Problem:** no rule id is named.
- **Readings:** (a) `format.key-unknown`, as an undotted name the profile
  does not define (7.2); (b) a new rule.
- **Corpus:** (a). No case.

### Q9. What else makes a profile file `profile.invalid`? (7.5, 7.6)

- **Sentence:** 7.5 lists four faults: an unknown key, a capability
  outside 7.6, a `refines` naming no fixed state, and a preset named like a
  built-in.
- **Not listed:**
  - `name` differing from the file stem (7.5 says it "equals the file
    stem");
  - a missing capability prerequisite (7.6: `checks` needs `claims`);
  - an `enum` shape without `values`;
  - an unknown shape;
  - an `extends` naming no profile.
- **Readings:** (a) each of these is `profile.invalid`; (b) only the four
  listed are.
- **Corpus:** the reader refuses all but the prerequisites. The cases use
  only the four listed faults (157 to 160).

### Q10. Delivery's "one obligation" cannot be written as profile data (4.1, 7.5, 7.6)

- **Sentence:** 7.6: `delivery` requires "Intent, Basis, Work order,
  Assurance; one obligation".
- **Problem:** profile data has no key for "at least one record of a type",
  so the normative `delivery.toml` cannot state it.
- **Readings:** (a) add a key (`[records] require = ["obligation"]`);
  (b) it stays a core rule of 4.1.
- **Corpus:** (b). `delivery.toml` says so in a comment.

### Q11. Built-in profiles whose requirements 7.6 does not give

- **Sentence:** 7.6 gives `release`, `incident`, `prd`, `architecture`
  and `test-plan` "their files' sections".
- **Problem:** the text names none of those sections. It also gives
  `roadmap` "phases as `phase` records" without saying whether a roadmap
  must have one.
- **Corpus:** the shipped files require no section and no record. The
  editor should supply the sections or say none are required.

## Specifications (4.5)

### Q12. A specification part that carries `version`

- **Sentence:** "Parts carry none."
- **Question:** is a part with `version` refused, and by which rule?
- **Readings:** (a) refused, by `format.key-unknown`; (b) read and ignored.
- **Corpus:** `version` is optional in `spec.toml`, so a part with one is
  accepted. No case.

### Q13. "A second file with profile `spec`"

- **Sentence:** "a second file with profile `spec` that is not `SPEC.md`
  and names no `part_of` is refused (`format.document-duplicate`)."
- **Questions:**
  - With no `SPEC.md`, is a lone `spec`-profile file without `part_of`
    "a second" one?
  - Must a part live under `spec/`, or may a `docs/records/` document with
    `part_of: doc:SPEC.md` be a part?
- **Corpus:** every `spec`-profile file other than `SPEC.md` without
  `part_of` is refused, wherever it is. Case 164 has `SPEC.md` present.

### Q14. "One sentence of at most 300 bytes" (`context.statement-shape`)

- **Problem:** "one sentence" has no mechanical definition. The text also
  does not say whether the warning is reported at the record's heading or
  at the `statement:` line.
- **Corpus:**
  - Only the 300-byte bound is checked.
  - It is reported at the heading.
  - Case 064 is over the byte bound.

### Q15. Where `context.section-ref` applies, and what counts

- **Sentences:**
  - 4.5: "A record SHOULD NOT cite another by a section number or `§`".
  - 10.1 says the same of every authored record, with no rule id.
- **Questions:**
  - Does the warning apply only to specification documents, or to every
    document?
  - What is "a section number"?
- **Corpus:**
  - Specification documents only.
  - A citation is `§` or `section <n>[.<n>]…`.
  - It is reported at the line where it occurs. Case 065 uses `§3.2`.

### Q16. Which records does "every record" of a specification count?

- **Sentence:** "A specification is `accepted` when every record is,
  `partially_accepted` when some are".
- **Questions:** are sections such as `S-purpose` counted? Are the parts'
  own `doc:` records? Is `doc:SPEC.md` itself?
- **Corpus:**
  - Every unit of `SPEC.md` and its parts, sections included.
  - The documents' own records are not counted.
  - Cases 405 and 406.
- **Corpus device:** the acts these cases give are verified without
  signatures, because the corpus carries no signature.

### Q17. The prefix registry against a migrated specification (Appendix D, 14)

- **Sentences:**
  - Appendix D: a dotted name whose prefix is "neither registered nor a
    reverse domain name" is `format.prefix-unregistered`.
  - 14: a migrated requirement carries `legacy.id`.
- **Problem:** `legacy` is neither registered nor a reverse domain name.
  The text also gives no test for "a reverse domain name".
- **Readings for the test:** (a) three or more labels (`com.example.risk`);
  (b) a first label that is a known top-level domain.
- **Corpus:** (a). Cases 062 and 063. `legacy` should be registered, or
  14 should use another name.

## Context (10)

### Q18. A ticked box without a journal event (11.1, 10.3)

- **Sentences:**
  - 11.1: an item is `done` when "its box is `[x]` and the journal backs it
    (11.4)".
  - 10.3: the compiler "MUST NOT read ... anything outside the working tree
    or any path the repository's version control ignores".
  - 10.4 says that only state-reading presets read the journals.
- **Questions:** in an `agent` or `human` packet, and in the index, is an
  `[x]` item `done`? Is it `done` for a tracker when no `work.item_done`
  exists?
- **Readings:** (a) the box alone decides when the journal is not read;
  (b) it is not `done`, and shows `open`. `war` shows (b).
- **Corpus:** the context cases that do not read state have no `[x]` item,
  so they do not depend on the answer. The tracker reads (b). No tracking
  case has an `[x]` without a `work.item_done`.

### Q19. Primary state: 10.2 against 11.1

- **Sentences:**
  - 10.2: the primary state is "the value of its state of kind `phase`,
    else `checklist`, else `disposition`, else `-`".
  - 11.1: "A state's `kind` in the model is `computed`, `recorded` ... or
    `declared`".
  - 11.1 gives another order: `superseded`; the phase; the item's states;
    the disposition; else `-`.
- **Problems:** the two disagree on kinds and on order. Under either, a
  `failing` check's primary state is `-`.
- **Corpus:** 11.1. Case 409 shows `failing` in `states.json`, not in a
  packet.

### Q20. `provenance` and `facet` (8)

- **Sentence:** `state = { record, kind, value, provenance, facet?,
  refines?, lapsed? }`.
- **Problem:** `provenance` is required and undefined; `facet` is
  undefined.
- **Corpus:** `states.json` compares `{record, kind, value, refines?,
  lapsed?}` only.
- **Also asked:** is `claimed` of kind `computed` (the claim store holds no
  event) or `recorded`? The corpus says `computed`.

### Q21. Does the walk follow the containment `part_of`? (10.3, 4.1, 4.4)

- **Sentences:**
  - Containment is "in the model but not in any `content_digest`".
  - 10.3 walks `out:part_of` ("its milestone, phase or parent").
- **Readings:** (a) it is walked, so a requirement reaches `doc:SPEC.md`,
  and through `in:part_of` from a `consequence` the consequences of the
  specification; (b) only authored `part_of` is walked.
- **Corpus:** (a). Cases 303 to 307. `war` agrees in 303.

### Q22. The document record's span and text (3.4, 8, 10.3)

- **Problem:** the document record is no unit, but packets give it in full
  (`[F]`) and need its revision.
- **Readings:** (a) its span is the frontmatter and the preamble, before
  the first unit; (b) the whole file.
- **Corpus:** (a). `war` agrees in 303.

### Q23. Is the preset named in every header? (10.3 against 10.4)

- **Sentences:**
  - 10.3's template: `· walk <name> ·`.
  - 10.4: "Without `--preset` the preset is `agent`. The packet header names
    the preset in place of `walk <name>`".
- **Readings:** (a) always (`preset agent (walk core)`); (b) only when
  `--preset` is given.
- **Corpus:** (a). Cases 303, 304, 310 and every preset case.

### Q24. Walk names and steps of the built-in presets (10.4, 7.5)

- **Problems:**
  - Presets with steps of their own (`agent.min`, `review`) have no walk
    name.
  - Profile data's preset keys have no `walk` key.
  - `review` ("the target, its obligations and what they evaluate") gives
    no steps.
- **Corpus:**
  - A walk is named after its preset.
  - `review` walks `in:evaluates` then `out:evaluates` (`out:evaluates` is
    not a core step).
  - Cases 305 and 307.

### Q25. `resume` has no rendering (10.4)

- **Sentence:** "the target's items with their state, holder and lease,
  done items with actor and date, then the next three items `next` would
  offer (11.5); depth 1; reads state".
- **Problems:** no rendering is given, nor where holder, lease, actor and
  date go. It is not said whether "next three" is over the repository or
  the target.
- **Corpus:**
  - The roots are the work document's record and its items, and the walk
    is `core` to depth 1.
  - Under each item a state line follows its `>` line:
    `state <primary>[ · holder <actor> · lease <until>][ · by <actor> ·
    <date> · <backed level>]`.
  - A `## Next` section lists up to three of the target's candidates.
  - Case 308 depends on all of this.

### Q26. Which bytes does the header's journal digest cover? (10.4)

- **Sentence:** "Its header records both: `· journal sha256:<12 hex> · at
  <time>`".
- **Readings:** (a) every journal of the repository, in path order;
  (b) the target's journal; (c) the journals and the claim store.
- **Corpus:** (a). Cases 307 and 308.

### Q27. What `agent.min` leaves out (10.4)

- **Sentence:** "leaves out `rationale` fields".
- **Problem:** a record's text is "the unit's exact span".
- **Readings:** (a) the field's lines (and continuations) are cut from the
  rendered text, and the `>` line still names the source revision; (b) a
  record with a `rationale` is given by summary.
- **Corpus:** (a). Case 305.

### Q28. The index: repository name and layout (10.2)

- **Problems:**
  - `# <repository name>` is not defined.
  - The text does not say whether a blank line separates groups.
  - "`> ` line is the work document's summary, or `-`", but other
    documents have groups too.
- **Corpus:**
  - The root directory's name. The runners name it `repo`.
  - A blank line before each `## <source>`.
  - A document's own `summary` for any document.
  - Case 309.

### Q29. An item inside a record's body (10.3)

- **Sentence:** "An item whose section is given in full is marked `[F]`
  and has no entry of its own".
- **Problem:** an item can sit in a record's body (3.8). Its model
  `section` is then the last section before the record, not the unit that
  holds its line.
- **Corpus:** the unit that holds the line. No case.

### Q30. Package markers (10.5)

- **Sentence:** the example marker names
  `OW-WAR-0146.md:7`, a file name, where packets elsewhere use
  repository-relative paths.
- **Questions:**
  - Is the document record (frontmatter, not a unit) wrapped in markers?
  - Is the `> <id> ·` line kept beside the markers?
- **Corpus:** a repository-relative path; the document record is not
  wrapped; the `>` line is kept. Cases 306 and 501 to 508.

### Q31. The date a box toggled in a package gets (10.5)

- **Sentence:** "`[ ]` to `[x]` records `done` ... and the writer then
  rewrites the line as 3.8 shows."
- **Problem:** the date of the done suffix is not given (a clock?).
- **Corpus:** the tracking time `at`. Case 503.

## Work tracking (11)

### Q32. A work document with no items is `draft`, so never a candidate (11.1, 11.5)

- **Sentences:**
  - 11.1: `draft` is "it has no item and no milestone".
  - 11.5: a candidate is "each work document with no items that is open and
    unclaimed".
- **Problem:** without milestones, such a document is `draft`, never
  `open`, so it is never a candidate.
- **Readings:** (a) as written; (b) 11.5 means "not done and unclaimed".
- **Corpus:** (a). No case.

### Q33. `item` and `target` in event payloads (11.2)

- **Sentence:** `work.item_done` carries `{item, target, on, ...}`, and
  `work.claimed` carries `{claim, target, ...}`.
- **Problem:** the meaning of each is not given.
- **Corpus:** `target` is the global id (`t-0d61d/i-4d11`) and `item` the
  local item id. A reader matches on `target`, else on the document and
  `item`.

### Q34. `work.item_done` against `work.item_reopened` (11.2)

- **Sentence:** "where two events conflict the rule says which wins (11.3,
  11.4)".
- **Problem:** neither section says which of these two wins.
- **Corpus:** the later `occurred_at`. No case.

### Q35. What makes a journal line malformed (11.2)

- **Questions:**
  - Is a line that parses but is not RFC 8785 bytes malformed?
  - Is an event whose `idempotency_key` does not match its content
    malformed?
  - Is `v: 3` malformed, or `UNKNOWN` (1: "a version was newer than the
    tool")?
- **Corpus:** a line is malformed when it does not parse, lacks a member,
  or has a `v` other than 1 or 2. Case 425 is invalid JSON.

### Q36. "Committed lines" (11.2)

- **Sentence:** "a tool MUST refuse a journal whose committed lines were
  changed or removed".
- **Questions:**
  - Committed in `HEAD`?
  - Is a line inserted between committed lines (as a union merge can
    produce) a change?
- **Corpus:** `HEAD`; every committed line must remain, in order (a
  subsequence). Cases 427 and 428.

### Q37. Where the claim settings live (11.3, 11.5)

- **Problem:** `claim_lease_minutes`, `claim_ttl_minutes` and
  `prior_secs` have no stated location.
- **Corpus:** the first two in `openwarrant.toml [claims]` (where 11.3
  puts `remote`); `prior_secs` in `[next]`. No case changes them.

### Q38. Claims that overlap (11.3)

- **Sentence:** "A claim on a work document covers all its items."
- **Problem:** whether claiming the document while another actor holds one
  of its items is refused is not said.
- **Corpus:** it is refused. No case; case 418 covers the converse.

### Q39. The age of a claim for `--steal` (11.3)

- **Sentence:** "older than `claim_ttl_minutes`".
- **Problem:** measured from what?
- **Corpus:** from `since`. Cases 420 to 422.

### Q40. The order of `next` (11.5)

- **Questions:**
  - Does an item's own `due` and `priority` come from its document? The
    item line has none.
  - Does a document's `depends_on` block its items?
  - What is the median of an even count?
  - Where does a document without `created_at` sort?
  - Should "a work document `part_of` another counts as an item of it"
    change the parent's phase?
- **Corpus:**
  - `due` and `priority` come from the document.
  - It does not block, but its waiting counts for `due` and remaining path.
  - The mean of the middle two.
  - Last.
  - Yes.
  - Cases 411 to 414 avoid the last three.

### Q41. A done item without a level suffix and with no journal event (11.4)

- **Sentence:** "An unbacked level reads down to the highest backed one".
- **Problem:** `claimed` is the lowest level, so what does an unbacked
  `claimed` tick read down to?
- **Corpus:** not `done` (Q18). No `work.tick-unbacked` is reported for
  it. No case.

## Checks (12)

### Q42. The verdict's spelling, and an item's revision (12, 11.2)

- **Sentences:**
  - `check.run` carries `{check, run, verdict, revision, ...}`.
  - 12: "A pass on an older revision of the check, or of an item, ticks
    nothing."
- **Problems:**
  - The verdict's values are not given.
  - The payload carries no item revision, so "an older revision of an
    item" cannot be told.
- **Corpus:** `PASS`, `FAIL` and `UNKNOWN`, as in section 1. Only the
  check's revision is compared. Cases 208 and 408 to 410.

### Q43. `reopen: true` (12)

- **Sentence:** the trigger table: "is reopened when `reopen: true`".
- **Problem:** the field table does not list `reopen`.
- **Corpus:** read as an OPTIONAL `true`/`false` field. No case.

### Q44. A kpi's `target` and `direction` (12)

- **Sentence:** "it has `direction` (`min` or `max`), an OPTIONAL `target`,
  and `mode`".
- **Questions:** is `direction` REQUIRED? Is `target` an integer or any
  number?
- **Corpus:** `direction` is required (case 153 refuses a bad value, not a
  missing one); `target` is any line. In `check.json`, `target` is a
  decimal number.

### Q45. "Last recorded run" (11.1, 12)

- **Sentence:** `failing` holds when "its last recorded run failed".
- **Questions:** last by `occurred_at`? Over all revisions of the check,
  or only the current one?
- **Corpus:** by `occurred_at`, over all revisions. Case 409.

## The corpus format (13.1)

### Q46. The tracking and apply case layout

- **Sentence:** 13.1 lists `journal/**, claims/**, at` and `package.md`.
- **Not fixed by the text:**
  - how the operation is named;
  - where `journal/**` files go;
  - how a committed journal is given (for `work.journal-rewritten`);
  - what a claim file's modification time is (version control does not
    carry it);
  - the form of a tracking refusal line;
  - how acceptance acts are given without signatures.
- **Corpus:** `conformance/format/README.md` fixes each.

### Q47. A lowercase line in an attribute block (3.5 against 3.6; not new in draft 3)

- **Sentences:**
  - 3.5: "A line whose first word is a core kind (3.6) or a dotted name
    followed by a space is a relation line. Any other line is refused
    (`format.attribute`)."
  - 3.6: "Any other kind is refused (`format.relation-kind`)."
- **Problem:** existing case 123 refuses `blocks REQ-pr1` as
  `format.relation-kind`, which needs `blocks` to be a relation line. By
  3.5 it would be `format.attribute`.
- **Corpus:** unchanged. Case 507 uses a line that is
  `format.attribute` under both readings.
