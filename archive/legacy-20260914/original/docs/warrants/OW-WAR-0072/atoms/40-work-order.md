---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32d9-76e0-8417-2d69773249ca
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-core/src/batch.rs`: `oh.war/batch/v1` — `Batch { schema, batch_id, drafted_at, signer_ref, acts: [BatchAct { act, target, bound_digest, reason, kind }] }`, canonical JCS under a new `DigestDomain::Batch`, `validate()` (non-empty, no duplicate target, every act kind known, every reason non-empty), and one `bound_digest_for(act)` helper the drafter and the ingest share so they cannot disagree.
2. `crates/openwarrant-cli/src/batch_cmd.rs`: `war sign --batch [targets…]` drafts the document from `sign::pending`, writes it to `docs/authority/batches/<batch_id>.json`, prints the list for the signer to read, and signs it with `ssh-keygen -Y sign -n oh.war/response`; `--response <file>` ingests, verifying the signature over the batch bytes before any record is written.
3. The ingest loop: for each act in the batch, call the act's existing ingest (authorize, resolve, correct, SAS accept) with the batch's signer and reason, writing each record with a new optional `batch_ref = { batch_id, digest }`. One transaction in spirit: nothing is written until every act validates, and a single refusal leaves the corpus untouched.
4. Refusals, each by name and each before any write: `batch.empty`, `batch.duplicate-act`, `batch.unknown-act` (not in the pending set), `batch.digest-moved` (names the act and both digests; refuses the whole batch), `batch.agent` (an agent-kind signer), `batch.not-permitted` (an act the signer's role cannot perform), `batch.signature` (the signature does not verify over these bytes), `batch.namespace` (signed under the wrong namespace), `batch.reason-empty`.
5. Attestation: one DSSE envelope over the batch document and every record it produced, plus the per-act envelopes each naming the batch digest as an additional subject (`attest.rs`, `OW-ADR-0015`'s shape).
6. `war console`: `s` on a checked set of two or more offers the batch path, one preset question per act kind as today, and one dialog for the batch.
7. `conformance/plants.d/96-batch.sh`: every refusal above, plus the positive — a two-act batch signed once writes two records, each citing the batch, and `war attest verify` accepts both.
8. `docs/SIGNING.md` (new): the batch act, what a signer is agreeing to, and why a moved digest refuses everything rather than the one act.

## Frozen Surfaces

The per-act record schemas (only an optional `batch_ref` is added), `oh.war/report/v1`, the DSSE envelope shape, the `oh.war/response` namespace.

## Premade Instructions

- Reuse each act's existing ingest. A second implementation of authorization
  is a second place for the thirteen requirements to disagree.
- Verify the signature before the first write, and re-check every bound digest
  at ingest time, not only at drafting time.
- A batch is refused whole. Partial application would mean the signer
  authorized a list they did not sign.
- No new dependency. `ssh-keygen` and the existing base64 are enough.

## Autonomy and Escalation

Tier T2. Escalate rather than decide: whether a batch may mix act kinds that
require different roles (draft says no — one role per batch); whether a batch
may include a SAS acceptance alongside Warrant acts (draft says yes, since one
signer performs both).
