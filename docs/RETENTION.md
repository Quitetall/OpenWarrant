# Retention, archiving and the 1,000-Warrant budget

OW-WAR-0120. On 2026-09-22 the owner asked three questions:

- Do journals need compacting?
- Should resolved Warrants be archived?
- What does the tool cost at 1,000 Warrants?

This document answers them from measurements. It also records a budget that a
gate holds.

## The answers in brief

- **Journals do not need compacting, and rewriting them is not an option.** They
  are about 1% of this corpus's bytes and grow in a straight line: roughly
  3.4 KB for each Warrant.
- **Archiving resolved Warrants would save almost nothing.** The bytes that
  grow are `implementation/` logs, and 99.8% of those belong to Warrants that
  are not resolved. Most `implementation/` files are referenced by no record.
  They are candidates for a later pruning decision, and this Warrant prunes
  nothing.
- **The budget did not hold at 1,000 Warrants.** Three of nine rows are within
  their limit. Six are over by at least ten times: `check --generated`, `next`,
  `sign --list`, `compile`, and both console paths. The cost comes from
  signature verification, and bytes have nothing to do with it. OW-WAR-0120's
  OBL-005 is refuted by this record.

## Journals

Measured on this repository's tracked `docs/warrants/` at commit `fdd346c5`
(136 Warrant directories):

| | value |
| --- | --- |
| journals | 136 |
| events | 1001 |
| bytes | 461991 (1.2% of 37058689 tracked bytes under `docs/warrants/`) |
| largest journal | 27 events, 13045 bytes |
| median journal | 3 events |
| mean bytes per event | 462 |

**Growth.** A journal grows with the acts taken on its own Warrant, not with
the corpus. At this repository's rate of 3397 bytes per Warrant, 1000 Warrants
hold about 3.4 MB of journal. The synthetic 1000-Warrant program measured
below holds 3249 events in 1526653 bytes. Its Warrants take fewer acts than
real ones, and its journals are still 7.7% of a corpus whose logs are small.
Neither figure is large enough to matter.

**Why compaction by rewriting is refused.** The journal is append-only by rule,
and the rule is load-bearing:

- §66.2: the local journal "SHALL not become a competing ledger". A compacted
  journal that replaces its lines is a second history.
- §33.8: "Compaction SHALL NOT launder untrusted influence". A summary written
  in place of events is exactly where that influence would disappear.
- `war check` refuses any edit or deletion of a committed line with
  `journal.rewritten` (`crates/openwarrant-cli/src/journal_cmd.rs`). A
  rewrite would have to be committed as a rewrite.
- WAR-SAS-RQ-084: historical records remain available.

If journals ever did need to shrink, the mechanism would have to be a checkpoint
that preserves the lines it summarizes. That choice is normative. It belongs in
an ADR and a later Warrant, not here.

## Archiving: what grows, and what must stay

Bytes by kind, from the same tracked files:

| kind | bytes | share |
| --- | --- | --- |
| `implementation/` | 24483683 | 66.1% |
| `gate-runs/` | 4058639 | 11.0% |
| corpus `generated/` | 2095955 | 5.7% |
| per-Warrant `generated/` | 1792920 | 4.8% |
| `atoms/` | 1304553 | 3.5% |
| `evidence/` | 1285810 | 3.5% |
| records (manifest, authorization, resolution, deliverables, ...) | 768962 | 2.1% |
| journals | 461991 | 1.2% |
| attestations, plan, verifications, amendments, corrections, other | 806176 | 2.2% |

What must stay:

- **Records, atoms, verifications, attestations, amendments, corrections and
  journals** are the history RQ-084 keeps. The tool reads them.
- **`gate-runs/`**: the receipts are §44.6 evidence. `war status` and `war
  resolve` read them from their location. The receipts name their stdout and
  stderr files (155 files, 4020548 bytes, referenced by path). No record
  names the other 82 by path: 57 `run.toml` files and 25 receipts, 38091
  bytes. The tool reads them from their location.
- **`generated/`**: these projections are rebuilt by `war compile`. They are
  not evidence, but they are committed and checked for drift, so they stay
  as long as `[generated] commit = true`.

What is a pruning candidate, for a later decision:

- **`implementation/`**: 742 files. 66 of them (4972709 bytes) are named by
  path in a tracked record, document or source file, and those must stay
  ("without removing referenced evidence", product spec, History and
  adoption). The other 676 files (19510974 bytes) are named by nothing. They
  are logs and captured output, and they are the candidates.
- **`evidence/`**: 35 files (115636 bytes) are named by path. 44 files
  (1170174 bytes) are not.

The path test is a heuristic. A file that nothing names can still be the
recoverable bytes a hash refers to. Whether a candidate may go needs the
owner's policy and a Warrant that proves each removed file is unreferenced.
This Warrant moves nothing.

**Resolved Warrants are not where the bytes are.** The 29 resolved Warrants
hold 42369 bytes of `implementation/`. The unresolved ones hold 24441314.
The three largest are OW-WAR-0111 (7.8 MB), OW-WAR-0085 (6.4 MB) and
OW-WAR-0086 (3.0 MB). Archiving resolved Warrants would not change the size
of the corpus. It would not change the tool's time either (see below).
`war archive` is preservation transport and not a way to set Warrants aside.
It stays that way.

## The budget, and the 1,000-Warrant measurement

