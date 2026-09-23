---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e81-73d3-b28f-cedb19f93c32
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `tools/friction/measure.sh`: the measurement.
   - Takes `--war <binary>`, `--runs <n>` (default 5), `--out <file>`, and
     optionally `--corpus <repo>` to time the read-only routine acts
     against an existing repository.
   - Setup, in a `mktemp -d` directory: `git init`, `war init --program`,
     the simulated human step (a generated key, `roles.toml` and
     `allowed_signers` written from it, a throwaway ssh-agent), `war sas
     propose`, `war sign <version> --ssh-sign`, `war check`, `war
     compile`, `war authorize <alias>`, `war sign <alias> --ssh-sign`.
   - Routine acts: the table in 20-basis U-001, each run `--runs` times,
     with median and maximum.
   - Each step records argv, exit code and wall time in milliseconds. A
     step with a non-zero exit records `unknown` time, every total that
     includes it is `unknown`, and the script exits 1.
   - Each human step records `kind: human`, what it asks for (files edited,
     commands, dialogs), and `time: not_measured`.
   - The record also carries the binary's `--version`, its build profile,
     the repository commit, OS, CPU model and the load average at start.
   - It writes nothing outside its temporary directory and `--out`. It
     kills the ssh-agent it started and restores `SSH_AUTH_SOCK`.
2. `docs/friction/baseline-1.json`: the first recorded baseline, from a
   release build of this repository's commit at delivery. It is output of
   the script, committed unedited.
3. `docs/FRICTION.md`:
   - the two targets and their source;
   - what the script measures, and what it cannot;
   - the baseline in a table, with human steps counted beside it;
   - a manual protocol for timing the human steps (administration, review
     and waiting, separately), recorded as a separate file;
   - the rule that a new baseline is a new file.
4. `conformance/plants.d/50-friction.sh`: on a scratch directory,
   - the script runs and its record has every step, with a time for each
     tool step and `not_measured` for each human step;
   - a wrapped `war` that exits 1 on `authorize` makes that step and the
     setup total `unknown`, and the script exit 1;
   - the repository tree is unchanged after the run, and
     `SSH_AUTH_SOCK` is what it was before.

## Frozen Surfaces

Every `war` command's behavior and output, every record schema, the
signing seam, `QUICKSTART.md`. The script is a consumer only.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any change to the list of routine acts beyond 20-basis U-001;
- any step the script cannot run without a real human key.

## Rollback

Delete the four files. Nothing else depends on them.
