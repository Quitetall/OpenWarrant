# Installing and updating `war`

One page for getting `war`, keeping it current, and knowing which one you are
running (OW-WAR-0143).

Published builds exist for Linux x86_64 and macOS on Apple silicon. On any
other platform, build from source (the third way below).

## Three ways to install

**1. `install.sh`** (recommended). It installs the newest release into the
layout `war update` manages. After that, `war update` owns the install.

```bash
curl -fsSL https://raw.githubusercontent.com/Quitetall/OpenWarrant/main/install.sh | bash
```

What it does:

- reads the releases list and picks the newest release (prereleases count
  until a stable release exists);
- downloads `openwarrant-<tag>-<os>-<arch>.tar.gz` and its `.sha256`, and
  refuses if the checksum is missing, cannot be computed, or does not match;
- unpacks into `~/.local/lib/openwarrant/<version>/` and checks that
  `bin/war --version` names that version;
- links `~/.local/bin/war` to it, but only when that path is absent or is
  already a link into `~/.local/lib/openwarrant`. Anything else there is
  named, with the two commands that would replace it, and left alone;
- warns when another `war` comes earlier on `PATH`, naming it and what it
  says it is.

`curl … | bash` trusts whoever serves the script. To pin it and read it
first, take the copy attached to a release:

```bash
curl -fsSLO https://github.com/Quitetall/OpenWarrant/releases/download/<tag>/install.sh
less install.sh
OW_VERSION=<tag> bash install.sh
```

Settings: `OW_VERSION` (a tag, default `latest`), `OW_BIN_DIR` (default
`~/.local/bin`), `OW_REPO`. `XDG_DATA_HOME`, when set, moves the install root
to `$XDG_DATA_HOME/openwarrant`, exactly as it does for `war update`.

**2. A verified tarball, by hand.** From the release page, download the
archive for your platform and its `.sha256`. Then:

```bash
sha256sum -c openwarrant-<tag>-linux-x86_64.tar.gz.sha256   # macOS: shasum -a 256 -c
mkdir -p ~/.local/lib/openwarrant/<version>
tar -xzf openwarrant-<tag>-linux-x86_64.tar.gz -C ~/.local/lib/openwarrant/<version>
ln -s ~/.local/lib/openwarrant/<version>/bin/war ~/.local/bin/war
```

Use exactly that layout and a symlink, and `war update` manages it from then
on. A copied binary is never managed.

**3. `cargo install`**, from crates.io or a checkout:

```bash
cargo install --locked openwarrant-cli
cargo install --locked --path crates/openwarrant-cli   # from a clone
```

This puts a binary in `~/.cargo/bin`. `war update` does not manage it and
will not overwrite it. Update it the same way you installed it.

## The one way to update

```bash
war update --check   # what is published, and how this build compares
war update           # install the newest release on this build's channel
```

- **The channel follows the running build.** A prerelease build such as
  `1.0.0-alpha.2` is on the preview channel, so `war update` offers
  prereleases. A stable build is offered stable releases. `--preview` forces
  the preview channel.
- **Newer means newer by SemVer precedence.** A build newer than every
  release is told `update.ahead`, and nothing is installed. `--to <version>`
  installs a named version, older ones included. `--force` reinstalls the
  version already running.
- **Nothing moves until the download is verified.** In order: the archive
  matches the `.sha256` published beside it; every file matches the archive's
  own `MANIFEST.json`; the unpacked `bin/war --version` names the release's
  version. Any failure leaves the install root and every link as they were.
  The new version goes in `~/.local/lib/openwarrant/<version>/`, and the
  previous version's directory is kept. To go back, point the link at it with
  `ln -sfn`.

What it refuses:

| rule | when |
|---|---|
| `update.unmanaged` | no `war` on `PATH` is a link into the install root. It names each one with the command for it: `cargo install --locked openwarrant-cli` for `~/.cargo/bin`, or remove the file and run `install.sh`. It downloads nothing and writes nothing. |
| `update.checksum-mismatch`, `update.no-checksum` | the archive does not match its `.sha256`, or there is none |
| `update.manifest-mismatch` | a file differs from the archive's `MANIFEST.json` |
| `update.identity-mismatch` | the archive's `bin/war` says it is another version |

