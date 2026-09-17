# OW-WAR-0097 implementation notes

Candidate: read-only `war doctor`, scoped to existing legacy record checks, stage
frontier and configuration diagnostics. No admission, authority or assurance grant.

Observed checks on Rust 1.97.1: two CLI tests pass (clean scaffold, generated drift,
malformed repository/authority TOML, directory authority path, unknown Warrant,
optional authority warning, exact remedies, no backend execution and byte-identical
repository snapshots). Clippy passes. Gate reaches 13/14; plant battery requires a
committed candidate before it can safely mutate/restore its fixtures.

Independent source reviews: spec PASS; standards PASS after correcting directory
handling, documentation overclaim and diagnostic paths. LAMU commit review and final
plant run remain pending at this source checkpoint.

Inherited limitation: some old check paths classify unavailable file reads as ERROR.
Doctor preserves those diagnostics and explicitly discloses incomplete classification.
Protected authority deployment and unified admission remain separate unfinished work.
