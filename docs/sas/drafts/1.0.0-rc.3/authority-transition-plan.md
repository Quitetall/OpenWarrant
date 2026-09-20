# Authority transitions: implementation scope

Status: owner-approved direction; candidate design, not SAS adoption or implemented
security. No authority records, keys, policies or signatures change through this
plan. Requirements live in the SDK contract; this document explains build order.

## Problem and desired interaction

The legacy loader reads roles.toml from the working tree. SSH verification reads
repository-local allowed_signers. Structural validation does not authenticate
who granted a role. Preventing an agent command from editing these files while
allowing unrestricted shell edits adds friction without isolating authority.

An agent should prepare a validated proposal. A human should review an exact
permission diff and approve once through a secure signing surface. Only the
trusted activation service may make it effective. Unsigned edits grant nothing.
Proposed command names such as `war authority propose` and `war authority approve`
are illustrative, not implemented commands or frozen API decisions.

## Build order and bounded acceptance

1. **Standard and SDK:** define versioned proposal, authority revision and signed
   transition records with real canonicalization and a dedicated signing domain.
   Define exact repository identity, parent/new digests, signer identity, permitted
   operation, key rotation and recovery semantics. Pure functions validate supplied
   records against explicit trusted facts; they never infer trust from a path.
   Positive: an authorized administrator's valid transition is eligible.
   Refuse: modified bytes, wrong repository/parent, unknown format, forged role,
   changed actor label, self-authorized new key and insufficient signers.
2. **Signature adapter and CLI:** prepare proposals and deterministic permission
   diffs; verify signatures against the previous authority set. Keep private keys
   outside agent access. Human control requires a signing mechanism the agent
   cannot silently operate, not merely a field saying actor_kind=human.
   Positive: exact reviewed proposal obtains a valid signature.
   Refuse: wrong key, altered subject, cancelled signing, unsigned replacement key
   list. Cancellation leaves effective authority unchanged.
3. **Protected activation:** use a separately protected service/account or harness
   boundary for trusted state, bootstrap root and verifier. Compare-and-swap the
   expected parent, persist transition and new head together, recover on crash.
   Positive: one valid transition activates exactly once.
   Refuse: concurrent stale proposal, rollback to old signed state, replay,
   direct writes by the execution agent and replacement of verifier/trust anchor.
   Root key alone is insufficient against replay without protected current state.
4. **Migration and UX:** preserve original role/key records and identify them as
   legacy, not retrospectively authenticated. An authorized human reviews and
   establishes the initial trusted baseline through the protected bootstrap path.
   Do not automatically trust keys taken from the same proposed repository data.
   Demonstrate prompt-only prototype completion without authority enrollment, then
   one human approval gesture for an actual grant. Document recovery and revocation
   behavior and their effects on current eligibility versus retained history.

## Integration boundaries

OpenWarrant owns record meaning, canonical bytes, transition checks and diagnostics.
The signing adapter establishes authentic signatures and human-control evidence.
The workflow/harness owns protected bootstrap, current-state storage, identity,
activation, access control and enforcement before privileged actions. Git records
history; protected review can govern merges but cannot prevent local file edits.
A verifier running under an unrestricted agent account cannot establish isolation
from that account. Hardware-backed signing still needs protected trust/activation.

Before implementation freezes a wire format, supply an ADR, concrete codecs and
positive/refusal fixtures. Select one reference isolation/signing backend and
state its threat boundary. Broader platform support may follow. Do not claim
production enforcement from SDK tests or a confirmation prompt alone.

No existing Warrant is closed by this plan. Implementation must be assigned to
new or revised unsigned scope without rewriting signed legacy contracts.
