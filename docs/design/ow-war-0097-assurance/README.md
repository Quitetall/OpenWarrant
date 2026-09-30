# OW-WAR-0097 assurance review package

This package is a draft. Nothing here replaces revision 1, activates a gate,
records a verifier verdict, or requests a signature against changed atoms.

The [amendment proposal](../ow-war-0097-assurance-amendment.md) explains the
missing gate reference. The [proposed assurance atom](60-assurance.proposed.md)
preserves the three behavioral obligations and adds the structural obligation.
Install neither copy over signed source before revision approval.

The [gate definition](software.cli.doctor@1.0.0.yaml) is intentionally `draft`.
The [adapter](doctor-gate.py) runs the existing acceptance tests with a fresh
temporary build directory. The source tree is its working directory. Cargo uses
the exact pinned toolchain and frozen dependency resolution. That toolchain must
already be available. Its artifacts and fixture files are outside that tree.

Run the bounded local observations from the repository root:

```bash
python3 docs/design/ow-war-0097-assurance/qualify.py
```

This takes archives of the named HEAD into disposable directories. It runs the
clean suite, introduces three separate doctor regressions, requires the named
fixture to detect each regression, and reruns the clean suite. It compares every
source file before and after each execution. Output and digests are in
`observations/qualification.json` and the referenced logs. These are performer
observations, not Warrant acceptance receipts or independent dispositions.
Logs are losslessly compressed as `.log.gz`; the manifest records the compressed
file digest and the original output digest. Large assertion dumps remain intact.

The fresh-build observation completed all five cases: two clean passes and
three detected regressions. Every source snapshot remained identical during
execution. The stored log hashes were checked against the observation manifest.

## Two boundaries found during preparation

1. **Shared build output can retain a planted binary.** The first experiment used
   one shared target directory for all archived sources. The final clean archive
   had older timestamps than the mutated source; Cargo reused the last mutant
   rather than rebuilding. The failed final control is retained under
   `observations/stale-target-control/`. Its JSON describes that first experiment;
   only the failing final log is preserved there. Earlier log paths in that JSON
   refer to the first experiment and must not be interpreted as the retry logs.
   The adapter now creates fresh output for each run.
2. **Missing nested tools are not classified correctly by the current runner.**
   `gate_cmd.rs::askability_of` probes the first argv element; `run_gate` maps a
   nonzero child exit to FAIL. A Python adapter with missing Cargo or Rust therefore
   cannot establish an UNKNOWN gate result merely by printing an unavailable
   prerequisite. This draft cannot be registered as ready until that boundary is
   addressed. A service-stage binding does not bypass the runner's mutating-gate
   refusal either (`run_cmd.rs` calls the same `run_gate`).

The unavailable-prerequisite boundary was also reproduced against the CLI in a
disposable synthetic gate fixture. Its report is
`observations/unavailable-prerequisite.json`; the report names and hashes the
exact observed executable. Reproduce with an explicit CLI path:

```bash
python3 docs/design/ow-war-0097-assurance/probe-prerequisite.py --war /path/to/war
```

The source implementation of `doctor` is not implicated by either boundary.
Do not weaken its obligations or mark this Warrant resolved to work around them.

## Remaining work

- Establish prerequisite/infra classification that preserves UNKNOWN through the
  actual gate runner. Keep this gate draft until then.
- Establish a deadline control that stops the nested Cargo process tree; killing
  only the Python adapter is not proof that the work stopped. No timeout
  qualification is claimed by these observations.
- Qualify and register the complete gate, including that unavailable-input control.
- Prepare and approve revision 2 against its exact amended contract.
- Record fresh behavioral and structural gate receipts with relevant source inputs.
- Obtain independent obligation dispositions and human resolution.

The proposed adapter would be installed as `tools/gates/doctor-cli.py`, and the
definition as `docs/gates/software.cli.doctor@1.0.0.yaml`. Declare both delivered
paths in the revised work order and manifest. Gate definition immutability applies
after registration; changing a registered definition requires a new version.
