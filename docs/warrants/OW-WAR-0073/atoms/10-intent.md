---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32db-7240-9a9d-cef5e5b41a38
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Two surfaces show the corpus and neither is where the work happens. The Pages
app is rich and read-only in a browser that cannot hold a key. `war console`
is one line-based screen that does the acts but shows almost nothing: no
corpus state, no obligation ladders, no receipts, no journal, and no way to
look at *why* a Warrant is not resolvable while deciding whether to sign it.
So the owner reads one surface and acts on another, and the friction the
console removed from typing is still there in navigation.

The owner's ask on 2026-09-12, after the console landed: "make a real ratatui
for this", full dashboard.

## Desired Outcome

`war tui`: one terminal application over the same records every other command
reads. Panes for the signing queue (checkboxes, presets, the batch act), the
question hotline, the frontier with its blocked edges, corpus status by rung,
obligation ladders with their verifications and admissibility, gate receipts
by class, and the journal as a timeline. Keyboard-driven, live-refreshing,
and — under SAS 1.1.0 §76.6 — a **rendering**: it issues commands and holds no
authority. Every act it starts is the same `war sign --ssh-sign` a hand would
run, and the confirm dialog stays the human's.

## Non-Goals

- Holding a key, caching a passphrase, or any path that signs without the
  agent's own confirmation.
- Replacing the Pages projection: the browser view stays, for people who are
  not at this workstation.
- Mouse support, themes, or a configuration surface for layout in 1.x.
- Writing any record the CLI cannot already write. Every write goes through an
  existing seam.
