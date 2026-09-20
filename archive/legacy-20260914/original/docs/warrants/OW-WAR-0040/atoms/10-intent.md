---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a20-77b7-800c-673a6394651b
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§20 of the resolved decisions says: define the Liminal protocol now, ship the
constrained Markdown adapter first. The Markdown adapter shipped. The Liminal
protocol does not exist, and §98 Phase 8's exit is that Liminal becomes the
SINGLE production document semantic compiler — meaning the Markdown adapter is
eventually retired.

`Quitetall/liminal` exists as a repository, created 2026-08-18, with no checkout
on this host.

## Desired Outcome

A Liminal adapter produces the same canonical IR as the Markdown adapter for the same sources, proven by a measured parity harness. Cutover happens only after parity is demonstrated, not asserted.

## Scope

The Liminal adapter (§82.2), adapter parity (§82.3), and cutover (§82.4, §97.5).

## Non-goals

- No replacement of the Markdown adapter until parity is measured. §97.5 keeps the old compiler as a compatibility oracle during that period.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-061` — Complete; governing section in Basis.