Beside a managed link, any other `war` on `PATH` is reported
`update.not-ours` and left untouched.

`OPENWARRANT_RELEASES_URL` points `war update` at another releases list.
Every use is reported `update.source`, because that server's checksums are
the ones believed.

## The notice

When a newer release is published, `war` prints one line on stderr after a
command finishes:

```text
war: v1.0.0-alpha.3 is published (this is 1.0.0-alpha.2). Update: war update --to 1.0.0-alpha.3
```

It installs nothing; only `war update` installs.

**What it sends.** At most once a day, `war` starts itself in the background
and makes one GET to the releases list:

```text
https://api.github.com/repos/Quitetall/OpenWarrant/releases
```

The request carries the request line, `host` and
`user-agent: openwarrant/<version>`. It sends no repository path, Warrant
alias, machine identifier or cookie. The command you ran never waits for it,
and a failure prints nothing. The answer is cached in
`~/.cache/openwarrant/release-check.json` (or under `$XDG_CACHE_HOME`): the
check time, the channel, and the newest tag or why there is none. Deleting
that file removes every trace.

**When it is silent.** It never prints to stdout. It is off, and makes no
request:

- when stderr is not a terminal;
- under `--json`;
- for `war update`, `war version` and `war mcp`;
- when either of these is set to a non-empty value:
  - `OPENWARRANT_NO_UPDATE_CHECK=1` turns it off for you;
  - `CI` turns it off in CI (most CI systems set it).

## What the checksum establishes, and what it does not

The `.sha256` is published in the same release as the archive. A match
establishes that the bytes you have are the bytes that release holds. It
catches a corrupt, truncated or substituted download.

A checksum from the same release is **not a signature**. It does not show
who made the release. Someone able to replace the archive on the release
page, or the account that publishes it, can replace its checksum too.
Releases are not signed today, and `war update` reports that on every
install as `update.unsigned`. Whether and how to sign them is an open
question (OW-WAR-0143, Q-001).

## A stranded install

`war update` first appeared after `v1.0.0-alpha.2`. A `war` from that
release, or older, answers `war update` with "unrecognized subcommand", and
nothing inside it can fix that. Three commands get out:

```bash
curl -fsSL https://raw.githubusercontent.com/Quitetall/OpenWarrant/main/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"   # and add this line to your shell profile
war version                            # the new build, and any old `war` it shadows
```

`install.sh` names any older `war` that comes first on `PATH`. `war version`
shows every `war` on `PATH` and what each one is. Once the new one runs
first, remove the old one. After that, `war update` keeps the install
current.

## Which build is this?

`war --version` prints a bare `war <version>` for a release build only.
Every other build says what it is:

```text
war 1.0.0-alpha.2                                                 a release
war 1.0.0-alpha.2 (unreleased, commit d2de140550cf, dirty, debug) a checkout
```

There are four build classes:

| class | what it is |
|---|---|
| `release` | built by the release workflow from the tag `v<version>`, on a clean tree. A tag that is not `v<version>`, or a dirty tree, fails the build. |
| `crate` | built from the published crate (`cargo install openwarrant-cli`). Its commit comes from `.cargo_vcs_info.json`. |
| `unreleased` | built from a git checkout, with its commit and whether tracked files were modified (`dirty`). |
| `unknown` | neither source was available. It is never taken for a release. |

`war version` (and `war doctor`) print the class, commit, dirty flag and
profile of the running binary, and every `war` on `PATH`.
`war version --json` carries them under `result.build`. A build of the same
version as a release, but not the release itself, is told `update.unreleased`
by `war update --check`, never `update.current`.

## Version skew: text newer than the binary

An agent follows AGENTS.md and the plugin's skills; `war` is whatever binary
`PATH` finds. When the text is newer, it names commands the binary may not
have, and an agent would read the old binary's refusal as a rule of the
repository. Two stamps say which `war` the text came with:

