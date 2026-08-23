---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be73-76f7-9aa7-d883cc39d51e
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a real KF instance answered typed actions
- **scope:** one running Knowledge Fabric instance, named and version-pinned.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** Knowledge Fabric, by exact commit or release.
- **evidence:** a recorded action envelope and its receipt, where `recorded_at`
  was assigned BY THE SERVER. A client-supplied value is refused, and that
  refusal is recorded too.

### OBL-002 — the enterprise identifier came from KF, not from us
- **scope:** §12.4 and §91.3 test 20.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** Knowledge Fabric's allocator.
- **evidence:** an identifier returned in a KF receipt, plus a recorded refusal
  of a locally-derived identifier of identical SHAPE. The contrast is the
  evidence: shape alone cannot distinguish them, provenance can.

### OBL-003 — Git remained Source Holder after registration
- **scope:** §91.3 test 21, for the registered Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** after registration, the authored atoms still resolve to
  Git and their digests are unchanged. A plant asserting KF as Source Holder for
  an authored atom is refused.

### OBL-004 — a §68 round trip preserved semantic and digest identity
- **scope:** the registered Warrant, exported into an empty compatible instance.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** export, reconnect preserved bytes, re-export, compare.
  Digest identity holds AND the recorded semantic-difference list is empty. A
  comparison run without reconnecting the bytes is refused as vacuous.

### OBL-005 — §91.3 and §91.13 are planted
- **scope:** §91.3 tests 19 and 22, §91.13 tests 91 through 95.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** seven entries in `conformance/` against the shipped
  binary, each rejected by a named rule with a named detail.

## Evidence

### EV-001 — the KF seam, read-only, against the live service
- **class:** evidence
- **kind:** external_tool_verdict
- **origin:** knowledge_fabric
- **admissibility:** authoritative_external
- **digest:** sha256:pending-receipt-binding
- **method:** `war kf health` against the running instance at
  `http://127.0.0.1:4000`, which answered
  `{"service":"openhuman-knowledge-fabric-api","status":"ok"}`
- **occurred at:** 2026-08-23

### EV-002 — the seam's two refusals
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** conformance/plant.sh — a §67 action without `--confirm-write`, and
  an `https://` URL this build cannot serve
- **occurred at:** 2026-08-23

### OBS-001 — KF was already running; nothing needed standing up
- **class:** observation
- **evidence:** EV-001
- **method:** `kf-postgres` and `kf-keycloak` up 6 days, `kf-minio` 4 days, and
  the API listening on :4000. The blocker was never KF's availability.
- **admissibility:** authoritative_external

### OBS-002 — the blocker was on this side: no HTTP client existed
- **class:** observation
- **evidence:** EV-002
- **method:** before this Warrant the workspace had 74 crates and no reqwest,
  ureq or hyper. §67's seam could not be reached because nothing here could
  speak HTTP at all.
- **admissibility:** performer_report_only

### OBS-003 — TLS is absent by LICENSE, and the gate is what said so
- **class:** observation
- **evidence:** EV-002
- **method:** every TLS configuration of ureq 3.4 tried — `rustls`,
  `platform-verifier`, `native-tls` — pulls Mozilla's CA bundle
  (`webpki-roots` or `webpki-root-certs`) under `CDLA-Permissive-2.0`, and
  `cargo deny check licenses` rejected each. The no-TLS build passes unchanged
  at 121 crates. This was measured across four configurations, not assumed.
- **admissibility:** controlled_measurement

### INF-001 — the missing TLS is a recorded limitation, not a latent failure
- **class:** inference
- **kind:** deductive
- **premises:** OBS-003
- **claim:** kf-seam-reachable
- **reasoning:** a network client that cannot do TLS is dangerous when it accepts
  an `https://` URL and fails inside the transport, because the operator reads
  that as the service being down. `Client::new` refuses such a URL and names both
  the missing feature and the licence that caused it, so the limitation is
  visible at the point of use and fixable by a decision rather than a debugging
  session.
- **admissibility:** controlled_measurement

### JDG-001 — OBL-001, OBL-002 and OBL-004 are held, not narrowed
- **class:** judgment
- **kind:** scope_holding
- **actor:** QuiteTall
- **acting role:** author
- **meaning:** all three require WRITING to KF — a recorded action envelope with
  a server-assigned `recorded_at`, an allocator-returned enterprise identifier,
  and a §68 round trip through a real instance. The adapter that would do it
  exists and is exercised read-only. The writes are held because the KF dev
  database is shared with another agent's in-flight test run, and because a §67
  action mutates an authoritative external record: doing that to someone else's
  working state to satisfy our own obligation is the wrong trade. Held pending
  authorization, not narrowed — nothing about the obligations has changed.
- **basis:** OBS-001, OBS-003
- **authority:** authorized
- **limitations:** one actor, so this judgment is not independently reviewed —
  §27.4 says role separation by one person is not organizational independence

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could a Warrant be registered while authority is quietly
confused?** Yes, and this is the failure the exit criterion is worded to prevent.
The tempting implementation registers a Warrant and, in the process, starts
treating KF as the answer to every question about it — including where its source
lives. Nothing would visibly break. `war check` would pass. The property that
died is one nobody looks at until Git and KF disagree about a byte.

OBL-003 exists because that failure is silent, and it is written as a digest
comparison rather than a design review for the same reason.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

One instance. §91.3 test 18 (the same local alias in two repositories must
not collide) is genuinely cross-instance and cannot be discharged against a single
KF, so it stays open here and is not claimed.
