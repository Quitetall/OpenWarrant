# Signature assertion repair evidence

Patch: `0b05e5f3`, based on `1dcbfd79`.

The stub controls execute the assertion extracted from the actual plant, with
its actual `line_has` helper. The first red run shows an exit-1 command
incorrectly accepted. The second red run shows an empty exit-0 report accepted.
Final controls reject both and the named OW-WAR-0073 refusal, and accept a
successful report without that refusal. Shell syntax and diff checks passed.

The full `69-current.sh` plant ran in a disposable local clone of this patch.
It used a copied Rust 1.97.1 CLI built from companion repair `91f912aa`, which
adds telemetry success envelopes to the same base; this is not a claim of a
fresh build of this exact branch. Binary sha256:
`3d2fec0606a4c56bba748deb0caf393131b6555fdfaeaf5b525f27ddc4dc11b3`.

The run exited 1: **29 passed, 2 failed**. The corrected signature assertion
and its controls passed. Failures were the positive corpus check and freshness
of CURRENT.md. Follow-up `war check --generated --json` reported drift in
CORPUS_STATUS.md/json/html and CURRENT.md/HISTORY.md. No projection or authority
record was changed to hide those findings. The whole plant is not qualified.

The separate telemetry repository gate is still running. No independent
verification, human signature, Warrant resolution or release claim is made.

## Integrated rerun

The combined branch regenerated the five stale projections through `war compile`
at `7fbd202f`. A 5,699-file byte snapshot proves authored Warrant, SAS, ADR and
authority files were unchanged. `war check --generated` then reported 2,001
passes, 352 warnings, zero errors; live status equals the generated JSON.

`69-current` was rerun in a clean detached worktree of `7fbd202f`, using the
same copied binary recorded above. It finished **31 passed, zero failed**,
exit 0. Its raw output is `runtime-integrated-69-current.log`. This establishes
the scoped plant outcome, not the whole repository gate or a Verified mark.
