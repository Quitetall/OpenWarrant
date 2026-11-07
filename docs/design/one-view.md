# One record, one view (W1.1, first slice)

`t-3c16c` W1.1: `prime`, `status`, `check`, `next`, `context` and the TUI read all
work through one list, and an item's level is derived from its fields.

## How each reader gets its work today

| Reader | Source | Sees |
|---|---|---|
| `war view tickets` | `Store::load_all`, `ticket::Others::of` (`warrants::other_rows`) | tickets, directory Warrants, read-in-place. Not single-file Warrants |
| `war view prime` | `Store::load_all` | tickets only |
| `war status` (terse) | `Corpus::status`, `Store::load_all`, `console::board_with` | three stores read separately |
| `war status --verbose`, `--json` | `status::build` (corpus entries, `Repository::warrant_dirs`) | directory Warrants, with computed rungs |
| `war view next` | `next.rs`: `Corpus::tickets`, `status`, `frontier` | tickets plus stages and acts |
| `war check` | `ticket::check`, `format_doc::check`, `corpus` | each encoding by its own rules |
| `war view context` | `model::build`, `model::build_tickets` | records, not work items |
| TUI, web page | `ticket::board` over `Work`, `console::board` | tickets only, plus acts |
| `frontier`, `console::board` | `Corpus::frontier`: stages of unresolved Warrants | stages, not items |

No reader lists a single-file Warrant (`docs/warrants/<alias>.md`, FORMAT-1.0 §4.1):
`warrant_dirs` skips them on purpose and only `format_doc` and the model read them.

## Design

`crate::work` is the one list. `Work::read(repo)` never fails and returns every item
of every encoding; `Work::read_with(repo, store)` keeps a command's actor, and
`Work::tickets_of(store)` is the store alone for a reader that shows only tickets.
Each `Item` has `id`, `title`, `state`, `level`, `encoding` and `place`. Ticket
rendering still needs a checklist, so `work.tickets()` hands back the records: the
list wraps the stores and adds no new one.

`work::level(Fields)` is pure. `Fields` holds two facts, each `Option<bool>` so that
what could not be read stays unknown:

| Encoding | `trigger` | `signature` |
|---|---|---|
| Ticket, either form | a gating test, a KPI that decides, or a minimum of `observed` or more | a `signed` minimum, from a milestone, its type or the repository preset |
| Directory Warrant | profile selects `evidence` or `verification` | profile selects `authorization`, `resolution` or `acceptance` |
| Single-file Warrant | as a ticket, or its profile selects them | as a ticket, or its profile selects them |
| Read in place | no: these formats declare none | no |

A signature read as required gives Order whatever else is unread. Otherwise anything
unread gives `UNKNOWN`, never Ticket. A preset's floor counts, because that is what
`war done` enforces: a Ticket under `regulated` could not finish on the agent's word.
`promoted_to` is not a level fact: the promoted Warrant is its own item, until promote
becomes adding a field. No level is ever written, so signed records are untouched.

## What moved, and what stayed

`prime`, `view tickets` and the terse status's ticket, ready and level figures read
`Work`. The terse status's `Warrants: N, M resolved` stays on `status::build`: a list
row reports the phase the journal records, while the rung counts a resolution only if
it binds the contract as it compiles now. Swapping them could read higher than
`--verbose`, so that is a decision for a later slice, not a refactor.

`prime` shows the level only when it is not the default, and counts work outside the
ticket store on one line, since `war start` takes none of it and the overview must not
grow with the corpus.

## New work is one document

`war create` writes one document at every level: `--test` makes it a Warrant,
`--sign` an Order. `war plan promote` adds the same field to work that exists,
in place, with the same id. `promoted_to` stays readable on the records that hold
it and is written only by `war plan new --from`.

`war plan new` still writes a directory Warrant, and `war plan --apply` and
`war init --program` still reach it, because authorize, verify and resolve read
only that encoding. Flipping its default would hand a user a Warrant none of those
acts can reach, and would break every script that edits the atoms after it.
`--format 1.0` writes a document in `docs/warrants/`. Retiring the directory writer
needs the Appendix C acts over a single-file subject (`warrant_dirs` and
`load_warrant` underneath `sign`, `compile`, `status::build`), which is its own
piece of work.

`check <alias>`, `status <alias>`, `view show <alias>`, `view next` and `view
context` now find a single-file Warrant. `next` lists it, and work read in place,
under `elsewhere`, which is absent when there is none.

## An Order reaches its signer

`war done` on an Order ticks what an agent may, runs what decides it, and hands the
rest to a person. The handover is a journal event, `work.awaits_signature`, because
nothing else tells an Order just created from one its agent finished: with no work
items of its own, the two look the same. The queue reads that event and the document
together, so a level is still never stored, and an item added or reopened since takes
the Order out of the queue until the next `war done`.

`war done` exits 0. An agent's act succeeded and nothing is left for it, as when it
ticks one item of many, and a non-zero code would send a harness back to retry work
that passed. Closed is not what an exit code says in either case: `ticket_state` is
`awaiting_signature`, and `awaiting_signature` lists the targets.

A sign-off is not a `sign::Pending`. It is one item's own record (`ticket::acts::sign`)
with none of those acts' drafting or batching, so it joins the queue at
`console::review_rows_with`, `war sign --list` and `next`. `war sign --all` and
`--batch` leave each out and name its command, and `war sign <item>` signs it alone.

## Single-file Warrants and Orders in status and the frontier

`Work::documents` is every single-file item whose level is not Ticket, in
`docs/warrants/` or `docs/tickets/`. `war status --verbose`, `--json` (as
`single_file`) and `war plan frontier` list them beside the corpus projection, not in
it. `status::build` feeds the committed `CORPUS_STATUS` files, so a document in it
would make every `war create --test` a drift of `generated/`, and `status.rs` is pinned
besides. `Work::documents` follows `read_in_place` and the tick tracker, which stay out
of the projection for the same reason. The terse status already counts them by level.

The frontier's `documents` is set for the command alone. A reader that dispatches
(`console`, `graph`) sees stages only, because no document has a stage to dispatch.

## What remains of W1.1

- FORMAT wording for the levels, which waits for a signing sitting.
