# OpenWarrant 1.0.0-alpha.6 — Lean Agent Loops and a Mission Control Preview

This unverified alpha updates the planning and tracking CLI from alpha.5. Ordinary work
still needs no signature: `war init`, `war create`, `war start`, `war done`.
Human signing remains a separate assurance action.

What is new for a user:

- **Mission Control preview (`war tui`).** Observe (`o`) and Orchestrate (`O`) screens beside the
  existing panes. This is a preview, not a finished workspace:
  - Works: Herdr agent lists and transcripts, the questions list, fleet and worktree counts,
    and batch SSH signing, which still shells out to `war sign --ssh-sign`.
  - Not real yet: token and dollar figures are placeholders; lease, scope and collision details
    do not show; "critical path" only means priority 0 or 1.
  - Does not work yet: create, dispatch and answer from the TUI, and the no-Herdr fallback pane.
    Use the CLI for those. See the lean programme, `t-3c16c`.
- **Herdr client.** `war tui` reads agents and transcripts from a running Herdr. Default session
  only for now; Herdr is optional.
- **The Two-Call Agent Loop.**
  - `war start` takes the next ready Warrant and outputs an actionable, token-budgeted brief:
    description, open checklist items, journal notes, dependencies, and closing instructions.
  - `war done` ticks every open item in one transaction or refuses cleanly if checks fail.
  - Worktree actor isolation: in a git worktree, the default actor is derived from the
    worktree name (`claude@w2`), preventing lease and claim collisions across concurrent agents.
- **Title and Fuzzy Search for Ticket Dispatch.**
  `war start`, `war done`, `war view prime`, and `war note` accept words of a Warrant's title
  or labels instead of requiring an exact hex ticket ID (e.g. `war start T8`, `war start monthly budget`).
- **One object model.** Project documents (`doc:<path>`), tickets and Warrants share
  `oh.war/model/v1`, and `war context` compiles them within a budget. A document over budget is
  refused rather than reduced, and sections of ordinary documents cannot be selected yet.

The published notes for this tag described the TUI items above as finished; this text corrects
them. The tag was published ahead of its gates, for Linux x86_64 only, so that tools still on
alpha.2 could move forward.

This is an unverified alpha preview, not Stable 1.0. Legacy reconciliation,
independent qualification, secure human acceptance and the adoption study
remain incomplete. No deployment permission is granted.
