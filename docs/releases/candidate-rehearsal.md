# Candidate packaging and installation rehearsal

OW89 packages the CLI, RC.3 standard candidate, complete committed SDK workspace
with fixtures, adapted skills, assets and license notices. Every payload has a
recorded size and SHA-256 digest. Repeated packaging of identical inputs produces
identical archives. The binary is caller-supplied; its source linkage remains
unestablished until native release build provenance is independently checked.

Build the CLI, then run:

```sh
cargo build --release --locked -p openwarrant-cli --bin war
python3 conformance/release/package.py --war target/release/war --output /tmp/ow-candidate
```

Read the archive name and hash from `/tmp/ow-candidate/candidate.json`, then run:

```sh
python3 conformance/release/install_check.py --archive <archive> --sha256 <expected-sha256> --output <new-receipt.json>
```

The checker verifies exact inventory and byte digests before running the CLI in
a new temporary directory. It executes version, SDK validation and interactive
source authoring, then removes its temporary installation. It never overwrites
an existing installation, adds shell profile entries or modifies a repository.
Keep installed versions in separate directories and choose one explicitly; ending
this rehearsal returns to the original tool without modifying it. This is a local
installation/rollback observation, not an operating-system package-manager test.

`CANDIDATE.md` is the archive entry point. The standard, skills and SDK remain together in the original source layout. `sdk-source.tar` preserves the full
committed workspace, including original document paths. Extract that source archive
separately to read the original README and run SDK files/tests with their fixtures.
The standalone CLI needs no LAMU service for these SDK examples. LAMU owns semantic
compilation; workflow applications own execution, storage and UI.

The release workflow builds native Linux/macOS binaries and rehearses each bundle.
A successful rehearsal is not permission to publish. All tag publication, including
registry publication reached through prerelease tags, is refused until the OW90/91
release path exists. Cargo publishes manifest versions, not tag names; a prerelease
tag must never bypass the stable-version guard. Human assurance and SAS acceptance remain separate.
No workflow dispatch, registry publish or public release is performed by these scripts.

Native macOS installation, production upgrade/fault recovery, Phase 3 workflow and
actual-user evidence remain required for full release qualification. The candidate
manifest reports `qualified: false` and `promotable: false` in every rehearsal.
