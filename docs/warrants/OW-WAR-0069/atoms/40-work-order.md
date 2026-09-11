---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-78ce-74b8-b6e8-7b8ea92afa04
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `docs/warrants/<alias>/basis.lock.toml` — an optional, digested Compilation
   Basis input. TOML, sibling to `deliverables.toml`, because OW-ADR-0003 makes
   atoms restricted YAML that refuses the nested maps this data needs. Each entry
   names an upstream: repository, revision, toolchain, and any interface schema
   version, named profile, or source-pack digest agreed at the boundary.
2. `DigestDomain::DependencyLock` with URI `oh.war/dependency-lock/v1`, added to
   the `ALL` array in `crates/openwarrant-compiler/src/digest.rs` with its count
   assertion bumped, so a changed pin moves the contract digest and the Warrant
   reads as amended.
   The field carrying the lock digest into the contract preimage SHALL be
   `Option` with `skip_serializing_if = "Option::is_none"`. A Warrant with no
   `basis.lock.toml` must serialise byte-identically into FormatBasis and keep
   the digest it has today; a non-optional field would move every contract digest
   in the corpus at once and stale every authorization.
3. Refusals, each with a named rule and each planted:
   - a revision that is not exactly 40 lowercase hex characters;
   - a toolchain given as a channel (`stable`, `nightly`, `beta`) or a range;
   - a duplicate upstream id within one lock;
   - a lock file present but unparseable.
4. `conformance/plants.d/69-*.sh` exercising every refusal above and one
   accepting case, using the `lib.sh` helpers from the base branch.
5. A SAS revision adding the lock to the §14 input list and the domain to §65,
   plus an ADR, retained on the reviewing session's reader determination that
   §101.3 applies. §101.3 is applied at SAS acceptance, which is the owner's act;
   if the owner determines otherwise the ADR deliverable is dropped.

## Allowed surfaces

`crates/openwarrant-compiler/src/digest.rs`; new basis-lock parsing and its
module; `conformance/plants.d/`; `docs/sas/` revision; `docs/adr/` if U-001
requires it; and this Warrant's own records.

## Frozen surfaces

Every file pinned by a resolved Warrant, including `crates/openwarrant-cli/src/verify.rs`
(resolved OW-WAR-0046). Other Warrants' directories. The `main` branch. The
`/mnt/4tb/OpenWarrant` checkout, which another session holds. Any consuming
program's repository — this Warrant changes the tool, never a consumer.

## Autonomy limits

The performing agent may draft, build, run the battery, and report. It may not
authorize, verify its own work, or resolve. Verification is performed blind by a
separate session that did not draft these records.

## Procedure

1. Add the digest domain and bump the count assertion; confirm the existing
   conformance test names the new URI.
2. Define the lock schema and its parser with every refusal returning a named
   rule before any value is trusted.
3. Wire the lock into the Compilation Basis so its digest participates in the
   contract digest.
4. Plant each refusal and the accepting case.
5. Run the full battery; record raw output as evidence.

## Rollback

The domain variant and lock file are additive: a Warrant with no
`basis.lock.toml` computes exactly the digest it computes today. Reverting is
removing the variant, the parser, and the plants. No recorded contract digest
changes as a result of this Warrant alone. That is not a claim about intent but a
property OBL-004 tests corpus-wide: the lock field is optional and skipped when
absent, so every Warrant without a lock serialises exactly as it does today.
