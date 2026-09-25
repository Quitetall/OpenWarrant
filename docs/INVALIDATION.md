# Custody, invalidation and dispute

What each act does, who may do it, and what it never rewrites
(OW-WAR-0136; SAS §41.5, §45, §56.4, RQ-057, §98 Phase 9).

Invalidating a gate is built as OW-WAR-0136 Q-001 (a) describes it: a human
act, signed with `war sign` by a human holding `resolver`, attested like the
other human acts.

## Custody audit

**What it answers.** Whether the evidence a resolution relied on is still the
evidence on disk, and what is and is not recorded about how that evidence was
kept.

**What makes it possible.** When `war sign --ssh-sign` records a resolution, its
attestation (`attestations/resolve-N.dsse.json`, OW-ADR-0015) names as subjects,
beside `resolution.toml` and the signed response, every receipt in
`gate_run_refs` and that receipt's `.run.toml`, stdout and stderr, each by
sha256. A receipt's own `receipt_digest` seals only its own fields: a receipt
replaced by a freshly minted one reseals, and every check that reads the seal
still passes. The attested digest is what catches it. `war attest <alias>
--verify` then reports `attest.subject-drift` naming the file.

**The command.**

```bash
war attest <alias> --custody                       # the audit, read-only
war attest <alias> --custody --record <auditor>    # and the record
```

For each relied-on receipt it lists the nine §41.5 fields, each either present
with its value or `UNKNOWN` with a reason. `UNKNOWN` fields are printed under
NOT CHECKED, never as a pass:

| field | where it comes from | when it is UNKNOWN |
|---|---|---|
| collector | the receipt's `runner` | the receipt names none |
| original digest | the resolve attestation's subject digest | `not attested` — every resolution signed before this change |
| transfer method | — | always: the receipt records no transfer, and nothing else does |
| storage event | `git log` of the receipt: the commit that added it, and how many rewrote it | not committed, or git cannot answer |
| instrument or runner identity | the receipt's `runtime_environment` | the receipt names none |
| calibration or qualification | the Gate Definition's qualifier and qualification digest | the gate declares none, or no definition is found |
| transformations | — | always: an absent record is not evidence that none occurred |
| access history | — | always: git records no reads |
| derivative lineage | the resolution that relied on it | never |

It fails (`attest.custody-drift`, exit 2) when an attested subject's bytes moved,
or — for a receipt that is not attested — when the receipt's own seal no
longer recomputes. It also verifies the resolve attestation's signature,
because a digest taken from an envelope whose signature does not verify is no
custody at all.

**Who may record it.** Any actor that is not the performer (Basis A-003).
`--record` by the performer is refused with `SelfAct`
(`attest.custody-self-act`) and writes nothing. The record is
`docs/warrants/<alias>/custody-audit.toml` (`oh.war/custody-audit/v1`): the
auditor, the time, the verdict (`intact` or `drifted`) and every field. An
existing record is not overwritten (`attest.custody-audit-exists`).

**What it never rewrites.** Nothing. The audit reads the resolution, the
attestations and the receipts; the only file it writes is its own record.

**What it cannot repair.** The resolutions signed before receipts were
subjects (29 in this repository when this was written). Their receipts'
original digest is `UNKNOWN (not attested)`, and only each receipt's own seal is
compared. Making them attested would mean re-signing them, which the audit does
not do. A resolution recorded through a batch (`war sign --batch`) is
attested by the batch envelope, which does not yet carry receipt subjects;
its audit reports the same `UNKNOWN`.

## Invalidation

**What it answers.** A Gate Definition version turns out to be unsound. Which
resolutions rested on it, and so stop being relied on?

**The request.**

```bash
war gate invalidate <gate_id>@<version> --grounds "<why>"
```

It emits `oh.war/invalidation-request/v1`: the gate, the sha256 of its
definition file, its lifecycle, the grounds, every resolution the sweep would
dispute — each with who signed it and why it is reached — every resolution
left standing, and who may sign. The sweep is `propagate_invalidation` (§45)
over the corpus: a resolution is reached when a receipt in its
`gate_run_refs` is a run of that gate, or when a Warrant it names as a parent
(§20.2) is reached and resolved; transitively. A version is required: a gate
across all its versions is never invalidated. Grounds are required (§56.4).
The request writes nothing.

**The act.**

```bash
war sign <gate_id>@<version> --grounds "<why>" --dry-run    # every refusal, nothing written
war sign <gate_id>@<version> --grounds "<why>" --ssh-sign   # the signature
```

