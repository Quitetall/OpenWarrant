# Publishing to crates.io

Nothing is published yet. This records what publishing will require, so the
first attempt is not the first time anyone learns it.

## Do the relicense first

A release published under AGPL-3.0-or-later stays under those terms permanently.
Publishing before the Apache-2.0 relicense would leave a version on crates.io
that can never be relicensed. See [`RELICENSING.md`](../RELICENSING.md).

## Publish order is forced, and dry-run cannot check it in advance

Cargo rewrites path dependencies to registry dependencies at publish time, so a
crate cannot be published — or even dry-run — until every crate it depends on is
already **on crates.io**. The order is:

```text
1. openwarrant-core        (no internal deps)
2. openwarrant-agent       (no internal deps)
3. openwarrant-compiler    (depends on core)
4. openwarrant-cli         (depends on core + compiler)
```

`xtask` is never published; it carries `publish = false`.

**The limitation to know about.** A dry-run only proves as much as the registry
can resolve:

```console
$ cargo publish --dry-run -p openwarrant-core     # works — no internal deps
$ cargo publish --dry-run -p openwarrant-cli
error: failed to prepare local package for uploading
Caused by:
  no matching package named `openwarrant-compiler` found
  location searched: crates.io index
```

That error is **expected and correct**, not a packaging defect. It means only
that `openwarrant-compiler` is not published yet. Downstream crates become
dry-runnable one step at a time as the upstream ones land, which also means a
publish sequence cannot be fully rehearsed — the later steps are genuinely first
attempts.

Consequence: publish upstream-first, verify each on crates.io before the next,
and be prepared for the final `openwarrant-cli` step to be the one that surfaces
a problem. A yanked version cannot be reused, so a mistake costs a version
number.

## Version numbers

The workspace is at `0.0.1` and every crate inherits it, so all four move
together. Internal path dependencies pin `version = "0.0.1"` explicitly; bumping
the workspace version means bumping those pins in the same commit, or the
published crates will require a version that does not exist.

Pre-1.0, the protocol is not stable. Any 0.x release may change the canonical
JSON shape, the digest domains, or the manifest schema.

## The binary name is not the crate name

`war` is **taken** on crates.io by an unrelated crate. This does not block us:
the binary is produced by `openwarrant-cli` via `[[bin]] name = "war"`, so

```console
$ cargo install openwarrant-cli
```

installs an executable called `war`. Only the *crate* name is unavailable.

`openwarrant`, `openwarrant-core`, `openwarrant-compiler`, `openwarrant-agent`,
and `openwarrant-cli` were all free when checked on 2026-08-19. Names are not
reserved by being free, so re-check before publishing.
