---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e92-7911-af69-ffdfe4fbbdbd
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. **Build identity.**
   - `crates/openwarrant-cli/src/build_identity.rs` (new). A pure module
     that both `build.rs` (by `#[path]`) and the crate use.
     - `gather(dir)` reads:
       - `git rev-parse HEAD`;
       - `git status --porcelain --untracked-files=no` (dirty when
         non-empty);
       - `.cargo_vcs_info.json` when there is no `.git`;
       - `OPENWARRANT_RELEASE_TAG`.
     - `classify(facts)` gives one of four classes:
       - `release`: the tag equals `v<CARGO_PKG_VERSION>` and the tree is
         clean;
       - `crate`: from `.cargo_vcs_info.json`, with its sha1;
       - `unreleased`: a git checkout, with its commit and dirty flag;
       - `unknown`: neither source. It is never `release` (Law 15).
     - A release tag with a dirty tree, or a tag that is not
       `v<CARGO_PKG_VERSION>`, is an error. `build.rs` panics with the
       reason, so a release build that is not what it says cannot be built.
   - `crates/openwarrant-cli/build.rs` (new):
     - runs `gather` and `classify` on the workspace;
     - emits `OW_BUILD_CLASS`, `OW_BUILD_COMMIT`, `OW_BUILD_DIRTY` and
       `OW_BUILD_PROFILE` through `cargo:rustc-env`;
     - reruns on `.git/HEAD`, `.git/index`, `Cargo.lock`, every workspace
       crate directory, and `OPENWARRANT_RELEASE_TAG`. For a worktree it
       locates `.git` through `git rev-parse --git-dir`.
   - `war --version`:
     - a `release` build prints exactly `war <version>`, so the release
       workflow's Package check and `package.py` hold;
     - every other class prints
       `war <version> (<class>[, commit <12 hex>][, dirty][, debug])`.
2. **`crates/openwarrant-cli/src/install.rs`.**
   - Identity:
     - `Install` carries the build identity;
     - `report()` names it in `install.version`;
     - `json()` adds a `build` object to `oh.war/install/v1`: class,
       commit, dirty, profile, releases URL.
   - Versions:
     - `precedes(a, b)` implements SemVer 2.0.0 §11 precedence by hand, with
       no new dependency, and unit tests from the spec's own examples;
     - `check` and `update` compare by it.
   - Channel: the default follows the running build. A prerelease version
     means the preview channel; `--preview` still forces it. The stable
     channel exists but is not the default while the running build is a
     prerelease.
   - Outcomes:
     - newer published: WARN `update.available`, with the exact command;
     - equal, and this is a `release`: PASS `update.current`;
     - equal, and this is not a `release`: WARN `update.unreleased`, naming
       the commit, never `update.current`;
     - this build is newer: PASS `update.ahead`. `update` installs nothing
       without `--to`.
   - Before any link moves, in this order, each failure leaving the install
     root and every link unchanged:
     1. the `.sha256` matches (as today);
     2. every entry in the archive's `MANIFEST.json` matches the extracted
        bytes (`update.manifest-mismatch`);
     3. the staged `bin/war --version` output begins
        `war <release version>` (`update.identity-mismatch`);
     4. WARN `update.unsigned` says a checksum from the same release is not
        a signature (Q-001).
   - Atomic switch:
     - an existing `<root>/<version>` is renamed aside and restored on any
       failure, instead of the current `remove_dir_all` before the rename;
     - the previous version's directory is kept.
   - Unmanaged installs:
     - no `war` on PATH is a managed symlink, and at least one is a script
       or binary: `update` refuses `update.unmanaged` before any download;
     - it names each entry with the command for its kind: a binary under
       `~/.cargo/bin` gets `cargo install --locked openwarrant-cli`; a copied
       binary or a script gets the `install.sh` line and the file to remove
       first;
     - it never writes to them.
   - `remedy(min_version)`: the one-line command that gets this install to
     at least `min_version`, for OW-WAR-0130's `compat.war-too-old` (Q-003).
   - `OPENWARRANT_RELEASES_URL` overrides `RELEASES`. Every use is reported
     WARN `update.source`.