- AGENTS.md ends with `<!-- openwarrant agents-md: written by war X -->`,
  written by the `war init` or `war agents-md` that made it. The line sits
  inside the managed block (below), so a CLAUDE.md carrying the block is
  read the same way;
- the Claude Code plugin's `.claude-plugin/plugin.json` `version` is the
  `war` release it ships with (it moves every release), read from the
  repository root and from `$CLAUDE_PLUGIN_ROOT` when the harness sets it.

`war doctor` and `war prime` compare each stamp with the running version and
warn `install.version-skew`, naming both versions and the update command,
when the stamp is newer. An equal or older stamp, or a file with none (written
before the stamps existed), says nothing.

## The pointer block in CLAUDE.md and AGENTS.md

An agent reads the repository's `CLAUDE.md` or `AGENTS.md` before anything
else. `war` keeps one small block in them, and nothing more:

```markdown
<!-- openwarrant:begin -->
<!-- Written by `war agents-md --block`, which rewrites the lines between these markers. -->
Ordinary coding needs no Warrant and no ticket: work here as in any repository.
OpenWarrant tracks optional plans and checklists in this repository; run
`war prime` to see what is tracked (open work, who holds what, recent notes).
<!-- openwarrant agents-md: written by war 1.0.0 -->
<!-- openwarrant:end -->
```

- `war agents-md --block` inserts or updates it in every root `AGENTS.md`
  and `CLAUDE.md` that exists, or writes an `AGENTS.md` holding only the
  block when neither does. `--file <path>` names another file (created when
  absent); `--stdout` prints the block. A `CLAUDE.md` that is a link to
  `AGENTS.md` is written once, through the link; a link to a file outside
  the repository (a shared or global `CLAUDE.md`) is refused,
  `agents-md.link-outside`.
- A file without the block gets it on the line after its last one. A file
  with it has only the lines between the markers rewritten. Every byte
  outside the markers stays as it was, and a second run changes nothing.
- The block never holds the Warrant you are working on or any other state
  that changes: `war prime` says that, and the file stays stable.
- Refused by rule, with nothing written to any file: a file with two blocks
  (`agents-md.block-duplicate`), a block that never closes
  (`agents-md.block-unterminated`), an end marker with no begin
  (`agents-md.block-unopened`), and a file whose last code fence never
  closes (`agents-md.fence-unclosed`). Each names the line to fix.
- `war init` writes the full AGENTS.md guide (which ends with the block)
  when there is none, and adds the block to an `AGENTS.md` or `CLAUDE.md`
  you already have, printing one line per file it touched.
- `war doctor` reports each root file's block: `doctor.agents-block` when it
  is this `war`'s, `doctor.agents-block-missing`, `doctor.agents-block-stale`
  (written by an older `war`, or edited between the markers), and
  `doctor.agents-block-malformed`. A block a newer `war` wrote is version
  skew, not stale.

The sections of these files are also records that plans can cite
(`md:CLAUDE.md#testing`); [docs/TYPES.md](TYPES.md) has the details.

## Signing setup: `war doctor`

Ordinary work needs no signing setup. When a person wants to sign off with
`war sign --ssh-sign`, `war doctor` probes what that needs without signing
anything: `ssh-keygen` on `PATH`, the agent behind `SSH_AUTH_SOCK` and the
keys `ssh-add -L` lists, `docs/authority/roles.toml` (a human with the
authorizer or resolver role and an `ssh_principal`), and an
`docs/authority/allowed_signers` line for that principal whose key the agent
holds. Each missing piece is a WARN that says what to run or add. At a
terminal, `war doctor --fix-signing` walks through them: it signs nothing,
writes `roles.toml` or `allowed_signers` only when the file does not exist
yet (from your answers, after showing the exact bytes and asking), and for a
file that exists prints the lines to add by hand, because no command edits
those files once written (OW-ADR-0021). Every `war sign` refusal names the
missing piece, carries a remedy, and ends "This blocks only the sign-off, not
your work."
