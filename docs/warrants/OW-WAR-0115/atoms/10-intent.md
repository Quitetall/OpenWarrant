---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd30-eabb-76a2-b970-42724167ad84
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war` opens the app only inside a repository, and only that repository. On
2026-09-23 the owner asked for the app, the "console or hub", to open from
anywhere: pick a project inside it, and see an overview of everything, its
status and its progress.

The same day showed why this matters beyond convenience. A command handed
to the owner failed because their shell was in one checkout and branch while
the Warrant lived in another, and because the `war` on their PATH was an
older installed build. A human juggling checkouts, branches and binaries is
the friction this program exists to remove.

## Desired Outcome

`war` from any directory opens the hub:

- **Inside a repository:** it opens that project, as today.
- **Anywhere else:** it opens the **Projects** pane, with every
  OpenWarrant repository this user has used. Each row shows the program
  name, path, branch, SAS in force, acts awaiting a signature, open blocking
  questions, and roadmap progress (phases achieved of total, once
  OW-WAR-0114's record exists; otherwise the §98 Objectives).
- **Enter** opens a project.
- **`p`** switches project from anywhere in the app.

Projects are remembered without anyone registering them. Running any
`war` command inside a repository adds it to a per-user list. `war
projects` lists, adds and forgets entries. A project whose path is gone is
shown as missing, never silently dropped.

Every command the hub hands a human runs with that project's `--root`, so
the directory the shell happens to be in never matters.

The hub also says which `war` it is (version and path), and warns when the
one on PATH is older than the running one. That is the other half of the
2026-09-23 failure.

## Non-goals

- Remote or shared registries. The list is one user's, on one machine.
- Acting across projects in one gesture. Signing stays per project, one
  child `war sign --root <project> …` at a time.
- Installing or updating `war`. The hub reports a stale PATH binary and
  names the command that fixes it; it does not replace binaries.
