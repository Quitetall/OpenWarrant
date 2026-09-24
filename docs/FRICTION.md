# Friction: what is measured, and what is not

OW-WAR-0118. This document explains the friction numbers and says what they
cannot show. The numbers come from `tools/friction/measure.sh`. The first
recorded baseline is `docs/friction/baseline-1.json`.

## The targets

`docs/design/openwarrant-product-spec.md`, "Friction targets and proof":

| Experience | Agreed target |
| --- | --- |
| Ordinary repository setup | Five to ten minutes or less |
| Routine Warrant administration after setup | At most 60 seconds; no manual record editing or required user shell commands |
| Human effort reporting | Administration, substantive review/decisions, and installation waits reported separately, and in total |

The spec calls these "targets, not measured achievements". It lists "the
real-work baseline and measurement procedure" as a contract still to be
specified. This document and the script are that procedure for the tool's
part. The human's part has only a manual protocol (below) and no recorded
measurement.

## What the script measures

`tools/friction/measure.sh --out <file> [--war <binary>] [--runs <n>] [--corpus <repo>]`

- **Setup.** In a fresh `mktemp -d` directory, the script follows
  `QUICKSTART.md`'s governed workflow from an empty directory to an authorized
  first Warrant: `git init`, `war init --program`, `war sas propose`, `war
  sign <version> --ssh-sign`, `war check`, `war compile`, `war authorize`,
  and `war sign <alias> --ssh-sign`. It repeats the whole sequence `--runs`
  times (default 5), each run in a new directory.
- **Routine acts.** The routine acts are the list in OW-WAR-0118 20-basis
  U-001: `new`, `check <alias>`, `compile`, `authorize` (the request), `sign
  --ssh-sign` (the tool's part), `next`, `status` and `resolve --dry-run`.
  They run on the program the first setup built. Each run creates its own
  Warrant and takes every act on it. The script keeps the list in one table,
  `ROUTINE`.
- **Corpus.** With `--corpus <repo>`, the script also times the read-only acts
  (`check <alias>`, `next`, `status`, `resolve --dry-run <alias>`) against an
  existing repository. It uses the repository's highest alias. The
  repository's `git status --porcelain` must be identical afterwards. If it is
  not, those rows are `unknown`.
- **Each step** records its argv, its exit codes and its wall time in
  milliseconds for every run. The step's value is the median (`median_low`)
  of those runs, reported beside the maximum. The time includes process
  start, because the time is taken with `date +%s%N` around the command.
- **Each record** carries `war --version`, the build profile and how the
  script determined it, the binary's sha256, the repository commit, the OS,
  the kernel, the CPU model, `nproc`, and the load average at the start and
  at the end.

The human's key is simulated. The script generates an ed25519 key and loads
it into an ssh-agent that it starts itself, without `-c`. It refuses to go on
unless that agent lists exactly its own key. That agent signs the same bytes
that a confirmed human key would sign (A-001), so the tool's time is real. The
human's confirmation takes no time in the simulation, and the script does not
time it. The script kills the agent and restores `SSH_AUTH_SOCK`. It writes
nothing outside its temporary directory and `--out`.

## What it cannot measure, and how it says so

- **Human steps are counted and not timed.** Each human step is a row with
  `kind: human`. The row lists what the step asks for: files edited by hand,
  commands typed, dialogs confirmed, and requests read. Its time is
  `not_measured`. The plants fail a record in which a human step carries any
  number, including zero.
- **A failed step is unknown.** A step that exits with a code other than its
  answer has `unknown` times. Every total that includes it is `unknown`, the
  record's `status` is `unknown`, and the script exits 1 (Law 15). Only
  `resolve --dry-run` answers with a non-zero code. It exits 2 when a Warrant
  is not ready, which is the command's verdict `not_ready`. The record lists
  that answer as `answer: not_ready`. An exit of 1 from `resolve --dry-run`
  is an error, and the script records it as `unknown`.
- **Setup target: neither met nor missed by this evidence.** The tool's part
  of setup takes well under a second. The human's part is three steps that
  nobody has timed. The five-to-ten-minute target is about the sum, so this
  record cannot say whether the target is met.
- **Routine target: the tool's part only.** One routine sequence takes the
  tool well under a second on a small program. The per-act figure on this
  repository's own corpus is larger: `war next` takes about 8 s here. Each
  routine sign still needs a human dialog and a typed `war sign`, and the
  target says "no required user shell commands". This record does not
  measure the dialog, and it cannot settle the shell-command clause.
- **Other operating systems.** The spec asks for each supported OS. This
  record covers the one machine it names (Linux x86_64).
- **Authoring.** The routine Warrants keep `war new`'s TODO atoms. The time
  to write real atoms is substantive work. It is not administration, and the
  script does not measure it.

## Baseline 1

The first run of setup includes a cold start. `war init --program` took 959 ms
in run 1 and 3 ms at the median. For that reason, every figure below is a
median of 5 runs, with the maximum beside it.

`docs/friction/baseline-1.json`, recorded 2026-09-24. Machine facts:

- **Binary:** `war 1.0.0-alpha.2`, built with the release profile
  (`target/release/war`). The binary's sha256 is `bdaa3db0…81bb`.
- **Repository commit:** `fdd346c54417069593d0f393dc63fe24d70424b8`, with no
  tracked changes.
- **Machine:** Linux 7.2.6 x86_64 on a 13th Gen Intel Core i5-13600K, with
  `nproc` 20.
- **Load average:** 22.71 / 22.88 / 24.19 at the start and 19.55 / 22.06 /
  23.85 at the end. The machine is shared, and a quieter machine should be
  faster (A-002).

Setup, from an empty directory to an authorized first Warrant:

| section | step | command | median ms | max ms |
| --- | --- | --- | --- | --- |
| setup | `git-init` | git init | 3 | 15 |
| setup | `init-program` | war init --program Friction --namespace FR | 3 | 959 |
| setup | `human-say-who-may-sign` | HUMAN: write roles.toml and allowed_signers; ssh-add -c | not measured | not measured |
| setup | `sas-propose` | war sas propose 0.1.0 | 3 | 18 |
| setup | `human-accept-sas` | HUMAN: read the acceptance request; type war sign; confirm one dialog | not measured | not measured |
| setup | `sas-sign` | war sign 0.1.0 --ssh-sign (tool part) | 16 | 68 |
| setup | `check` | war check | 10 | 40 |
| setup | `compile` | war compile | 13 | 51 |
| setup | `authorize` | war authorize FR-WAR-0001 | 5 | 18 |
| setup | `human-authorize-first-warrant` | HUMAN: read the authorization request; type war sign; confirm one dialog | not measured | not measured |
| setup | `authorize-sign` | war sign FR-WAR-0001 --ssh-sign (tool part) | 16 | 87 |

The tool's setup total is 68 ms at the median over the 5 runs, with a maximum
of 1,244 ms (the cold run). Three human steps are counted beside it: two files
edited, five commands typed, two dialogs and four requests read. None of them
is measured.

Routine acts, one Warrant per run, on the scratch program:

| section | step | command | median ms | max ms |
| --- | --- | --- | --- | --- |
| routine | `new` | war new | 3 | 3 |
| routine | `check-alias` | war check <alias> | 10 | 12 |
| routine | `compile` | war compile | 93 | 157 |
| routine | `authorize` | war authorize <alias> | 3 | 3 |
| routine | `human-confirm-routine-signature` | HUMAN: read the request; type war sign; confirm one dialog | not measured | not measured |
| routine | `sign` | war sign <alias> --ssh-sign (tool part) | 35 | 45 |
| routine | `next` | war next | 53 | 63 |
| routine | `status` | war status | 14 | 29 |
| routine | `resolve-dry-run` | war resolve --dry-run <alias> (answers not_ready, exit 2) | 6 | 9 |

One routine sequence takes the tool 217 ms at the median, with a maximum of
317 ms. One human step, the signature dialog, is counted and not measured.
`compile` grows with the program because each run adds a Warrant.

Read-only acts on this repository's corpus (about 140 Warrants, alias
OW-WAR-0140):

