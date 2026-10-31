# Contributing to OpenWarrant

## Read this first: the licensing constraint

OpenWarrant ships **Apache-2.0** (relicensed 2026-09-12, OW-ADR-0017); before that date it shipped AGPL-3.0-or-later and was relicensed
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

## Checks before merging and releasing

The PR tier runs locally and in Fast CI with the same command:

```bash
cargo xtask-fast --base origin/main
```

It builds the workspace, checks formatting and linting, runs workspace tests
including refusals, checks licenses, record/projection integrity, agent guidance,
schema drift and candidate acceptance. `--base` names the receiving base for
acceptance checks; CI uses `HEAD^1` on its merge candidate. `--list` prints the
steps without executing them. This tier does not run the full violation battery
or establish independent qualification.

The complete checkpoint gate remains:

```bash
cargo xtask gate
```

Its fourteen steps include build, formatting, linting, tests, licenses, schemas,
corpus drift, attestations and the full planted-violation battery. It reports
every failing step rather than the first. Run it before tags, releases or
downstream pins, nightly and on demand. A fast-tier pass is not a full-gate pass.

During the fast-tier rollout, the existing required `gate` remains in place.
Once `fast-gate` is green on the receiving candidate, require that explicit
status before retiring the old PR status. The full gate then moves off routine pushes. Release qualification keeps
the full gate and its finite deadline on the exact release commit.

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

## Ids that parallel branches mint

Many agents work on branches at once without coordinating, so nothing a
branch creates may take the "next number" from a shared sequence — two
branches both take it, and the merge renumbers one by hand. Tickets and items
are hash ids (`t-xxxx`, `i-xxxx`). Two other names were sequential and
collided; both are now safe to create on any branch:

- **Amendments** (`docs/warrants/<alias>/amendments/`, §31). Start one with
  `war sign amend <alias>`: it writes `AM-<n>-<hash>.yaml`, where `<n>` is one more
  than the highest ordinal on your branch (the number people cite) and
  `<hash>` is 4-16 lowercase hex digits minted fresh, and leaves the fields
  only a person can state empty — `war check` refuses the record
  (`amendment.invalid`) until they are written. Two branches that both write
  amendment 4 get `AM-004-1f3a` and `AM-004-c07e`: two files, no conflict.
  `war sign sas repin` mints the same way. **Order** is by ordinal, then the
  record's `effective_time`, then file name, so same-numbered amendments from
  two branches sort by when they take effect. Records named `AM-<n>` before
  this scheme keep their names and bytes — signed contracts cite them — and
  sort by their number, exactly as before. A file whose name is not
  `AM-<n>` or `AM-<n>-<hash>`, or whose `id:` is not its file name, is
  refused by name (`amendment.id`). Never rename an existing amendment.
- **Plant files** (`conformance/plants.d/NN-<name>.sh`). `NN` is a grouping,
  not an id: two files may share it, and several do. The runner sources the
  files in byte order of their names (`LC_ALL=C`), starts each from the
  restored tree, and fails by name a file that leaves a change `restore`
  cannot undo, so no file's result depends on which files ran before it. Pick
  the group a new plant belongs to; do not hunt for a free number.
  `conformance/plants.d/99-plant-order.sh` and `31-amendment-ids.sh` are the
  plants for both rules.

## Changing the standard

The standard is [docs/format/FORMAT-1.0.md](docs/format/FORMAT-1.0.md); how it
changes is [docs/format/GOVERNANCE.md](docs/format/GOVERNANCE.md). A change to
what a conforming reader must do is a spec change first: amend the text, the
schemas under `schemas/format/` and a case under `conformance/format/` together,
then the code. A case's expected output comes from the spec, never from what
`war` happens to print. The spec text is CC-BY-4.0; the schemas and corpus are
Apache-2.0.

## Making a change

1. **Open a Warrant.** `war plan new "<title>"` — this project is built through its
   own Warrants (SAS §93). Fill in the intent, basis, work order, milestones,
   and assurance atoms.
2. **Write the decision down if it is one.** A normative decision is a
   first-class ADR under `docs/adr/atoms/` (SAS §19). A choice already
   authorized by the Warrant's autonomy envelope is an execution choice, not a
   new decision.
3. **Build and gate.** `cargo build --workspace && cargo xtask gate`.
4. **Recompile projections.** `war admin compile`, then `war check --generated` to
   confirm no drift. Generated files are committed in this repository.
