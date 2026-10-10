# BLUT native verifier process adapter

Candidate CLI-library adapter, performer-reported. This is not an independent
verdict, provider qualification, participant acceptance or human signature.

`openwarrant_cli::runtime_capture::blut_process` implements the existing pure
`ReceiptVerifier` seam through BLUT's `openwarrant verify` process interface.
`BlutProcessVerifier::for_recorded_dispatch` reads the retained Dispatch and its
matching compile event. It refuses superseded attempts. The capture and current
basis assessor still decides source eligibility; this constructor does not grant
permission or establish authentic execution.

The host supplies `BlutProcessConfig`: a trusted provider interface, absolute
verifier/public-key/plan/job/producer paths, scratch root, deadline and response
limit. It must authenticate and protect those inputs. A path or identity label
is not protection. Configuration is never discovered from a receipt. The
adapter has no signing key and clears the caller's environment before invoking
the verifier. It sends the original receipt bytes and source-derived binding in
private temporary files, then removes those files. Native job files are read by
BLUT, not rewritten here. Unix process groups, bounded stdout/stderr and a
maximum 60-second configured deadline bound the process observation. This is
not a sandbox or a guarantee against hostile filesystem stalls.

BLUT's provider-owned interface is implemented in
[draft PR 103](https://github.com/Quitetall/blut/pull/103), code revision
`9f75a3fff4f818eb82ca0e220388827c9ca3e50a`; its pushed checkpoint
`0f1dbe8043d1eee50d8255ff199509fbbd9f1979` passed hosted technical CI. The CLA
check remains failed, the PR remains draft, and neither is waived here.
OpenWarrant invokes a separate process and adds no BLUT library dependency.

Validated responses retain BLUT's BLAKE3 receipt and catalog identities. Local
raw-byte identity remains OpenWarrant's SHA-256 domain. The expected catalog
identity must come from an independently resolved actual catalog; the legacy
`blut@<schema-pin>` lowering hint is not that identity. Completed execution must
have a lifecycle reference matching its checked native file table. Native file
references are URI-escaped `file://` paths under the configured job; they do not
declare accepted deliverables. Failed, canceled and deadline outcomes remain
distinct. Confinement, dollar cost and spend-cap enforcement remain UNKNOWN.
The adapter cannot issue assurance.

## Evidence and bounds

Public capture tests use an actual recorded prototype Dispatch and synthetic
neighbor processes. They exercise unavailable evidence, response mapping,
wrong attempt, wrong hash domain, missing/mixed lifecycle, unsafe/duplicate file
paths, false assurance, contradictory response/exit status, malformed/duplicate
JSON, excessive stdout/stderr, a nonresponsive child and environment leakage.
These tests do not validate a real BLUT seal or establish native execution.

`provider-check-red.log` records the missing public module. The mapping and
environment RED logs record observed behavior failures before the respective
implementation. A build observation timeout in `provider-red.log` is UNKNOWN,
not a failed test. GREEN and repository-check logs are retained here when run.

The final pinned Rust 1.97.1 public capture run passed all 15 tests
(`provider-final-tests.log`). `cargo fmt --all --check`, `git diff --check`
and `war compile --json` exited zero. The aggregate repository gate is a
separate check; these results do not stand in for it.

Still required: a real BLUT native job and seal from the exact recorded Dispatch,
trusted catalog/key/build custody, connection into configured workflow policy,
Katana's provider-owned interface, and independent/participant qualification.
OW-WAR-0149 remains in progress. The default CLI still has no enrolled trusted
native verifier.
