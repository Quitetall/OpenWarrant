# Packet 3 — Knowledge Fabric

**To:** the Knowledge Fabric team
**From:** OpenWarrant (`Quitetall/OpenWarrant`), 2026-09-03
**Blocks:** OW-WAR-0028 (typed actions), OW-WAR-0029 (registration and identity),
OW-WAR-0044 (the Phase 4 exit)
**Governing text:** SAS §67 (controlled actions), §12.4–§12.7 (identity and
federation), §83 (KF integration), §68 (portable export)

## 1. What we are asking for

Two things, and the second is the one that has never been possible:

1. A **reachable KF instance exposing the §67 typed actions**, so state changes
   go through actions rather than direct field edits.
2. An **enterprise identifier, allocated by KF**, for at least one Warrant.

Every Warrant in this repository carries `enterprise_id = ""`. Our manifest
validator *refuses* any locally-set value as fabricated, per §12.4. That refusal
is correct and it is also the entire blocker: there has never been a legitimate
way to obtain an identifier, so the refusal has never met a real allocator.

## 2. The governing text

**§12.4 Enterprise identifier** — "Knowledge Fabric eventually allocates the
official identifier under the OpenHuman Identifier Registry. The enterprise
identifier SHALL NOT be fabricated from a filename or local sequence. A WAR may
remain valid as a local draft before allocation. It may not claim globally
authorized or effective state until registered through Knowledge Fabric."

**§12.5 Federation record** — KF SHALL map: UUID, repository/subsystem, local
alias, enterprise identifier, **Source Holder**, classification, current contract
revision, lifecycle projection, relations.

**§12.6 Offline creation** — offline creation SHALL use UUID identity. The local
flow must keep working with KF unreachable.

**§67** — the initial action vocabulary is thirty-two typed actions across four
groups (contract, execution, evidence, terminal/administrative), listed in full
in the SAS. Requirement RQ-076 is that state changes go through these rather
than direct status edits.

**§67.1** — every controlled action carries: `action_type`, `actor_id`,
`acting_role_id`, `organization_id`, `target_ids`, `payload`, `reason`,
`idempotency_key`, `request_id`, `expected_version`, `effective_at`,
`max_classification`.

**§67.2** KF assigns `recorded_at`. **§67.3** a mutation cites the version it
read; drift fails rather than overwrites. **§67.4** equivalent retries replay the
first committed result; conflicting reuse of a key is rejected.

**§83.4 Generated TypeScript types** — "JSON Schema and OpenAPI generation SHALL
produce KF-facing types. Generated types are projections of the Rust-owned
protocol."

## 3. What we have already built

> **Answered 2026-09-03.** KF replied to this packet. Items 2, 5 and 6 are
> resolved and struck through below; items 1, 3 and 4 stand, with their real
> constraints now recorded. Two defects the exchange found are ours: our client
> omits `expectedVersion`, and it sends generic action names instead of the
> thirty-two §67 ones. Both are fixes to `war kf`, not asks of KF.

- `war kf health` (read-only) and `war kf act` (writes, and refuses to write
  without `--confirm-write`), over HTTP or HTTPS, defaulting to
  `http://127.0.0.1:4000`.
- The client posts to `POST /actions/:actionType` with the body
  `{targetIds, payload, reason, idempotencyKey}` and the actor identity
  (`actor`, `acting_role`, `organization`) as **headers**, so a receipt cannot
  be re-attributed by editing a payload. That shape was read off your running
  service's route contract rather than invented here — **please confirm it is
  still current**, because it is narrower than §67.1's twelve-field envelope
  (see item 5 below).
- Caller-supplied idempotency keys, with a client-side minimum length of 8 so a
  too-short key fails locally rather than as an unhelpful 400. Deliberately
  caller-supplied: a key the server generates is not idempotency.
- Nothing on our side stamps a timestamp. §67.2 says KF assigns `recorded_at`,
  and a client that helpfully filled it in would be manufacturing the exact
  field the obligation exists to check.
- The §68 one-file canonical export, so a Warrant can be handed over whole.
- The refusal in §1 above: a locally-invented `enterprise_id` is rejected, and a
  plant proves it.

## 4. What we need returned — artifacts, not assertions

1. **An endpoint we can reach**, with credentials or a local run recipe, and the
   base URL if it is not `127.0.0.1:4000`. **Status 2026-09-03: not yet, stated
   without overstatement.** A Debian 13 host that is deliberately not the build
   workstation is up, with the six host requirements probed, bubblewrap
   qualification passing, and PostgreSQL 18.6 running `jit=off`. It has no `kf`
   database, no promoted release, no TLS and no Keycloak realm, so there is no
   URL. Their commissioning verifier reports `unverifiable` as a *failure*
   precisely so a half-built host cannot be cited as one — which is the same rule
   as ours, arrived at independently.
