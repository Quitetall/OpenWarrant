---
schema: oh.war/atom/v1
warrant_uuid: 01a0a2da-52d6-75b0-835e-7cef44cb0a1f
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The human receives raw progress JSON while the existing HTML view reports legacy
resolution rather than prototype implementation completion.

## Desired Outcome

One read-only HTML viewer works offline and through a loopback server refreshed
from current repository records at a configurable interval. Work-stop responses
link this viewer. Completed work and verification remain separate.

## Scope

CLI viewer adapter, offline snapshot, loopback refresh, explicit agent work-report
display, local evidence links and progress skill guidance.

## Non-goals

No compiler, database, remote hosting, multi-user authentication, write API, signing,
authority inference or implementation of future SDK lifecycle/assurance records.
