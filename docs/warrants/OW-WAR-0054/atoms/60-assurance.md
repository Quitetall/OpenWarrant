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
- **scope:** `masterRecord(P,O,C,T) == permission(O,C)` for a test subject, both
  directions reported separately.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two plants and their refusals. One inserts a record the subject
  cannot see and requires the gate to refuse for over-disclosure; one omits a
  record the subject can see and requires refusal for under-disclosure. Each
  refusal names the direction and the offending record. A run in which the gate
  passed is not evidence for this obligation — only the two refusals are, plus a
  positive control so that a gate refusing everything cannot discharge it.

### OBL-002 — nothing publishes without a matching permission-set digest
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
- **scope:** every table reachable in `permission(O,C)`, classified as materialized
  into KF-governed rows or resolved live from an external system.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a classification covering the whole permission set with no table
  unaccounted for, and for every live-resolved table a recorded decision: either
  it is materialized before it may enter a document, or the document annotates
  that section as outside the invariant. A classification with an unclassified
  remainder is refused; "the rest are probably materialized" is the assumption
  this obligation exists to prevent.
- **note:** this obligation exists because the residual risk below is real and
  had no gate. Without it an implementer reaches the compiler, finds a large
  share of the permission set resolved live, and decides it under schedule pressure
  — which is how the completeness claim ends up quietly not covering part of
  itself.

### OBL-007 — the relevance closure terminates, proven on a planted cycle
- **scope:** the fixpoint over composition, version and provenance relation
  types seeded by a person's anchors.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a planted cycle in a provenance chain — `derived_from` back to an
  ancestor — from which the closure returns rather than exhausting. Of the 41
  registered relation types only six declare `acyclic`, and **not one provenance
  type is among them**, so termination here is a property of the traversal and
  cannot be inherited from the ontology. A closure that has never met a cycle is
  not evidence that it survives one.
- **note:** `acyclic` is declared per relation TYPE, never per propagation class,
  so a class is only as safe as its current members. `supersedes`, `amends` and
  `extends` are acyclic today and the version class traverses both directions on
  that basis; adding one cyclic type to that class later would make bidirectional
  traversal non-terminating with nothing in the ontology objecting. The
  termination proof must therefore hold for the traversal itself, not rest on
  which types happen to populate a class.

### OBL-008 — fan-out is measured, and every ceiling governs rendering only
- **scope:** the size of `permitted(P,T)` — which is what the record actually
  contains — AND relevance closure size per anchor type, both measured on real
  data; and every threshold in every renderer.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** both measurements, plus a planted oversized
  subtree proving that the master record still CONTAINS every member while a
  rendering references rather than inlines them, and says which it did. A
  threshold that removes a record from the record itself is refused.
- **note:** membership is decided by the invariant; presentation is decided by a
  rendering. They must not be decided together. A ceiling that dropped members
  would reintroduce the second cause of absence — policy, rather than clearance —
  that the maximal closure exists to eliminate, and would reintroduce it in the
  layer nobody audits. The measurement is for storage and rendering budgets, not
  for deciding what someone is entitled to.
- **note:** an earlier draft of this obligation measured relevance fan-out only.
  That was the wrong quantity once membership moved: record size is
  |permitted(P,T)|, and relevance governs sectioning. Measuring only the closure
  would have left the obligation meant to catch "a record nobody can open"
  looking at a number that does not determine record size.

### OBL-009 — the anchor and propagation policy lives in the ontology
- **scope:** `registry.relation_type` and `ontology/relation-types.yaml`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** relation types declare which may anchor to a person and which
  propagation class they belong to, and the compiler reads that declaration
  rather than carrying its own list. Today the registry holds only `id`,
  `inverse_label`, `acyclic` and `is_symmetric` — there is no domain, no range
  and no propagation class, so a compiler written now would necessarily carry a
  hand-maintained mirror of an ontology it cannot check itself against. A planted
  disagreement between declaration and compiler behaviour must be refused.
- **note:** propagation CLASS and anchor DEPTH are separate degrees of freedom and
  both must be declared. The five-class table in `10-intent.md` enumerates the
  first only; the second — how far each anchor's interest reaches through its
  permitted classes — is what makes `owned_by` differ from `was_associated_with`
  and is not stated anywhere yet.

