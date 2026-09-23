---
schema: oh.war/atom/v1
warrant_uuid: 01a0d049-9bcc-7a20-9d41-db7d11e0fb0d
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- §46 (independent verification) and §46.2's admissible inputs:
  `verify.rs`'s `admissible_for` judges every ingested verdict against its
  own independence flags. The repository declaration does not override it.
- RQ-053: performer reports cannot satisfy independent gates.
- §27.4: the role actually exercised is recorded, and no view claims
  four-eyes review when none occurred.
- `bundle.rs`: `war verify --bundle` and `--run`, and the
  `oh.war/verification-bundle/v1` it hands to `verifier_argv`.
- OW-ADR-0021: this Warrant declares `openwarrant.toml`. On authorization it
  governs the `[verify]` and `[independence]` tables it changes.

## Assumptions

- A-001: `claude -p --disallowedTools <every tool>` gives the model no way
  to read or write the filesystem or run a process. The plant records the
  argv a fake `claude` receives, so a change to the list is caught.
  Confidence: medium. This holds for the tools Claude Code ships; a future
  tool not on the list would be available, so the plant pins the list.
- A-002: the model the owner performs with (Opus 5.5 in this corpus) differs
  from the verifier's default (`claude-sonnet-5`). The wrapper does not
  assume it: `distinct_model_required` is true only when
  `CLAUDE_PERFORMER_MODEL` says so and differs. Confidence: high.

## Unknowns

- U-001 (non-blocking): whether §46.3's `controlled` minimum wants
  `distinct_model_required`. `war check` reports what the declaration meets
  either way. This Warrant does not decide it.

## Residual risks

- R-001: a verifier model can be wrong. A wrong `established` is the costly
  error, so the prompt makes `not_established` the answer whenever evidence
  is absent or only asserted. The obligations' own refusal plants remain
  the check on the work, not the verifier's opinion.
