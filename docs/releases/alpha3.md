# OpenWarrant 1.0.0-alpha.3 — tickets are Warrants, and ordinary work is ordinary

**What changed since alpha.2.** OpenWarrant now works as a fast planning and
tracking kit for agentic development. Authority is still available, but only
where you ask for it:

- **Ordinary coding needs no Warrant.** Everything an agent reads says so
  first: AGENTS.md, the skills, the MCP instructions and empty states. The
  rules for Warrants that require sign-off describe what the tool refuses for
  that Warrant; they never forbid work. Every signing error says how to fix
  it, and ends "This blocks only the sign-off, not your work".
  `war admin doctor` checks your signing setup without signing anything.
- **Tickets are Warrants.** A Warrant's minimum is a title. One id space
  covers light Warrants (`t-…`), directory Warrants (`NS-WAR-NNNN`), and
  OpenSpec or Spec Kit folders read in place. You can bring work over:
  `war admin import beads|openspec|speckit`.
- **A small surface.** `war --help` lists 12 daily verbs and five groups.
  Every earlier command spelling still works, with identical output.
- **Concurrency.** Claims are shared across a clone's worktrees and leased,
  and writes are compare-and-set. Claims can span machines through a git
  ref. In a stress test, 32 checkouts raced for 100 items: each was claimed
  exactly once and no update was lost.
- **How a tick was earned.** Add tests, KPIs or milestones to any Warrant.
  Each tick records whether it was claimed, observed (its checks ran and
  passed), independently verified, or signed. A claim never displays as
  verified.
- **One model, many documents.** Typed records and relations compile into one
  model (`war plan model`). PRD, architecture, test-plan and agent-packet
  projections render from it, and `war plan impact <record>` names what a
  change affects.
- **CLAUDE.md and AGENTS.md** get a small managed block, and their sections
  become records that Warrants can cite.
- **Presets and roles.** `war init --vibe|--team|--regulated` sets how much
  rigour a repository asks for. Under `--vibe`, the first `done` takes three
  commands. `[roles]` mirror GitHub levels, and `war check --pr` (also a
  GitHub Action) asks a PR for an official Warrant at its author's level.
  Signatures are batched at release with `war sign release`, or off.
- **A dependency graph that runs.** `war start <id>` opens one node in its
  own worktree with session-only harness settings. `war evidence go` runs
  the frontier in dependency order under a concurrency cap and budgets,
  using estimates learned from the journal. Other tools (GitHub Agent HQ,
  Gas Town) plug in as executors; their results pass the same checks.
- **A score and a file ledger.** `war status` ends with a score from 1 to
  1000, a seven-dimension scorecard, a level and next steps. A dimension
  that cannot be measured reads UNKNOWN and earns nothing.
  `war check --floor` keeps CI from dropping a level. Small per-file "why"
  atoms compile into a gitignored `.openwarrant/ledger.jsonl`, which reads
  and writes Agent Trace and git-ai notes.
- **Every development document is a type.** Roadmap, spec and ADR are
  ordinary types over today's stores, ROADMAP.md is a projection, and
  release, incident and exit reports ship in the core; `ops` and `quality`
  packs add more. A doc no type claims counts against document coverage
  until `war plan type <file> <type>` adopts it.
- **`war admin host`** is the standalone half of Liminal hosting: pure, with
  conformance fixtures.

## Install

Download the archive for Linux x86-64 or macOS Apple Silicon and its `.sha256`
file from this release. Verify the checksum before extracting:

```sh
sha256sum -c openwarrant-v1.0.0-alpha.3-linux-x86_64.tar.gz.sha256
# macOS: shasum -a 256 -c openwarrant-v1.0.0-alpha.3-darwin-arm64.tar.gz.sha256
mkdir openwarrant-alpha3
tar -xzf openwarrant-v1.0.0-alpha.3-linux-x86_64.tar.gz -C openwarrant-alpha3
./openwarrant-alpha3/bin/war --version
```

Use the absolute path to that `bin/war`, or add its directory to PATH. Check
`command -v war` so an older global install does not take precedence. If you
use the Claude Code plugin, update it too; the plugin version now matches this
release, and `war admin doctor` warns when the two disagree.

## Start once per repository

```sh
war init --vibe                # namespace from the directory name; asks nothing
war create "what this work does"
war done <id>                  # under --vibe, done claims for you
```

A plain `war init` works the same, with `war claim <id>` before `done`.

`war next` shows what is ready. Nothing in this loop is signed. Sign-off,
verification and resolution are opt-in, per Warrant type.

## Upgrading from alpha.2

- Nothing is migrated. Existing ticket files, Warrant directories and signed
  records read byte for byte.
- Old command spellings keep working. Scripts need no change.
- Claims now live in the git common directory. A claim held in an old
  per-worktree location is still honoured.
- Hosted model output gains `instruction` records wherever an AGENTS.md or
  CLAUDE.md exists.

## Limits

This alpha is not Stable 1.0, SAS acceptance, Verified qualification or
permission to deploy.

**Known gaps in what ships:**
- `war admin export beads` writes Beads' format, but no Beads binary has
  read it back.
- The OpenSpec and Spec Kit fixtures were written from those projects'
  documentation, not produced by their tools.
- The TUI and web UI still say "Tickets" in places.

**Gate:** the integration head passed `cargo xtask gate` in 14 steps, with
1597 planted checks and 1455 tests, 0 failed. No crates.io packages are
published by this workflow.
