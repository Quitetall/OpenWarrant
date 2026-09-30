---
schema: oh.war/atom/v1
warrant_uuid: 01a0b48a-8fe4-7f11-839d-3291d3dad0a2
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

1. Define a durable queue request bound to exact Warrant source, optional stage,
   execution configuration digest and existing execution inventory. Explicit
   authenticated enqueue authorizes one dispatch, not repeated retries.
2. Support bounded owner-configured availability observation. Distinguish available,
   unavailable and unknown. Show waiting-for-agent, blocked, unknown, cancelled,
   dispatching and dispatched separately. Observation cannot grant work permission.
3. Reuse the current admission evaluator before dispatch and the exclusive writer
   claim at launch. Source/configuration changes require a new explicit request.
   Reject verified-start and unknown-cost hard-cap violations through existing gates.
4. Persist dispatch consumption before side effects. On crash or ambiguous launch,
   preserve unknown state and inspect exact retained attempts; never replay blindly.
   Repeated polling, duplicate enqueue and restart cannot create duplicate writers.
5. Expose queue listing, enqueue and cancellation through authenticated local API
   and browser, with deterministic report pointers and clear pending reasons.
   Cancellation prevents future launch; it does not claim to stop running work.
6. Exercise actual HTTP, subprocess and Git boundaries: agent unavailable then
   available, unchanged ready dispatch, revoked/changed scope refusal, dependencies,
   repeated poll/restart, ambiguous launch, cancellation and hostile probe output.
   Retain browser observation and full gate evidence before implementation completion.

Autonomy: prompt-authorized unverified implementation. Preserve signed records,
resolved pins and actual history. No paid calls; no model reviews unless requested.
