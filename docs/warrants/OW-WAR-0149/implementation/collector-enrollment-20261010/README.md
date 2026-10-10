# Collector enrollment: scoped SDK observations

Candidate SDK policy/codec for OW-WAR-0149. The seven controls use a synthetic
signature adapter: they establish relationships and refusals, not cryptography,
human presence, operator custody, activation or native runtime enforcement.

- Seven targeted controls PASS.
- Core all-target/all-feature suite: 704 PASS, zero failures/ignored, 21 suites.
- Core all-target/all-feature Clippy with warnings denied PASS.
- Formatting and whitespace checks PASS.
- First API RED: missing module. First implementation compile: unsupported
  hexadecimal formatting, corrected with explicit lowercase digest bytes.
- Default-profile compile timed out before any test verdict. Lean rerun passed;
  debug assertions and test scope were unchanged.

Exact commands use Rust 1.97.1, `--locked`, `-p openwarrant-core`, the test
`runtime_collector`, and then `--all-targets --all-features`. Scoped Clippy uses
`--all-targets --all-features -- -D warnings`. The passing local runs set
`CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, DEV/TEST DEBUG=0, and clear the compiler
wrapper while reusing the project's existing target cache. No paid model calls.
The full repository gate and reference transport remain separate requirements.

See [contract and boundaries](../../../../design/runtime-collector-enrollment.md).
No signature, disposition, human acceptance or Warrant resolution was issued.

Full record diagnostics are retained byte-for-byte in `record-check.json.gz`;
`record-check-summary.json` gives readable counts and the original byte digest.
