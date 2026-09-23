---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eaf-7041-b99b-eaed6ac66f86
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `tools/scale/synth-corpus.sh`: `--war <binary> --n <count>
   --resolved <count> --out <dir>`.
   - Builds a program in `<dir>` (refuses a non-empty one) through the
     real commands, with a throwaway key in a throwaway ssh-agent and
     throwaway performer and verifier identities.
   - Adds one `implementation/` log per resolved Warrant, of the real
     corpus's median size.
   - Prints the counts it reached by phase, from `war status --json`,
     not from its own tally.
2. `tools/scale/budget.toml`: the commands and limits in 20-basis U-002,
   with the build profile and corpus shape they apply to.
3. `tools/scale/budget.sh`: `--war <binary> --root <corpus> --budget
   <file> [--runs <n>] [--out <file>]`.
   - Times each command `--runs` times (default 5); records median and
     maximum.
   - A non-zero exit makes that row `unknown`.
   - Exits 1 if any row is over budget or unknown, naming each.
4. `docs/scale/baseline-1000.json`: `budget.sh`'s output on a
   1,000-Warrant corpus from `synth-corpus.sh`, release build, committed
   unedited.
5. `docs/RETENTION.md`:
   - journals: measured size and growth, projected to 1,000, and why
     compaction by rewriting is refused (§66.2, §33.8,
     `journal.rewritten`);
   - archiving: bytes by kind (`implementation/`, `gate-runs/`,
     `generated/`, journals) for this corpus, which are referenced by a
     record and so must stay, and which are pruning candidates for a later
     decision;
   - the budget, the 1,000 measurement, and how to re-run it.
6. `conformance/plants.d/52-retention.sh`, at a small N:
   - `synth-corpus.sh --n 12 --resolved 6` builds a program where `war
     check` exits 0 and `war status --json` counts 6 resolved;
   - `budget.sh` with that corpus and the real budget passes;
   - with a budget of 1 ms it exits 1 and names every command;
   - with one manifest corrupted, the rows that fail are `unknown` and it
     exits 1;
   - the repository tree is unchanged afterwards.

## Frozen Surfaces

Every `war` command's behavior and output; every record schema; every
journal; every file under `docs/warrants/`. Nothing is moved or pruned.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any change to a budget number;
- any change to a `war` command, including an optimization;
- any proposal to prune, move or checkpoint a record.

## Rollback

Delete the six files. No record depends on them.
