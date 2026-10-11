# OW119 identity-control observations

Performer evidence only. No independent verdict, signature or resolution is recorded.

Hosted CI [38148947355](https://github.com/Quitetall/OpenWarrant/actions/runs/38148947355), source `47cc456348ae0ebe4291177647b6f563fb191c21`, passed the 32 named CLI identity controls. The retained section records clean inputs and refusals for duplicate UUIDs, mismatched atoms, changed committed identities, alias references and invalid UUID versions. Missing Git reports UNKNOWN; existing ADR alias and v4 findings remain warnings. It observed six alias warnings and thirteen v4 ADR warnings, once each, without changing ADR files.

Focused Cargo tests at `8f777d38d84e7ef96dab6b2c54352453f02559ff` passed nine identity tests and the BLUT stage-identity test. Rust 1.97.1; locked, offline, package `openwarrant-cli`, library filters `identity_check::tests` and `blut::tests::the_stage_identity_is_the_uuid_and_never_the_alias`. This is a focused run, not a new full gate. The manifest proves the relevant current source and plant bytes equal the hosted source.

OBL-005 names the original 23 ADR atoms. Their exact identities at initial Warrant source `5408b60f56c4ccecb5c74407ec8d5e33a9466ef9` remain unchanged. The hosted corpus has 36 ADR atoms; this larger population is stated explicitly, rather than silently changing the obligation. The machine observation retains all 23 original paths and identities.

Evidence supports the five bounded obligations. An independent reviewer must assess the evidence and delivery records before formal assurance or human resolution.