| section | step | command | median ms | max ms |
| --- | --- | --- | --- | --- |
| corpus | `check-alias` | war check OW-WAR-0140 | 171 | 192 |
| corpus | `next` | war next | 8064 | 8698 |
| corpus | `status` | war status | 591 | 1130 |
| corpus | `resolve-dry-run` | war resolve --dry-run OW-WAR-0140 (answers not_ready, exit 2) | 115 | 118 |

`war next` is the one routine act whose tool time on a real corpus is a
visible part of the 60-second budget. OW-WAR-0120 carries the budget at scale.
This Warrant does not tune any number.

## Timing the human steps by hand

Only a person can measure the human part. To do it:

1. Use a machine with `war` installed, an ssh key, and a terminal. Keep a
   stopwatch or a clock with seconds visible.
2. Follow `QUICKSTART.md`, "Governed legacy workflow", steps 1 to 4, in an
   empty directory. For each human step in the tables above, note three
   separate durations:
   - **administration:** typing commands, copying and editing
     `roles.toml` and `allowed_signers`, and loading the key;
   - **review and decision:** reading the SAS, reading each request, and
     deciding to confirm;
   - **waiting:** installation, builds, and anything else where the person
     waits on the machine.
3. Record each duration in seconds, or record `not_measured` for a step you
   did not time. Never record zero for a step that took time.
4. Write the result as its own file, `docs/friction/human-<n>.json`. Name the
   person's role (not their name, unless they agree), the date, the `war
   --version` and the machine. List each step id from the baseline with its
   three durations, and give the totals for each kind and overall.
5. Never merge a human record into a scripted baseline, and never add its
   numbers to the tool's numbers in one field. A reader compares the two
   records side by side.

## New baselines

A new baseline is a new file: `docs/friction/baseline-2.json`, and so on. A
new baseline never replaces an edited old one. Each file is the script's
unedited output from one run. A change meant to improve a number is measured
against the file recorded before it. If this document's tables change, they
must match the baseline they cite, and `conformance/plants.d/50-friction.sh`
checks that every row matches.
