# Example 1 — a code Warrant, walked from its records

OW-WAR-0046, *Discharge the Phase 6 exit: a delivery closes only through
bounded proof*. Profile `delivery`, assurance `controlled`. Everything below
is read from `docs/warrants/OW-WAR-0046/`; run the commands to see the same.

## What the Warrant claims

Two §106 rows, both `complete`: RQ-053 and RQ-054 (a delivery closes on
bounded proof; the thirteen §56.1 requirements are computed). Two
deliverables, both content-addressed files: `resolve.rs` (D-001, "the
thirteen requirements, computed") and `verify.rs` (D-002, "the independent
verification seam"). Five obligations in `60-assurance.md`, each with a
bounded scope, a gate, and the evidence it expects:

| obligation | claim |
|---|---|
| OBL-001 | a real gate run produced a complete receipt |
| OBL-002 | an adequacy review executed attacks |
| OBL-003 | all thirteen requirements are computed, and each unmet one is named |
| OBL-004 | dispute and annulment preserved history |
| OBL-005 | §91.10, §91.11 and §91.12 are planted |

```bash
war show OW-WAR-0046            # the compiled Warrant
war check OW-WAR-0046           # the record is sound: exit 0
```

## The journal, in order

```bash
war journal OW-WAR-0046
```

| when | event | who |
|---|---|---|
| 2026-08-21 | `draft.created` | `agent://claude` |
| 2026-09-02 | `authorization.recorded` | `person://Brian Lam` |
| 2026-09-02 | `verification.recorded` ×5 | `agent://lamu:mimo-v2.5-pro` |
| 2026-09-03 | `sync.receipt_attached` | `agent://claude` |
| 2026-09-03 | `verification.recorded` ×4 | `agent://lamu:mimo-v2.5-pro` |
| 2026-09-03 | `resolution.recorded` | `person://Brian Lam` |

Read it as the loop: the agent drafted; a human authorized (`authorization.toml`,
`acting_role = "authorizer"`, `2026-09-02T06:00:00Z`); a verifier that is
not the performer recorded five verdicts; the gate run's receipt was
attached; a second round of verification followed the receipt; a human
resolved.

## The evidence

```bash
ls docs/warrants/OW-WAR-0046/gate-runs/
#   software_repo_war-check_1_0_0.receipt.json   the §44.6 receipt, bound to the contract digest
#   software_repo_war-check_1_0_0.run.toml       the run record
#   software_repo_war-check_1_0_0.stdout.txt     what the gate printed
```

`war evidence record OW-WAR-0046` produced these. The receipt names the
contract digest it ran against; a receipt from an earlier contract is
inadmissible, not stale-but-fine.

## The verification

Five files under `verifications/`, one per obligation, each `disposition =
"established"`, each from `agent://lamu:mimo-v2.5-pro` — a separate model
in a separate context, handed the request `war verify OW-WAR-0046
--performer claude` emitted. Had the verifier been the performer, `war verify
--response` would have refused and written nothing.

## The resolution

```toml
# resolution.toml
common_outcome = "satisfied"
profile_outcome = "delivered"
resolved_by_ref = "person://Brian Lam"
artifact_manifest_digest = "sha256:08d9ed4a…"     # sha256(deliverables.toml)
```

`war resolve --dry-run OW-WAR-0046` lists the thirteen requirements it met.
The resolution binds the deliverables record by digest: from this moment
`resolve.rs` and `verify.rs` are pinned, `war pins --resolved-only` lists
them, and the plugin's edit guard refuses to touch them. They move only
through the correction act — `war correct OW-WAR-0046 D-001`, signed by a
human — which is what OW-WAR-0064 added.

## What to copy

- obligations with a **scope** and the **evidence** they expect, written
  before the work;
- one gate the assurance atom cites, so `war evidence record` has something
  to run;
- a verifier that is not you;
- a `deliverables.toml` that names files, so the resolution can pin bytes.
