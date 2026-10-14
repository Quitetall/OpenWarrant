# Restore one run budget without forgetting held reservations

Advance the run-wide spend boundary needed by unattended preparation. The pure
SDK must reconstruct recorded reservations and charges after a host restart,
retain uncertain costs and poisoned admission, and refuse an invalid history or
an unexpected policy. It must not silently create a fresh spending allowance.

This ticket covers typed in-memory event replay and its refusal controls. It does
not define a wire format, authenticate history, implement durable storage, prove
provider charges, activate a policy or complete independent Warrant verification.
The host integration must still protect one complete run history and persist each
reservation before any paid execution. No paid backend is called for these tests.
