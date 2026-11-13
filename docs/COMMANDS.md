# The command surface

`war --help` lists six daily commands, then five advanced groups. `war sign`
stays for humans, outside the agent loop. Earlier spellings remain compatible
and keep their report envelope's command name.

| Daily command | Purpose |
| --- | --- |
| `war init` | Set up a repository |
| `war create "Outcome"` | Create one Markdown Warrant with a stable ID; every worktree sees it at once. A likely duplicate of open work is refused by name (`--force` creates anyway) |
| `war start` | Claim ready work and receive its context |
| `war done --note "Result"` | Record completion under the declared conditions |
| `war status` | Read progress |
| `war check` | Check record integrity; this does not run all acceptance checks |

Bare `war` opens the human TUI. Agents use explicit commands with `--json`.

`war init` records the repository's test as `[tickets] done_test` when it finds
one (`war init --done-test ["<command>"]` sets it later), and `war done` then
runs it on work with no check of its own: a pass ticks at observed, a failure
ticks nothing. A `war done` that finishes a Warrant ends with `next: <id> <title>`
when something is ready, `war status` names the first two ready Warrants, and
`war view prime --ready` prints the ready rows alone (docs/TICKETS.md, "For
agents").

All six take `--interactive` (`-I`), which asks for what is not given and then does
what the plain command does: `create` asks for a title, steps, a test and a
sign-off; `start` and `done` pick from what is ready or held; `status` and `check`
pick what to read. On `init` it is `--guided`. A caller with no terminal, or under
`--json`, is refused by name (`interactive.no-terminal`) before any question.
`-i` stays `--item` on `create`.
Advanced operations are grouped by purpose:

| Group | Common commands |
| --- | --- |
| `plan` | `add`, `note`, `edit`, `interview`, drafting and roadmap |
| `view` | `next`, `show`, `prime`, lists, web overview and timeline |
| `admin` | `claim`, `worktrees`, setup, import/export and maintenance |
| `evidence` | Executable checks, verification and execution |
| `sign` | Optional authority and secure human acceptance |

`war next`, `show`, `claim`, `add`, `note` and `edit` still work. New messages
use their grouped spellings. `war create --legacy` explicitly requests the
older manifest-and-atoms layout; existing records keep their encoding.

For table import use `war admin import table roadmap.md --section "Queue"`.
See [milestone tables](MILESTONE-TABLES.md) for mapping and preservation rules.
`war admin worktrees --prune` is a dry run; `--apply` requests conservative
cleanup. Unknown ownership or process activity keeps the workspace.

## The rules

- **Six daily commands.** Plant 130 checks the six-command cap and grouped
  membership. New advanced operations belong in the existing groups.
- **Every earlier spelling is a hidden alias.** `war board` and `war view
  board` parse to the same clap variant and run the same code, so their
  output is the same bytes. Plant 131 runs each one both ways and compares
  the bytes, and plant 195 does the same for each fold.
- **Retiring a spelling is one line on stderr, for a person.** At a terminal, a
  retiring spelling prints what replaces it. A pipe prints nothing, because
  scripts and plants compare output. `OPENWARRANT_DEPRECATIONS=warn` prints it
  anywhere, for someone sweeping a script, and `off` silences it. Nothing is
  removed yet: removal is a later act, after a stable release has carried the
  line (`crates/openwarrant-cli/src/surface.rs`).
- **The envelope's `command` field does not move.** Each command keeps the
  value it always printed, under either spelling: `war view warrants
  --json` and `war tickets --json` both say `"command": "tickets"`. A new
  spelling has no value of its own; it prints its leaf command's. The one
  new command, `war view timeline`, is `war status --timeline` and says
  `"command": "status"`, as that one does.
- **Messages name the new spellings.** Remedies, hints and suggested
  commands say `war admin compile`, not `war compile`; a test parses every
  one against the real clap tree and refuses a hidden spelling. Text in a
  file a resolved Warrant pins (`war admin pins --resolved-only`) cannot be
  edited and keeps the earlier spelling, which still works.
- **MCP tool names do not change** (`war_create`, `war_next`, …). The
  default table is the daily set; `--all` serves every tool (below).

## Parity: all of the function remains

Every command in the clap tree has a row, and `tests/parity.rs` proves it: it
walks the tree and refuses a command with no row, then parses every spelling in
this table with the real parser. Plant 130 runs every old spelling, and plants 131
and 195 compare bytes. So a function can leave this table only by leaving the
tree, and that test fails.

**Use** counts how often `war <word>` appears in this project's agent session
log, the one record of real use: daily is 200 or more, often is 20 or more, rare
is the rest. It sets what gets a flag on the six and what does not.

**Folds.** Only true duplicates moved. `status --next` and `--ready` are the two
reads an agent makes most after `check`; `status <id> --view` is `view show`;
`start --claim` and `done --release`, `--reopen` are the three halves of `start`
and `done` that lived under `admin`. Each runs the same code and keeps its
envelope's command name. `view next` and the like stay current as the group's
spelling, because messages still name them; retiring those starts with those
messages. Everything else under a group is distinct function, not a duplicate,
so it stays there. Its top-level spelling is what retires.

<!-- parity:start -->
| Function | Spelling now | Other spellings that still work | Use | State |
| --- | --- | --- | --- | --- |
| Set up a repository | `war init` | none | daily | surface |
| Create work, at any level | `war create` | none | daily | surface |
| Take work and read its brief | `war start` | none | daily | surface |
| Finish work, or tick one item | `war done` | none | daily | surface |
| Where the work stands | `war status` | none | daily | surface |
| Check the records | `war check` | none | daily | surface |
| Sign what waits for a person | `war sign <target>` | none | daily | surface |
| Draft from a sentence | `war plan "<sentence>"` | none | daily | group |
| Read an edited package back | `war apply <package>` | none | often | kept: FORMAT-1.0 §10.5 |
| Refresh the release notice | `war __release-check` | none | machine | kept: started by `war` itself |
| Add an item to a Warrant, or a test, KPI or milestone | `war plan add` | `war add` | daily | group; `war add` retiring |
| Append a dated note to a Warrant: context for whoever comes next | `war plan note` | `war note` | daily | group; `war note` retiring |
| Ask a question of whoever holds the parent work (OW-ADR-0034) | `war plan note <id> --ask "<question>"` | `war note <id> --ask "<question>"`, `war plan ask`, `war ask` | often | group; `war note`, `war ask` retiring |
| Answer a question: any actor but the asker below an Order; a person on an Order or `--needs-human` (OW-ADR-0034) | `war plan note <id> --answer <q> "<answer>"` | `war note <id> --answer <q> "<answer>"`, `war plan answer`, `war answer` | often | group; `war note`, `war answer` retiring |
| Change a Warrant's type, labels, epic or priority | `war plan edit` | `war edit` | often | group; `war edit` retiring |
| Resume an intake or Warrant interview: unanswered blockers, ranked and counted. Read-only; answers and authorization remain separate acts | `war plan interview` | `war interview` | often | group; `war interview` retiring |
| Create a draft Warrant (§71.2): a master in docs/work/ by default; from a preset, typed atoms, each heading followed by the question it answers (OW-ADR-0022) | `war plan new` | `war new` | daily | group; `war new` retiring |
| Move a single-file Warrant into its directory beside its state, byte for byte: same id, same contract (docs/design/one-format.md) | `war plan expand` | `war expand` | rare | group; `war expand` retiring |
| Add the field that raises an item's level, in place: no new document | `war plan promote` | `war promote` | daily | group; `war promote` retiring |
| Render a declared projection of records (OW-WAR-0148 M6): one set of records, many documents. `prd`, `architecture`, `test-plan` and `agent-packet` ship as document types (profiles/*.toml, `form = "document"`); a program declares its own. Prints the rendering (Markdown or JSON) and writes nothing; `--json` returns it as `oh.war/projection/v1`, every line traced to the record id and revision it came from. `war admin compile` writes the projections of the documents an area declares in `docs/records/<area>/documents.toml` | `war plan render` | `war render` | often | group; `war render` retiring |
| What a change to one record affects (OW-WAR-0148 M3): the records that reach it through incoming relations, transitively; the Warrants, tickets and record atoms that hold or name them; the obligations that evaluate them, with verdicts bound to the revision they judged (a verdict on an older revision stays recorded and reads stale); the roadmap phases and generated views they feed. Read-only | `war plan impact` | `war impact` | daily | group; `war impact` retiring |
| The specification (FORMAT-1.0 §4.5): its acceptance, record by record; the request a person signs to accept it; and the migration of a legacy SAS into `SPEC.md` and `spec/<area>.md` | `war plan spec` | `war spec` | often | group; `war spec` retiring |
| The compiled corpus as one document, `oh.war/model/v1` (OW-WAR-0148): every record with its revision and governor, every relation, the states the builders derive, and a diagnostic per relation whose target is not a record. Read-only; the same tree gives the same bytes | `war plan model` | `war model` | daily | group; `war model` retiring |
| The states of FORMAT-1.0 §11.1 (draft 3, revision 1) over the work documents a tracker works: each with its family (`phase`, `checklist`, ...), its provenance (`computed`, `recorded`, `declared`) and, for a declared state, what it refines and whether it lapsed; and the findings of reading them (journal refusals, a line newer than v2 as UNKNOWN, unbacked tick levels). Read-only: a pure function of the documents, journals, claim store and a time | `war plan states` | `war states` | often | group; `war states` retiring |
| Enter a declared state on a record (OW-WAR-0148 M4): a refinement of a fixed kernel state that the record's profile declares in `[[states]]` (`in_review` refines `in_progress`). An authored event in the journal of the Warrant or ticket that owns the record. It holds only while its fixed parent holds and lapses when the parent stops; refining an authenticated state (`verified`), it is entered only while that state already holds. It satisfies no resolution check and no capability gate | `war plan state` | `war state` | often | group; `war state` retiring |
| The roadmap: one per program, beside the SAS (OW-ADR-0023). Without a subcommand, its phases in dependency order with members, exits and whether each is achieved | `war plan roadmap` | `war roadmap` | daily | group; `war roadmap` retiring |
| The document types this program admits (OW-WAR-0148 M18): each with its form, the store it reads, its capabilities and where it comes from (built in, profiles/, a pack). `add` installs a pack | `war plan types` | `war types` | often | group; `war types` retiring |
| Adopt a document in place as a document type (OW-WAR-0148 M18): records `path → type` in docs/types.toml, and never writes the document. An untyped document counts against `war status`'s document coverage; an adopted one counts for it | `war plan type` | `war type` | often | group; `war type` retiring |
| The stages that can start now (OW-WAR-0068): open, unblocked by their milestone's `depends_on`, and not yet dispatched. Derived from the same records a resolution reads; never a status claim | `war plan frontier` | `war frontier` | daily | group; `war frontier` retiring |
| Estimates learned from the journal (claim to done, by type and label), the order ready work starts in, and the critical path (OW-WAR-0148 M15) | `war plan estimate` | `war estimate` | often | group; `war estimate` retiring |
| Every question across the corpus, blocking and open first, each with the command that answers it (OW-WAR-0069) | `war plan questions` | `war questions` | often | group; `war questions` retiring |
| Ask a question that blocks a stage (OW-WAR-0069). Anyone asks; `war plan note --ask` is the short form. Neither act authorizes anything | `war plan ask` | `war ask` | often | group; `war ask` retiring |
| Answer a question (OW-WAR-0069, OW-ADR-0034): who may answer follows the ladder, and a refusal writes nothing | `war plan answer` | `war answer` | often | group; `war answer` retiring |
| The answers a stage's performer should read before it starts | `war plan answers` | `war answers` | often | group; `war answers` retiring |
| §28.4 authorization. Emits a request; ingests a human's signature | `war sign authorize` | `war authorize` | often | group; `war authorize` retiring |
| Resolve a Warrant: emit the request, evaluate it, or ingest a signature | `war sign resolve` | `war resolve` | daily | group; `war resolve` retiring |
| Correct a delivered artifact of a RESOLVED Warrant (OW-WAR-0064): emit the request naming the pinned digest and the file's digest now, or ingest a human's signed correction. The pin in `deliverables.toml` is never edited; the correction is appended beside it and the superseded digest stays visible | `war sign correct` | `war correct` | daily | group; `war correct` retiring |
| Start a §31 amendment record for a Warrant: writes `amendments/AM-<n>-<hash>.yaml` with its id minted so two branches amending the same Warrant never write the same file (t-dc28), and the fields left for you to fill. `war check` refuses it until they are | `war sign amend` | `war amend` | often | group; `war amend` retiring |
| Attestations for ssh-signed acts (OW-ADR-0015): list them, or verify every signature and subject digest against the tree today | `war sign attest` | `war attest` | often | group; `war attest` retiring |
| A standing authorization (OW-ADR-0029): propose a class, show classes, apply one to a Warrant. Signing and revoking a class are `war sign standing:<id>@<rev>` and `… --revoke` | `war sign standing` | `war standing` | often | group; `war standing` retiring |
| The SAS as a controlled document (§101): propose, accept, diff, status | `war sign sas` | `war sas` | daily | group; `war sas` retiring |
| Warrants waiting on a human act (read-only; OW-WAR-0070) | `war sign inbox` | `war inbox` | often | group; `war inbox` retiring |
| Draft, authenticate and activate authority changes under previous trusted state | `war sign authority` | `war authority` | often | group; `war authority` retiring |
| Review an exact native acceptance response; explicitly request SSH signing. No TTY approval or agent-signing fallback. Without --ssh-sign, no write | `war sign native-resolve` | none | rare | group |
| Approve a Warrant as an official plan (docs/PRESETS.md) | `war sign approve` | none | often | group |
| Sign what the preset asks for before a release, in one batch | `war sign release` | none | often | group |
| Run the gates the Warrant's assurance atom cites and mint each §44.6 receipt into `docs/warrants/<alias>/gate-runs/`, bound to the Warrant's current contract digest. Refuses a Warrant that cites no gate, and a named gate the Warrant does not cite | `war evidence record` | none | daily | group |
| Inspect or run local gate definitions (§44) | `war evidence gate` | `war gate` | often | group; `war gate` retiring |
| §46 independent verification. Emits a request; ingests verdicts | `war evidence verify` | `war verify` | daily | group; `war verify` retiring |
| Take Warrants to their sign-off unattended (t-cee5): deliver, run each gate-executed stage (`war evidence run`), record each cited gate not already admissible (`war evidence record`), run the configured independent verifier (`war evidence verify --run`), record the document gates last — then print what is left for a human. Idempotent and resumable: what is current is skipped. Never signs, never writes a disposition, never asks. See docs/SIGNING.md "One sitting" | `war evidence prepare` | `war prepare` | daily | group; `war prepare` retiring |
| The ops / runs work kind (1.0 plan C4b): run a `service` stage's gate under its wall time, mint the receipt, write the submission | `war evidence run` | `war run` | often | group; `war run` retiring |
| Perform an agent stage with the configured performer (OW-WAR-0069): the Dispatch goes in on stdin, a Stage Submission comes back on stdout, and it is ingested through `war evidence submit`'s refusals. It cannot decide the work is done — §51.2 — and writes nothing if the performer sends nothing | `war evidence perform` | `war perform` | daily | group; `war perform` retiring |
| Run the work graph: what is ready, critical path first, each node in a worktree of its own (OW-WAR-0148 M15) | `war evidence go` | `war go` | often | group; `war go` retiring |
| Ingest a Stage Submission something else produced (§51): it must name a dispatch this Warrant compiled and may not request its own resolution | `war evidence submit` | `war submit` | often | group; `war submit` retiring |
| KPIs (OW-WAR-0148 M13): run each one, journal every value, and say its latest, best and target. Ticks nothing | `war evidence kpi` | `war kpi` | often | group; `war kpi` retiring |
| The assurance mark (OW-ADR-0025, proposed; OW-WAR-0135): evaluate a resolved Warrant against a versioned baseline, and emit an `oh.war/mark/v1` statement only when every requirement is met. Unmet and UNKNOWN requirements are named; a baseline not `accepted` earns no mark. Derived from records already signed: it grants no authority | `war evidence mark` | `war mark` | often | group; `war mark` retiring |
| The document work kind (1.0 plan C4a): review every Warrant's Markdown deliverables — digest, citations, placeholders, independent review | `war evidence document` | `war document` | often | group; `war document` retiring |
| The agent loop, measured (1.0 plan F1): scaffold a throwaway program per task, draft, dispatch, perform, verify blind, and ask what a resolution would say. Nothing is authorized, resolved or signed | `war evidence eval` | `war eval` | often | group; `war eval` retiring |
| Retain or inspect dispatch-bound runtime captures. Retention is not native verification | `war evidence runtime` | `war runtime` | rare | group; `war runtime` retiring |
| What is ready, and whose step it is | `war status --next` | `war view next`, `war next` | daily | flag added; `war next` retiring |
| Show a Warrant by its id | `war status <id> --view full_warrant` | `war view show`, `war show` | daily | flag added; `war show` retiring |
| The web UI: Progress on the canonical roadmap, the queue with each act's dry-run verdict, questions, frontier, corpus and help, served on 127.0.0.1 with a per-session token (OW-WAR-0116). A button starts only `war sign <target> --ssh-sign` or an automatic remedy; your key's dialog is the signature | `war view ui` | `war ui` | daily | group; `war ui` retiring |
| The app: every pane of the corpus, the queue, help and setup, in the terminal. `war` with no arguments is the same thing (OW-WAR-0112) | `war` | `war view tui`, `war tui` | often | duplicate; retiring |
| Read-only project board, including every stage and exact approval commands | `war view board` | `war board` | often | group; `war board` retiring |
| One screen for everything you owe: the acts awaiting your signature as a checklist, the questions an agent asked, and the stages it can start (OW-WAR-0069). Check rows, pick a reason from your presets, sign the batch. It never signs for you: each row is your own ssh confirmation | `war view console` | `war console` | often | group; `war console` retiring |
| Tell the human when an act awaits them: print the pending set, then poll the record trees and print what appears or is signed away | `war view watch` | `war watch` | often | group; `war watch` retiring |
| List remaining Warrant records and status, read-only, from live sources. `progress` is an alias. Legacy resolution is not implementation completion | `war view overview` | `war overview`, `war view progress`, `war progress` | often | group; `war overview` retiring; aliases retiring |
| Every Warrant, its type, state and progress: the light ones (tickets) first, then those with a directory, then those read in place. Filters narrow it to exactly the Warrants every one admits | `war view warrants` | `war warrants`, `war view tickets`, `war tickets`, `war view ls`, `war ls` | often | group; `war warrants` retiring; aliases retiring |
| What can start now: open, unclaimed, unblocked items across Warrants, most urgent and oldest first. A directory Warrant's ready stages are in `war view next` | `war status --ready` | `war view ready`, `war ready` | daily | flag added; `war ready` retiring |
| What is open: one line per open Warrant (progress, who holds it, what it waits on) and done work by id; one Warrant in full with an id; `--ready`, the ready rows alone | `war view prime` | `war prime` | daily | group; `war prime` retiring |
| The records a task needs, cited by id, inside a byte budget | `war view context` | `war context` | daily | kept: FORMAT-1.0 §10.3 |
| The corpus timeline (`oh.war/corpus-timeline/v1`): every journal event by day. The same as `war status --timeline`, and its envelope's command is `status`, as that one's is | `war status --timeline` | `war view timeline` | often | duplicate; retiring |
| Inventory workspaces and conservatively reclaim only finished tool-owned ones | `war admin worktrees` | `war worktrees` | rare | group; `war worktrees` retiring |
| Take an item or a whole Warrant, so no other agent works it | `war start <id> --claim` | `war admin claim`, `war claim` | daily | flag added; `war claim` retiring |
| Compile the configured projections (§71.8) | `war admin compile` | `war compile` | daily | group; `war compile` retiring |
| Read-only repository diagnostics; never signs, repairs, or starts work | `war admin doctor` | `war doctor` | daily | group; `war doctor` retiring |
| Every file a Warrant's `deliverables.toml` pins, with the Warrant's state | `war admin pins` | `war pins` | daily | group; `war pins` retiring |
| Report six readiness dimensions; unavailable checks block readiness | `war admin preflight` | `war preflight` | often | group; `war preflight` retiring |
| Semantic difference between the committed compilation and a fresh one (§71.10) | `war admin diff` | `war diff` | often | group; `war diff` retiring |
| Read a Warrant's local draft journal (§66), or reconstruct one from its records. There is no way to append by hand: events come from the commands that change records | `war admin journal` | `war journal` | often | group; `war journal` retiring |
| Declare a Warrant's deliverables delivered (t-39dc): record §37.2 provenance on each — the sha256 of the file now, how it was made, the build of `war` that recorded it — and set `content_addressed`. Refuses a resolved Warrant (its `deliverables.toml` is bound), a missing file, and a path a later authorized Warrant governs (OW-ADR-0021); any refusal writes nothing. Idempotent: a deliverable already recorded at its bytes is left as it is | `war admin deliver` | `war deliver` | often | group; `war deliver` retiring |
| Compile a Stage Dispatch for one stage of a Warrant (§47) | `war admin dispatch` | `war dispatch` | often | group; `war dispatch` retiring |
| Capture or check portable context for an existing Dispatch; never execute it | `war admin dispatch-bundle` | `war dispatch-bundle` | often | group; `war dispatch-bundle` retiring |
| The commit message, drafted from the records that changed (OW-WAR-0069) | `war admin commit` | `war commit` | often | group; `war commit` retiring |
| Renew the lease on your claims (or the one named), so no other agent reclaims them while you work. Every war command you run renews them too; a claim whose lease runs out is taken by a plain `war admin claim` | `war admin heartbeat` | `war heartbeat` | often | group; `war heartbeat` retiring |
| Give a claim back without finishing the item | `war done <id> --release` | `war admin release`, `war release` | often | flag added; `war release` retiring |
| Untick a done item: its box opens again, its done suffix goes, and the journal records why (`work.item_reopened`, FORMAT-1.0 §11.2) | `war done <id> --reopen --reason why` | `war admin reopen`, `war reopen` | often | flag added; `war reopen` retiring |
| Give an unsigned Warrant a free alias: its `local_alias`, its directory, and a journal line. A Warrant with an authorization keeps its alias; that is refused by name | `war admin renumber` | `war renumber` | often | group; `war renumber` retiring |
| Write the AGENTS.md this repository ships, for the repository's namespace. `war init` writes it once; this rewrites (--force) or prints it. `--block` keeps only a small managed pointer block in the files you already have (M16) | `war admin agents-md` | `war agents-md` | often | group; `war agents-md` retiring |
| Bring work in as Warrants (OW-WAR-0148 M10): `beads <file.jsonl>` (Beads' issue JSONL, as `bd export` writes it), `openspec <dir>` (each change of an OpenSpec folder) or `speckit <dir>` (each feature of a Spec Kit `specs/` folder). Each becomes a light Warrant (a ticket) with its tasks as items. Running it again changes nothing; input it cannot map is refused by name, and a refusal writes nothing. To keep working in those tools instead, read the folder in place: `[[adapters]]` in openwarrant.toml (docs/TYPES.md) | `war admin import` | `war import` | often | group; `war import` retiring |
| §68 portable export and round trip; or, as `war admin export beads`, every light Warrant (a ticket) as Beads issue JSONL on stdout, in the shape `bd export` writes, which `war admin import beads` reads back (OW-WAR-0148 M10) | `war admin export` | `war export` | often | group; `war export` retiring |
| Import a legacy ADR corpus (§96), discharging OW-WAR-0043 | `war admin migrate` | `war migrate` | often | group; `war migrate` retiring |
| Experimental preservation transport. Imported records grant no authority | `war admin archive` | `war archive` | often | group; `war archive` retiring |
| Feed a harness's own task list into the tick ladder (OW-WAR-0148 M13). Proposes ticks; writes only with `--apply`; never blocks | `war admin bridge` | `war bridge` | often | group; `war bridge` retiring |
| Host the WAR domain for Liminal, `oh.war/liminal-v1` (OW-WAR-0148 M8; SAS §82.2; docs/LIMINAL_HOST.md): one request on stdin, one response on stdout. Pure — no repository, no file, no network, no process. Exit 0 compiled, 1 refused, 2 compiled with something not established. `--export` writes the request that reproduces this repository's model instead | `war admin host` | `war host` | often | kept: the Liminal protocol |
| Run an offline SDK operation from an explicit JSON request. Always emits JSON | `war admin sdk` | `war sdk` | often | kept: harness configuration |
| Serve the Model Context Protocol over stdio (OW-ADR-0014): the daily commands as tools, answering in the CLI's text; `--all` serves every read, request half and agent-permitted write; no signing, no ingest. `--describe` prints the tool table instead of serving | `war admin mcp` | `war mcp` | daily | kept: harness configuration |
| §67 Knowledge Fabric seam. `health` reads; `act` WRITES and needs --confirm-write | `war admin kf` | `war kf` | often | group; `war kf` retiring |
| §94 telemetry baseline, §95 untracked-work candidates, §100 metrics | `war admin telemetry` | `war telemetry` | often | group; `war telemetry` retiring |
| Bind a Warrant's machine scope to Bonsai evidence | `war admin bonsai` | `war bonsai` | often | group; `war bonsai` retiring |
| Lower a computational Warrant's stage graph into a BLUT PlanSpec (§49) | `war admin blut` | `war blut` | often | group; `war blut` retiring |
| The repositories this user works in, remembered on use (OW-WAR-0115): the list `war` opens from anywhere. Each row is read from that project's own records; a path that no longer holds one is reported missing, never dropped silently. `OPENWARRANT_NO_PROJECTS=1` turns remembering off | `war admin projects` | `war projects` | often | group; `war projects` retiring |
| Install a published release into ~/.local/lib/openwarrant and repoint the links this tool put on PATH. Reads the network; never touches a `war` it did not install | `war admin update` | `war update` | often | group; `war update` retiring |
| Which `war` this is, where it came from, and what else answers to that name on PATH. Needs no repository | `war --version` | `war admin version`, `war version` | often | duplicate; retiring |
| git's merge driver for ticket files: checklists item by item, notes appended (`merge=war-ticket` in .gitattributes). `--install` writes the .gitattributes lines and this clone's git configuration | `war admin merge-ticket` | `war merge-ticket` | often | kept: git configuration |
| Show the repo preset, or switch it (docs/PRESETS.md) | `war admin preset` | none | often | group |
| The file ledger: why each file changed (docs/LEDGER.md) | `war admin ledger` | none | often | group |
| The compliance score, its weights, and its in-toto statement (docs/SCORE.md) | `war admin score` | none | often | group |
<!-- parity:end -->

## MCP: the daily set, in the CLI's text

`war admin mcp` serves seven tools by default, the daily commands an agent
runs plus the two it needs between them. The CLI's folds are arguments:

| Tool | CLI | Folded in, as arguments |
| --- | --- | --- |
| `war_create` | `war create` | `force` (create despite `ticket.duplicate`) |
| `war_start` | `war start` | `claim`, `paths`, `steal` (`war_claim`) |
| `war_done` | `war done` | `release`, `reopen` with `reason` |
| `war_status` | `war status` | `next` (`war_next`), `ready` (`war_ready`), `target` with `view` (`war_show`) |
| `war_check` | `war check` | |
| `war_context` | `war view context` | |
| `war_note` | `war plan note` | `ask`, `blocking`, `needs_human`; `answer` with `text` |

`war admin mcp --all`, or `[mcp] tools = "all"` in `openwarrant.toml`,
serves the whole table (42 tools), so nothing an MCP client had is gone.
A `war_start` reply carrying task context always returns the report envelope,
in structured content and the text fallback, including with `json: false`.
Claim-only calls retain the terse reply. Other replies use the text the CLI
prints. A finding reads `refused (rule): ...`
and a nonzero exit ends `exit N, verdict`. `json: true` on any call, or
`[mcp] structured = true`, returns the `oh.war/report/v1` envelope instead.
Either way no signing, ingesting or other human act is a tool; `war admin
mcp --describe` prints the table and that refusal list. Plant 230 and the
`mcp::tests` budget test hold the default `tools/list` under 6,400 bytes;
`tools/measure-tokens.sh` measures it.

## Historical command inventory

The following inventory and M12 discussion describe the earlier surface. The
six-command table above governs current help placement.

The inventory below is every top-level spelling of `war` at 9dc413e2 (86
commands, three aliases), with its home now. "Envelope" is the `command`
value of the `--json` envelope, the same under either spelling.

### Daily verbs (top level, listed)

| Spelling | Envelope |
|---|---|
| `war init` | (prints lines; no envelope, as before) |
| `war create` | `create` |
| `war next` | `next` |
| `war start` (M15, new; the worktree path is `war start --worktree`) | `start` |
| `war claim` | `claim` |
| `war done` | `done` |
| `war add` | `add` |
| `war note` | `note` |
| `war edit` | `edit` |
| `war show` | `show` |
| `war status` | `status` |
| `war check` | `check` |

### `war plan …`

`war plan interview <intake-key-or-warrant-id>` resumes the unanswered blockers
and reports bounded recorded-question counts. See [INTERVIEWS.md](INTERVIEWS.md).

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war plan "<sentence>"`, `--proposal`, `--apply`, `--records` | the same | `plan.request`, `plan.validate`, `plan.interview`, `plan.apply` |
| `war plan new` | `war new` | `new` |
| `war plan expand` | `war expand` | `expand` |
| `war plan promote` | `war promote` | `promote` |
| `war plan render` | `war render` | `render` |
| `war plan impact` | `war impact` | `impact` |
| `war plan spec status`, `accept`, `migrate` (FORMAT-1.0 §4.5, §14, new) | `war spec` | `spec.status`, `spec.accept`, `spec.accept.request`, `spec.migrate` |
| `war plan model` | `war model` | `model` |
| `war plan states` (FORMAT-1.0 §11.1, new) | none | `states` |
| `war plan state` | `war state` | `state` |
| `war plan roadmap` | `war roadmap` | `roadmap` |
| `war plan types`, `war plan types add` (M18, new) | `war types` (hidden, as every group member) | `types`, `types.add` |
| `war plan type` (M18, new) | `war type` (hidden, as every group member) | `type` |
| `war plan frontier` (`--all`: the whole work graph, M15) | `war frontier` | `frontier` |
| `war plan estimate` (M15, new) | `war estimate` | `estimate` |
| `war plan questions` | `war questions` | `questions` |
| `war plan ask` | `war ask` | `ask` |
| `war plan answer` | `war answer` | `answer` |
| `war plan answers` | `war answers` | `answers` |

### `war sign …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war sign <target> …` (`--ssh-sign`, `--list`, `--dry-run`, `--batch`, …) | the same, unchanged | `sign` |
| `war sign authorize` | `war authorize` | `authorize.request`, `authorize` |
| `war sign resolve` | `war resolve` | `resolve.request`, `resolve.dry_run`, `resolve` |
| `war sign correct` | `war correct` | `correct.request`, `correct` |
| `war sign amend` | `war amend` | `amend` |
| `war sign attest` | `war attest` | `attest`, `attest.custody` |
| `war sign standing` | `war standing` | `standing` |
| `war sign sas` | `war sas` | `sas`, `sas.accept.request` |
| `war sign inbox` | `war inbox` | `inbox` |
| `war sign authority` | `war authority` | `authority` |
| `war sign native-resolve` (experimental) | none: only under the group | `sign.native_resolve` |
| `war sign approve` (M14) | none: new, and only under the group | `sign.approve` |
| `war sign release` (M14) | none: `war release` is `war admin release` | `sign.release` |

### `war evidence …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war evidence record` | the same | `evidence` |
| `war evidence gate` | `war gate` | `gate`, `gate.invalidate` |
| `war evidence verify` | `war verify` | `verify.request`, `verify`, `verify.bundle`, `verify.run` |
| `war evidence prepare` | `war prepare` | `prepare` |
| `war evidence run` | `war run` | `run` |
| `war evidence perform` | `war perform` | `perform` |
| `war evidence go` (M15, new) | `war go` | `go` |
| `war evidence submit` | `war submit` | `submit` |
| `war evidence kpi` | `war kpi` | `kpi` |
| `war evidence mark` | `war mark` | `mark` |
| `war evidence document` | `war document` | `document.review` |
| `war evidence eval` | `war eval` | `eval.run`, `eval.verify`, `eval.ordinary` |

### `war view …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war view ui` | `war ui` | (a server; `ui` devices: its own) |
| `war view tui` | `war tui` | (a terminal app; `--json` refused) |
| `war view board` | `war board` | `board` |
| `war view console` | `war console` | `console` |
| `war view watch` | `war watch` | `watch` |
| `war view overview` (alias `progress`) | `war overview`, `war progress` | `overview` |
| `war view warrants` (aliases `tickets`, `ls`) | `war warrants`, `war tickets`, `war ls` | `tickets` |
| `war view ready` | `war ready` | `ready` |
| `war view prime` | `war prime` | `prime` |
| `war view context` (Format 1.0, new) | `war context`, the spelling the format and `llms.txt` name | `context` |
| `war view timeline` | `war status --timeline` (new name) | `status` |

### `war admin …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war admin compile` | `war compile` | `compile` |
| `war admin doctor` | `war doctor` | `doctor` |
| `war admin pins` | `war pins` | `pins` |
| `war admin preflight` | `war preflight` | `preflight` |
| `war admin diff` | `war diff` | `diff` |
| `war admin journal` | `war journal` | `journal` |
| `war admin deliver` | `war deliver` | `deliver` |
| `war admin dispatch` | `war dispatch` | `dispatch` |
| `war admin dispatch-bundle` | `war dispatch-bundle` | `dispatch-bundle` |
| `war admin commit` | `war commit` | `commit` |
| `war admin heartbeat` | `war heartbeat` | `heartbeat` |
| `war admin release` | `war release` | `release` |
| `war admin renumber` | `war renumber` | `renumber` |
| `war admin agents-md` | `war agents-md` | `agents_md` |
| `war admin import` | `war import` | `import` |
| `war admin export` | `war export` | `export`, `export.progress`, `export.progress.verify` |
| `war admin migrate` | `war migrate` | (prints lines, as before) |
| `war admin archive` | `war archive` | `archive` |
| `war admin bridge` | `war bridge` | `bridge` |
| `war admin host` | `war host` | (its own protocol, `oh.war/liminal-v1`) |
| `war admin sdk` | `war sdk` | (its own envelope, as before) |
| `war admin mcp` | `war mcp` | (JSON-RPC on stdio) |
| `war admin kf` | `war kf` | (prints the server's answer) |
| `war admin telemetry` | `war telemetry` | `telemetry` |
| `war admin bonsai` | `war bonsai` | (prints the evidence document) |
| `war admin blut` | `war blut` | `blut` |
| `war admin projects` | `war projects` | `projects` |
| `war admin update` | `war update` | `update` |
| `war admin version` | `war version` | `version` |
| `war admin schemas` (built with `--features schema`) | `war schemas` | `schemas` |
| `war admin merge-ticket` | `war merge-ticket` | `merge-ticket` |
| `war admin preset` (M14) | none: new, and only under the group | `preset` |
| `war admin ledger` (M17: `record`, `prune`, `import`, `export`) | none: new, and only under the group | `ledger.record`, `ledger.prune`, `ledger.import`, `ledger.export` |
| `war admin score` (M17) | none: new, and only under the group | `score`, `score.weights`, `score.statement`, `score.verify` |
| `war admin worktrees` (alpha.6) | none: new, and only under the group | `worktrees` |

### Top level, hidden, no group

| Spelling | Why |
|---|---|
| `war __release-check` | started detached by the release notice; never typed |
| `war apply <package>` | FORMAT-1.0 §10.5: reads an edited `war view context --preset human` package back; the other half of that command, so it spends no thirteenth daily verb |
| `war` alone | the terminal app, as `war view tui` |

## Choices, and why

- **`ready` is folded into `next`.** `war next` already lists the ready
  items of light Warrants first, then the acts waiting on an agent or a
  person, so a newcomer needs one verb for "what now". That keeps the
  daily list at eleven with room for `war start`. The ready list alone is
  `war view ready`; `war ready` still works.
- **`prime` is under `view`.** It is the arriving agent's read, named in the
  instruction block `war admin agents-md --block` writes; it does not fit in
  the twelve without dropping a verb a person types daily. The block now
  says `war view prime`; a block written earlier says `war prime`, which
  still works.
- **`context` is under `view`, and `war context` works.** Format 1.0
  section 10.3 names the command `war context <target>`, and every line of
  `docs/generated/llms.txt` tells an agent to run it, so the top-level
  spelling answers, hidden like every member's earlier spelling. It is an
  agent's read, as `prime` is, and a person types the twelve daily verbs; the
  help lists it with `prime` under `view` rather than dropping a verb.
- **`war sign` is both the signature and the group.** clap tells them apart
  cleanly: the group's members are matched only as the first word after
  `sign` (`args_conflicts_with_subcommands`), and no sign target (an alias,
  a SAS version, `<alias>/<D-id>`, `roadmap`, `<gate>@<version>`,
  `standing:<id>@<rev>`, `recover:<id>`, `t-…`) is ever a member's name. So
  `war sign <alias> --ssh-sign` works exactly as before, and `war sign
  --as alice authorize` still treats `authorize` as a target, as it always
  did. No other name was needed.
- **`war plan` is both the drafting request and the group.** The same rule:
  a member's name is matched only as the first word after `plan`. A
  one-word request that is also a member's name (`war plan model`) runs the
  member; `war plan -- model` drafts it.
- **`sas` moves whole into `sign`.** `sas accept` is the human act; propose,
  diff, status and repin prepare or read it, and one group keeps one
  command.
- **`authority` is under `sign`**: who may sign is itself a signed change.
- **`inbox` is under `sign`**: it lists the human acts waiting.
- **`mark` and `document` are under `evidence`**: the mark is evaluated from
  the evidence on record, and `document review` is what a document gate
  runs.
- **`promote` and `state` are under `plan`**: promoting a light Warrant
  drafts a directory one, and a declared state is part of the type's
  workflow model, beside `model` and `impact`.
- **`release` is under `admin`, beside `heartbeat`**: both manage claim
  leases rather than the work itself. **`reopen`** (FORMAT-1.0 §11.2's
  `work.item_reopened`) sits beside them: unticking an item is rare, and a
  thirteenth daily verb is not spent on it.
- **`diff` and `commit` are under `admin`**, beside `compile`: both read the
  compiled projections.
- **`dispatch` and `dispatch-bundle` stay under `admin`**, as planned: the
  agent's stage loop (`war next` hands an agent `war admin dispatch <alias>
  <stage>`) is plumbing a newcomer never types.
- **`ledger` and `score` are under `admin`** (M17, docs/LEDGER.md,
  docs/SCORE.md): both maintain what compile regenerates. The score itself
  is also the last block of `war status --verbose`, and its CI floor is a flag of the
  daily `war check` (`--floor <rev>`, envelope `check.floor`), so neither
  needs a thirteenth daily verb.
- **`merge-ticket` is listed under `admin`.** It was hidden at the top level
  because git, not a person, runs it; `war admin merge-ticket --install` is
  the spelling messages name. The git configuration it writes still calls
  `war merge-ticket`, so clones configured earlier keep merging.
- **`timeline` is new under `view`**: `war status --timeline` under the name
  a reader looks for.
- **M14 adds three members, each only under its group** (docs/PRESETS.md):
  `war sign approve` and `war sign release` are a person's acts, so they are
  `sign`'s; `war admin preset` sets the repository up, so it is `admin`'s.
  None has a top-level spelling: `war release` already was the claim's
  release (`war admin release`), and a new command needs no earlier one.
  The PR gate is a flag of a daily verb, `war check --pr <number>`
  (envelope `check.pr`); without `--pr`, `war check` reads no network.
- **FORMAT-1.0 §12's checks are subcommands of the daily `war check`**:
  `war check run [<target>]`, `evaluate`, `fire <trigger>` and
  `install-hook` (envelopes `check.run`, `check.evaluate`, `check.fire`,
  `check.install-hook`). They run a document's `check` records rather than
  check the records, so they need no thirteenth verb; `war view watch
  --checks` is the same runner as a poller. A target is a work document's
  id, a check's id or `<document>/<check>`, for any Format 1.0 work document:
  a ticket, or a Warrant in `docs/warrants/` or `docs/work/`, whose runs are
  kept in its own state directory. A run is current until a source file
  changes; records about other work (journals, check output, other work
  documents) are not source.
- **`types` and `type` are new under `plan`** (OW-WAR-0148 M18): a type is
  how a document is planned, beside `model`, `render` and `roadmap`.
  `war plan types` lists the types, `war plan types add <pack>` installs a
  pack, `war plan type <file> <type>` adopts a document in place. Like every
  group member they also answer, hidden, at the top level (`war types add
  ops`, `war type <file> <type>`, the spellings the plan's decision table
  uses); messages name the `plan` spellings.

- **`start` is the twelfth daily verb; `go` is under `evidence`** (M15,
  docs/GO.md). `war start` is what a person or an agent types to pick up one
  piece of work, so it sits beside `claim` and `done`: it claims the next ready
  Warrant and prints only what its work needs (2026-10-23, to cut what an agent
  reads to begin). `war start --worktree <id>` is M15's path: the node in a
  worktree of its own, with the harness started there. `war go` runs
  the whole graph unattended: it is the stage runner `war evidence perform`
  generalized, so it joins `perform` and `run`, and the daily list stays at
  twelve. `war go` still parses, hidden, as every member does; messages name
  `war evidence go`.
- **`estimate` is under `plan`**, beside `frontier`: both read the graph and
  write nothing.

## What still names an earlier spelling, on purpose

Each of these still runs, so nothing here is broken; each keeps its bytes for
a reason that outweighs one more line in the new spelling.

- Files a resolved Warrant pins and no later Warrant governs (`war admin pins
  --resolved-only`): their bytes cannot move without a signed correction.
- Log and banner prefixes (`war ui: act …`, `war ui: pair a device …`):
  operators and plants read them; they name the program, not a suggestion.
- The self-signed certificate's common name, git's merge-driver line
  (`war merge-ticket %O %A %B %P`, already written into clones' git
  configuration) and the generated TypeScript header.
- The Claude Code hooks (`.claude/hooks/`): they may run an older `war`,
  which knows only the earlier spellings.
- The adopt Warrant's atoms and the program SAS template that `war init
  --program` writes: they are record content, not suggestions.
