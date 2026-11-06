# OpenWarrant Mission Control: Two-Pillar Interactive Terminal UI

Owner-directed working vision, 2026-11-04.
Design specification for the next-generation Ratatui interface (`OW-WAR-0112` successor).

---

## 1. Problem & First Principles

### The Problem with the 12-Tab Viewer
The initial OpenWarrant terminal viewer (`crates/openwarrant-cli/src/tui/mod.rs`) faithfully mapped the internal protocol taxonomy to twelve separate tabs: `Setup`, `Help`, `Queue`, `Questions`, `Frontier`, `Corpus`, `Obligations`, `Evidence`, `Journal`, `Roadmap`, `Projects`, `Tickets`.

While technically complete and structurally safe (clean terminal restoration, zero key leakage), this layout imposes severe friction:
1. **Protocol internals are exposed as navigation**: Low-level validation rungs (`Corpus`, `Obligations`, `Evidence`) compete for screen space with daily developer tasks (`Tickets`, `Queue`).
2. **Passive viewer, not a workspace**: The human can observe and batch-sign, but cannot draft, create, assign, dispatch, or answer questions without exiting to a shell prompt.
3. **Subprocess bouncing**: Operations like `Setup` (`war init`) or roadmap editing exit raw mode, shell out to the CLI, prompt "press Enter to return", and reboot the TUI.
4. **Zero visibility into autonomous agent execution**: As agents (Claude Code, Gemini, local models) work concurrently in isolated worktrees, the human cannot see their live edits, heartbeats, token burn, or write-scope boundaries.

### The First-Principles Solution: Two Core Jobs
From first principles, a developer or engineering lead sitting in front of OpenWarrant needs only two things:
```
┌────────────────────────────────────────────────────────────────────────┐
│  [1] OBSERVE (Information, Topology, Telemetry & Live Execution)       │
├────────────────────────────────────────────────────────────────────────┤
│  [2] ORCHESTRATE (Draft, Create, Assign, Dispatch, Unblock & Sign)     │
└────────────────────────────────────────────────────────────────────────┘
```

The entire interface collapses from a 12-tab pager into a responsive, high-density **Two-Pillar Mission Control** built on Ratatui.

---

## 2. Pillar 1: OBSERVE (The Situational HUD)

The Observe mode provides total operational awareness at a glance: where the program stands, where parallel agents are working, what they are spending, and what they are blocked on.

### 2.1 Top-Line Telemetry & Fleet Vitals
Positioned permanently in the header bar:
- **Token Burn & Spend**: Total tokens consumed across all sessions, dollar-denominated cost estimate based on active model rates, and burn velocity (tokens/min).
- **Fleet Vitals**: Active agents count, active worktree count, and write-scope collision warnings.
- **Global Milestone Progress**: High-level completion gauge (e.g. `Phase 1: 68% · 24/35 Warrants`).

### 2.2 Milestone Progress Bars & Task Column (Left Pane)
A structured columnar view of roadmap milestones:
- Each milestone renders with a Ratatui `Gauge` showing progress toward the milestone exit criteria.
- Under each milestone, tasks/Warrants are listed with:
  - Stable task ID and descriptive title.
  - State badge: `DONE` (green), `ACTIVE` (cyan), `READY` (white), `BLOCKED` (yellow/dim).
  - **Token Burn & Cost per Task**: Displayed inline (e.g. `$0.42 · 28k tok`).
  - Distinction between *unverified completed work* and *formally accepted work*.

### 2.3 Task Dependency Tree / DAG (Center Pane)
A visual hierarchy rendering dependencies, parallel tracks, and critical paths:
- Shows which tasks are unblocked and executable in parallel vs. which tasks form the critical path.
- **Active Task Metadata**: For any task currently claimed by an agent, the tree node displays:
  - **Agent / Model Name**: e.g., `claude-opus-5.5-w1`, `gemini-3.8-flash`.
  - **Last Heartbeat**: Relative time since last agent activity (e.g., `4s ago`).
  - **Lease Freshness**: Countdown to claim TTL expiry (e.g., `fresh (3m 42s left)`).
  - **Claimed Worktree Directory**: e.g., `.worktrees/wt-78a2`.
  - **Assigned Write Path Scope**: e.g., `crates/openwarrant-cli/src/context/**`. Collisions between overlapping active writers are highlighted in bright red.
- **Direct Dispatch Action**: Highlighting any unblocked node and pressing `d` immediately transitions to dispatching an agent to that node.

### 2.4 Live Transcript Streaming via Herdr Integration (Right Pane)
Integrates directly with the local `herdr` daemon (`~/.config/herdr/herdr.sock`):
- Connects over the UNIX domain socket to query running agent panes (`herdr agent list`).
- On a 250ms polling tick, streams terminal output and live file diffs from the focused agent (`herdr agent read <id>`).
- Lets the human watch Claude Code or other agents as they reason, call tools, and edit code in their isolated worktree.
- Allows switching between agent transcripts via `[` and `]`.

### 2.5 Unified Questions Hub (Bottom Drawer)
When any agent encounters an ambiguity, missing requirement, or design decision (via `oh.war/intake-questions/v1` or an interview hook):
- The question immediately surfaces in the centralized Questions Hub across all worktrees.
- Shows: Target task, asking agent, question text, and the agent's recommended answer.
- Pressing `a` opens an inline answering box to resolve the question on the spot, unblocking the waiting agent without leaving the TUI.

---

## 3. Pillar 2: ORCHESTRATE (The Action Center)

