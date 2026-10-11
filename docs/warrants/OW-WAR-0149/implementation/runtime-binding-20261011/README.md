# Source-checked native binding export

Implementation source `473741cfe8ba0da834f4f458006e9b31c117d091`; shared OW-WAR-0149 (`01a0f502-4941-70a1-a446-e1eb77dff191`). Ordinary ticket t-c6cf2 is implemented, unverified; this does not resolve OW149.

`war runtime binding <alias> <retained-dispatch-id> --json` and the reference SDK `runtime_capture::binding` derive existing provider fields from the retained Dispatch and unique compile event. The compiler checks complete current source inventory, milestone membership and the existing canonical Dispatch digest. Superseded attempts refuse; missing source/event evidence is UNKNOWN. Rechecking detects observed changes without claiming a lock or authenticating local history. No receipt or journal entry is created. The output explicitly leaves execution eligibility unassessed and assurance false.

Both actual native providers use `warrant_ref`, `contract_digest`, `dispatch_digest`, `stage_id` and `attempt_id`. Native interfaces and digest domains are unchanged. Source inspection and an actual Katana producer refusal corrected an initial mistaken use of shortened Katana field names. The initial synthetic shape test did not establish compatibility. Native red/green logs retain that defect and its correction. Katana draft PR 14's documentation was corrected in commit `4aad94518a6ac8ceb697c74d07c5d6d679c21bb6`; its previous implementation source remains unchanged.

Final local checks on Rust 1.97.1:

- Runtime-capture module: 34 PASS, two external native tests ignored by default.
- Existing compiler basis controls: seven PASS.
- The actual Katana fixture roundtrip was separately selected and PASS using the exported binding, with frozen producer/verifier binaries from merged Katana `9f2f531e1af29cf07e510c11dd9c6dd9de02d6ef`.
- Workspace fmt and all-target/all-feature Clippy with warnings denied: PASS.

The Katana fixture uses actual local kernel/receipt verification with scripted model responses and an instrumented head. It proves the bounded source/export/native transport roundtrip, not real-model task execution, protected collector custody, sandbox enforcement, metered cost, participant acceptance, independent assurance or human acceptance. No paid model call occurred. Full release qualification remains open.

The initial refusal-control run changed an undeclared `message` field instead of the real `objective`; its failed expectation was corrected against the actual Dispatch type. The queued compiler check was stopped while it waited for the shared build cache; only `compiler-tests-final` is the completed compiler verdict. All raw observations remain alongside this note.

Projection check: the first check overlapped generation and reported corpus JSON/HTML drift. A later standalone status agreed with the generated semantic snapshot (the live compliance score is intentionally outside the committed projection). The complete serial check returned 2028 PASS, 629 WARN, zero ERROR/UNKNOWN. No authored source was narrowed or changed to remove that diagnostic.
