---
schema: oh.war/atom/v1
adr_uuid: 01a1fc0a-4340-759c-9e3e-471e1e3751fe
local_alias: OW-ADR-0037
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
---

# ADR OW-0037: Resources: one build cache, build slots, few worktrees, a cleanup lane

## Status

Proposed 2026-11-20. Directed by the repository owner the same day: agents should
use the machine well. One shared or seeded build cache, not ten cold builds; no
forgotten worktrees; no new abstraction where one exists. The cleanup pass after
the work (docs, architecture, resources) must always be possible, and it is the
default. Ships in alpha.9 as `war` configuration and the `war` extension; no format
revision.

## Context

Most of it exists. `worktrees.rs` registers every worktree `war` makes
(`oh.war/managed-workspace/v1`, under the git common dir) and prunes conservatively.
`war start` writes the harness settings `env` and `.openwarrant/session.json` in
the worktree. Lanes (OW-ADR-0036) order the work. A claim lock is a temp file and a
hard link. What is missing: a cache every worktree shares, a limit on concurrent
builds, a limit on worktrees, an everyday cleanup, and a lane that does the pass.

## Decision

1. **`[resources.env]`**: variables every `war`-made worktree gets. `{common}` is the
   clone's git common dir, so `CARGO_BUILD_BUILD_DIR = "{common}/openwarrant/build"`
   is one cache for all of them. `war start` writes them to the Claude settings `env`
   and to `.openwarrant/env` (`KEY=VALUE`, shell-quoted), and names them in one line
   of the brief.
2. **`war admin seed`** runs `[resources] seed` once in the main checkout with that
   env and records the revision in `{common}/openwarrant/seed.json`. `war start`
   adds one line when the seed is missing or its revision is not an ancestor of the
   base. Nothing builds on its own.
3. **Slots.** `war run <slot> -- CMD…` (canonically `war evidence run`) runs CMD with
   the env while holding a slot: lock files `{common}/openwarrant/slots/<slot>.<n>.json`,
   taken by hard link. `[resources.slots] build = 1` sets how many; an unconfigured
   slot holds one. Waiting, it says once who holds it (actor, pid, since). A holder
   whose pid is gone (or reused, by start time) is taken over, and that is said; a
   holder in another pid namespace is never judged dead. The exit code is CMD's. A
   holder that cannot be read is UNKNOWN: CMD does not run.
4. **Worktrees.** A lane's items share one managed worktree on `war-go/<lane>`
   (`war start --worktree <item>`, or `war start --lane <l> --worktree`).
   `[resources] max_worktrees = N` refuses worktree N+1 (`resources.worktree-limit`),
   naming the idle managed ones.
5. **`war admin cleanup`** (also `war cleanup`) is the prune of `war admin worktrees`
   with branches added: a dry run by default; `--apply` removes finished managed
   worktrees and deletes `war-go/*` branches merged into the integration revision,
   checked out nowhere and unclaimed (a compare-and-delete on the ref). `war done` on
   a lane's last item and the plugin's Stop hook print at most one line about
   leftovers. Neither blocks.
6. **The cleanup lane.** With `[lanes] cleanup = true` (the default), `war plan lane
   new` ensures one lane named `cleanup` that `depends_on` every work lane, owns `**`,
   and holds three items: the docs pass, the architecture pass and the resource pass.
   Its `war.for` is unset and its brief recommends a different runner. Ensuring it
   is idempotent.
7. **`war.reuse`**: a lane's list of paths or modules to build on, set with `war
   plan lane new --reuse`, named in its runner's brief as `Reuse: …`. It refuses
   nothing, so it is **not critical** (FORMAT-1.0 7.3) and is not listed in
   `requires`.

## Options considered

- **A new verb for slots.** `war run` already names the service-stage run; a slot run
  is told apart by `--`. A seventh daily verb would break the small surface (M12).
- **Remove worktrees by age.** Rejected: an old worktree may hold work. The existing
  conservative prune decides, and unknown activity keeps a worktree.
- **Build automatically on `war start`.** Rejected: the owner asked for builds to be
  deliberate. `war start` only says the seed is stale.

## Consequences

- A repository without `[resources]` is unchanged, except that `war plan lane new`
  now adds the cleanup lane; `[lanes] cleanup = false` turns that off.
- On a host that hides a same-user process, activity is UNKNOWN and `war admin
  cleanup --apply` keeps every worktree, as `war admin worktrees` does. Merged
  branches no worktree holds are still deleted.
- A slot is per clone (its common dir), not per machine.
