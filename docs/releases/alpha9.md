# OpenWarrant 1.0.0-alpha.9: Lanes and Shared Resources

This unverified alpha is the build the Argos release rounds run on. Ordinary work still
needs no ticket and no signature. This release adds lanes, so several agents can work in
parallel without colliding. It also lets those agents share one machine: one build cache,
builds that take turns, and a cleanup pass after the work.

What is new for a user:

- **Lanes** (OW-ADR-0036, proposed). A lane is an ordered path of work for one runner, an
  agent or an orchestrator. It sets what goes first and what goes last, and it owns files.
  - `war lane new <name> --paths <glob>… [--for <who>] [--after <lane>] [--reuse <path>…]`
    creates one. `war create … --lane <name>` adds an item last in it. `war start --lane
    <name>` hands over its next item, in the lane's one worktree.
  - Two lanes running at the same time can't own the same files (`lane.overlap`, in
    `war check` and at `lane new`). Lanes in sequence can.
  - `war done` refuses work that touched files outside its lane and names each file
    (`lane.outside-paths`). `war lane widen <lane> <glob>` is the recorded way to add
    files. The Claude Code plugin stops the edit before it happens.
  - A gate, tag or release item that `depends_on` lanes waits until all of them are done.
    A lane whose items are all done is done.
  - `war.for` says who an item suits (`claude`, `codex`, `gemini`, or a role). It's a
    recommendation only. `war status --ready --for <who>` filters by it.
- **Shared resources** (OW-ADR-0037, proposed), in `openwarrant.toml` `[resources]`:
  - `[resources.env]` gives every `war`-made worktree the same build settings, for example
    one shared `CARGO_BUILD_BUILD_DIR`. `war admin seed` builds that cache once, and
    `war start` says when the seed is stale.
  - `war run build -- <cmd>` takes a turn on a shared build slot (`[resources.slots]`).
    While it waits, it names who holds the slot.
  - `max_worktrees` makes `war start` refuse an extra worktree and name the idle ones to
    reuse. `war admin cleanup [--apply]` removes finished worktrees and merged `war-go/*`
    branches. Where the host hides processes, it falls back to `idle_after` (default
    24h): nothing changed in the worktree for that long.
  - A lane plan gets a final `cleanup` lane by default (`[lanes] cleanup = true`), worked
    by a different runner: a docs pass, an architecture pass (merge duplicate abstractions
    or justify them) and a resource pass.
- **opencode usage.** `war done` records the tokens of an opencode session and its
  subagents, read-only from opencode's database. Unreadable is `unknown`, never zero.
  Reasoning tokens are shown separately.

Fixed since alpha.8:

- A parent with some `part_of` children done reads `in_progress`, not `open`
  (FORMAT-1.0 11.1).
- `war context <item>` includes its REQ's constraints when the ticket has no Intent.
- `war claim` and `war start` no longer crash inside a pid namespace.

Repository: the project moved to `OpenHuman-Technologies-LLC/OpenWarrant`. GitHub redirects
the old address, so alpha.8 and earlier keep updating.

Compatibility: `[resources]` and `[lanes]` are new keys, and the `war.lane` and `war.for`
frontmatter keys are new too. A repository that uses them needs alpha.9. An older reader
reports every check over a lane document `UNKNOWN`, because lane documents list
`requires: [war.lane]`. FORMAT-1.0 Appendix D now registers the `war` prefix.

This is an unverified alpha preview, not Stable 1.0. Legacy reconciliation, independent
qualification, secure human acceptance and the adoption study remain incomplete. No
deployment permission is granted.
