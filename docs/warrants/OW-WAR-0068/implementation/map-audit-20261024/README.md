# W68 map example observations

A synthetic decision Warrant retains an unanswered cache-retention choice as
A-001 with epistemic_status blocking_unknown. No choice was supplied and no
option was accepted. Status lists A-001 and keeps the Warrant draft/unresolved.

The first scaffold failed because its ADR was authored inside the Warrant.
That separate generator defect is recorded under W113. After correcting the
demo's source ownership to an external proposed ADR, `war check` passed.
Replacing blocking_unknown with invented_status still passed the same check.
This is a structural-validation gap: `Repository::load_rationale` rejects
unknown enum values, but check.rs does not call it. A valid blocking unknown
must remain a valid draft record; invalid rationale must not be silently ignored.

These observations do not satisfy the original map criterion that war check
reads the assumptions. The example remains incomplete pending the tool fix.
No real architecture, authority, signature or assurance record was changed.

## Corrected recorded example

The corrected tool generated a new decision Warrant and its proposed external
ADR. The fixture input still has no selected retention mode. The agent filled
the options and consequences without inventing a choice. `war check` now names
A-001=blocking_unknown as parsed record data; `war status` lists the same unknown
and keeps the Warrant draft. An invented status is refused as rationale.invalid.

[The proposed map](example/proposed-decision.md) and
[its assumption](example/rationale.toml) are actual recorded outputs. No mode
was selected, no architecture was accepted, and no signature or disposition was
created. This proves the bounded map-record transport, not a real project's
decision quality. Owner first-run review and qualification remain separate.
