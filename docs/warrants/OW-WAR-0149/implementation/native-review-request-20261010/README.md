# Native runtime review request

Source: `ab29b1912a7b6d341ca0eea82517f7fb31d19cf9`; integration with the refreshed parent: `18d0ebd05fc8f5ba38520db2359ab3ded55c3076` (identical implementation tree).

The SDK `resolution_cmd::request_with_runtime` and CLI `war resolve <alias> --runtime-selection <file-or-> [--native-store <protected-store>] --json` now assemble a read-only review packet. It contains the existing resolution request, exact selected capture digests, assurance-case and artifact-manifest snapshot digests, and fresh runtime assessment. Request preparation reads current sources and uses the existing trusted provider callback. It cannot reuse a saved native verdict, write a resolution, sign, or award assurance.

The legacy request, response and recorded-resolution schemas remain unchanged. The new packet is `oh.war/runtime-resolution-request/v1-draft.1`. It reports `response_ingestion_supported=false` and `assurance_granted=false`. Native response ingestion is still unavailable: supplying `--response` with a runtime selection reports named UNKNOWN before reading that response or writing a resolution. This is preparation for human review, not completed human-resolution integration.

Thirty-one runtime tests passed, with two optional native tests ignored. Seven legacy JSON envelope tests passed. Actual Katana and BLUT two-UID fixture runs passed file/stdin review requests, native assessment and existing refusal controls. Fresh native matches left the other completion gates unmet. A newly compiled attempt refused an old selected capture while retaining its history. All-feature/all-target Clippy and formatting passed on Rust 1.97.1.

One intermediate regression run failed with `Unavailable("Text file busy (os error 26)")` in the synthetic protocol fixture. That fixture now publishes each response script as a fresh inode instead of rewriting its executable in place. The failure is retained; no production deadline, expected refusal or UNKNOWN classification was weakened. An earlier compile error and a test-launch environment argument error are retained too; those are not provider qualification results.

No production custody, human presence, spend accounting, confinement, independent verdict, participant session or acceptance was established. OW149 remains in progress. OW95 still has no participants.

## Required next work

1. Define and implement an exact signed native-resolution response and persisted evidence binding. The human must accept the contract, assurance snapshot, artifact manifest and selected runtime evidence shown in the request. A legacy response must not silently stand for that new binding.
2. Reassess current sources, latest attempts, native policy and original provider inputs during response ingestion and the real signing path. Refuse stale snapshots or revoked/changed policy; retain unavailable observations as UNKNOWN. Existing-record signature supplementation must not skip this check.
3. Verify signature matching against typed fields. The generic authority verifier currently searches response text for schema/digest strings; the separate authorization-record path already compares typed fields. Source inspection is established, but a resolution substitution exploit has not yet been reproduced. Do not assume this generic path proves every native binding.
4. Connect workbench draft/attempt identities to actual retained Dispatch contracts through an explicit versioned mapping. Do not infer identity from a title, or select an older successful attempt.
5. Complete independent and actual-provider qualification. External BLUT contributor agreement and real OW95 sessions remain human/external acts.
