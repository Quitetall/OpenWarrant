# Interfaces

Two repositories that depend on each other can't collide on files. They collide
on the boundary between them. An interface document is that boundary, written
down: numbered clauses both sides build against, so both can work on one feature
at the same time.

It takes one command to add, and it's as light as a ticket. A clause exists to
stop a collision or to make work faster. If it does neither, leave it out.

```sh
war plan types add interfaces
```

Put the document in `docs/records/<area>/`, in the lower layer: the side the other
depends on. It owns the interface.

```markdown
---
format: openwarrant/1.0
profile: interface
version: 1.0.0
owner: hpr
parties: hpr, wmd
summary: "What the game engine may rely on from the physics runtime"
---
# Physics interface

## CL-step · clause — A step advances the world by exactly dt
status: agreed
verify: test
check: cargo test --test contract_step

`World::step(dt)` advances simulated time by `dt`, and no more.

## CL-handle · clause — Rigid body handles are stable across a step
status: tbd
verify: test
owner: hpr
due: 2026-11-20

Whether a removed handle may be reused in the same frame is not decided.
```

## Clauses

| Field | Required | Meaning |
|---|---|---|
| `status` | yes | `agreed`; `tbd`, not decided; `tbr`, decided but to be reviewed |
| `verify` | yes | How it's proved: `test`, `analysis`, `demonstration` or `inspection` |
| `check` | no | The command that proves it, the same on both sides |
| `owner` | no | Who settles an open clause |
| `due` | no | When, as `YYYY-MM-DD` |

Each clause states one fact the other side can observe. An undecided value is a
`tbd` clause with an owner and a date, never a gap for someone to guess at.

`war check` refuses a clause with no `status` or `verify`, and a document with no
`owner`, `version` or `parties`.

## Using one

An agent reads the clause it needs, not the whole document:

```sh
war context CL-handle
```

Clause ids are global within a repository, so give each interface its own prefix
when you have several.

Run a clause's `check` in both repositories' CI. The lower layer ships first, and
the other side moves its pin in a ticket of its own, with that test as its trigger.

## Across repositories

Name the other repository in `openwarrant.toml`, once on the side that depends on
it. A submodule says its own commit:

```toml
[repos.hpr]
path = "vendor/hpr"
url = "https://example.com/hpr.git"
```

A sibling checkout adds `pin = "<commit>"`; a Cargo git dependency has its commit
in `Cargo.lock`, found by `url`. The commit is what this repository has committed,
never a working tree, so one commit reads one way. `war status` shows it.

WMD then reads HPR through the pin, from HPR's git objects at that commit:

```sh
war context hpr:CL-step          # one clause
war context hpr:t-abc            # one item
war view prime hpr:t-abc         # one item in full
```

Nothing here touches the network or edits the other side. A repository that isn't
declared or isn't on this machine, or a commit it doesn't hold, reads UNKNOWN. An
id the pin doesn't hold is refused, never searched for. Local commands carry on.

## What's next

Showing work as *done upstream, not pinned here* is W6 of the lean programme,
still landing. Until then, link a GitHub issue per feature, with a sub-issue in
each repository.
