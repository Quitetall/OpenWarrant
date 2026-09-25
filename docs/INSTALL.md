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