### OBL-010 — a person's ceiling comes from a clearance model, not from a caller
- **scope:** the derivation of `max_classification` for a subject, and every path
  that reaches `core.set_access_context`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a ceiling resolved from an authoritative clearance model, and a
  refused attempt to supply one directly. Today `maxClassification` passes
  unvalidated through `packages/authorization/src/identity.ts:250` and there is no
  model to clamp against, so the value the whole record's membership depends on
  is currently caller-asserted. The refusal is the evidence; a resolver that
  merely prefers the model while still accepting an override discharges nothing.
- **note:** this is upstream of the compiler because membership is permission and
  permission is determined solely by ceiling. Compiling an exact record around an
  unvalidated number produces precision about the wrong set, in both directions
  and without a signal.

### OBL-011 — entitlement subtracts, is reasoned, and can never fail by omission
- **scope:** `permitted(P,T) = permission(O,C) ∖ exclusions(P,T)` and every write
  to the exclusion store.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a record excluded from one person and not another, with the
  exclusion carrying subject, reason class, free-text reason, authorizer and
  time; the same record still present in the other person's record; and the
  exclusion appearing in the first person's withheld ledger. Plus the negative
  that matters most: **a person with no exclusion rows sees everything
  `permission(O,C)` admits.** An implementation that requires a grant before a
  record is visible is refused outright, whatever it is called.
- **note:** default-open is the whole point. A grant-based layer fails by
  omission — forget to grant, and the record is invisible with nothing recording
  why — which is the exact bug this Warrant exists to make impossible. The test
  for that failure is the empty-exclusions case, not a happy path.

### OBL-012 — the org view opens without a second authorization
- **scope:** `permitted(P,T) ∖ relevance(P,T)` as a first-class surface.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** every member of the org view resolves to full content for that
  person with no further check, no elevation and no request — because the
  decision was made when it entered `permitted`. A path that re-authorizes on
  open is refused: it would mean either the entitlement layer did not decide, or
  it decided twice with two chances to disagree.
- **note:** the org view is a view, not a leftover. If it were merely titles
  behind a gate it would be a wall, and people would route around it — which is
  how shadow copies start.

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

### OBS-001 — permission is enumerable; it is not person-scoped
- **class:** observation
- **evidence:** EV-001
- **method:** `core.set_access_context(p_organization uuid, p_max_classification
  text)` sets `kf.organization` and `kf.max_classification`, and row-level
  security on 76 tables is enforced against those. No policy in
  `row_security.sql` reads `kf.actor` or `kf.acting_role`, both of which exist but
  carry authorship rather than visibility. So `permission(O,C)` is a query, and is
  identical for every person at one organization and ceiling. Relevance is
  separately enumerable because `org.person.id` references `core.object(id)`,
  which makes a person a node in the typed graph.
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
the whole permission set and proving nothing was missed. Drop the second direction
and the document still looks complete, the gate still passes, and the sentence
this Warrant exists to make true becomes false in the only direction the reader
cannot detect. OBL-001 requires a plant in each direction for that reason, and
`40-work-order.md` forbids narrowing to `⊆` without amending the Warrant.

**Adversarial question two: could the gate compare the compiler to itself?** Yes,
if `permission` were computed by the compiler that produces the master record. The
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
says the document is compiled from federated material, and `permission(O,C)` is
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
enumerating the whole permission set across 76 policy-governed tables and diffing
it, per subject, per compilation. Nothing here has measured that cost. It is the
likeliest practical reason the expensive half gets quietly dropped, so the
enumeration's cost should be measured during STAGE-001 — while it is still the
subject of the work — rather than discovered in STAGE-002 when a compiler already
depends on it.

**One instance, one actor.** §46.3's independence minimum is unmet and is
reported as unmet.

**ADR 0008 is on the critical path, not beside it.** Two facts make it so.
`core.set_access_context` takes `(organization, max_classification)` and no
policy reads an actor, so a person's permission set is entirely determined by
their ceiling — and deriving that ceiling from a person is exactly what ADR 0008
leaves undecided, with `maxClassification` passing unvalidated through
`packages/authorization/src/identity.ts:250` and no clearance model to clamp
against. Then, because composition runs the full subtree bounded only by
clearance, that same undecided ceiling is the ONLY bound on how large a person's
file becomes.

So classification correctness now governs volume as well as confidentiality, and
a wrong ceiling is not merely a leak — it is a leak of unbounded size. Separately
`war kf` still carries no classification header, so a disclosure act cannot
express its ceiling on the wire. This is upstream of STAGE-002, not of STAGE-004
as previously recorded.
