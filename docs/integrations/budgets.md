# Shared context budget and cache contract

Revision 1 implementation draft, shared OW-WAR-0082. OpenWarrant owns this contract;
LAMU implements accounting and reuse over the projection provider.

`account_entry` counts final UTF-8 ENTRY bytes, including framing, with ceiling
bytes/4 token estimate. The stated budget is inclusive. Excess refuses; required
content is never truncated. This estimate is not a model tokenizer or spend meter.

`basis_key` hashes the entire explicit F5 basis in the SDK's domain-separated form.
Source revisions, policy facts, records, role, selection inputs, access facts,
compiler identity, budget and options affect identity. The cache also keys every
operational limit and checks captured descriptors against the basis before reuse.
No live repository, model or policy lookup occurs implicitly.

`ProjectionCache` retains at most one immutable Arc result. Its positive capacity
bounds serialized retained payload plus ENTRY and key, not allocator overhead.
All separately retained fields, including basis_digest, are charged. An oversized
valid result is returned uncached; failed or oversized replacement preserves the
previous entry and all caller-held views. Callers control the lifetime of their
own Arc references. Any changed key recomputes; this deliberately favors correctness
and a small implementation over partial graph-cache invalidation.

T38-T41 exercise inclusive boundaries, framing, key changes, and reactivation of a
previously omitted rule. These checks confer no readiness, signature or assurance.
Profile: `lamu.openwarrant.budgets/1`; exact SDK/source profile inherited explicitly.
