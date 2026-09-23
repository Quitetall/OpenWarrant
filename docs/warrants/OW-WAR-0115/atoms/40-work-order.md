---
schema: oh.war/atom/v1
warrant_uuid: 01a0cd30-eabb-76a2-b970-42724167ad84
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/projects.rs`:
   - The per-user list: load, touch on use, add, forget, and mark missing.
   - `war projects [--add <path> | --forget <path>] [--json]`.
   - Opt-out with `OPENWARRANT_NO_PROJECTS=1`.
2. `lib.rs`:
   - Every command that opened a repository touches the list, best-effort;
     a failure never fails the command.
   - `war` with no repository found opens the hub's Projects pane instead of
     Setup-for-cwd. Setup remains one keypress away, for a new project here.
3. `tui/mod.rs`:
   - The **Projects** pane: a row per project with name, path, branch, SAS,
     pending acts, blocking questions and progress.
   - Enter opens; `p` switches from any pane; `n` starts `war init` in a
     chosen directory.
   - The status bar shows the current project's name and path.
   - Every child `war` command already passes `--root`; a plant keeps it
     so.
4. **The binary line.** The Help pane's first row shows the running `war`'s
   version and path, and warns when `war` on PATH resolves to a different
   version, naming the install command.
5. `docs/TUI.md`, `README.md`: `war` from anywhere; the Projects pane; the
   list and its opt-out.
6. `conformance/plants.d/71-hub.sh`, all on scratch corpora with
   `XDG_CONFIG_HOME` pointed at a temporary directory:
   - running `war status` in two repositories lists both;
   - a deleted repository shows as missing;
   - `--forget` removes one;
   - the opt-out writes nothing;
   - no terminal refuses by name, as before;
   - a grep shows every signing child in `tui/` passes `--root`.

## Frozen Surfaces

Every record schema, `oh.war/report/v1`, the signing seam. The list is not a
record: it lives outside every repository.

## Premade Instructions

- A project's facts come from that project's records, through the CLI's own
  functions. The list stores where a project is, never what it says.
- Test every command the hub hands a human with that project's `--root` and
  a dry run, the way `war next` will after OW-WAR-0113.

## Autonomy and Escalation

Tier T2. Escalate rather than decide whether the hub should also discover
repositories by scanning a directory the user names (draft says no:
remembered on use, or added by hand).
