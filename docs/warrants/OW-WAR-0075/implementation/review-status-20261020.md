# Review observations and remaining qualification

A private local LAMU 1.0.0-rc.1 (`a36bd4f`) served the selected Qwen model
from a separate read-only checkout. All four answers completed and are
retained, with their exact input views, archives and provider logs.

The received dispositions were established for OBL-001, OBL-002 and OBL-004,
and not_established for OBL-003. None of this batch was ingested to clear an
obligation: source inspection found that OBL-001's evidence claimed FOOTER-08
coverage not shown in the supplied parser-only observations. The response
was preserved, not rewritten. The active verification files retain their
earlier bytes; qualification must be reevaluated against the changed subject.

Additional actual public SDK and core-probe executions now provide the
roundtrip/readback observations under `footer08-readback-20261020/`. The
fixture expectations were unchanged. A fresh subject-bound review is still
required after those additions and the refreshed structural gate. A secure
human acceptance of the exact result remains separate.

The model runtime used a private local-only registry, isolated network,
read-only subject and 94,208-token q4_0 context. Each exact input remained
within the unchanged 60,000 estimated-token policy; the 4,096-token output
ceiling was checked against LAMU's conservative prompt estimate before
dispatch. Normal GPU admission remained active. Foreign embedding and
desktop processes were preserved. Local compute cost was not metered.

The private model runtime was stopped, its snapshot retained, and its
temporary runtime directory removed after reference checks. Raw material:
`/mnt/4tb/artifacts/OpenWarrant/keep/ow75-fresh-review-20261020/`.
No signature, resolution or assurance mark was created.

## Follow-up review, 2026-10-21

A second isolated local batch received the same four dispositions. Its exact
inputs and raw answers remain under `verifications/` and the durable evidence
root. No answers were ingested. Source inspection found the OBL-002 answer
relied on different fixture names for its mutation claim; this does not by
itself rule out name-based behavior. The actual unseen-name/content regression
had passed but its execution result was not present in that model input.

D-067 now records a fresh actual run of `check_observations.py`, its output,
exit code, source identities and binary digest. This adds evidence, not an
independent verdict. The original observations and answers remain unchanged.
A fresh review must use the expanded subject. The later private runtime is
still owned by this review and will be stopped after the batch.
