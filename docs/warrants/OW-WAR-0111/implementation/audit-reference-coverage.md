# Local journal and audit reference coverage

Local archive assembly now checks retained journal envelopes, explicit record
content digests, receipt digests and referenced receipt output paths. Gate receipt
seals recompute through the existing domain-separated validator. Historical receipt
paths resolve within their own retained commit, never to current replacement files.
Unknown event semantics and provider action envelopes remain unavailable pending
an appropriate resolver. Inspection recomputes any retained audit coverage claim.

Legacy events without a content-address field remain as recorded. Preservation
must not retrofit a later envelope requirement or invent a receipt claim. Explicit
references whose bytes cannot be found remain unavailable. Digest equality and
path presence do not authenticate actors, prove the original output, or grant
assurance. Authorization/resolution semantics remain owned by their records.

Validation: two audit unit tests, twelve preservation integration tests and
warnings-denied all-target CLI Clippy passed with Rust 1.97.1. Tests cover matching
and missing record bytes, changed bytes, legacy no-digest events, unsupported
event types, valid receipt seals, missing output paths and changed sealed fields.
The receipt fixture copies the retained OW-WAR-0030 receipt; unit output bytes are
synthetic presence fixtures, not a claim to reproduce its original run.

Real OW-WAR-0030 capture still reports audit coverage unavailable. Event
01a06512-d489-7df0-8e10-58db963f5bfb names verification digest
sha256:8eb3041554c8471f70c79096cb5b7520dcb2f6344a58dbcf1c3196b715ab236f.
Those exact bytes were not found in this retained local archive. Nine current or
historical journal snapshots carry that same reference. This is not proof that no
other store retains the bytes. No record was rewritten to hide the gap.
