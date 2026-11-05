# OpenWarrant 1.0.0-alpha.6 — Two-Pillar Mission Control TUI, Herdr IPC, and Lean Agent Loops

This unverified alpha updates the planning and tracking CLI from alpha.5. Ordinary work
still needs no signature: `war init`, `war create`, `war claim`, `war done`.
Human signing remains a separate assurance action.

What is new for a user:

- **Two-Pillar Mission Control TUI (`war tui`).**
  - **Observe (`o`)**: High-level situational telemetry HUD, milestone progress bars,
    task dependency DAG with critical path calculation and write-scope collision warnings,
    live writer heartbeats and lease freshness, live streaming agent transcripts via Herdr
    Unix socket IPC, and a unified question/blocker resolution drawer.
  - **Orchestrate (`O`)**: Direct work item creation (form-based and natural language drafting),
    multi-model agent dispatch (Claude Opus 5.5, Sonnet 4.6, Gemini 3.8 Flash, Gemini 3.5 Pro,
    local runners), question resolution, and batch SSH signing queue (`[s]` batch sign).
  - Terminal-only guard (`tui.no-tty`), zero signing key material in memory, strictly shelling out
    to child `war sign --ssh-sign` processes.
- **Herdr IPC Socket Integration (`~/.config/herdr/herdr.sock`).**
  OpenWarrant can now connect directly to the Herdr multi-agent daemon to stream active agent
  sessions, real-time command output, tool invocations, and live transcripts directly into
  the TUI HUD.
- **The Two-Call Agent Loop.**
  - `war start` takes the next ready Warrant and outputs an actionable, token-budgeted brief:
    description, open checklist items, journal notes, dependencies, and closing instructions.
  - `war done` ticks every open item in one transaction or refuses cleanly if checks fail.
  - Worktree actor isolation: in a git worktree, the default actor is derived from the
    worktree name (`claude@w2`), preventing lease and claim collisions across concurrent agents.
- **Title and Fuzzy Search for Ticket Dispatch.**
  `war start`, `war done`, `war view prime`, and `war note` accept words of a Warrant's title
  or labels instead of requiring an exact hex ticket ID (e.g. `war start T8`, `war start monthly budget`).
- **Context Compiler & Document Object Model Integration.**
  Project documentation (`doc:<path>`), tickets, and Warrants share the same unified
  object model (`oh.war/model/v1`), allowing deterministic context budget compilation and
  least-privilege write scope synthesis.

This is an unverified alpha preview, not Stable 1.0. Legacy reconciliation,
independent qualification, secure human acceptance and the adoption study
remain incomplete. No deployment permission is granted.
