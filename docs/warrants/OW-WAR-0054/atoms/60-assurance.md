---
schema: oh.war/atom/v1
warrant_uuid: 01a03fb3-3dba-7205-8fdf-bbd7354e0aa3
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the invariant gate has been made to fail in both directions
- **scope:** `documentSet(P,T) == visibleSet(P,T)` for a test subject, both
  directions reported separately.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two plants and their refusals. One inserts a record the subject
  cannot see and requires the gate to refuse for over-disclosure; one omits a
  record the subject can see and requires refusal for under-disclosure. Each
  refusal names the direction and the offending record. A run in which the gate
  passed is not evidence for this obligation — only the two refusals are, plus a
  positive control so that a gate refusing everything cannot discharge it.

### OBL-002 — nothing publishes without a matching visible-set digest
- **scope:** every compiled document and every rendering of it.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a manifest carrying `compiled_at` and the digest of the
  enumeration it matched, plus a refusal when a document is served whose digest
  no longer matches a re-enumeration. The refusal is the evidence; a manifest
  that is merely present proves only that a field was written.

### OBL-003 — withheld and withdrawn items are enumerated with reasons
- **scope:** anything the subject can see that is not in the document, and
  anything that has left the set since the previous compilation.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a removed record appears as withdrawn with time and reason, and a
  record under `core.retention_hold` appears as withheld with its reason. The
  discriminating test is against SILENT ABSENCE, not against absence — the two
  are indistinguishable to a reader, which is the whole defect.

### OBL-004 — every disclosure and every derived subset mints a warrant and receipt
- **scope:** the first real disclosure, and one derived subset of it.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** Knowledge Fabric, by exact release identifier.
- **evidence:** a runtime warrant naming subject, scope, recipient and authorizer,
  and a delivery receipt whose timestamp was assigned BY THE SERVER. A
  client-supplied delivery time is refused, and the refusal is recorded.

### OBL-005 — the first transport is revocable and opens no new outbound credential
- **scope:** the signed expiring link, before any other transport exists.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a link that expires, a link that is revoked before fetch and then
  refuses, and an access log entry per fetch. Plus the negative: no SMTP, OAuth
  or third-party credential is introduced by the work discharging this Warrant.

### OBL-006 — the invariant's boundary is resolved before the compiler exists
- **scope:** every table reachable in `visibleSet(P)`, classified as materialized
  into KF-governed rows or resolved live from an external system.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a classification covering the whole visible set with no table
  unaccounted for, and for every live-resolved table a recorded decision: either
  it is materialized before it may enter a document, or the document annotates
  that section as outside the invariant. A classification with an unclassified
  remainder is refused; "the rest are probably materialized" is the assumption
  this obligation exists to prevent.
- **note:** this obligation exists because the residual risk below is real and
  had no gate. Without it an implementer reaches the compiler, finds a large
  share of the visible set resolved live, and decides it under schedule pressure
  — which is how the completeness claim ends up quietly not covering part of
  itself.

## Evidence

### EV-001 — the primitives were read from the tree, not remembered
- **class:** evidence
- **kind:** static_analysis
- **origin:** performer
- **admissibility:** performer_report_only
- **digest:** sha256:pending-receipt-binding
- **method:** grep over `database/migrations/` and `packages/` in
  `/mnt/4tb/openhuman-knowledge-fabric` for the access, tombstone, outbox and
  federation primitives, and for any outbound transport
- **occurred at:** 2026-08-26

### EV-002 — the host was read while running
- **class:** evidence
- **kind:** external_tool_verdict
- **origin:** knowledge_fabric
- **admissibility:** authoritative_external
- **digest:** sha256:pending-receipt-binding
- **method:** `systemctl list-units 'kf-*'`, `curl` against :4000 and :443, and
  `psql -c "select datname from pg_database"` on the `kf-dogfood` host
- **occurred at:** 2026-08-26

### OBS-001 — the invariant is checkable because the access context is a function
- **class:** observation
- **evidence:** EV-001
- **method:** `core.set_access_context(uuid, text)` exists and row-level security
  is enforced against it on 76 tables. `visibleSet(P)` is therefore a query. Had
  it not been, the claim in `10-intent.md` could only have been asserted, and
  this Warrant would not be worth writing.
- **admissibility:** performer_report_only

### OBS-002 — the delivery socket exists and nothing drains it
- **class:** observation
- **evidence:** EV-001
- **method:** `core.outbox` carries `action_id`, `topic`, `payload` and
  `delivered_at` with a partial index on undelivered rows, and is written inside
  the action transaction. No consumer exists. Separately, zero matches for
  nodemailer, SMTP, SendGrid, Postmark or any mailer across all TypeScript. The
  transactional half was designed and the transport half was never written.
- **admissibility:** performer_report_only

### OBS-003 — no instance is running and no production database exists
- **class:** observation
- **evidence:** EV-002
- **method:** zero `kf-*` units installed, :4000 refuses connections, nginx
  answers 502 with nothing behind it, and `:5432` holds `keycloak` and the
  templates only. The `kf` database does not exist and
  `/etc/kf/migrator/database-url` is 0 bytes. The verified release is unpacked at
  `/opt/kf` and has never been installed.