2. ~~**The action types, as KF actually names them.**~~ **ANSWERED
   2026-09-03: verbatim, no mapping.** `@kf/warrants` owns all thirty-two §67
   names exactly as the SAS spells them, all thirty-two perform typed writes, and
   an unrecognised action type is refused as `unknown_action`. The generic forms
   our client sends (`document.create` and friends) are wrong and are being
   removed.
3. **One real allocation.** Corrected by KF 2026-09-03: **registration and
   allocation are two acts, not one.** A Warrant can be registered and hold no
   identifier — which is exactly the state §12.4 calls a valid local draft, so
   the correction sharpens our own requirement rather than bending it. The ask is
   therefore: register, then allocate.

   KF confirms it *can* allocate — `core.allocate_enterprise_id` locks the
   sequence, skips occupied numbers, computes a Damm check digit, writes a ledger
   row, and returns the identifier read back from durable state. Two conditions
   neither side can waive: allocation is a **human-only act** (a service actor is
   refused every institutional act by name), and `allocate_enterprise_identifier`
   declares `requires: act`, so a live act grant must reach the target or the
   organization. `WAR` was allocated in the OpenHuman registry on 2026-09-02 and
   the pack carrying it signed 2026-09-03, so `OH-WAR-NNNNNN-C` is issuable;
   before that date it would have been refused by name.
4. **A federation record** with the nine §12.5 fields, `Source Holder` among
   them, set to `git`. KF's position, which we accept: this should be *produced
   by the registration act*, not hand-written, or it is an assertion rather than
   a record. Deferred with item 1.
5. ~~**The version field for §67.3.**~~ **WITHDRAWN — the premise was false.**
   `expectedVersion` has been on the wire since the route existed
   (`apps/api/src/routes/actions/write-route.ts`). We were misled by a KF
   docstring that listed four body fields and omitted it; KF has corrected that
   comment. The defect is ours to fix: our client does not send the field.

   The real contract:

   ```text
   body:    { targetIds, payload, reason, idempotencyKey, expectedVersion?, effectiveAt? }
   headers: x-kf-actor, x-kf-acting-role, x-kf-organization
   ```

   Drift is refused, never overwritten: `version_conflict`, HTTP 409. Siblings
   are `illegal_transition` and `idempotency_conflict` (409) and
   `actor_not_authorized`, `role_not_held`, `classification_not_granted`,
   `act_not_granted`, `separation_of_duty` (403). Of §67.1's twelve fields KF
   takes ten and **assigns** two: `request_id` and `recorded_at` are
   server-derived and not accepted from the caller, which is a stronger reading of
   §67.2 than accepting a field and ignoring it. `effectiveAt` must be a
   canonical four-digit-year RFC 3339 millisecond instant; a non-canonical one is
   refused rather than normalised.
6. ~~**Confirmation that registration does not transfer source authority.**~~
   **CONFIRMED 2026-09-03, and built that way.** `work.warrant` uses our UUIDv7
   *as* the object id, so KF does not mint a rival identity for the same thing.
   `work.warrant_contract_revision` is append-only and stores the digest, basis
   and canonical IR **as OpenWarrant computed them** — KF does not recompute a
   digest and assert its own answer, because that would make it a second
   authority for a fact Git owns. Recorded on their side as ADR 0019 and
   KF-SAS-RQ-132/133. The round trip still has to be *run*; what is settled is
   that the design does not take source authority.

## 5. Sequencing note — the TypeScript types

§83.4 requires KF to consume **generated** types rather than reimplement WAR
semantics. Those types come from our schema pack (OW-WAR-0032), which is our work
and is not yet built. So:

- **Now:** you can build against the §67 envelope and the SAS sections above.
- **When 0032 lands:** we send generated JSON Schema and TypeScript, and you
  swap to them. Please do not hand-write WAR semantics in TypeScript in the
  meantime — §77.3 and §83.4 both keep TS as an integration layer, and
  unwinding a hand-written implementation later is worse than waiting.

## 6. What we will refuse

- **An identifier we made up.** §12.4 and a passing plant (§91.3 test 20). If
  you send us a format and ask us to generate our own, we cannot accept it —
  allocation is the thing we need from you.
- **A direct status edit.** RQ-076 and OW-WAR-0028 OBL-002; ideally that path
  does not even compile.
- **A stale write that wins.** §67.3: drift must fail, not overwrite. Once item
  5 above is answered we will race a stale version and require the refusal.
- **A duplicated action applied twice.** §67.4.
- **Registration that also takes Source Holder.** §91.3 test 21.
- **A KF that has to be up for us to work.** §12.6 and RQ-070: the full local
  flow must still run with KF unreachable. KF rightly points out that this is not
  something they can hand us — it is a property of *our* client, verified by
  taking them away — so it is listed here as a refusal, never as a deliverable.

## 7. How to send it back

An endpoint plus a short note answering items 2–6 is enough to start. The
allocation itself and the federation record should come back as records we can
commit under `docs/warrants/OW-WAR-0029/evidence/`, with digests.
