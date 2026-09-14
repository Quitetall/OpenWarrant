# SDK ownership, compiler delegation and optional assurance

Status: proposed amendment for SAS RC.3, based on the owner's decisions in this
conversation. No allocated ADR identifier, signature or runtime cutover is claimed.

## Context

RC.2 assigned semantic compiler implementation to OpenWarrant and Stable publication
after its second phase. The owner now assigns semantic compilation to LAMU and
other providers, keeps a small OpenWarrant SDK, adds a four-phase path through a
real workflow webapp and hardening, and permits prompt-only unverified work while
reserving the common Verified mark for secure human acceptance plus evidence.

## Decision

Adopt the ownership table in SAS §2 and the SDK boundary in §6. OpenWarrant owns
standard data/meaning/conformance and convenient document/record helpers. LAMU
owns its semantic compiler; apps own workflow controls. Keep a typed reference
adapter instead of a duplicate small production compiler. Use the same SDK in
standalone tools and compiler consumers. Standard versions remain independent of
provider versions, storage backend and model runtime.

Retain context-profile semantics and tests while reassigning implementation. Carry
RC.2 wire formats and immutable examples explicitly; add separately versioned agent
acts and RC.3 assurance requirements rather than reinterpret old signed records.
Maintain first-party war skills as attributed adaptations of Matt Pocock's methods,
with OpenWarrant artifacts and SDK validation as their output boundary.

The consolidated context model uses named views over owned, versioned sources.
Queries select packets; they do not replace task identity or grant permission.
Cross-project work binds one shared contract revision. Evidence deletion preserves
signed history and exposes lost support. Work and agent stops retain distinct
meaning; user-selected work changes and automatic harness updates use separate
boundaries. See SAS §§19–20 and their normative companions.

## Alternatives and consequences

- Keeping two semantic compilers duplicates closure, selection and budget rules.
  A small reference adapter proves integration without taking on that maintenance.
- Requiring the full LAMU service for parsing adds unnecessary setup. The SDK remains
  standalone; compiler integrations are qualified separately.
- Requiring OpenWarrant approval or signatures on every prototype stalls work.
  Humans and agents can execute and report unverified work from a prompt. A
  professional workflow may enforce pre-work conditions; later final-result
  qualification may instead happen during release review with one exact batch
  acceptance. Neither path fabricates earlier approval or weakens the mark.
- Treating a secure signature as sufficient assurance ignores behavioral evidence.
  Qualification still requires independent checks and every baseline criterion.

Unsigned plans require new ownership/basis/phase assignments. Signed OW-WAR-0074
and all historical records remain unchanged. Current legacy SDK/CLI/skills do not
implement this amendment simply because its text exists. Migration and validation
records are in this source set; product acceptance is not inferred from a doc test.
