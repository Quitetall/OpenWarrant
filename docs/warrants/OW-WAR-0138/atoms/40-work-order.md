---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-605c-73a0-af03-c3ad0e0efc55
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

M1 — the contract:

1. `docs/AUTHENTICATION.md` (new):
   - what authenticates a human act (key, binding, presence);
   - what a session is: the unlocked OS session holding a loaded agent key,
     and nothing `war` issues itself;
   - the protected policy keys;
   - for each claim, the control, test or operator duty that carries it.

M2 — actor and presence:

2. `crates/openwarrant-cli/src/authority_check.rs`. With a store
   configured:
   - the principal-to-actor binding, kind and roles come from the store at
     its current head (`allows` with `--expected-head`);
   - a record verified only against `roles.toml` is
     `authority.legacy-fallback` (an error).

   Without a store, behavior is unchanged plus one
   `authority.unprotected` warning.
3. `crates/openwarrant-core/src/presence.rs` (new): parse an sshsig
   blob's key type and, for `sk-` keys, the flags and counter. The module
   is pure, and it is tested against signatures from a generated
   `sk`-shaped fixture.
4. `crates/openwarrant-cli/src/sign.rs`:
   - record `presence = "verified" | "unverified"` in the response;
   - refuse `sign.presence-required` under the policy before anything is
     renamed into place;
   - refuse `--as` naming an actor the verifying key is not bound to
     (`sign.actor-key-mismatch`).

M3 — protected policy:

5. `crates/openwarrant-core/src/authority_transition.rs`: a v2 revision
   carrying actor kind per principal and a `policy` table. v1 still
   decodes. A v1-to-v2 transition is a signed transition like any other.
6. `crates/openwarrant-core/src/config.rs`: the protected keys are
   resolved from the store when one is configured. A differing
   `openwarrant.toml` value is `policy.unprotected-divergence` (an error),
   and the store's value governs.

M4 — records and plants:

7. `docs/THREAT_MODEL.md`:
   - rows 1 and 6 gain their narrowed residuals: presence verifiable with
     `sk` keys; the register protected when a store governs;
   - the candidate table drops "legacy consumers remain legacy" for
     `war sign`.
8. `docs/cli/authority.md`: the cutover, one command at a time.
9. `conformance/plants.d/58-authn.sh` (new), the refusals in the assurance
   atom.

## Frozen Surfaces

- The sshsig namespaces (`oh.war/response`, `oh.war/dsse`).
- Every existing response and authorization, and their verification.
- 0096's v1 revision bytes and digests.
- `oh.war/report/v1`.

## Premade Instructions

- Fail closed. When the store is configured but unreadable, the result is
  `authority.verify-unavailable`, never a fall back to `roles.toml`.
- Never infer presence. Absent flags mean `unverified`.
- Test with disposable keys only. No owner key and no real store.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:

- U-001, before M2;
- any change to `allowed_signers` semantics;
- any new dependency (parsing sshsig needs none).

## Rollback

Unset the store in configuration: `war` returns to the legacy path with
the `authority.unprotected` warning. Responses signed with a `presence`
field remain readable by older builds, because it is an added optional
field. A v2 store revision cannot be rolled back to v1. Recovery is a
forward transition (0096).
