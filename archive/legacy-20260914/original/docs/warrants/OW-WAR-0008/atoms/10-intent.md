---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd0-d818-7a39-bd24-88638a843026
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A Warrant has no state. `war check` reports whether a record is well-formed; it
cannot say whether that Warrant is drafted, authorized, executing, blocked,
resolved, superseded, or annulled. The Warrant Overview omits a status column
entirely rather than invent one.

§24 decomposes state into five orthogonal axes precisely so that "blocked" does
not erase "in execution" and "disputed" does not erase "resolved". Collapsing
them into one enum — the obvious shortcut — destroys exactly the distinctions the
decomposition exists to keep.

## Desired Outcome

Every Warrant carries phase, execution condition, common outcome, currency, and resolution standing. Illegal transitions fail closed. The Overview reports real state.

## Scope

The five state axes of §24, the legal transitions of §24.7, the truthful combinations of §24.6, and material-amendment and post-resolution transitions (§24.8, §24.9).

## Non-goals

- No gate execution driving state; that is OW-WAR-0020.
- No KF-side lifecycle; that is OW-WAR-0029. State is computed locally first.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-032` — state is decomposed into phase, condition, outcome, currency, and standing. Complete.