- **admissibility:** authoritative_external

### INF-001 — M1 through M4 are not blocked by the absent host
- **class:** inference
- **kind:** deductive
- **premises:** OBS-001, OBS-003
- **claim:** invariant-provable-without-commissioning
- **reasoning:** the invariant is a comparison between two sets derived from a
  database, and `tests/database/` already starts a harness per suite. Nothing in
  STAGE-001 through STAGE-003 requires a served endpoint, a systemd unit or a
  production database. ADR 0009 was deferred on the objection that it was a
  feature on a system nobody runs; that objection reaches STAGE-004 and does not
  reach the three stages before it, so the Warrant is sequenced rather than
  blocked.
- **admissibility:** controlled_measurement

### JDG-001 — nothing here is discharged, and the Warrant claims nothing
- **class:** judgment
- **kind:** scope_holding
- **actor:** QuiteTall
- **acting role:** author
- **meaning:** all seven milestones are open and no obligation has evidence
  beyond the basis measurements above. This Warrant authorizes work; it does not
  report any. The obligations are written so that a passing test cannot discharge
  them on its own — OBL-001 needs two refusals, OBL-002 needs a refusal, OBL-003
  discriminates against silent absence, and OBL-005 includes a negative. That is
  deliberate, because the failure mode of this capability is invisible when it
  succeeds wrongly.
- **basis:** OBS-001, OBS-002, OBS-003
- **authority:** authorized
- **limitations:** one actor authors and verifies, so §27.4 applies — role
  separation by one person is not organizational independence, and `war check`
  reporting §46.3 unmet for this Warrant is correct rather than a
  misconfiguration.

## Gate Adequacy

Required at `controlled`.

**Adversarial question one: could this ship while the invariant is quietly
one-directional?** Yes, and it is the likeliest way this goes wrong. Checking
`⊆` — nothing in the document that the subject cannot see — is what every export
tool already does, and it is much cheaper than `⊇`, which requires enumerating
the whole visible set and proving nothing was missed. Drop the second direction
and the document still looks complete, the gate still passes, and the sentence
this Warrant exists to make true becomes false in the only direction the reader
cannot detect. OBL-001 requires a plant in each direction for that reason, and
`40-work-order.md` forbids narrowing to `⊆` without amending the Warrant.

**Adversarial question two: could the gate compare the compiler to itself?** Yes,
if `visibleSet` were computed by the compiler that produces `documentSet`. The
comparison would then be a tautology and would pass forever. STAGE-001 is ordered
before STAGE-002 to prevent it: the enumeration exists, and is made to fail,
before there is a compiler whose output could define it.

**Executed attacks:** none. This Warrant has not been executed, and the two
counterexamples above were found by reasoning rather than by running anything.
They are recorded as counterexamples because each names a specific way the
capability passes its own checks while being wrong — not as a plan to look for
one later.

- **outcome:** counterexample_found, gate_strengthened, gap_accepted

Both counterexamples changed the Warrant rather than being noted beside it.
Question one produced OBL-001's requirement of a plant in EACH direction plus a
positive control, and `40-work-order.md`'s prohibition on narrowing to `⊆`
without amendment. Question two produced the ordering constraint that STAGE-001
precedes STAGE-002. The accepted gaps are the two in Residual Risk below, and the
absence of executed attacks, which cannot be closed before the work exists.

## Residual Risk

**Federated material may exceed what the access context governs.** The intent
says the document is compiled from federated material, and `visibleSet(P)` is
enumerated from KF's row-level security. Where a federated source is mirrored
into KF objects the two coincide; where content is resolved live from an external
system, that system's access model is not KF's, and the invariant is only as
strong as the mirror.

This now has a gate — OBL-006 and M2 force the classification and the decision
before STAGE-002 — but the gate does not make the risk go away, it only makes it
impossible to walk past. Two residues remain and are accepted rather than solved:
materializing carries its own completeness question, because a mirror that has
not caught up satisfies the invariant while defeating the intent; and annotating
sections weakens the claim from "the file is complete" to "the file is complete
where it says it is", which is a different promise than the one in
`10-intent.md`. Choosing between them is work for M2, and whichever is chosen
should be recorded as an amendment if it changes what the intent promises.

**The `⊇` direction may be expensive.** Proving no under-disclosure means
enumerating the whole visible set across 76 policy-governed tables and diffing
it, per subject, per compilation. Nothing here has measured that cost. It is the
likeliest practical reason the expensive half gets quietly dropped, so the
enumeration's cost should be measured during STAGE-001 — while it is still the
subject of the work — rather than discovered in STAGE-002 when a compiler already
depends on it.

**One instance, one actor.** §46.3's independence minimum is unmet and is
reported as unmet.

**ADR 0008 is open upstream.** `war kf` carries no classification header. A
disclosure act bounded by a classification ceiling cannot express that ceiling on
the wire today, so M5 either waits for that decision or records precisely what it
could not assert.
