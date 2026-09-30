---
schema: oh.war/atom/v1
warrant_uuid: 01a0948f-2334-7dc2-abe2-0839b7d82e1d
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The owner's workflow, stated on 2026-09-12: a master record first (the SAS
and the glossary), a grilling for the main decisions, then a descent from
the whole program to features to Warrants on one master tracker; each
Warrant started by a tick on a numbered list; each running agent handed the
relevant context and a hotline to the owner, who answers blocking questions
from every agent in one place, front-loaded so downtime is minimal; the
harness itself inside the system. Today the pieces exist as commands
(`war init --program`, `/war-grill`, `war frontier`, `war dispatch`, `war
run`, `war sign --list`, `war watch`) and the joins are missing: no board,
no tick that becomes a signature, no question record, no runner for agent
stages.

## Desired Outcome

`war board` renders the master tracker (program, objectives, Warrants,
stages, the numbered approval list); a tick on that list is a `war sign
--ssh-sign` at the workstation and nothing less; agents ask through `war
ask` and read `war answers`, the owner answers in one queue that `war watch`
raises; `war perform` runs an agent stage through a configured performer
with its Dispatch and records the submission; `war questions` front-loads
every open question across Warrants so the owner answers in batches.

## Scope

The commands above, their records under the Warrant directory, MCP tools for
`ask` and `answers`, the board as a view of the existing projections.

## Non-goals

- A tick in a browser that signs by itself: the signature stays a key
  holder's act at a keyboard. The board produces the command; the human runs it.
- Scheduling and containment of performers.

## SAS and Roadmap Traceability

`sas://WAR-SAS-RQ-042`, `sas://WAR-SAS-RQ-044`, `sas://WAR-SAS-RQ-070`;
`roadmap://OW-PHASE-5/dispatch`.
