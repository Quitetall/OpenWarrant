# Alpha.4 publication observation

Observed from GitHub and the installed CLI on 2026-10-17.

- Release `v1.0.0-alpha.4` is published as a prerelease from
  `6d5d2935ef1ed1c91af296770f4e0e4be6e6f332`.
- Release workflow 38956967591 passed the gate, Linux x86_64 build,
  macOS ARM64 build, publish and alpha-preview publication jobs. Crates
  publication was skipped, as intended for this prerelease.
- Release assets include both native archives, checksums, installer and
  per-platform candidate/install observations. Exact asset digests and job
  results are retained in the adjacent JSON files.
- The installed user CLI reports `war 1.0.0-alpha.4`. This observation does
  not reinstall or change it.

This closes the publication task as reported work. It does not establish
Stable qualification, independent acceptance, production isolation, current
main publication or a common assurance mark. Later main fixes are not
retroactively part of this published source.

The earlier claim command used the repository's configured `claude` actor
by default. Codex made that call; it released that own claim and claimed
the item explicitly as `agent://codex`. The journal preserves both events.
