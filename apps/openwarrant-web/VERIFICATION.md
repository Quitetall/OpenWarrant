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

Preparation is idempotent for the same exact work. Reusing an identity for changed
work refuses. Preparation never launches a process. Current writer uncertainty,
missing prerequisites and stale source/policy refuse preparation. Required
protection evidence remains a separate dispatch condition.

## Current boundary

The internal process controller supports signed protection receipts, a separate
Git repository, protected-check observations, exact-result binding and durable
single-use claims. Public start routes, browser controls, aggregate loop budgets,
automatic repairs and rebuttal routing are not yet wired. A consumed claim without
a live process observation is UNKNOWN after interruption; it cannot relaunch.
Tests use synthetic processes and disposable machine keys, not independent human
review or deployment sandbox qualification.
