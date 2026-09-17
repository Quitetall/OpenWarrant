# OW11: partial Preflight CLI

Source: `1bb248699e38ac6de5f279380ab63a4943e1ba40`. Implementation remains **partial and unverified**.
No legacy authorization, assurance disposition or resolution changes here.

`war preflight <alias>` lists all 41 checks across six SAS §32 dimensions.
The existing core readiness model blocks on every failed or unknown check.
Local observations do not establish a live actor path, protected gate execution,
side-effect authority or human acceptance. No state is persisted.

See [command reference](../../../cli/preflight.md) for behavior and JSON limits.

## Evidence

Six public CLI tests and Clippy passed on Rust 1.97.1. Independent Spec review
passed for the explicitly partial scope. Standards review found two report-truth
defects: invalid graphs could leave reachability PASS, and damaged authorization
containers appeared absent. Both were fixed and independently rechecked.

The local full gate passed 14 steps and 308 controls **before those final two
fixes**. Its log name preserves that boundary. Exact final source must pass the
protected-main PR gate before merge. These tests and reviews are development
evidence, not formal Warrant verification.

Actual free-local LAMU `review_commit` returned PASS WITH NITS. Its claimed
last-graph-wins defect is not present: the observation recorder applies `worst`
on every insertion, and the multiple-graph regression passes. Its suggestion to
make a confirmed absent authorization UNKNOWN was rejected: known absence fails
this legacy authorization requirement, while inability to inspect it is UNKNOWN.
The coarse UNKNOWN status suggestion adds no defect; observations carry reasons.

## Remaining work

- Supply concrete actor-path, gate, context and authority observations before
  claiming complete readiness. The recorded Warrant basis explicitly allows only
  partial Preflight until the live actor path exists.
- Correct the stale generic checker text through OW-WAR-0005/D-002's human
  correction path. [Prepared patch](proposed-diagnostic-correction.patch) changes
  only that warning; the pinned source remains unchanged. Existing source digest
  is `sha256:a5b6b65308e19021833bbf2942f8f0b5cf36cd07b0721891788714234b28ae5a`;
  proposed digest is `sha256:e929310b3bbfb10be68801f3e750e313deffc8f9cfd6ea8a3b5e80151f37aeb9`.
  This patch is a review artifact, not an applied or signed correction.
- Human acceptance and legacy resolution remain separate acts.
