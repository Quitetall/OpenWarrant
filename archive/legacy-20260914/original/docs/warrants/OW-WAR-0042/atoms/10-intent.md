---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b570-7f57-85b2-0f8189873d9e
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 2's exit is "a vague engineering request produces a reviewable
valid draft without direct model file mutation." OW-WAR-0034 and OW-WAR-0035
delivered the Draft Proposal protocol, §74.3's seven atom operations, §74.4's
eight-step gauntlet, and `war plan` as both halves of §75.2's seam.

No agent has ever been on the other end of that seam. `war plan` emits a request
and stops, and says so. Every test of the return path uses a proposal this
project wrote about itself, which is the performer grading its own work in
another costume.

§91.8's seven tests (52–58) have zero citations anywhere in the repository.

## Desired Outcome

A real drafting agent receives a request over §75.2's protocol, returns a
Draft Proposal it authored, and that proposal is validated, diffed, reviewed and
applied — or refused — without the model ever writing a file.

## Scope

§74 in full, §75's adapter protocol, §71.3–§71.4, and §91.8 tests 52–58.

## Non-goals

- No Katana execution. That is OW-WAR-0045; this is planning only.
- No claim that the agent's drafts are good. §74.8 constrains honesty, not quality.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-071` — Complete; §71.3 governs.
- `WAR-SAS-RQ-072` — Complete; §74 governs.
- Discharges §98 Phase 2 exit and §99 criteria 1, 2 and 3.
