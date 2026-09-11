# Basis

## Governing text

- `docs/sas/{{program_file}}_SAS.md` at the revision `war sas propose 0.1.0`
  records; §6.10 (levels), §98 Phase 1 (this phase), §106 rows RQ-001–003.
- OpenWarrant's own SAS, as installed with the `war` binary (`war --version`).

## Prerequisites

- `docs/authority/roles.toml` written by a human from `roles.toml.example`,
  naming at least one human with the authorizer and resolver roles.
- `docs/authority/allowed_signers` written by a human from its `.example`,
  with the key loaded under `ssh-add -c`.

## Unknowns

- Whether a second signer is wanted. `roles.toml.example` shows one; nothing
  here requires it.
