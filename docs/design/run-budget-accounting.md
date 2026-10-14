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

The current Claude verifier adapter supplies neither an authoritative charge bound
nor run-wide enforcement. Its SDK cost estimates and post-response budget threshold
cannot qualify this integration. This module launches no backend, grants no
execution permission, and does not close the paid verification or operator gates.
