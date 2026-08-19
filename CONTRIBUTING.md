# Contributing to OpenWarrant

## Read this first: the licensing constraint

OpenWarrant ships **AGPL-3.0-or-later** today and is intended to be relicensed
**Apache-2.0** when it goes public. That is not an administrative detail you can
ignore — it constrains what you may contribute.

**Every dependency must be MIT and/or Apache-2.0.** A GPL, AGPL, LGPL, or
weak-copyleft crate adopted now could not be relicensed later, and the deeper it
sits in the graph the more code has to be rewritten when that is discovered.
`cargo deny check licenses` enforces this inside the gate, so a pull request
adding a copyleft dependency fails automatically rather than in review.

**Contributions are accepted under the terms in [`RELICENSING.md`](RELICENSING.md).**
By opening a pull request you agree that your contribution may be distributed
under Apache-2.0 as well as AGPL-3.0-or-later. Without that, a single
contribution from a person who later becomes unreachable makes the relicense
impossible for everyone.

## The gate

One command decides whether a change is acceptable:

```bash
cargo xtask gate
```

Five steps: `fmt`, `clippy`, tests, licenses, and the planted-violation battery.
It exits zero only when every step passes, and it reports **every** failing step
rather than the first.

CI runs exactly this. There is no separate CI configuration to keep in sync, so
"green locally" and "green in CI" cannot come to mean different things.

## Rules that are not style preferences

**The toolchain is pinned exactly.** `rust-toolchain.toml` names the version.
A newer clippy is not a superset of an older one — lints are added, removed, and
renamed in both directions — so state the toolchain in any claim about a gate
result, or do not make the claim.

**A test that cannot fail is not a test.** Every control must have a case that
plants a violation and asserts the specific refusal. `conformance/plant.sh` does
this for the shipped binary; unit tests do it for functions. If you add a
validator, add its plant. A test asserting `things.len() == 4` proves nothing
about what the four things are.

**Never fix a document to satisfy a tool.** If the parser and the corpus
disagree about format, establish which is wrong before changing either. Making a
valid document invalid to make a linter green falsifies the record, which is the
one thing this project exists to prevent.

**Unknown is not failure and not pass.** A check that cannot run reports
`UNKNOWN`. Do not degrade it to `ERROR` (which makes a sound Warrant look
defective) or to `PASS` (which makes an unasked question look answered).

**No placeholder canonicalization, ever.** Anything that changes the bytes a
digest is computed over is a wire-format change and needs an ADR first. A digest
minted under a stand-in is indistinguishable from a real one after the fact.

## Making a change

1. **Open a Warrant.** `war new "<title>"` — this project is built through its
   own Warrants (SAS §93). Fill in the intent, basis, work order, milestones,
   and assurance atoms.
2. **Write the decision down if it is one.** A normative decision is a
   first-class ADR under `docs/adr/atoms/` (SAS §19). A choice already
   authorized by the Warrant's autonomy envelope is an execution choice, not a
   new decision.
3. **Build and gate.** `cargo build --workspace && cargo xtask gate`.
4. **Recompile projections.** `war compile`, then `war check --generated` to
   confirm no drift. Generated files are committed in this repository.
5. **Open a pull request.** Describe the diff you are actually submitting, not
   the state of the tree.

## Commit messages

Say what changed and why it is correct. Where a review or a test found a real
defect, record what the defect was — the commit log is the only place that
survives. Where a review finding was a false positive, say that too, and say why
it did not reproduce.

Do not describe the tree; describe the diff. A message claiming "5 manifests" when
the diff contains 4 makes the log unusable as evidence.

## Reporting bugs

Open an issue with the exact command, the full output, and the toolchain version.
For anything security-relevant, read [`SECURITY.md`](SECURITY.md) first and do
not open a public issue.
