# Current model and implementation baseline

The owner adopted the newer model as the baseline for continued work on
2026-11-07. The alpha.6 implementation plan is historical context. Remaining
work is reconciled against this model and the lean programme, `t-3c16c`.
Format edition acceptance and release qualification follow their recorded
governance.

## One item that gains requirements

A work order describes one outcome and keeps its identity as detail grows.
Its completion level is derived from its effective requirements.

| Level | Completion requirement |
| --- | --- |
| Ticket | The worker reports completion. |
| Warrant | Every required current check or trigger passes. |
| Order | Required checks pass and a human supplies the required signature. |

A title is enough for a Ticket. Tests, milestones, dependencies, context and
signature requirements can be added to the same item. Repository requirements
contribute to the effective level. An unread requirement stays UNKNOWN.
Permissions describe where and how work may execute, independently of this
level. The meaning and covered revision of each signature come from its act.
The Verified mark requires the applicable independent evidence and secure human
acceptance of the exact result.

## Documents and authored masters

The adopted direction has two kinds, work orders and documents. The proposed
Format revision 4 aligns their tiers.

| Document | Work order |
| --- | --- |
| Spec | Roadmap |
| ADR | Order |
| Design, including an interface contract | Warrant |
| Guide | Ticket |

An authored master holds the maintained source. Views supply navigation,
progress and readable project overviews derived from those sources. A source's
stable ID carries identity, while its filename slug supplies a readable name.
Signing a Spec or Roadmap covers that document's own declared subject. Each
contained item's requirements and evidence determine its own status.

The shared subject loader on main reads the supported layouts into one model.
The proposed master layout is `docs/work/<id>/<id>-<slug>.md`, with state beside
the source. Existing documents retain their recorded identities and history.
Further revision 4 constraints and views proceed through the pending format
work and its conformance cases.

## One daily loop for humans and agents

The daily commands remain `init`, `create`, `start`, `done`, `status`, `check`.
Interactive entry points serve humans. JSON and MCP expose the same operations
to agents. The default MCP surface stays small and advanced tools are disclosed
when needed.

Starting work delivers the required context and records its execution place.
Questions reach their answerer and answers return to the asking worker. Live
changes identify the affected context and required next action. Completion
uses the item's current requirements and retains the actual result history.

## Continue from current work

Use the current records and implementation to assess every remaining alpha.6
item. Carry forward missing behaviour and evidence into the lean programme.
Keep completed code, historical evidence and pending qualification separately
identified. Integrate the active format, views, question and place work through
the repository's PR and CI process.

## Sources

- [Daily model and implemented commands](../TICKETS.md)
- [Shared document loader and master layout](one-format.md)
- [Standard direction and historical SAS standing](../format/README.md)
- [Format edition governance](../format/GOVERNANCE.md)
- [Proposed revision 4 and ladder](https://github.com/Quitetall/OpenWarrant/blob/512f55456501e372a3eed69543ffd1f5b84a5dee/docs/adr/atoms/OW-ADR-0034-format-revision-4-masters-and-ladder.md)