3. **`crates/openwarrant-cli/src/notice.rs` (new).** The release notice.
   - Cache: `$XDG_CACHE_HOME/openwarrant/release-check.json`, or
     `~/.cache/openwarrant/…`. It holds the check time, the channel, the
     newest tag or the UNKNOWN reason, and nothing else.
   - Shown at the end of a command, as one line on stderr, when all hold:
     - the cache says a newer release exists on this build's channel;
     - stderr is a terminal;
     - `--json` is absent;
     - `CI` is unset or empty;
     - `OPENWARRANT_NO_UPDATE_CHECK` is unset or empty;
     - the command is not `update`, `version` or `mcp`.

     The line: `war: <tag> is published (this is <version>). Update: <remedy>`.
   - Refresh: when the cache is older than 24 hours and every condition
     above holds, `war` spawns itself as `war __release-check`, detached,
     with null stdio. That hidden command reads the releases list with a
     5-second timeout and writes the cache. The parent never waits.
   - It sends the request line and a `user-agent: openwarrant/<version>`.
     No repository path, alias, machine identifier or cookie.
   - A failure is cached as UNKNOWN with its time, so an offline machine
     tries once a day, and prints nothing.
4. **`crates/openwarrant-cli/src/lib.rs`**, limited to:
   - the `version` string clap prints (item 1);
   - `war version --probe <dir>`: hidden; prints `gather(dir)` then
     `classify`, as JSON, for the plants;
   - the hidden `__release-check` command;
   - the one call to `notice::after(command, mode)` after dispatch;
   - the default channel for `Command::Update`.
5. **`crates/openwarrant-cli/src/tui/mod.rs`**, `binary_row` only.
   - It compares the running build's identity line with the PATH `war`'s
     `--version` line, whole, not its last word.
   - Its remedy is `install::remedy` in place of the fixed
     `cargo install --path` text.
6. **`install.sh`**, rewritten against the current release contract. Same
   interface: `OW_VERSION`, `OW_REPO`, `OW_BIN_DIR`, plus
   `OW_RELEASES_URL` for the plants.
   - Resolves the newest release from `/releases`, prereleases included,
     until a stable release exists.
   - Downloads `openwarrant-<tag>-<os>-<arch>.tar.gz` and its `.sha256`, and
     refuses on a missing checksum, a missing checksum tool or a mismatch.
   - Extracts into `~/.local/lib/openwarrant/<version>/` (the `war update`
     layout) and checks that `bin/war --version` names the version.
   - Links `$OW_BIN_DIR/war` to it only when that path is absent or already
     a symlink into the install root. Otherwise it prints what is there and
     the two commands to replace it, and exits non-zero without touching it.
   - Warns when another `war` precedes `$OW_BIN_DIR` on PATH, naming it and
     its `--version`. That is how a stranded alpha.2 learns it is shadowed.
7. **`.github/workflows/release.yml`**:
   - the Build step sets `OPENWARRANT_RELEASE_TAG` to the tag, or to
     `preview_tag`, so release binaries are `release` class;
   - `install.sh` is attached to each release, so a tag-pinned URL exists.
8. **`docs/INSTALL.md` (new).**
   - The three ways to install: `install.sh`, a verified tarball by hand,
     `cargo install`.
   - The one way to update: `war update`, and what it refuses.
   - The notice: what it sends, to which URL, and the two ways to turn it
     off.
   - What the checksum establishes and what it does not.
   - The stranded alpha.2 path, in three commands.
   - The four build classes, and how to read `war version`.
9. **`README.md`**: its "Build the current CLI" section gains one line
   pointing to `docs/INSTALL.md` and the `install.sh` command.
   **`QUICKSTART.md`**: step 0 does the same. Nothing else in either file.
10. **`conformance/plants.d/61-update.sh` (new).**
    - A local `python3 -m http.server` serves a fixture releases list and
      archives made from the battery's own `war`, with a request log.
    - `HOME`, `XDG_*` and `PATH` point into a temporary directory.
    - It pairs every claim in 60-assurance.md with its refusal.
    - It never reads the real GitHub, and never writes outside its
      temporary directory.

## Frozen Surfaces

- `oh.war/report/v1` and every record schema. `oh.war/install/v1` only
  gains the optional `build` object.
- Every `CARGO_PKG_VERSION` string written into a record or protocol reply
  (20-basis.md).
- The release asset names, the archive layout, `package.py`, and the
  Package step's `war --version` equality.
- `install.rs`'s rule that only a symlink resolving into the install root is
  repointed.
- `install.sh`'s environment interface.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:

- any network read other than the one releases URL, or any field sent
  beyond the request line and user-agent;
- any case in which the notice would print to stdout, or would delay a
  command's exit;
- a change to what any record carries;
- a need to touch `compat.rs` (Q-003), `stop-check.sh` (Q-002), or signing
  (Q-001).

## Rollback

Revert the declared files.

- Installs made by the new `install.sh` stay valid: they use `war update`'s
  existing layout.
- The notice cache is disposable. Deleting
  `~/.cache/openwarrant/release-check.json` removes every trace.
- A release built with `OPENWARRANT_RELEASE_TAG` stays a valid release to
  the old code, which reads only the version.
