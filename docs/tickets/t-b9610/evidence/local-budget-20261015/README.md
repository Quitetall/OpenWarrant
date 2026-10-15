# Local verifier compiler-budget admission

Owner selected a local LAMU verifier. The private endpoint uses a private
network namespace, local-only model registry and read-only repository mount.
This does not establish model inference or Warrant qualification.

The actual OW-WAR-0003 publisher produced five obligation packets, each about
445,000 estimated tokens against the configured 48,000-token budget. They remain
byte-for-byte in the retained artifact store. They were not submitted to a model.
The first packet's refusal and unchanged SHA-256 are recorded in
`actual-oversize-refusal.json`.

The standalone local client previously accepted compiler-marked over-budget
packets. A new public-client control fails on the previous code in all seven
cases, retained in `budget-red.log`. The corrected client rejects oversized and
malformed budget metadata before health requests, inference or review-log
creation. All eight transport tests pass in `budget-green.log`, including the
existing successful transport, binding and refusal cases. Tests use synthetic
HTTP servers, never a model.

This enforces the compiler's declared budget metadata; it does not implement a
new tokenizer or prove that arbitrary caller-supplied estimates are honest.
Snapshot closure and packet-size optimization remain open. No verdict,
authority act, resolution or assurance mark was recorded.
