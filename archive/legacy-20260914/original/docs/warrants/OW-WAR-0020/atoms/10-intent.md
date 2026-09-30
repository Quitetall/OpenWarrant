---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-766a-916c-656c6b37998b
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

This is the Warrant that makes 'an unaskable gate cannot pass' true rather than
aspirational — SAS §99 acceptance criterion 19.

§44.1 separates ASKABILITY from RESULT: a gate that cannot be asked has no
verdict, and reporting one is a fabrication. §44.2's execution statuses are
exactly the vocabulary §96.4 requires migration to preserve — malformed,
foreign_working_directory, missing_tool, missing_script, missing_crate, mutating,
timeout, failed, passed, not_run — and the rule that 'could not ask' must never
collapse into 'failed'.

## Desired Outcome

Gate runs produce receipts with an explicit execution status and a verdict that exists only when the gate was askable. A required unknown blocks resolution. Invalidation propagates to dependent resolutions.

## Scope

Askability (§44.1), execution status (§44.2), verdict (§44.3), required passing result (§44.5), gate receipts (§44.6), shell strings (§44.7), mutating gates (§44.8), and invalidation (§45).

## Non-goals

- No Katana execution; a gate run here is local. Remote execution is OW-WAR-0026.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-054` — Complete; governing section in Basis.
- `WAR-SAS-RQ-057` — Complete; governing section in Basis.
