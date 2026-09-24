---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e92-7911-af69-ffdfe4fbbdbd
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The owner, 2026-09-24: "make sure war update works and is a real option. How
do we prevent us from having old war versions? … If the friction is just from
having bad versions of openwarrant installed, that could be a problem if
someone doesn't ever upgrade."

Each of the following was observed on 2026-09-24:

- **Every existing install is stranded.** `war update` and `war version`
  arrived in b2e75943, after the `v1.0.0-alpha.2` tag (02c1fc9b). The owner's
  `war` on PATH (`~/.cargo/bin/war`, a symlink into
  `~/.local/lib/openwarrant/1.0.0-alpha.2/`) answers `war update` with
  "unrecognized subcommand". A self-updater only helps from the first release
  that has one. No published release has one.
- **The documented bootstrap is broken.** `install.sh` (the
  `curl … /main/install.sh | bash` the repository documents) builds the asset
  name `war-<tag>-<target-triple>.tar.gz`. That is the v0.1.0 naming. Both
  1.0.0 alphas publish `openwarrant-<tag>-<os>-<arch>.tar.gz` with `bin/war`
  inside. For alpha.2 the script's URL returns 404. It also installs a copied
  binary, which `war update` treats as not its own and will not repoint.
- **`war update` with no flags finds nothing.** Its default channel is
  stable. Every published release (v0.1.0, v1.0.0-alpha.1, v1.0.0-alpha.2)
  is a prerelease, so the default reports UNKNOWN `update.unavailable`
  "no published stable release". Only `--preview` works.
- **A build does not say what it is.** A build from today's source reports
  `war 1.0.0-alpha.2`, 62 commits after the tag of that name. `war --version`
  carries no commit, no dirty flag and no released/unreleased mark. So:
  - `war update --check --preview` answers PASS `update.current` for code
    that is not the release;
  - the TUI Help pane's binary row (OW-WAR-0115, `tui/mod.rs` `binary_row`)
    compares by version string, so it cannot tell that build from the
    release.
- **Version comparison is equality.** `check` and `update` compare
  `release.version == current`. A build newer than the newest release is
  told the older release "is published … `war update` installs it": a
  downgrade presented as an update.
- **Nothing tells a user a newer release exists.** They must think to run
  `war update --check`.

## Desired Outcome

1. **A build says what it is.** `war --version`, `war version` and
   `war doctor` name the commit, whether the tree was dirty, and whether
   this is a release. The bare `war <version>` form belongs to a release
   build alone. The records `war` writes keep the package version they carry
   today.
2. **`war update` works from every version that has it:**
   - it defaults to the channel the running build belongs to;
   - it compares versions by SemVer precedence and never offers a downgrade
     unasked;
   - it verifies what it downloads before any link moves;
   - it refuses a `war` it does not manage, with the exact command that
     would fix it.
3. **A notice when a newer release exists.** At most one network read a
   day, never blocking, never on stdout, and off when disabled, in CI, under
   `--json` or when stderr is not a terminal. It is one stderr line naming
   the newer release and the command that installs it.
4. **A bootstrap that works.** `install.sh` installs the current release
   layout into the same managed location `war update` uses, verified the
   same way. A stranded install, and a first install, both get one
   documented command, and from then on `war update` owns the install.
5. **One install document** (`docs/INSTALL.md`). It covers:
   - the three ways to install and the one way to update;
   - what the notice sends and how to turn it off;
   - what the checksum does and does not establish;
   - how an alpha.2 user gets out.

   `README.md` and `QUICKSTART.md` point to it.

## How this fits OW-WAR-0130

OW-WAR-0130 (authorized) plans `[project] requires_war`: a repository names
its minimum `war`, checked at discovery, and refused `compat.war-too-old`.
This Warrant does not duplicate it. It supplies the other half:
`install::remedy`, which turns a minimum version into the exact command for
this install. That is `war update --to <v>` for a managed install, or the
`install.sh` line for an unmanaged one. 0130's refusal can then say how to
get out, as well as why. The wiring into `compat.rs` is Q-003 (20-basis.md).

A `war` older than 0130 cannot enforce `requires_war`. `[project]` does not
deny unknown fields, so an old binary ignores the key silently. Neither
Warrant can reach an install that predates both. That is why the bootstrap
and the documents are deliverables here, not afterthoughts.

## Non-goals

- Signing releases. The workflow signs nothing today (`release.yml`: no
  signing step). This Warrant states what a checksum from the same release
  does not establish, and reports `update.unsigned`. Q-001 asks whether
  signing is a child Warrant.
- Automatic installation. The notice tells; only `war update` installs.
- Carrying build identity into records: `sign.rs`, `dispatch.rs`
  `compiler_digest`, `status.rs`, `eval.rs`, `progress.rs`,
  `preservation`, the MCP `server_info` and `webui`. They keep
  `CARGO_PKG_VERSION`. Changing a recorded string per commit would move
  digests and is its own decision.
- Windows, Linux aarch64 and Intel macOS builds. `host_asset` refuses them
  today and still will.
- Package managers (Homebrew, apt, nix).
- Reaching installs that predate this Warrant from inside `war`. They are
  reached through the documents, the release notes and, if Q-002 says so,
  the Claude plugin's hook.
