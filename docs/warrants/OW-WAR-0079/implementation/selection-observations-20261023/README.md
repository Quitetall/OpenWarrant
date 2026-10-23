# Current W79 selection observations — 2026-10-23

Performer observations, not independent verdicts or secure human acceptance.
The original implementation notes and evidence remain unchanged.

The real provider at `2572894f28ce76c2c7e6c6c9ae6be51a7f400fa1` passed all
six native selection tests at its published SDK pin. This covers cross-source
routing, exact transitive background, duplicate reasons, cycles, access denial,
missing/conflicting members, optional-to-required behavior and resource limits.
`native-selection.json` retains commands, full output, source hashes and profile.

The new public observer passed seven fresh input cases through the real compiler
example and a separate SDK CLI: required UNKNOWN inclusion, optional UNKNOWN
omission, known FALSE despite a missing stage, changed source text, bounded cycles,
required missing content and selected conflicts. `wire.jsonl` retains 45 actual
calls: 33 SDK operations and 12 compiler calls. The source documents are retained
under `inputs/`; summary byte ranges locate each complete case in the wire.
Missing targets and conflicts returned the named diagnostics with no partial
package. Five successful cases repeated byte-for-byte. Actual entry bytes and
bound UTF-8 source spans match the SDK's unit lookup, including Unicode text.

Case labels are not sent to the producer. It receives random directory names and
fresh document content. The source mutation changes the source/package digests;
the prior source remains unchanged. Required background stays verbatim, while an
unselected background unit stays out of ENTRY and selected references. Original
source blobs remain in the package for integrity; this is not unit-level secrecy.

The compiler example captures one source per call. Cross-source/access/limits
coverage comes from the separate native tests, not that example. Package integrity
checks explicitly leave semantic assurance unestablished, and readiness remains
blocked. The compiler build name is a claim bound here to an observed binary/hash,
not a signed provider attestation. Linux execution only. No service deployment,
model quality, Phase 2 exit or human qualification follows these observations.
