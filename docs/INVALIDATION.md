# Custody, invalidation and dispute

What each act does, who may do it, and what it never rewrites
(OW-WAR-0136; SAS §41.5, §45, §56.4, RQ-057, §98 Phase 9).

Status on this branch: the custody audit and the invalidation **request** exist.
The invalidation **ingest**, the dispute records it would write, and the
standing they would drive do not: who may invalidate a gate, and whether that
is a signed act, is OW-WAR-0136 Q-001, and the owner has not answered it. This
page says which is which, and does not describe the missing half as if it ran.

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

**The request (exists).**

```bash
war gate invalidate <gate_id>@<version> --grounds "<why>"
```

It emits `oh.war/invalidation-request/v1`: the gate, the sha256 of its
definition file, its lifecycle, the grounds, every resolution the sweep would
dispute — each with who signed it and why it is reached — and every resolution
left standing. The sweep is `propagate_invalidation` (§45) over the corpus: a
resolution is reached when a receipt in its `gate_run_refs` is a run of that
gate, or when a Warrant it names as a parent (§20.2) is reached and resolved;
transitively. A version is required: a gate across all its versions is never
invalidated. Grounds are required (§56.4). The request writes nothing.

**The ingest (does not exist).** Q-001 decides whether invalidating is a fifth
human act signed with `war sign` by a holder of `resolver` (a), the same act
by a new `gate_steward` role (b), or an unsigned finding by any registered
actor that is not the performer (c). Until it is answered:

- no `docs/gates/invalidations/<gate>@<version>.toml` is written;
- no `docs/warrants/<alias>/disputes/DSP-NNN.toml` is written;
- `war check` reports no resolution as disputed;
- a receipt of a gate someone wants invalidated stays admissible for §56.1
  requirement 5.

Whatever Q-001 selects, the ingest will refuse the performer of any Warrant it
reaches, the definition file will not be edited (§43.3), and no
`resolution.toml` will be rewritten: standing is to be read from the disputes,
never written into the record (§45 clause 4).

## Dispute

A dispute (§56.4) identifies the challenged resolution, its grounds, the
affected evidence or judgment, the reliance policy, its owner — the resolver
who signed — and the re-verification it requires. Here the only source of a
dispute is an invalidation's ingest, so there are none yet. Closing a dispute
(§45 clause 6: re-verify on a new gate version, then resolve the dispute or
annul) is out of OW-WAR-0136's scope and belongs to a later Warrant.

## Who may do what

| act | who | refused to |
|---|---|---|
| resolve (unchanged) | a human holding `resolver` | the performer (`SelfAct`), every agent |
| custody audit, read | anyone | — |
| custody audit, record | any actor but the performer | the performer (`SelfAct`) |
| invalidation request | anyone; it writes nothing | — |
| invalidation ingest | **Q-001, unanswered** | the performer of any Warrant it reaches, whatever Q-001 says |
