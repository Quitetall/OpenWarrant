# Explicit archive resource limits — unverified implementation

The existing SDK accepts caller-selected archive, record-count and aggregate
content budgets. The CLI currently always uses the SDK defaults: 33,554,432
archive bytes, 4,096 records and 16,777,216 total content bytes. The retained
BLUT producer is 33,119,377 bytes, before its job inputs or history. The default
budget cannot retain that executable as part of a complete native archive.

This slice exposes the existing budgets on each archive operation:
`--max-archive-bytes`, `--max-records` and `--max-content-bytes`.
Unspecified budgets retain the SDK defaults. Values must be positive integers
with room for the bounded reader's one-byte overflow probe; there is no
unlimited value and no implicit growth. Content bytes cover the sum of all
resolved records, including external evidence, not a per-file allowance.

Example for a caller that permits a larger archive:

```sh
war archive import source.json imported --max-archive-bytes 134217728 --max-content-bytes 67108864
war archive reexport imported again.json --max-archive-bytes 134217728 --max-content-bytes 67108864
```

Limits do not change archive canonical bytes, digests, coverage claims or
inert import permissions. Larger budgets do not activate authority or make
unavailable native references complete. Native input retention, full native
import/re-export, the KF runtime round trip and independent qualification remain
unfinished. OW111 remains in progress.

The public CLI regression uses an explicitly labeled synthetic transport
fixture, not a BLUT execution or qualification record. It checks rejection at
the default content budget and exact source-detached import/re-export under
an explicit larger budget. Further refusal checks cover the individual budgets,
external aggregate bytes and invalid limit values. All checks use Rust 1.97.1.

Observed results so far: the large-record regression passed; all 23 preservation
integration controls passed; strict all-target/all-feature CLI Clippy and
formatting passed on Rust 1.97.1. `native-limit-commands.json` records real
export, inspect and runtime-basis queries against the retained CPU fixture.
`native-limit-observation.json` records identical default/explicit exports from
the same source. The older fixture archive differs only in the actual archive
producer executable digest, which correctly changes when the CLI is rebuilt.
These observations do not establish native runtime coverage or assurance.

The initial fixture-construction failure is retained separately: the synthetic
binary originally had no coverage classification. The fixture was corrected
before the meaningful RED run, which refused the unsupported CLI option.
No production validation was weakened.

The first full CLI run passed 445 library tests, 3 binary tests and 222 integration
tests, with 4 ignored. One integration test failed: the committed corpus
projection was stale after the authored OW111 progress update. Its exact failure
is retained in `full-cli-tests.log.gz`; regenerate with `war admin compile` and
rerun the check, rather than change the assertion or the authored record.
