# Verification candidate

OW-WAR-0107 is in progress. Verification does not award the assurance mark.
Prototype completion remains available without verification.

## Prepare a job

Start the service with its existing execution configuration plus
`--verifier-config PATH --verifier-issuer PATH`. These paths are operator inputs;
HTTP clients cannot select a command, actor, issuer key or configuration path.

The verifier configuration uses `oh.war/verifier-config/v1` with `performer` and
`verifier` objects, each containing `id` and `argv`, plus `cost_mode` (`free` or
`unknown`), `spend_limit_usd` (number or null), and `timeout_seconds` (1–7200).
Distinct actor names are necessary but do not prove independent execution.
Unknown cost cannot satisfy a mandatory spend cap.

The issuer file contains `schema: "oh.war/verifier-issuer/v1"`, `public_key` and
`principal`. The public key must be an SSH Ed25519 key for dispatch authentication.
Keep its private key outside agent access. A signature establishes origin and
binding; the harness must also enforce the protections it attests.

Authenticated routes:

- `POST /api/verification` with exactly `attempt_id` and `verification_id`
  (lowercase UUIDs) prepares a job from a completed current execution.
- `GET /api/verification` lists retained jobs.
- `GET /api/verification/<verification_id>` returns the exact request, admission
  basis and retained state.
- `POST /api/verification/<verification_id>/start` accepts exactly
  `payload_base64` and `signature_base64`. These encode the original signed
  `oh.war/harness-protection/v1` receipt and SSH signature. The receipt binds
  `basis_sha256`, the verification UUID as `nonce`, integer `issued_at_unix` and
  `expires_at_unix` (at most five minutes apart), `evidence_ref`, and `protections`.
  Required protection names are `separate-context`, `read-only-candidate`,
  `protected-checks`, and `protected-control-storage`; each must report `pass`.
  The signature namespace is `openwarrant-harness-protection`.

Preparation is idempotent for the same exact work. Reusing an identity for changed
work refuses. Preparation never launches a process. Current writer uncertainty,
missing prerequisites and stale source/policy refuse preparation. Required
protection evidence remains a separate dispatch condition.

## Current boundary

The internal process controller supports signed protection receipts, a separate
Git repository, protected-check observations, exact-result binding and durable
single-use claims. Start authenticates and consumes a claim before scheduling a
background process; poll the job route for results. Replay never launches again.
Job views separate the historical observation from `effective_verdict`. Missing
or corrupt retained protection receipts set `evidence_state` to `unavailable` and
the effective verdict to `unknown`, while preserving the original observation.
Restoring the exact receipt restores the evidence view without rewriting history.
Another running or uncertain verifier blocks new dispatch. Browser controls, aggregate loop budgets,
automatic repairs and rebuttal routing are not yet wired. A consumed claim without
a live process observation is UNKNOWN after interruption; it cannot relaunch.
Tests use synthetic processes and disposable machine keys, not independent human
review or deployment sandbox qualification.