5. **Open a pull request.** Describe the diff you are actually submitting, not
   the state of the tree.

## The speed budget

The verbs an agent runs all day stay fast on a large program: `war view
ready`, `war next` and `war claim` each at most 50 ms median, for a release
build, on 10,000 Warrants (9,800 tickets and 200 directory Warrants, nothing
signed). `tools/scale/budget-10k.toml` holds the numbers; changing one is the
owner's decision, never the way to turn the check green.

```bash
cargo scale-budget                     # build, make the corpus, time it
cargo scale-budget --keep              # ... and say where the corpus is
cargo scale-budget --corpus <kept-dir> # time a copy of a corpus made before
```

It exits 0 only when every row is within its limit, and 1 naming each row
that is over or unknown. Building the corpus takes minutes, so this is not a
step of `cargo xtask gate`: run it before a release, and with any change to
what those three commands read. The battery exercises the same tooling at a
small size, refusals included, in `conformance/plants.d/174-scale-budget.sh`.
Time on a quiet machine: the numbers are wall clock, and a loaded one (or an
output file on a busy disk) measures the machine.

The three verbs (and `war view tickets`, `war status`) answer unchanged
tickets from a local cache under git's common directory
(`<common>/openwarrant/cache/`, `crates/openwarrant-cli/src/ticket/cache.rs`):
an entry is used only when every file it came from stats exactly as it did,
and anything else is read from the files. `OPENWARRANT_NO_CACHE=1` turns it
off, `OPENWARRANT_CACHE_TRACE=1` says what each run took from it, and `war
admin doctor` says where it is. A file changed less than two seconds ago is
never cached, so a corpus just copied (`--corpus`, `cp -a`) is read from its
files until it settles; the plant is `conformance/plants.d/180-ticket-cache.sh`. The signed path at scale is
the separate 1,000-Warrant budget in `docs/RETENTION.md`.

`war next`, `war status` and the overview and board views also keep each
signature `ssh-keygen -Y verify` accepted, keyed by every byte it read, in
`<common>/openwarrant/cache/ssh-verdicts/`
(`crates/openwarrant-cli/src/ssh_verdicts.rs`; never read by a command that
judges, gates or signs). It is bounded: the first use in a run prunes every
entry unused for 90 days, then the least recently used until at most 4,096
remain. It holds only passes, so a pruned entry costs one `ssh-keygen` run
and never changes a verdict. `OPENWARRANT_NO_CACHE=1` turns it off too; the
plant is `conformance/plants.d/181-ssh-verdict-cache.sh`.

## Commit messages

Say what changed and why it is correct. Where a review or a test found a real
defect, record what the defect was — the commit log is the only place that
survives. Where a review finding was a false positive, say that too, and say why
it did not reproduce.

Do not describe the tree; describe the diff. A message claiming "5 manifests" when
the diff contains 4 makes the log unusable as evidence.

## Releasing: what moves with the version

The Claude Code plugin's version moves **every release**: `.claude-plugin/plugin.json`
carries the `war` release it ships with as its `version`, equal to
`crates/openwarrant-cli/Cargo.toml`'s. A unit test
(`skew::tests::the_shipped_plugin_tracks_this_release`) fails a version bump
that leaves the plugin behind. The same bump regenerates
`docs/agents/legacy-warrant-workflow.md` (`war admin agents-md --stdout >
docs/agents/legacy-warrant-workflow.md`), because the AGENTS.md template ends
with the version that wrote it.

Both stamps are how `war admin doctor` and `war view prime` warn about version skew
(`install.version-skew`): an AGENTS.md or plugin newer than the running `war`
describes commands the binary may not have, and an agent would read the old
binary's refusal as a rule of the repository. An equal or older stamp, or none,
says nothing. `crates/openwarrant-cli/src/skew.rs` is the mechanism.

## Signing, and what it does not defend

`war sign --ssh-sign` is only as strong as the ssh agent behind it: the key
must be loaded with `ssh-add -c`, so every signature asks the human through
the agent's confirmation dialog. `war` cannot verify that; it is the one
thing the operator must get right. The full list — ten threats, each with
its control, its residual, and the test or plant that exercises it — is
[`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md). A change to a control changes
that file in the same commit.

## Reporting bugs

Open an issue with the exact command, the full output, and the toolchain version.
For anything security-relevant, read [`SECURITY.md`](SECURITY.md) first and do
not open a public issue.
