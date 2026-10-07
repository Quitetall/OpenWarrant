---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e92-7911-af69-ffdfe4fbbdbd
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- No §106 row names installation, updating or build identity. The nearest
  normative text:
  - §7: readiness tests "the environment, network route, workspace, identity,
    capabilities, and tools that the actual performer or verifier will use".
    A `war` that cannot say which build it is cannot be tested that way.
  - Law 15: a check that could not run is UNKNOWN. An unreachable release
    list is UNKNOWN, never "current". A build with no identity is "unknown",
    never "release".
  - §9: version 1 does not require cryptographic signatures. So unsigned
    releases are admissible, and saying so is required (Q-001).
  - §69.2: a minor version may add optional fields. `oh.war/install/v1`
    gains a `build` object this way.
- `crates/openwarrant-cli/src/install.rs`, as it stands. Its rules are kept:
  - `observe`, and `Kind` (symlink, script, binary);
  - `host_asset` (linux-x86_64 and darwin-arm64 only);
  - `update` repoints only a symlink resolving inside the install root, and
    refuses a release with no `.sha256` beside the archive.
- `.github/workflows/release.yml` and `conformance/release/package.py`: the
  release contract this Warrant must not break.
  - The archive is `openwarrant-<label>-<system>-<machine>.tar.gz`, holding
    `bin/war`, `MANIFEST.json` (the sha256 of every file) and the rest.
  - `<archive>.sha256` sits beside it.
  - The Package step requires `war --version` to print exactly
    `war <tag without v>` for a preview build.
  - `package.py` requires the output to start with `war `.
  - No step signs anything.
- OW-WAR-0130 (authorized, not delivered): `[project] requires_war` and
  `compat.war-too-old` in a new `compat.rs`. This Warrant adds the remedy
  those use, not the check (10-intent.md).
- OW-WAR-0115 (the Help pane's binary row, `tui/mod.rs` `binary_row`). This
  Warrant changes what that row compares: build identity, not the version
  string.
- OW-ADR-0021: this Warrant declares every path it will write in
  `deliverables.toml`. Among them:
  - `README.md`, whose latest owner is OW-WAR-0116 (D-009);
  - `tui/mod.rs` and `lib.rs`, shared with other Warrants.

  Their edits are limited to the sections named in the Work Order.

## Facts observed 2026-09-24 (the Problem's evidence)

- The installed `war --version` is `war 1.0.0-alpha.2`, and `war update`
  there is "unrecognized subcommand". `git merge-base --is-ancestor`: the
  commit adding `install.rs` is not an ancestor of `v1.0.0-alpha.2`.
- `target/debug/war --version` reports `war 1.0.0-alpha.2` at 244f538e, 62
  commits after the tag.
- `war update --check --preview` answers PASS `update.current`.
  `war update --check` answers UNKNOWN `update.unavailable`
  "no published stable release".
- `GET https://api.github.com/repos/Quitetall/OpenWarrant/releases` lists
  three releases, all prereleases, none drafts. Assets:
  - v1.0.0-alpha.1 and v1.0.0-alpha.2 carry
    `openwarrant-<tag>-{linux-x86_64,darwin-arm64}.tar.gz` and a `.sha256`
    each;
  - v0.1.0 carries `war-v0.1.0-<triple>.tar.gz`.
- `install.sh` builds `war-<tag>-<triple>.tar.gz` and expects `<name>/war`
  inside. For alpha.2 that URL returns 404. It installs with
  `install -m 0755`, which gives a plain binary: `update.not-ours`.
- A `v1.0.0` tag exists on origin (5ab36365) with no release. Publication
  is blocked by the workflow's "Refuse publication" step. Nothing here reads
  tags; releases are the only source.
- `crates/openwarrant-core/src/config.rs` `Project` does not
  `deny_unknown_fields`, so a `war` before 0130 ignores `requires_war`.
- `war --version` is clap's `version` attribute, from `CARGO_PKG_VERSION`.
  There is no `build.rs` in `openwarrant-cli`.
- These embed `CARGO_PKG_VERSION` in records or protocol output: `sign.rs`,
  `dispatch.rs`, `status.rs`, `eval.rs`, `progress.rs`, `preservation*`,
  `mcp/mod.rs`, `webui/mod.rs`. They are frozen here.

## Assumptions

- A-001: the GitHub releases API stays readable without authentication, at
  60 requests an hour per address. One read a day per install is far inside
  that. Confidence: high.
- A-002: `cargo publish` writes `.cargo_vcs_info.json` (the git sha1 and
  whether the tree was dirty) into the published crate. So a
  `cargo install openwarrant-cli` build can name its commit with no `.git`.
  Confidence: high; the file is standard cargo behaviour.
- A-003: `rerun-if-changed` on a directory makes cargo rescan it
  recursively. So `build.rs` watching the workspace crate directories,
  `.git/HEAD` and `.git/index` reruns when a tracked source file changes.
  A stale dirty flag is the failure this rules out. Confidence: medium. A
  file outside the watched directories can change the tree without a rerun,
  so the watched set is the workspace's crates plus `Cargo.lock`.

## Unknowns and questions for the owner

- Q-001 (non-blocking): should releases be signed, and how?
  - Today the `.sha256` comes from the same release as the archive. It
    catches corruption and truncation. It does not catch a compromised
    release, or a compromised account that replaces both files.
  - Options:
    - (a) the owner signs a `SHA256SUMS` with the existing SSH key, in a new
      `oh.war/release` namespace, as a human release act. `war` embeds the
      allowed-signers line and verifies it.
    - (b) GitHub build-provenance attestations, verified with
      `gh attestation verify`. This needs `gh`.
    - (c) neither, and say so.
  - Recommendation: (a), in a child Warrant. It reuses the key and the
    `ssh-keygen -Y verify` path `war` already has, and needs no new tool on
    the user's machine. This Warrant delivers (c)'s honesty now:
    `update.unsigned` WARN, and the document's statement.
- Q-002 (non-blocking): should the Claude plugin's `stop-check.sh` name a
  `war` too old to have `war update`?
  - It already runs whatever `war` is on PATH. So it is the one channel that
    reaches a stranded alpha.2 user of this plugin.
  - Recommendation: yes, as a one-line version floor in a follow-up. It is
    outside this Warrant's files on purpose, because it concerns the plugin
    and not `war`.
- Q-003 (non-blocking): who wires `install::remedy` into 0130's
  `compat.war-too-old`?
  - Recommendation: 0130's STAGE, since its message already names both
    versions and a remedy line is within its obligation.
  - The alternative is declaring `compat.rs` here. That makes this Warrant
    its later owner under OW-ADR-0021 before 0130 has delivered it.
- Q-004 (non-blocking): should an unreleased build get the notice?
  - Recommendation: yes, when its version precedes the newest release.
    Precedence is what matters, and a developer can turn it off.

## Residual risks

- R-001: `OPENWARRANT_RELEASES_URL` (the plants' seam) can point `war update`
  at another server, whose `.sha256` it would believe. Anyone who can set
  the environment can already set PATH. The override is still visible:
  `update.source` WARN names it on every use.
- R-002: `curl … | bash` trusts the host serving the script. The document
  gives the tag-pinned URL, and download-read-run as the alternative.
- R-003: the notice teaches users to expect a line on stderr. A script that
  captures stderr at a terminal will see it. It is off whenever stderr is
  not a terminal.
