# Forced cross-workspace Cargo cache reuse

Create both supplied source workspaces before either build. In disposable scratch, run each app using the same explicit `build.build-dir`, with `build.rustc-wrapper=""`, `--locked --offline` (generate each lockfile first). The observed second build succeeded yet its binary printed `1`, while its source requires `2`. Rebuilding the after workspace with a different build-dir printed `2`. Logs and `cache-observation.json` retain the result. This minimal reproduction has no dependencies outside its two source crates.

Do not force unrelated worktrees into one cache. The workstation's existing Cargo configuration already uses `/mnt/4tb/build/cargo/{workspace-path-hash}`. The corrected historical comparison uses that automatic separation. No global configuration or shared cache was changed or removed.