The response (`oh.war/invalidation-response/v1`) names the gate, the
definition digest, the grounds — the signer's words, never drafted — and the
aliases it disputes, so the signature covers exactly that list. A hand-signed
response is ingested with `war gate invalidate <gate>@<version> --response
<file>`; it must be the response under `docs/authority/responses/` whose `.sig`
verifies. The ingest refuses, and writes nothing, when:

| refusal | rule |
|---|---|
| the signer is an agent — by kind, whatever the file says (§27.2) | `invalidation.agent` |
| the signer does not hold, or does not act as, `resolver` | `invalidation.not-permitted` |
| the signer is the performer (Basis A-001) | `invalidation.self-act` (`SelfAct`) |
| the response is not signed, or its signature does not verify | `invalidation.unsigned` |
| the definition file moved since the response was drafted | `invalidation.stale` |
| the sweep no longer reaches exactly the aliases signed | `invalidation.sweep-moved` |
| the version is already invalidated | `invalidation.exists` |
| no grounds; a malformed time; an unknown gate or signer | `invalidation.no-grounds`, `…effective-time`, `…unknown-gate`, `…unknown-actor` |

Then it writes, each file created new and never overwritten:

- `docs/gates/invalidations/<gate>@<version>.toml` (`oh.war/invalidation/v1`):
  the gate, the definition's file and digest, the grounds, the signer, the
  response and its digest, the resolutions left standing, and each dispute
  by path and sha256;
- `docs/warrants/<alias>/disputes/DSP-NNN.toml` (`oh.war/dispute/v1`) for every
  reached resolution, and a `dispute.recorded` event in that Warrant's
  journal;
- with `--ssh-sign`, an attestation under
  `docs/gates/invalidations/attestations/` whose subjects are the record,
  the response and every dispute.

The definition file is not edited (§43.3). No `resolution.toml`, receipt, run
or attestation is written: standing is read from the disputes, never written
into the record (§45 clause 4). An invalidation is never batched; it is signed
on its own screen.

**What reads it.**

- `war check` reports `resolution.disputed` for a resolution with an open
  dispute, naming the grounds and the reliance policy.
- A receipt of an invalidated gate is not admissible for §56.1 requirement 5 on
  a Warrant not yet resolved: `war check` says `evidence.gate-invalidated`,
  naming the invalidation, and `war resolve` counts the requirement unmet. A
  resolved Warrant's receipts are history; the invalidation disputes its
  resolution instead.

**An unsigned invalidation never counts.** Every reader believes a record only
when the response it names is on disk at the digest recorded, says the same
gate, digest, grounds and signer, and its signature verifies as a human
holding `resolver` who is not the performer. A record or dispute written by
hand — or one whose `.sig` has gone — disputes nothing: `war check` reports
each such dispute as `resolution.dispute-unsigned` and leaves the resolution
standing as recorded, and the receipt stays admissible
(`evidence.invalidation-not-counted`). A dispute edited after it was written is
`resolution.dispute-edited`; one the record does not list is
`resolution.dispute-unlisted`.

## Dispute

A dispute (§56.4) identifies the challenged resolution, its grounds, the
affected evidence or judgment, the reliance policy, its owner — the resolver
who signed the invalidation — and the re-verification it requires. Reached
directly, the affected evidence is the receipt and the re-verification is a
re-run under a new, qualified version of the gate; reached through a parent,
it is that parent's resolution, and its own dispute closes after the
parent's. Every dispute is `open`. Closing one (§45 clause 6: re-verify on a
new gate version, then resolve the dispute or annul) is out of OW-WAR-0136's
scope and belongs to a later Warrant; until then, a dispute is not edited.

## The demonstration gate

`docs/gates/ops.exit-demo@1.0.0.yaml` (Q-002 (b)) exists to be invalidated: it
passes when this file exists and is not empty, and nothing else relies on it.
The exit is shown in this repository by one small Warrant resolved against it
by the owner, and then the owner invalidating it, so that only that
resolution is disputed. Both are the owner's acts; neither is recorded here.

## Who may do what

| act | who | refused to |
|---|---|---|
| resolve (unchanged) | a human holding `resolver` | the performer (`SelfAct`), every agent |
| custody audit, read | anyone | — |
| custody audit, record | any actor but the performer | the performer (`SelfAct`) |
| invalidation request | anyone; it writes nothing | — |
| invalidate a gate | a human holding `resolver`, signing with `war sign` | the performer (`SelfAct`), every agent (by kind), an unsigned response |
| dispute | written only by a signed invalidation | anyone writing one by hand: it does not count |
