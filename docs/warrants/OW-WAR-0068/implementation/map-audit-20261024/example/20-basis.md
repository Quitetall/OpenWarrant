---
schema: oh.war/atom/v1
warrant_uuid: 01a16e00-1278-74cc-a8f2-56f185959825
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Context

The synthetic case.json input has retention_mode null. No decision owner has supplied a value.

## Options

- Memory: fast but does not survive process restart.
- Disk: survives restart but requires retention and cleanup rules.
- None: no cache, with no persistence benefit.

## Unknowns

A-001 in rationale.toml records the missing choice as blocking_unknown. It affects only selecting and implementing a retention mode. Collecting bounded requirements can continue.
