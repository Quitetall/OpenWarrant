# Source currency before capture publication

Unverified implementation under OW-WAR-0149, based on
`e48150ccebd1fd50bcd625e5b41e1032e709cad8`. No independent disposition,
participant acceptance, human act or release qualification is recorded.

## Observed gap

A native verification callback can outlive the source snapshot used by import.
The public SDK control changes an otherwise valid intent source inside a
synthetic verifier. The unchanged import published a new capture whose native
observation said `matches`, despite the changed current basis. `red.log` retains
the terminal failure; it is a synthetic source-binding control, not native
execution evidence.

## Repair and limits

Import reloads the current source basis, exact retained Dispatch and matched
compile-event bytes, and the stage-attempt frontier after native verification.
An observed change returns `runtime.capture-changed` as UNKNOWN before creating
or publishing a retained capture. The control checks that prior capture bytes
and count stay unchanged. It also retains the existing matching, mismatch,
current-selection, missing-provider and unknown-cost checks.

This is not an atomic transaction, writer fence or authentication mechanism.
The host still serializes or isolates writers; changes after the final read or
changes hidden between reads are outside this observation. Receipt bytes are
still captured once. Historical inspection and retention remain distinct from
current-attempt eligibility. No wire schema, provider seal or canonical hash
domain changes, and no historical capture is rewritten.

## Validation

Rust 1.97.1 (`8bab26f4f`, 2026-07-14):

- Red public-seam control: FAIL as expected, import returned a retained match.
- Repaired focused control: PASS, one test, 79.85 seconds (`green.log`).
- Full CLI integration suite: PASS, 213 tests, four explicit ignored fixtures,
  36.83 seconds (`integration.log`).
- `cargo fmt --all -- --check`: PASS.
- Strict CLI all-target/all-feature Clippy (`-D warnings`): PASS (`clippy.log`).
- Own freshly built CLI compilation and `war check --generated`: PASS,
  2,028 pass, 625 warnings, zero errors/unknown results. These are record checks,
  not gate execution or independent qualification.

The ignored native-producer fixtures remain unqualified. Hosted full gate and
production/native workflow qualification are separate requirements.
