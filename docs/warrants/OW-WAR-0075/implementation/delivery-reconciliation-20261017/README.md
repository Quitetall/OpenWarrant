# W75 delivery reconciliation

Observed 2026-10-17 from source
`d2131e2ecb8011ec07ef6151677eb88eff2375e1`, using Rust 1.97.1.

The exact public command `cargo run --locked -j 1 -p openwarrant-core
--example sdk_probe -- --scope 75 --fixtures conformance/sdk` passed all
22 document cases. Its output explicitly excludes context/readiness. A
fresh runtime filename and case id pass with the valid RC.3 source; the
same file with trailing bytes fails that case and exits 1. The copied
fixture directory was removed after both processes ended. Tracked
fixtures were unchanged. Adjacent observations retain actual argv, output,
source/executable hashes and exit codes. These are performer observations.

All 31 paths from the retained 2026-10-09 proposal exist. The current pin
report lists none under another Warrant. Actual delivery dry run accepts
the set. Each artifact was then recorded by `war admin deliver` with its
own actual input hash, explicit current reuse method, attempt and Codex
producer. This records present reuse, not original authorship of shared
files. Git retains that authorship and earlier versions. The old proposal
and authorization are byte-identical to their inputs.

The active declarations also include this report and its observation file,
with their own current production provenance. The authorization is a
legacy record without a signed ownership set: these declarations do not
grant ownership or attribute later work to an old signature. No signed
contract, authorization or effective policy was rewritten.

The original missing-declaration diagnostic and earlier test observations
remain under `implementation/preparation-20261009/`. That preparation
report describes the earlier state; this report records the subsequent
performer declaration and actual delivery.

Not established here: full phase/release qualification, independent
verification, secure human acceptance or legacy resolution. No obligation
disposition was authored. The cited Warrant gate is run separately after
the input tree is committed; its actual receipt is the evidence for that
run. Passing parser fixtures is not an assurance mark.
