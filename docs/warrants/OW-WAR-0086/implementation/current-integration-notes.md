# Current real provider integration

The scoped W86 SDK/provider implementation is complete and unverified. The real
LAMU provider at `2572894f28ce76c2c7e6c6c9ae6be51a7f400fa1` passes all six
integration profiles, the adapter controls and the separate SDK-only consumer.
The complete external provider suite passes all 28 tests. The record and legacy
preservation exchange passes; repeated compilation returns identical package
bytes. Changed package bytes and duplicate entries refuse. Inputs remain unchanged
and the provider checkout is clean.

## Correct resolved dependency evidence

A real Cargo path override exposed a provenance gap: the previous driver passed
while reporting the manifest's original SDK pin, even though Cargo resolved the
current local SDK. The retained red observation is a failed refusal control; its
`passed` field is not qualification under the declared profile.

The fixed driver checks Cargo's actual resolved dependency before provider work
and records the complete metadata in a successful receipt. The same real override
now refuses without publishing a result. The four fast protocol/output controls
pass. A clean original provider profile also passes. No published contract, SDK
pin or provider source was changed. The fix is committed as `ea0a65c6a`;
execution receipts bind the exact input bytes before later documentation and
evidence attachment.

[Current observations](current-integration/source-observation.json) identify the
code, producer, consumer and test inputs. Adjacent files retain the full real
profile receipt, before/after refusal records and logs. The complete red receipt
is retained as explicitly failed evidence. [Driver instructions](../../../../conformance/integration/adapter/README.md)
describe reproduction. LAMU remains an external AGPL provider; no dependency was
adopted into the OpenWarrant SDK.

## Qualification remains separate

The second consumer proves package integrity, not local semantic coverage. The
record exchange remains not ready and unqualified without caller-established
trust. Fixture records, archived signatures and provider statements do not supply
human acceptance or an independent verdict. No model or database service ran in
this compiler test. Native Phase 1 evidence is retained under W85; this driver
does not establish another platform's execution.

No Phase 2 exit, assurance mark, SAS adoption or release is declared. Independent
phase review and secure human acceptance remain open. Next: pass the protected
gate on the consolidated candidate, then continue the remaining release and
qualification scope.
