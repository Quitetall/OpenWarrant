# Type packs

A pack is a versioned set of document types beyond the core ones
(OW-WAR-0148 M18, decision 26). Each is a directory here: a `pack.toml`
naming its version and its profile files, and the files themselves, in the
same `oh.war/profile/v1` format as `profiles/*.toml`. Anyone can write one.

```sh
war plan types                    # every type this program admits, and where each comes from
war plan types add ops            # packs/ops/ of this repository, else the ops pack this build ships
war plan types add ./my-pack      # or a directory of your own
```

The two packs here are also built into `war`, byte for byte, so `war plan
types add ops` works in a repository without a `packs/` directory.

`war plan types add` validates every file of the pack against the types the
program already has, and writes nothing unless all of them are admitted: a
profile that selects a capability outside the closed set, or whose name
(or a projection's name) a type of the program already uses, refuses the
whole pack. An admitted pack's files are copied into `profiles/` and the
install is recorded in `docs/types.toml`.

| pack | types |
|---|---|
| `ops` | `runbook`, `slo`, `rollout`, `incident-review` |
| `quality` | `threat-model`, `performance-budget`, `test-charter` |

docs/TYPES.md, "Every development document is a type", has the reference.
