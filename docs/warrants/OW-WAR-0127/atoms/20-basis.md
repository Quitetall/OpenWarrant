---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f55-7171-906f-45d79a408ce3
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §11 (component ownership): native systems own domain-native facts.
- SAS §13: every atom or bound record SHALL declare its Source Holder; the
  kinds are `git`, `fabric_native`, `external`, `generated_projection`,
  `runtime_receipt`. §13.2: a bound fact held elsewhere is not edited
  inside the Warrant.
- SAS §37.1 (`target_ref` as a URI), §37.2 (provenance, including Source
  Holder), §37.5 (ownership of a delivered *path*).
- SAS §56.1 requirements 2 and 3 (deliverables exist; digests verify).
- RQ-065 (this Warrant); RQ-011 (bound atoms are edited only through their
  owning authority); RQ-036 and RQ-037 (delivered-path ownership).
- Law 15: unknown is not failure and not pass.
- OW-ADR-0021 (ownership), OW-ADR-0012 (the correction act), OW-ADR-0008
  (§91.2 test 14: atoms declare no Source Holder yet; not changed here).
- `crates/openwarrant-core/src/seam.rs`: "the other system owns its own
  facts, and OpenWarrant records references to them". This Warrant applies
  the same rule to deliverables.
- The path reads this Warrant changes: `resolve.rs` (existence, digests),
  `check.rs` (drift), `pins.rs` (refresh), `correct.rs`, `bundle.rs`,
  `status.rs`. `deliverable.rs` holds the model and its validation.
- Knowledge Fabric, read at `/mnt/4tb/openhuman-knowledge-fabric` commit
  `3d3c871e`: `content.external_locator` records `system`, `external_id`,
  `uri`, and `authority` in (`authoritative`, `evidence`, `mirror`,
  `lookup`) — "a mirror must never be mistaken for the authoritative copy"
  (`database/migrations/20260811000900_content.sql`). A candidate form, not
  a requirement on OpenWarrant.
- OW-WAR-0114 named this gap (`roadmap://OW-PHASE-4/native-authority`).

## Assumptions

- A-001: no deliverable in this corpus uses a non-path `target_ref`. A
  search of every `deliverables.toml` found none at drafting time, so a
  new form changes no existing record. Confidence: high; OBL-006 checks it.
- A-002: a native reference belongs in the authorizer's signed set. The
  authorizer should see it, even though §37.5 ownership does not apply to
  it. Confidence: medium; the ADR records it.

## Unknowns

- U-001 (blocking): the form of a native reference. The SAS shows only
  `git://...`. Nothing can be parsed until this is fixed.
  *Resolution requirement:* the owner answers Q-001.
- U-002 (non-blocking for this Warrant): how a native artifact's presence
  and digest can ever be established here, so a Warrant with a required
  native deliverable can resolve. Until it is answered, such a Warrant
  reports §56.1 requirements 2 and 3 unmet, with the reason. This Warrant
  delivers that honest report and nothing more.
- U-003 (non-blocking): whether `openwarrant.toml` must list the native
  systems a repository may name. Default here: no list; the ADR records
  the question.

## Questions for the owner

- Q-001 (U-001): what does a native `target_ref` look like?
  (a) `external://<system>/<external_id>`, mirroring KF's
  `content.external_locator` (`system`, `external_id`);
  (b) one scheme per kind (`dataset://…`, `cad://…`), with the holder named
  in provenance;
  (c) only `kf://` references to an artifact version KF has registered, so
  a native artifact must pass through KF first.
- Q-002 (U-002), for the record only: how is a native artifact's digest
  established? (a) a verifier-run gate whose receipt records what it
  fetched (§44.6); (b) the holder's own receipt, attached as evidence;
  (c) not yet: such Warrants stay unresolvable, as this Warrant leaves them.

## Residual risks

- R-001: §56.1's requirements are booleans, so "held elsewhere, not
  observable here" is reported as unmet with a reason and an UNKNOWN
  diagnostic, not as a third value. A reader of the boolean alone sees
  "unmet". Changing the model is out of scope.
- R-002: this Warrant declares seven existing files that other Warrants
  govern. On authorization it becomes their owner under OW-ADR-0021.
- R-003: a performer could declare a native reference to escape pinning of
  something that is really a repository file. The reference must carry a
  non-`git` holder; the ADR states that a repository file declared as
  native is a false record, and the refusal is limited to what the tool
  can see.
