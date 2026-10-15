# Git inventory diagnostic observation

A real corrupt Git index made the public verifier request return `ERROR`, hiding
Git's exit status and stderr. The failed request could not obtain source evidence;
it did not establish a defective Warrant. The candidate uses the existing
`ObservationUnavailable` error and keeps the first 2048 stderr bytes plus the
process exit status. A truncation marker makes omitted stderr explicit. The Gitless
fallback and source selection rules are unchanged. No verdict is written.

Observed with Rust 1.97.1:

- Baseline request: ERROR, generic message; retained in `red.stdout.gz`.
- Candidate real Git success → corrupt index → exact restore: PASS; the corrupt
  index reports UNKNOWN, exit status 128 and `index file smaller than expected`.
- Public verifier suite in isolated TMPDIR: 50 passed, zero failed, including an
  oversized-stderr control and the existing reviewed-subject refusal cases.
- Registered `84-git-inventory-diagnostics.sh` plant: PASS.
- All-target CLI Clippy with warnings denied: PASS.

The initial Cargo invocation named an obsolete standalone test target and failed
before running tests. The first compiled test used a snapshot helper that required
existing verdicts; it failed in fixture setup and was corrected to compare optional
files. Both failures are retained. The first broader run used the shared temporary
directory and returned 18 passed / 32 failed, with actual Git stderr stating it was
not a Git repository. The isolated replay passed 50/0; it does not repair or explain
all historical source-inventory failures. These are performer observations.

Reproduce the paired public control:

```bash
python conformance/fixtures/verifier/git-inventory.py "$PWD" "$PWD/target/debug/war" /path/to/isolated/scratch
```

Run the broader suite with an isolated temporary directory:

```bash
TMPDIR=/path/to/isolated/scratch cargo test --locked --offline -p openwarrant-cli --test all verification_subject_cli::
```

`manifest.json` binds the tested source files and losslessly compressed raw logs.
The original full-sequence failure's cause and repair remain UNKNOWN; i-9f40 stays
open. Full CI, independent verification, human acceptance and release qualification
are not established by this observation. Only OW148/D-035 records the current
edited source; older overlapping declarations and resolved manifests are unchanged.
