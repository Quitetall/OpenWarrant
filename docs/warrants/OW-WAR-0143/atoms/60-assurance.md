---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e92-7911-af69-ffdfe4fbbdbd
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

Every plant below runs in `conformance/plants.d/61-update.sh`:

- against a local fixture server (`OPENWARRANT_RELEASES_URL`,
  `OW_RELEASES_URL`) whose request log it reads;
- with `HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME` and `PATH` in a temporary
  directory.

No obligation makes a claim about the real GitHub, a real network failure
mode other than a refused connection, or a platform other than the one the
battery runs on.

## Acceptance Obligations

### OBL-001 — a build says what it is, and only a release build says only its version
- **scope:**
  - `build_identity.rs`'s `gather` and `classify`, exercised through
    `war version --probe <dir>` on scratch git repositories and scratch
    directories;
  - `war --version` of the battery's own binary.

  No claim about cargo's rerun behaviour (A-003) beyond what a probe
  observes.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Accepted:
    - a clean scratch repository probes as `unreleased` with its HEAD
      commit and `dirty: false`;
    - the same probe with `OPENWARRANT_RELEASE_TAG=v<version>` gives
      `release`, and the identity line is exactly `war <version>`;
    - a directory with only a `.cargo_vcs_info.json` probes as `crate` with
      that sha1;
    - a directory with neither probes as `unknown`.
  - Refused:
    - the scratch repository after one tracked file is modified probes as
      `dirty: true`;
    - the same dirty tree with `OPENWARRANT_RELEASE_TAG` set is refused,
      non-zero, and the reason names the dirty tree;
    - a tag that is not `v<version>` is refused, naming both;
    - `unknown` is never `release`.
  - The battery's binary, which is never a release build, prints a
    `--version` line that is not the bare `war <version>`, and that names
    `unreleased` or `unknown`. `war version --json` carries the same class in
    `build.class`.

### OBL-002 — nothing switches until the download is verified
- **scope:** `install.rs` `update`, against fixture releases built from the
  battery's `war`, with a managed symlink on the temporary PATH.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Accepted: a well-formed fixture release installs under
    `<root>/<version>/`, repoints the managed link and keeps the previous
    version's directory. The report carries `update.unsigned`.
  - Refused: each of the following exits non-zero with the named rule. The
    listing and link targets of the install root are byte-identical before
    and after, and no `*.incoming.*` directory is left.
    - an archive with one byte changed: `update.checksum-mismatch`;
    - no `.sha256` asset: `update.no-checksum`;
    - an archive whose `MANIFEST.json` disagrees with one file:
      `update.manifest-mismatch`;
    - an archive whose `bin/war` reports another version:
      `update.identity-mismatch`.

### OBL-003 — `war update` finds the right release and never downgrades unasked
- **scope:** `install.rs` `check` and `update` over fixture release lists,
  and `precedes` over SemVer 2.0.0 §11's example ordering.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Accepted:
    - with the running build a prerelease and a list containing only
      prereleases, `war update --check` (no flags) reports
      `update.available` or `update.current`, not `update.unavailable`;
    - §11's list `1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < … <
      1.0.0` is ordered by `precedes` in the unit test.
  - Refused:
    - a fixture whose newest release precedes the running version gives
      `update.ahead`, and `war update` installs nothing (request log: no
      archive GET);
    - with a release equal to the running version and a non-`release` build,
      the check says `update.unreleased`, naming the commit, and never
      `update.current`.

### OBL-004 — an unmanaged `war` is never overwritten, and the refusal says what to do
- **scope:** `install.rs` `update` and `remedy`. PATH holds, in turn:
  - a copied binary;
  - a `#!` script;
  - a binary at a temporary `.cargo/bin/war`.

  In each case no managed link exists.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Refused: each case exits non-zero with `update.unmanaged`, before any
    download (request log: no archive GET). The file's bytes, mode and
    mtime are unchanged. The message carries the exact command for its kind:
    - `cargo install --locked openwarrant-cli` for the cargo path;
    - the `install.sh` line for the others.
  - Accepted: with a managed link beside an unmanaged `war`, `update`
    repoints the managed one and reports the other `update.not-ours`,
    untouched. `remedy("99.0.0")` gives `war update --to 99.0.0` for a
    managed install, and the `install.sh` line for an unmanaged one.

### OBL-005 — one line on stderr when a newer release exists, and silence otherwise
- **scope:** `notice.rs` and its hook in `lib.rs`. `war status` on a scratch
  repository runs under `script` (a pty) or with stderr to a file, with a
  seeded cache and the fixture server.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Accepted:
    - with a cache naming a newer release, the pty run's stderr carries
      exactly one line naming that tag and the remedy, and stdout is
      byte-identical to a run with the notice disabled;
    - with a stale cache, two runs within a minute make at most one request
      to the fixture server.
  - Refused: in each of the following cases stderr carries no notice line
    and the server logs zero requests. For `--json`, stdout also parses as a
    single `oh.war/report/v1` envelope.
    - `OPENWARRANT_NO_UPDATE_CHECK=1`;
    - `CI=true`;
    - `--json`;
    - stderr not a terminal;
    - `war version`.
  - Refused: with the server stopped (connection refused) and a stale
    cache, exit code and stdout equal those of the disabled run, and no
    notice is printed.

### OBL-006 — the bootstrap installs a managed `war`, and refuses what `update` refuses
- **scope:** `install.sh` against the fixture server (`OW_RELEASES_URL`),
  with the temporary `HOME` and `OW_BIN_DIR`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Accepted: on an empty `HOME`, the script installs
    `~/.local/lib/openwarrant/<version>/bin/war` and links
    `$OW_BIN_DIR/war` to it. The installed `war update --check` then reports
    that link as `install.on-path`, not `update.not-ours`.
  - Refused:
    - a tampered archive: the script exits non-zero with nothing under the
      install root and no link;
    - a plain file at `$OW_BIN_DIR/war`: it is left byte-identical, and the
      script exits non-zero naming it;
    - a fake `war` printing `war 1.0.0-alpha.2` earlier on PATH: the script
      warns and names it.

### OBL-007 — the install document says what is sent, what is checked, and how a stranded install gets out
- **scope:** `docs/INSTALL.md`, checked mechanically by the plant. No claim
  about the prose's quality.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Accepted: the document contains:
    - the releases URL, equal to the `RELEASES` constant in `install.rs`;
    - both opt-outs (`OPENWARRANT_NO_UPDATE_CHECK`, `CI`);
    - the words "not a signature";
    - the stranded-install section's `install.sh` command, equal to the
      one `remedy` prints;
    - the four build classes.
  - `README.md` and `QUICKSTART.md` each link `docs/INSTALL.md`.
  - Refused: a copy of the document with the opt-out variable renamed makes
    the check fail.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-002's, and with it
OBL-004's.

- An updater that switches before it verifies, or writes over a `war` it
  did not install, replaces the stale-binary problem with a worse one: a
  binary nobody chose.
- OBL-001 is the owner's question answered at its root. Without an honest
  identity, neither the notice nor `update` can tell old from new.
