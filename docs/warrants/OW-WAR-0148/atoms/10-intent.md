---
schema: oh.war/atom/v1
warrant_uuid: 01a0feb6-ab80-73a5-abcb-7ade9cfaaeaa
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The owner, 2026-10-02: OpenWarrant should be an engineering planning kit built
on a standalone compiler and reusable primitives, where "verification and more
is a matter of assigning it to that type of document." The proposal is
`/mnt/4tb/openwarrant-semantic-model.md`.

Today that is true only in part, and the gaps show in four places.

- **Capabilities are implicit.** Every directory under `docs/warrants/` gets
  the whole contract path. That means `check_one`'s rule families (`check.rs`),
  the 13 §56.1 checks (`resolve.rs:92-224`), `next::derive`'s acts and the
  `WarrantRung` ladder. A ticket avoids that path only because it lives in
  another directory. Behaviour is chosen by profile *name* in
  `sign.rs:1353`, `resolution_cmd.rs:644`, `standing.rs:495` and
  `ticket/mod.rs:1847`.
- **A decision Warrant can never resolve.** Check 12
  (`resolve.rs:521-540`) requires a milestones atom the decision profile does
  not have, so OW-WAR-0071 is stuck.
- **There is no shared compiled model.**
  - `war compile` builds the corpus status about six times.
  - `resolve::assess` runs in both status and frontier.
  - The web UI, TUI and MCP server each rebuild everything.
  - Two builders give different Warrant states for the same Warrant:
    `status.rs:197-215` and `compile.rs:729-740`.
- **The signature does not cover the profile.** A Warrant names its profile
  by string, and the profile file's bytes are not part of anything signed. A
  profile edited after authorization changes the rules a signed Warrant is
  held to, and nothing notices.

Records and relations exist only as special cases: obligations, deliverables,
phases, ticket items and blockers, parents, supersession. No general record
can be addressed and related, and no command answers "what does this change
affect?"

## Desired Outcome

- **M1: one compiled corpus model, read by every client.**
  - It is built once per process.
  - `war model --json` emits `oh.war/model/v1`: records, relations, states
    and diagnostics.
  - `compile`, `check`, `status`, `next`, `console`, the web UI, the TUI and
    MCP read it.
  - Their outputs are byte-identical to before, except for the state
    disagreement, which is fixed.
  - `war prepare` runs a gate once per commit and shares the run.
- **M2: a document's type chooses its capabilities.**
  - `profiles/*.toml` declare `capabilities` from a closed kernel set, and
    every per-kind behaviour reads them.
  - A decision Warrant can resolve.
  - A new Warrant's manifest pins its profile file's digest, so the
    signature covers its type. A profile changed under a signed Warrant is
    reported.
- **M3: records and typed relations.**
  - Authored record atoms carry stable ids and per-record revisions.
  - Relations use a closed core set of kinds plus inert namespaced kinds.
  - `war impact <record>` names what a change to a record affects.
- **M4: states.** A fixed kernel set, computed or authenticated, plus
  declared states that refine a fixed state and never stand in for one.
- **M5: tickets on the kernel, and ticket features.**
  - Ticket items are records, and blockers are `depends_on` relations.
  - Ticket types, labels, epics, filters and search are available.
  - `war create --issue <n>` makes a ticket from a GitHub issue. Write-back
    on done (comment, close) is approved by the owner (2026-10-02) and
    opt-in.
  - A password-reset demonstration compiles a ticket, a roadmap view, a
    Warrant draft and an agent packet from one set of records, and `war
    impact` names what a changed requirement affects.

## Non-goals

- **Closing the backlog.** The 113 authorized, unresolved Warrants stay
  parked. Verification and `war prepare` remain opt-in.
- **Moving any existing contract digest.** `WarIr`, the digest's inputs and
  the schema pack version (0.2.0) are unchanged. The profile pin applies
  only to manifests written after it lands.
- **PRD, architecture and test-plan document types.** They follow once the
  ticket, roadmap and Warrant types share records in the demonstration.
- **Codex's `codex/reviewed-subject-binding`.** It is not touched.
- **Granting authority by data.** A type can require an act. It can never
  supply one, loosen independence, or invent an act kind.