The Orchestrate mode gives the human active control over drafting, dispatching, unblocking, and certifying work.

### 3.1 Draft & Create Work (NLP & Direct)
Creates new Warrants/tickets as single-source Markdown documents:
- **NLP Generation**: An input box allows typing a high-level engineering request (e.g. *"Add rate limiting with token bucket fallback to dispatch"*). Pressing `Enter` runs OpenWarrant's deterministic drafter (`war plan --draft`) to propose structured items, obligations, and suggested gates.
- **Direct Form**: An instant editor modal for typing a title, checklist items, and dependency links (`--after <id>`).
- Written directly to `docs/tickets/` or `docs/warrants/` without manual file scaffolding.

### 3.2 Task Dispatch & Worktree Provisioning
Provides a clean interface for assigning work to autonomous workers:
1. **Target Selection**: Pick an unblocked task from the tree.
2. **Model / Harness Selection**: Choose from configured performers (`claude-opus-5.5`, `claude-sonnet-4.6`, `gemini-3.8-flash`, local runner).
3. **Write Scope Assignment**: Declare the file/directory glob the agent is permitted to touch (e.g. `crates/openwarrant-cli/src/dispatch/**`). OpenWarrant verifies that no active lease overlaps this scope.
4. **Execution Mode**: Choose between an isolated Git worktree or the main workspace.
5. **Launch**: Spawns the worktree, records the scoped claim in the journal, and launches the agent inside Herdr (`herdr agent start`).

### 3.3 Question & Blocker Resolution
- Complete interview interface for structured design decisions (`war plan interview <target>`).
- Supports multi-choice selection, freeform rationale entry, and default acceptances.
- Records the answer directly into the Warrant's intake record, updating the task's context packet for subsequent agent turns.

### 3.4 Approval & Signing Queue (Human Authority Seam)
Where finished work is presented for human sign-off:
- Lists completed tasks awaiting sign-off, SAS amendments, and resolution receipts.
- **Diff & Evidence Inspector**: Selecting an item displays the git diff, test/gate receipts, and compliance verification report in the inspector.
- **Batch SSH Confirmation**:
  - `Space` checks rows, `a` checks all.
  - Pressing `s` executes a single batch signing operation (`war sign --batch=... --ssh-sign`).
  - Terminal raw mode is suspended cleanly for the hardware security key / `ssh-add -c` confirmation prompt and resumed immediately upon completion.

---

## 4. Architecture & Invariants

### 4.1 Safety Invariants
1. **Zero Key Delegation**: The TUI never touches private keys, never invokes `ssh-keygen`, and never accesses `SSH_AUTH_SOCK` internally. Every signature remains an interactive human confirmation through `ssh-add -c`.
2. **Single Source of Truth**: The TUI computes no shadow state. It strictly consumes existing canonical data providers (`status::build`, `ticket::claim`, `spend::report`, `watch::fingerprint`, `context::adapter`).
3. **Terminal Integrity**: Entered and exited through an immutable `Drop` guard. Panics and exits cleanly restore the alternate screen, cursor, and terminal echo.

### 4.2 Herdr IPC Interface
- Connects to `~/.config/herdr/herdr.sock` via `std::os::unix::net::UnixStream`.
- Protocol: JSON-RPC request/response matching Herdr's API schemas (`agent.list`, `agent.read`, `agent.prompt`).
- Fallback: When Herdr is not running, the transcript pane displays standard Git diffs and journal progress for the selected worktree.

### 4.3 Input & Navigation Model
- **Mode Toggle**: `Tab` or `1` / `2` switches between **Observe** and **Orchestrate**.
- **Global Keys**:
  - `j` / `k` or `↑` / `↓`: Navigate trees and lists.
  - `h` / `l` or `←` / `→`: Navigate across split panes.
  - `/`: Fuzzy filter across visible tasks and agents.
  - `?`: Toggle contextual keyboard shortcuts overlay.
  - `q`: Cleanly exit the app.
- **Observe Actions**:
  - `Enter`: Inspect selected task / zoom transcript.
  - `[` / `]`: Cycle between active agent transcripts.
  - `a`: Quick-answer focused question in the bottom hub.
  - `d`: Jump to Orchestrate with the selected task pre-filled for dispatch.
- **Orchestrate Actions**:
  - `c`: Create new task.
  - `s`: Sign/approve checked batch.
  - `x`: Run automated remedy.

---

## 5. Implementation Roadmap

1. **Step 1: State Machine & Split Layout**
   - Replace the 12-variant `Pane` enum in `crates/openwarrant-cli/src/tui/mod.rs` with `ViewMode::Observe` and `ViewMode::Orchestrate`.
   - Implement the master-detail split layout (Left: Milestone list & DAG, Right: Transcript/Inspector, Bottom: Questions drawer).

2. **Step 2: DAG, Telemetry & Claims Renderer**
   - Wire `context::adapter::Graph` into a tree renderer.
   - Embed token burn, last heartbeat, worktree path, and lease countdown into active tree rows.

3. **Step 3: Herdr Socket Client**
   - Implement `crates/openwarrant-cli/src/herdr.rs` for IPC over `herdr.sock`.
   - Stream live terminal output into the Ratatui transcript widget.

4. **Step 4: Interactive Orchestration Flows**
   - Implement inline task creation drawer (NLP prompt + direct input).
   - Implement dispatch modal connecting to `ticket::claim_scoped_at` and `herdr agent start`.
   - Wire the in-TUI question answerer and review queue.
