---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-75b4-9586-0aae240f38bc
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A Warrant that exists only as canonical JSON is unreadable by the person who has
to authorize it. The SAS's answer is a generated Markdown parent — one complete
human document projected from the same Basis (§15.2).

The danger in that answer is the reason this Warrant exists separately: a
generated document committed to Git looks exactly like an authored one. Someone
will edit it. Without a drift check, that edit becomes an invisible fork between
what the document says and what the Warrant means, and the document is the thing
people read.

## Desired Outcome

`war compile` produces the human parent and `WAR.json` from one Basis. Every
generated file carries the §17.1 header. `war check --generated` fails when a
committed view differs from a fresh compilation, and that failure has been
observed.

## Scope

The full-Warrant Markdown projection of §103, `WAR.json`, the generated header,
`war compile`, and drift detection.

## Non-goals

- No Work Order or Assurance Case projections yet. §17.5 lists nine views;
  this Warrant delivers `full_warrant` and `canonical_json`.
- No source maps, so no editor integration that maps a parent edit back to its
  atom (§17.4). The minimal v1 CLI is explicitly permitted to simply refuse
  direct parent edits, and it does.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-012` — generated atoms and parents are not directly editable.
  Complete: the drift check is what makes this enforceable rather than advisory.
- `WAR-SAS-RQ-014` — full WAR Markdown and canonical JSON compile from one
  Basis. Complete, jointly with OW-WAR-0003.
- `WAR-SAS-RQ-075` — generated views are drift-checked. Complete.
