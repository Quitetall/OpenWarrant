# One run budget — SDK accounting primitive

`openwarrant_core::spend::RunBudget` performs integer USD-micro accounting for one
trusted host-owned run. Its default policy ceiling is USD10. The host may supply
another human-approved ceiling, or explicitly no ceiling. The SDK does not change
an active repository policy or authorize spending.

Reserve a unique call/attempt ID against an independently established maximum
charge before requesting execution. An estimate or missing bound cannot satisfy
a hard ceiling. One ledger covers all callers, retries and recoveries; do not
construct a separate $10 ledger for each bundle. Duplicate IDs refuse. Settling
an actual authoritative charge releases unused reservation; missing settlement
keeps the whole reservation. Unknown spending is not zero: accounting reports a
known settled subtotal, held maximums and unsettled/unknown-bound counts separately.
A charge above its guaranteed bound is recorded and poisons further admission.
An arithmetic overflow also poisons admission and retains the unsettled reservation;
the settled subtotal is not an exact total in that breached state.

With an explicitly undefined ceiling, unknown bounds additionally require the
host's explicit allow_unknown policy. That mode reports unknown accounting rather
than claiming an enforced monetary ceiling. The policy has no mutation method.

Runnable public controls:

```sh
cargo test -p openwarrant-core --test spend
```

This is pure arithmetic, not billing, persistence, a sandbox or a provider guarantee.
The trusted host must own one durable ledger, serialize reservations across callers,
commit them before any paid request, protect policy/state from the performer, and
independently authenticate provider bounds and charges. Crashes cannot reconstruct
a fresh full budget: restore authenticated state/events, including unresolved
reservations. Reobserve effective policy before admission; a policy change
requires stopping new reservations while preserving existing accounting.
Repeated construction of an in-memory value does not enforce a cap.
Do not decode a model's claimed estimate as an AuthoritativeMaximum. Integer zero
is only a known zero when the host actually established that charge/bound.

## Recovery from host-owned history

`RunBudget::history()` returns the recorded policy and ordered typed events.
`RunBudget::restore(expected_policy, &history)` replays that complete history.
Unsettled reservations remain held, unknown calls stay unknown, and duplicate,
out-of-order or impossible events refuse recovery at the offending index. A
different expected policy refuses. An observed overcharge or arithmetic overflow
restores a poisoned ledger; it does not grant a fresh ceiling. The history retains
the actual reported charge even when its subtotal cannot fit in a `u64`.
Such an overflow cannot subsequently be overwritten by settling that call again.

These Rust types define no wire format, signature or persistence backend. A host
must authenticate the history's run identity, completeness, order and policy,
and persist each reservation before dispatch. Merely replaying a well-formed but
truncated or substituted history cannot detect its missing events. A restored
ledger alone therefore establishes no custody or crash-safe spending guarantee.
Refused reservations and missing settlements do not append budget events, since
they do not change accounting; a host may retain those attempts separately.

The current Claude verifier adapter supplies neither an authoritative charge bound
nor run-wide enforcement. Its SDK cost estimates and post-response budget threshold
cannot qualify this integration. This module launches no backend, grants no
execution permission, and does not close the paid verification or operator gates.