`tools/scale/budget.toml` holds 20-basis U-002 as the owner accepted it by
authorizing: each read command at most 10 s median and `compile` at most
30 s, for a release build on 1000 Warrants with at least 500 resolved. The two
console rows are O-001's interactive paths, given the same 10 s as the read
commands. The `1`, `s` path O-001 timed cannot be budgeted: without an
ssh-agent it ends in `sign.ssh-refused` (exit 2, so the row would read
`unknown`), and with an agent it would sign. The row measures `1` followed by
end of input instead.

`docs/scale/baseline-1000.json` is `tools/scale/budget.sh`'s unedited output,
recorded 2026-09-24T03:16:41Z. Machine facts:

- **Binary:** `war 1.0.0-alpha.2`, built with the release profile. The binary's
  sha256 is `bdaa3db0…81bb`, built from `fdd346c5`. The record names
  `0afa1ba1`, the checkout's commit when it ran, which changes no source.
- **Machine:** Linux 7.2.6 x86_64 on a 13th Gen Intel Core i5-13600K, with
  `nproc` 20.
- **Load average:** 1.82 / 7.56 / 12.93 at the start and 1.54 / 1.94 / 5.99 at
  the end.
- **Corpus:** 1000 Warrants by `war status --json`: 500 resolved, 249
  authorized and 251 draft. It was built by `tools/scale/synth-corpus.sh --n
  1000 --resolved 500`.
- **Runs:** 5 runs per row. The cap is 10 times the limit.

| row | command | limit ms | median ms | verdict |
| --- | --- | --- | --- | --- |
| `check` | war check | 10000 | 8371 | within |
| `check-generated` | war check --generated | 10000 | >= 100000 | **over** |
| `next` | war next | 10000 | >= 100000 | **over** |
| `status` | war status | 10000 | 2929 | within |
| `pins-resolved-only` | war pins --resolved-only | 10000 | 2724 | within |
| `sign-list` | war sign --list | 10000 | >= 100000 | **over** |
| `compile` | war compile | 30000 | >= 300000 | **over** |
| `console-json` | war console --json | 10000 | >= 100000 | **over** |
| `console-select` | war console, answering `1` | 10000 | >= 100000 | **over** |

The budget did not hold. A `>=` row was still running at the cap on its first
run, and `budget.sh` stopped it. That row's time is a lower bound and its
verdict is over; its later runs were skipped. No row is `unknown`: every
command that finished exited 0. Generating the corpus also ran a whole-corpus
`war compile` on the 1000 Warrants. That compile took about 62 minutes at a
load of about 20. `synth-corpus.sh` logged the figure and `budget.sh` did not
measure it.

### Where the time goes

The time goes to signature verification, not to bytes. Every command that is
over verifies recorded signatures by spawning `ssh-keygen -Y verify` once per
check. It repeats checks, and nothing caches a result. Counted with `strace`
on synthetic programs (release build, during development):

| program | recorded responses | `sign --list` spawns | `next` | `compile` | `check` | `status` |
| --- | --- | --- | --- | --- | --- | --- |
| 12 Warrants | 22 | 65 | | | | |
| 100 Warrants | 176 | 3849 | 3923 | 8068 | 199 | 74 |

Eight times the responses brings about 59 times the spawns, so the growth is
roughly quadratic in signed records. `war sign <alias>` spent 9 s per resolution in
this loop at 60 Warrants, and 13 to 23 s at 100. That is why `synth-corpus.sh` records
its resolutions through `war resolve --response` by default. `check`,
`status` and `pins` grow linearly and stay within the budget.

A fix is a later Warrant's work. §88 requires an ADR and differential
conformance for any optimization that could change output. The obvious
candidate is verifying each signature once per process.

### What this measurement is, and is not

- The machine is the one named above. The record makes no claim about any
  other machine (R-001).
- The corpus is synthetic (20-basis A-002, A-003). Its authorizations share one
  batch signature. Its resolutions are drafted by the script, signed with a
  throwaway key, and ingested by `war resolve --response`, without the DSSE
  attestation `war sign` adds. Its gate is a fixture: `sha256sum --check` over
  the fixture files. Its atoms are short and it has no amendments or
  corrections. Each of these makes it cheaper than a real corpus of the same
  size, so a row that is over here would be over on real records too.
- Each resolved Warrant carries one `implementation/` log of 1461 bytes. That
  is the median over this repository's resolved Warrants, as A-002 asks. The
  large logs in this repository sit in unresolved Warrants, which the median
  does not see.

## Re-running it

The 1000 run is a release-time act (R-002). The battery exercises the gate at
N=12 only, in `conformance/plants.d/52-retention.sh`.

```bash
cargo build --release -p openwarrant-cli
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
  tools/scale/synth-corpus.sh --war target/release/war --n 1000 --resolved 500 --out /tmp/scale-1000
tools/scale/budget.sh --war target/release/war --root /tmp/scale-1000 \
  --budget tools/scale/budget.toml --out docs/scale/baseline-<n>.json
```

Generation takes about an hour and a half on the machine above. `budget.sh`
takes about 15 minutes with the default cap. `--cap-factor 0` removes the cap
and measures every row to completion, which takes hours. A new measurement is
a new file and never an edit of `baseline-1000.json`. Changing a number in
`budget.toml` is the owner's decision (40-work-order, Autonomy and
Escalation). It is never the way to turn a red gate green.
