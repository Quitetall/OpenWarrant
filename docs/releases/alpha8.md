# OpenWarrant 1.0.0-alpha.8: Checked Completion and Shared Work

This unverified alpha is the frozen build for the next round of testing. Ordinary work
still needs no ticket and no signature. This release makes `war done` check the work,
makes new tickets visible everywhere at once, and moves new work to one layout.

What is new for a user:

- **Checked completion.** `war init` records the repository's test command as
  `[tickets] done_test` (`war init --done-test [CMD]` sets it in an existing repository;
  deleting the line turns it off). When a ticket has no test of its own, `war done` runs
  it. A pass ticks at `observed`; a failure or UNKNOWN refuses and ticks nothing.
- **Shared tickets.** A new ticket is visible in every worktree and clone of the
  repository, through the claims remote (`war init --claims-remote origin`). A likely
  duplicate is refused, naming the match; `--force` creates it anyway.
- **Cheaper turns.** `war done` names the next ready item. `war status` shows the first
  ready rows. `war view prime --ready` lists ready rows only. `.openwarrant/next.md`, an
  untracked per-worktree file, holds the brief of the work you hold or the ready rows,
  for harnesses that load instruction files.
- **One layout for new work.** New work goes to `docs/work/<w-id>/<w-id>-<slug>.md`.
  `[tickets] layout = "tickets"` keeps the old `docs/tickets/` layout. To move existing
  unsigned tickets, run `war admin migrate --dry-run`, then `war admin migrate`. Old
  signed records stay readable in place and are never moved.
- **The ladder and ids.** Four tiers: Spec and Roadmap, ADR and Order, Design and
  Warrant, Guide and Ticket. Ids are `w-` for work and `d-` for documents. A question
  is answered at its own level and routed to the orchestrators. Each work order names
  its repository, worktree, branch and base commit, and `war start` refuses work held
  from another worktree or repository, naming it.
- **Smaller context by default.** `war context <work item>` returns the work, the records
  it links (requirements, constraints, decisions, obligations) and its parent's intent.
  Completed sibling phases, rejected options and spec frontmatter are left out; on a
  phased roadmap a packet went from 7,912 to 3,487 bytes. `--preset agent` or `--full`
  gives the old, wider walk.
- **Close at the first passing test.** Agent text now says to close work the first time
  its test passes, before docs or lint polish, instead of "with your last test run". In
  the study, agents that waited for their "last" run hit the turn cap with the work done
  and never closed it. `war done` still runs the test and refuses if it fails.
- **The actor comes from the harness.** With no `OPENWARRANT_ACTOR`, `war` names the
  agent after its harness (Claude Code, Codex, opencode, Gemini CLI, Cursor), says so,
  and `war start` and `war done` agree on it.
- **Views are generated on main only.** Pull requests carry no generated views.
- **Speed at scale.** `war create` stays fast at about 10,000 tickets, and the read path
  (`ready`, `next`) is several times faster, within 100 and 150 ms at that size. The status page embeds its timeline without
  payloads (27 MB down to 1.8 MB).

Fixed since alpha.7 (from a field report):

- `war check run` works on a single-file Warrant and its `CHK-*` checks.
- A phase with a single-file Warrant is derivable, and a later phase's tickets wait until
  the earlier phase is done (`ticket.blocked`).
- `war start` adds the linked REQ, CON and DEC records to the brief.
- Running a check twice with no edits no longer reports `check.source-stale`.
- `war done <ticket> --check` raises an earlier `claimed` tick to `observed`.
- `war view show` names the real file and the real phase title.
- A claim read while another clone renews it no longer reads UNKNOWN (`git cat-file …:
  bad file`); `war` lists the claim again and reads it as it is now.

Compatibility: `[tickets] done_test` is a new key, and the policy file rejects unknown
keys, so alpha.7 refuses a repository that sets it; `[tickets] layout` is new too. The default layout changed: with no setting, `war create` now writes
`docs/work/`, not `docs/tickets/`. Set `layout = "tickets"` to keep the old one.

This is an unverified alpha preview, not Stable 1.0. Legacy reconciliation, independent
qualification, secure human acceptance and the adoption study remain incomplete. No
deployment permission is granted.
