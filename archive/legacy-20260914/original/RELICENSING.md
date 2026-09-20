# Relicensing: AGPL-3.0-or-later → Apache-2.0

## Intent

OpenWarrant is distributed today under **AGPL-3.0-or-later**. The intent is to
relicense to **Apache-2.0** when the project is made public.

This file exists so the relicense stays possible. A relicense is not a decision
you can make at the end; it is a set of constraints you either honoured from the
first commit or did not.

## The two preconditions

### 1. No dependency may forbid it

Every dependency must be permissive — MIT and/or Apache-2.0, or another licence
on the allowlist in [`deny.toml`](deny.toml). A GPL, AGPL, or LGPL dependency
cannot be relicensed by us at all, and adopting one means rewriting whatever
depends on it later.

This is enforced mechanically, not by review discipline:

```bash
cargo deny check licenses
```

which runs as a step inside `cargo xtask gate`. Weak-copyleft licences
(MPL-2.0, EPL-2.0) are deliberately absent from the allowlist: they do not
infect our source, but they carry per-file source-disclosure obligations that a
permissive redistribution story should not silently inherit. If one is ever
genuinely needed, it goes in as a narrow `exceptions` entry naming the crate,
never onto the blanket allow list.

**Current state — measured 2026-08-19, not assumed.** 70 third-party packages:

| count | declared licence |
|---:|---|
| 54 | `MIT OR Apache-2.0` |
| 6 | `MIT` |
| 5 | `Apache-2.0 OR MIT` |
| 1 | `MIT/Apache-2.0` |
| 1 | `Unlicense OR MIT` (`memchr`) |
| 1 | `Apache-2.0 OR BSL-1.0` (`ryu-js`) |
| 1 | `(MIT OR Apache-2.0) AND Unicode-3.0` (`unicode-ident`) |
| 1 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` (`r-efi`) |

Two entries deserve a note, because a naive scan for "LGPL" or "AND" in that
column raises an alarm that dissolves on reading:

- **`r-efi`** is triple-licensed with `OR`. We take the Apache-2.0 option; the
  LGPL branch is one we decline, not one we inherit. (It arrives transitively
  through `getrandom` and is only compiled for UEFI targets.)
- **`unicode-ident`** uses `AND`, which really is conjunctive — but the
  conjunct is Unicode-3.0, a permissive data licence on the Unicode tables, and
  it is on the allowlist deliberately.

No dependency imposes copyleft on our source. The Apache-2.0 path is open, and
`cargo deny check licenses` re-verifies it on every gate run rather than leaving
this table to age.

### 2. The copyright must be ours to relicense

A relicense requires permission from every copyright holder. One contribution
from someone who later becomes unreachable blocks it permanently, for everyone.

Two mechanisms, and this project uses the first:

- **Contribution terms.** [`CONTRIBUTING.md`](CONTRIBUTING.md) states that
  opening a pull request agrees the contribution may be distributed under
  Apache-2.0 as well as AGPL-3.0-or-later. This must be in place *before* the
  first outside contribution, not retrofitted after.
- **Sole authorship.** While the project has one author, the question does not
  arise. It stops being true the moment it stops being true, which is why the
  terms above are already written down.

**Current state:** single author. The contribution terms are in place ahead of
need.

## What the flip actually requires

When the decision is made, in this order:

1. Confirm `cargo deny check licenses` is green — the standing guarantee.
2. Confirm every contributor is covered by the contribution terms
   (`git log --format='%an <%ae>' | sort -u`).
3. Replace `LICENSE` with the Apache-2.0 text and add `NOTICE`.
4. Change `license = "AGPL-3.0-or-later"` to `"Apache-2.0"` in the workspace
   `Cargo.toml`, and update the per-crate `exceptions` in `deny.toml`.
5. Update the `SPDX-License-Identifier` header on every source file.
6. Record the change as an ADR. Relicensing is a normative decision and gets a
   first-class record like any other.
7. Flip repository visibility to public — and **at the same time**, move CI off
   the self-hosted runner, because a fork pull request against a public
   repository with a self-hosted runner executes attacker-supplied code on the
   workstation. This is not optional and is not a follow-up task.

## What relicensing does not do

It does not change the licence of any version already distributed under
AGPL-3.0-or-later. Those releases remain available under those terms. Apache-2.0
applies to the versions released under it and afterwards.

## Why Apache-2.0 rather than MIT

Apache-2.0 carries an express patent grant; MIT does not. For a protocol
implementation others are meant to interoperate with, an explicit grant is worth
more than the brevity of MIT. The same reasoning chose `serde_jcs` over an
MIT-only alternative when the two were otherwise tied (OW-ADR-0001).
