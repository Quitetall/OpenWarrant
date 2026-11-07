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

## Waiting on the other side

A ticket can depend on an item or a clause, here or in a named repository:

```sh
war create "Use the new step" --depends-on hpr:t-abc
```

This writes `x.depends_on`, listed in `requires:`. The item stays out of
`war view ready`, and `war view prime` says why in one line:

```
held: hpr:t-abc: done upstream, not pinned here (pin 5f951cf85a11, upstream 9cbe852edd4d); move the pin
```

That line appears once `origin`'s copy is done and fetched, and the pin isn't there
yet. It's what lets two repositories release on their own cadence. Moving the pin
and committing is what releases the item, in a ticket of its own if it's more than
a bump. An explicit `war start` still works: nothing stops ordinary work.

| The dependency is | `ready` | At `war done`, a Ticket | At `war done`, a Warrant or Order |
|---|---|---|---|
| Met: the pin holds it done | yes | closes | closes |
| Open, tbd, or done upstream only | no | warns and closes | refused, nothing written |
| UNKNOWN | no | warns and closes | UNKNOWN, stays open |

## Keeping an interface honest

`war check` counts open clauses across the interfaces this repository reaches, its
own and those at its pins, and flags any past its `due` date. `--verbose` shows the
count. It's a warning, never a failure.

`war check --clauses` runs each clause's `check` here, the same command on both
sides. A clause with no `check`, or a `tbd` one, reads UNKNOWN: nothing was
established, and nothing failed. Keep it for CI, because UNKNOWN exits 2.

Name a clause as a dependency the same way as an item, and a `tbd` clause holds the
work out of `ready` until it's agreed and pinned:

```sh
war create "Reuse handles" --depends-on hpr:CL-handle
```

`held: hpr:CL-handle: tbd (owner hpr, due 2026-11-20)`. A `tbr` clause is decided,
so it doesn't hold anything.

Each side records what it conforms to, as a digest of the interface revision:

```sh
war check --clauses --record     # writes docs/interfaces.lock; refused if a clause failed
```

Commit that file. `war status` then shows drift in one line, and stays quiet about
upstream until the pin moves:

```
Interfaces: drift: hpr:docs/records/physics.md is 1.1.0 (9e1f...), this side conforms to 1.0.0 (41ab...)
```

A clause's `check` from a pinned repository runs on this machine, like any code the
pin brings in, and only when you ask for `--clauses`.

## Changing an interface

A change is a ticket of type `change-request` naming its clauses, classed by its
effect. It lives in the repository that owns the interface:

```sh
war create "Reuse handles in one frame" --change breaking --clause CL-handle
war create "Add a mass getter" --change compatible --clause CL-mass
```

| Class | Level | What closes it |
|---|---|---|
| `compatible` | Warrant | Its trigger: the clauses' checks, or `--test` if none has one |
| `breaking` | Order | One sign-off per party of the interface, by different people |

A breaking change gets a checklist item per party, `accept: hpr` and `accept: wmd`.
`war done` refuses it and names the command a person runs, `war sign <id>/<item>
--ssh-sign`, in the owning repository, whose `allowed_signers` lists both owners. An
agent drafts the change and never signs it.

`war plan impact` lists the running claims a change reaches on both sides, for a
change request or a clause:

```sh
war plan impact t-abc            # from the owning side
war plan impact hpr:CL-handle    # from the side that depends on it
```

The owning side names its users so it can look. `downstream` says the dependency
runs the other way, so nothing is read through a pin:

```toml
[repos.wmd]
path = "../wmd"
downstream = true
```

A side with no checkout on this machine reads UNKNOWN, not "no claims".

## An example: WMD and HPR

HPR is the lower layer, a submodule of WMD at `vendor/hpr`, and owns
`docs/records/interfaces/physics.md`.

1. **The agreement.** HPR's agent adds `CL-handle`, `status: tbd`. WMD's agent starts
   `war create "Pool bodies" --depends-on hpr:CL-handle --test "cargo test pool"`. It
   stays out of `ready`, and prime says who settles it and by when.
2. **The work.** HPR settles the clause and ships, then pushes. WMD fetches:
   `git -C vendor/hpr fetch`. Prime now says *agreed upstream, not pinned here*.
3. **The pin.** A WMD ticket moves the pin and commits. The held item is released,
   `war check --clauses` runs the same tests on both sides, and
   `war check --clauses --record` clears the drift line.
4. **A later break.** HPR opens `--change breaking`. `war plan impact` names WMD's
   running claim, both owners sign, and that claim hears the clause changed once it
   reaches it.

## Limits

Reads never fetch: nothing is known of the other side until `git fetch`. Only a
checkout on this machine shows the other side's claims. Dependencies run one way:
one side pins, the other is `downstream`, and an adapter handles anything that
would make a cycle.
