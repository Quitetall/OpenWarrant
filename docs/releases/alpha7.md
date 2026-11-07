# OpenWarrant 1.0.0-alpha.7: One Item, Three Levels

This unverified alpha is the frozen build for the next round of testing. Ordinary work
still needs no ticket and no signature. When you track work, it is one document with
three levels, worked with six commands.

What is new for a user:

- **One item, three levels.** A ticket is one Markdown document, `docs/tickets/<id>.md`.
  Its level comes from its fields and is never stored:
  - **Ticket** (default): done when the agent says so.
  - **Warrant**: `war create --test "<command>"`; `war done` runs the test.
  - **Order**: `war create --sign`; a person signs at the end. `war done` on a finished
    Order hands it over, exits 0 and prints the `war sign` command. `war sign --list`,
    `war status` and `war view next` name it. An agent can't tick the signature.
  - `war plan promote` raises a level in place and keeps the id.
  - Any level can declare its paths, tools and commands (`--path`, `--tool`, `--command`).
- **Six commands.** `init`, `create`, `start`, `done`, `status`, `check`. `--interactive`
  (`-I`) asks a person what is missing, and never prompts a caller without a terminal.
  Earlier spellings still work and are retiring (`docs/COMMANDS.md`).
- **Mission Control (`war` at a terminal).** Watch, Act, Records (`3`) and Repos (`p`),
  all from real records. Unknown usage reads `UNKNOWN`; no figure is invented.
  Create, dispatch into a worktree, answer and sign work from the app. Herdr is optional.
- **Context that serves real documents.** `war context` takes a long document, a section
  (`COMBAT.md#friendly-fire`), a topic (`"friendly fire"`) or a code path. Over budget it
  returns the most relevant units and declares every cut; `--strict` refuses and `--full`
  returns everything. Each packet reports its source bytes against the bytes returned.
- **Agents start with their work.** `war start` gives a short brief with the ticket's
  sections. The Claude Code prompt hook loads the brief of the item a prompt names.
- **Live updates.** When a document an agent was given changes, its next `war` call says
  so once. At `war done`, a Warrant or Order waits until the change has been seen.
- **Work across repositories** (`docs/INTERFACES.md`). Named repositories, an `interface`
  document type with clauses and their verification, change requests classed breaking or
  compatible, and "done upstream, not pinned here" read through a submodule or
  `Cargo.lock`. A breaking change takes both owners' signatures.
- **Smaller setup.** `war init` writes no `schemas/` (they are in the binary; `--schemas`
  writes a copy). `war init --ci` writes one opt-in pull-request workflow.

Fixed since alpha.6, among others: a ticket's own sections never reached the brief;
`war done` could disown its own claim when started from another terminal; a note from
Mission Control never reached an agent in a worktree.

This is an unverified alpha preview, not Stable 1.0. Legacy reconciliation, independent
qualification, secure human acceptance and the adoption study remain incomplete. New
directory Warrants are still written by `war plan new`. No deployment permission is granted.
