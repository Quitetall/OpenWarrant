---
schema: oh.war/atom/v1
warrant_uuid: 01a0da65-9e85-7531-a20e-8e695ecfa9ee
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — create → ready → claim → done needs no signature and no human
- **scope:** a scratch program from `war init --program` (45-tickets.sh),
  every command run with stdin closed and no terminal; and the scratch repos
  of `tests/tickets_cli.rs`. No claim about a repository whose own policy
  adds a gate to tickets (none can today).
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - accepting: a two-item ticket is created, both items claimed and done,
    the ticket reads `done`, the ticked line reads
    `— done by claude, <date>: <note>`, the journal holds `ticket.item_done`;
  - accepting: no `authorization*`, `resolution*` or `*.sig` exists under
    `docs/tickets/`, and `war sign --list` shows the same pending acts
    before and after;
  - refusing: `war create --draft` with no `[plan] drafter_argv` is refused
    `ticket.no-drafter` and creates no ticket; with a fixture drafter the
    items are exactly its two deliverables.

### OBL-002 — a claim is exclusive, named, and stealable only when stale
- **scope:** one item claimed by several actors in one scratch program;
  the default TTL and `claim_ttl_minutes = 0`.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - accepting: eight `war claim` processes started together: exactly one
    exits 0 and one `ticket.claimed` event is journalled (also 12 processes
    in `tickets_cli.rs`, 32 threads × 20 rounds in `claim.rs`);
  - refusing: a second agent's claim of a held item exits 2
    `ticket.claimed-by-other` naming the holder and since when;
  - accepting: past the TTL, `--steal` takes the claim and journals
    `ticket.claim_stolen` with whom it was taken from;
  - refusing: without `--steal` a stale claim is refused with the steal
    hint; with `--steal` a fresh claim is refused `not stale`.

### OBL-003 — done needs your own claim
- **scope:** the same scratch program.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - accepting: done by the holder ticks the one line and releases the claim;
  - refusing: done of an item someone else holds (`ticket.claimed-by-other`),
    of an unclaimed item (`ticket.not-claimed`) and of an unknown id
    (`ticket.unknown`), each exit 2, with no box ticked.

### OBL-004 — the checklist file is the state
- **scope:** checklists written by `war create` and then edited by hand
  (reordered, reworded, a prose line and an id-less item added); the unit
  fixtures in `ticket.rs` (LF, CRLF, no final newline, nested and starred
  items, code fences, links). No claim about Markdown constructs none of
  them use.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - accepting: `war ready` lists the hand-edited items in the file's order
    with the person's wording, the id-less one included;
  - accepting: a `done` changes exactly two lines — its own, and the id-less
    line given an id — and every other byte is unchanged (the plant diffs
    the file; `ticket.rs` tests assert byte-for-byte);
  - refusing: `war check` names a bad box (`ticket.checklist-malformed`),
    a duplicate id (`ticket.item-duplicate`), an `after` naming no item or
    no ticket (`ticket.blocker-unknown`, twice), and a cycle
    (`ticket.blocker-cycle`, unit test), each exit 2.

### OBL-005 — blockers keep items out of ready
- **scope:** `after` on an item of the same ticket, on another ticket's
  item (`t-x/i-y`) and on a whole ticket.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - refusing: while their blockers are open the items are absent from
    `war ready`, and `war claim` of one exits 2 `ticket.blocked` naming what
    it waits on;
  - accepting: once the blocker is done the item appears in `war ready`,
    and the item behind it still does not.

### OBL-006 — the authority layer is opt-in
- **scope:** `war check` on a scratch program holding unsigned tickets;
  the profile registry and manifest validation as unit-tested.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - accepting: `war check` exits 0 with `ticket.well-formed` and no ticket
    finding about authorization, evidence or verification;
  - accepting: `war promote <ticket>` writes a draft delivery Warrant through
    `war new` carrying the ticket's description, records `promoted_to`, and
    the Warrant has no authorization;
  - refusing: a Warrant manifest that names profile `ticket` is refused
    (`profile ticket is a working form`); a second promote is refused
    `ticket.already-promoted`; the registry refuses every malformed working
    form (`role.rs` tests: no core role, a compiler-produced or foreign
    role, a duplicate, an unknown form, `core_roles` without the form, an
    acceptance role, a core profile with a form).

### OBL-007 — prime shows what is left, and compacts what is done
- **scope:** a scratch program with open, recently done and 2020-done
  tickets and a note; `compact_after_days` at its default of 7.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - accepting: `war prime` lists the remaining items with their claims and
    blockers and the recent note, and `war prime <ticket>` counts the done
    items without listing them;
  - refusing: no done item appears as a task line, and the 2020 ticket is
    exactly one line under **Done earlier**.

### OBL-008 — the same loop over MCP, and still no authority tool
- **scope:** `war mcp` in a scratch program; the live tool router.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - accepting: `war_claim`, `war_done` and `war_prime` work a ticket and the
    checklist names the MCP actor (plant; `tickets_cli.rs` adds create,
    add, note and show);
  - refusing: a second actor's `war_claim` is refused naming the holder,
    and `mcp::tests` still find no signing or ingesting tool registered.

### OBL-009 — each ticket command answers fast on this corpus
- **scope:** this repository's working tree (146 Warrants) on the machine
  running the battery; the commands `create`, `ready`, `claim`, `done`,
  `add`, `note`, `prime`, `prime <t>`, `show`, `tickets`, `check <t>`.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:** each exits 0 in under 2000 ms (asserted; measured 5–13 ms
  on 2026-09-25), and the ticket the plant made is removed afterwards.

## Gate Adequacy

Required at `basic`. The load-bearing refusals are OBL-002's second-claim
and OBL-003's unclaimed done: a claim that did not exclude, or a done that
did not require one, would let two agents finish one item, which is the
failure the loop exists to prevent. OBL-006's refusal of a Warrant naming
the working form is what keeps the contract corpus unchanged.
