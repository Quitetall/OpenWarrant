# Telemetry source and retained baselines

For exact retained Git sources, use:

```sh
war telemetry --derived --commit HEAD --out artifacts/new-observation.json --json
```

The command resolves the selected revision before collection. `result.source_basis`
names the resolved commit and distinguishes frozen sources from a live working
observation. Missing sources remain UNKNOWN. The result is a measurement, not
qualification or evidence that it predates tuning.

Without `--derived`, the legacy v1 collector reads the live working tree and live
history. Its `--commit` value is a declared label. It is not a source pin, and the
report says `immutable_sources_established: false`. The legacy artifact format and
serialized bytes stay unchanged.

Both modes preserve existing artifacts. Identical byte replay succeeds; different
bytes, linked files and nonregular destinations refuse. Choose a new output path
for a new observation. `--verify` compares bytes without replacing the artifact;
it does not validate the baseline's collection method or qualification.

Unknown human timing, safety, repair follow-up and escape windows are not zero.
One observation supplies no improvement or reduction claim.
