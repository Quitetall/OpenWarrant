# Bind authorized Dispatch to exact signed current contract and immutable context holders

## Reproduced defect — 2026-10-09

Changing IX-WAR-0003's milestones fixture to add `executor_ref: agent://fixture`
changes the compiled contract digest. `war dispatch IX-WAR-0003 STAGE-001`
without `--prototype` still exits zero and writes a packet using the older
signed authorization. The same command warns about a floating context holder
but does not refuse emission. The unchanged fixture authorization was used;
no signature, provider execution or assurance was produced.

[Observed digests and binary identity](../evidence/observation.json),
[actual report](../evidence/report.json), [packet](../evidence/dispatch.json),
and [context](../evidence/context.json) preserve the observation.

## Next implementation

Compare the signed authorization subject to the current compiled contract before
writing any packet or journal event. A stale subject must refuse authorized
emission. Explicit prototype mode must downgrade to prototype authority rather
than borrow an old authorization. Missing required immutable source holders must
remain visible and prevent authorized emission; prototype work remains possible.
Check that a nested non-Git project cannot borrow its containing repository's HEAD.

Add refusal controls proving no packet, context or dispatch journal write occurs.
Update deliberately changed fixture tests to request prototype mode explicitly;
do not rewrite signatures or remove refusal plants to make tests pass.
The implementation item is still open. No code fix or full-gate result is claimed.
