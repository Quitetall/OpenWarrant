# OW-WAR-0096 implementation

Source: `639b866313ccb98981a32c57a28e0e690b9275cf`. Candidate standard/SDK/reference workflow implemented and tested.
Production cutover remains open; no actual authority grant or SAS acceptance.

[Operator guide](../../../cli/authority.md) documents drafting, canonical bytes,
bootstrap, exact signing, one-command approval/activation, history export and
current-state role decisions. SDK is pure; signing adapter authenticates previous
keys. Atomic snapshot includes history and head. Linux runner excludes external
authority state and signing sockets from task execution.

Rust1.97.1: full gate14steps/308controls PASS; eight SDK tests and one end-to-end
CLI test with multiple explicit controls passed. Tests include hostile PATH,
forged signatures, modified proposals, replay, concurrent activation, migration
retention, combined approval and same-account mode refusal. Local Linux sandbox
probe passed. Independent reviews and actual LAMU review completed.

## Remaining before real use as security enforcement

Provision protected operator account/store, immutable trusted executable/runner
and human-controlled signing path. Independently review/bootstrap actual keys
and repository identity. Route privileged consumers through authenticated actor
and current-state checks; disable legacy fallback in that trusted workflow.
Observe real human approval/cancellation and separate-account denials.
Legacy roles.toml consumers remain legacy; this candidate does not silently
replace their authority, retroactively authenticate history or claim a mark.
These host-specific cutover steps are not established by disposable fixture keys.

## Activation receipt continuation

Owner-approved feedback adds atomic local activation observations: exact proposal,
previous/new heads, authenticated signers, process UID and wall-clock seconds.
Older snapshots remain readable with explicit missing-receipt count. Receipt
metadata does not establish human presence, trusted time or portable authenticity.

Focused CLI test passes, including each subject/signer mismatch, invalid sequence,
old snapshots and process UID. Clippy passes on Rust 1.97.1. Independent source
reviews: spec PASS and standards PASS after adding all refusal cases and updating
the threat model. Full repository gate and LAMU commit review pending at this
source checkpoint. Production deployment and legacy consumer cutover remain open.

Receipt source f5acf9c subsequently passed full gate: 14/14 steps, 308 plants,
Rust 1.97.1. LAMU commit review PASS WITH NITS; verified non-defects retained in
activation-receipts/evidence/review.md. Protected deployment and consumer cutover
remain unestablished; OW96 remains in progress.

## Configured authority consumer cutover — 2026-10-09

Requests and role consumers now use the configured store's current v2 actor,
kind, principal and known Warrant roles. A malformed repository roles file does
not replace that source. An unavailable or v1 store does not fall back to local
grants. The projected assignment records identify the store/head; effective
time remains unknown because this projection does not establish activation time.
Standalone attestation emission and verification use the same current public
keys. Legacy forensic checks remain diagnostic and grant no protected authority.

The public CLI regression failed before the change, offering the fixture's local
human instead of the configured store actor. After the change, 42 targeted
controls pass, including native grant removal, forged local roles, unavailable
stores, protected-key attestation and exact batch signing. Earlier failed
diagnostic controls were repaired in production code, without weakening plants.
See [retained controls](consumer-cutover-20261009/native-refusal-controls.log).
Tests use disposable fixture keys and an explicitly unprotected test store; no
real store was activated and no human acceptance is implied. Protected operator
deployment, actual human key custody/presence, cancellation and separate-account
isolation remain unestablished. This Warrant remains in progress.
