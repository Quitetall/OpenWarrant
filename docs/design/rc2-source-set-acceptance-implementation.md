# Using the source-set acceptance extension

This is the implementation guide for OW-WAR-0074 revision 2. The signed design
is `rc2-source-set-acceptance.adr.md`; this guide does not replace that decision
or accept the SAS. Run the implementation build in its own checkout.

## Propose and inspect

```sh
cargo build --workspace
./target/debug/war sas propose 1.0.0-rc.2 \
  --source docs/sas/drafts/1.0.0-rc.2/WAR_Software_Architecture_Specification.md \
  --source-set docs/sas/drafts/1.0.0-rc.2/source-set.json \
  --adr docs/design/rc2-source-set-acceptance.adr.md
./target/debug/war sas accept 1.0.0-rc.2 --json
./target/debug/war sign 1.0.0-rc.2 --show
./target/debug/war sas status
```

Proposal is performed once per edition. The real candidate has already been
proposed in the implementation checkout. The inspection commands are read-only.
The three proposal options must occur together. Without them the legacy
single-document proposal keeps its previous meaning and revision store.

After review, the authorized human signs through the existing signing command.
The captured adoption decision supplies the ADR reference automatically; a
different explicit `--adr` is refused. Direct v2 response ingestion requires its
valid SSH signature. The existing terminal-confirmation route is also retained.

## What the subject covers

The v2 revision's `sha256` is the complete acceptance-subject digest. Separate
`source_set.main`, `source_set.manifest`, and `source_set.decision` identities
contain repository-relative paths and unprefixed SHA-256 values. The subject
also contains the predecessor version and its recorded subject digest.

The manifest uses `oh.war/spec-source-set/1.0.0-rc.2`. Its exact bytes bind every
member's path, byte length, digest and role. Member paths are relative to the
manifest directory. Both `normative` and `reference` members are mandatory
capture inputs; `reference` does not mean missing bytes are allowed. The main
document must be a normative member.

The subject root uses RFC 8785 canonical JSON with the domain prefix specified
by the adoption decision. The independent ASCII vector lives under
`conformance/fixtures/sas-source-set/`. Existing v1 digest domains do not change.

## Capture and recovery

Bounds: 1,024 members; 8 MiB per member or adoption decision; 2 MiB manifest;
32 MiB total captured input. Paths must be relative, use slash separators, and
contain no empty, dot, traversal, backslash, colon or NUL components. Symlinks
are refused at every component. These are capture limits, not agent isolation.

Snapshots live under the configured SAS revisions directory at
`source-sets/<edition>/`. Capture files and the manifest are synced before an
exclusive publication exposes the complete proposal record. Acceptance holds
a per-edition lock, validates the snapshot and current inputs, and replaces
only the still-proposed record after authentication.

A stale input requires a new proposal. Existing snapshots, locks, and pending
publication files are never overwritten or automatically discarded. After an
interruption, inspect them and the revision record before moving unaccepted
temporary files aside. A published record followed by a directory-sync failure
is reported explicitly as published with uncertain durability.

`sas.capture-intact` checks retained bytes. `sas.current-differs` reports a
separate current-input disagreement. A later edit does not rewrite accepted
history. New Warrants pin the complete v2 subject; previously authorized
Warrants keep their prior SAS pin until an authorized amendment changes it.
The existing normative view projects the selected main document's captured
bytes. General projections over all source-set documents remain compiler work.

## Verification

```sh
cargo test -p openwarrant-cli --test sas_source_set
python3 conformance/fixtures/sas-source-set/check_changing_input.py target/debug/war
cargo xtask gate
```

The Rust tests create disposable repositories with fictional actors and keys.
They do not accept or sign the real SAS. The concurrent-write probe is bounded
and reports UNKNOWN if scheduling does not expose an inconsistent read.
Required legacy delivery corrections and independent verification remain
separate from these performer observations.
