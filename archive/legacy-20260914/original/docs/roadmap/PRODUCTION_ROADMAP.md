# OpenWarrant Production Roadmap

The SAS says *what* the system becomes. This says *when and in what order* (§6.3),
and what each release gate means.

Governing specification: `docs/sas/WAR_Software_Architecture_Specification.md`
v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.

## Beta opened 2026-08-21 — alpha reached, and what that does and does not mean

**All 40 alpha Warrants are resolved as of 2026-08-20, and beta opened
2026-08-21.** Measured, not asserted:

```
cargo xtask gate    PASS, 7/7 steps
conformance/plant.sh    36 planted violations, 36 rejected by their intended control
war check --generated   212 pass · 2 warn · 0 unknown · 0 error
cargo test --workspace  479 tests
```

Alpha means **every SAS capability exists and OpenWarrant implements its side of
every protocol.** It does not mean the system has been used.

Read the four honest limits below before treating this as a finished product:

1. **One neighbour has now answered; three have not.** Corrected 2026-08-21 —
   this limit previously said all four had "no checkout on this host", which was
   false for two of them: BLUT is at `training/engine` in the LamQuant tree and
   the Knowledge Fabric at `/mnt/4tb/openhuman-knowledge-fabric`. Only Katana and
   Liminal are genuinely absent. The sentence overstated the limit, which is the
   less common direction for a claim to rot but still rot.

   BLUT has since answered for real. `war blut <alias> --verify <blut-binary>`
   writes the lowered PlanSpec to a file and runs `blut plan check --json` on it.
   On 2026-08-21 a cookbook binary read OpenWarrant's bytes and first **refused**
   them, then — once the adapter stopped using WAR stage ids as BLUT stage names
   — **accepted** a two-stage lowering (`materialize_dataset_path` ->
   `filter_dataset`, fingerprint `a2005e3c9535…`, exit 0). Both verdicts are
   recorded as `authoritative_external` in OW-WAR-0047's assurance atom.

   The first refusal was correctly reported and wrongly explained here. It was
   attributed to Warrants naming `STAGE-NNN` identifiers no cookbook compiles in
   — true, but not the cause. The cause was the adapter having nothing else to
   name a stage with. A real external tool refusing for a real reason still
   supported the wrong conclusion about what was possible, which is worth more
   as a lesson than the acceptance is.

   What this does **not** establish: an execution. BLUT typechecked a plan; it
   did not run one. OW-WAR-0047's OBL-001 wants
   status, artifact and lineage receipts from a real BLUT run, and a typecheck
   produces none. The obligation stays open. Katana and Liminal remain
   unexercised, and the KF adapter has still never been spoken to despite the
   checkout being right there.
2. **The execution plane is not wired into the binary at all.** This is
   stronger than "has no plants", and it was measured on 2026-08-20 rather than
   assumed: of twenty alpha types sampled — `Admissibility`, `Independence`,
   `EvidenceItem`, `Observation`, `Judgment`, `GateReceipt`, `KatanaReceipt`,
   `ResolutionChecks`, `PreflightReceipt`, `ClaimGraph`, `ActionEnvelope`,
   `PortableExport` and the rest — **twenty are unreachable from `war` and the
   compiler.** §40.7's six prohibited substitutions are enforced by a function
   nothing calls.

   The alpha claim stands as worded: the capability exists, typed and tested.
   But a reader could reasonably infer that `war check` enforces these rules on
   a corpus, and it did not.

   **Three closed 2026-08-21.** `EvidenceOrigin` and `Admissibility` are parsed
   from declared obligation bullets and enforced at read time, so §40.7's first
   prohibited substitution is reached by the shipped binary and planted
   (`a performer report admitted as independent`). `GateReceipt` is minted by
   `war gate --run` — a completed run writes a §44.6 receipt with a sealing
   digest, and an unaskable run writes none, which is planted too.

   **Five closed.** `Independence` is read from `openwarrant.toml` and checked
   against §46.3 per level; `ResolutionChecks` is computed by
   `war resolve --dry-run`, which reports 11 of 13 requirements unmet and
   refuses to close any Warrant; `AmendmentRecord` is parsed and validated from a
   Warrant's `amendments/` directory; §40's four evidence record types are
   parsed from the assurance atom's `## Evidence` section, with §91.11 tests 76
   through 81 planted against the shipped binary; `ClaimGraph` detects cycles in
   the claim/evidence graph (§36.4, §91.10 test 74). **Nine remain**, and
   OW-WAR-0046 owns them.
3. **Two adequacy reviews report no executed attacks, and say so.** OW-WAR-0008
   and OW-WAR-0017 are the two `war check` still warns about. That is the check
   working: the state model and the epistemic classes have nothing to attack
   until real authorizations and judgments exist.
4. **This repository authored and verified itself**, and now says so where a
   machine can read it. `openwarrant.toml` declares all nine §46.1 dimensions
   false, and `war check` reports that 27 basic and 22 controlled Warrants fail
   §46.3's minimum, naming the missing dimensions. Until 2026-08-21 this limit
   was a paragraph in this file that no tool could act on.

Beta is where 1 and 2 close. Release is where the hardening Warrants are written.

## Release gates

| Gate | Means | Complete when |
|---|---|---|
| **Alpha** | Feature complete. Every SAS capability exists and OpenWarrant implements its side of every protocol. | All alpha Warrants resolved |
| **Beta** | Tested. Integration against the live Katana, BLUT, Knowledge Fabric and Liminal systems; conformance across two real hosts. | All beta Warrants resolved |
| **Release** | Hardened. High-assurance controls, security boundary, performance, observability, and the contractor profile. | All release Warrants resolved |

**Alpha explicitly does NOT include** hardening, performance work, the security
boundary beyond what correctness requires, signature infrastructure, or the
contractor Work Order profile. Those are named below and deferred on purpose so
that "feature complete" is a claim about capability, not about production
fitness. A capability that exists but has not been hardened is alpha; calling it
release would be the kind of overstatement this system exists to prevent.

**The adapters are alpha on our side only.** §11.2 makes OpenWarrant "the
protocol and compiler surface" — so compiling a Dispatch, lowering a stage graph,
and defining a receipt schema are alpha. Executing against a live kernel is beta,
because Katana and Liminal have no checkout on the development host and a
protocol proven only against a mock is not proven.

## Phase mapping (SAS §98)

The table that mapped each §98 phase to its Warrants and a hand-written status
is retired. `docs/warrants/generated/CORPUS_STATUS.md` lists every Objective
(one per §98 phase) with its Warrants, its ladder, and whether its Exit is
recorded, blocked or open — compiled from the records, never typed. §98
itself, in the SAS, is the phase list.

### Phase 1 is resolved but not whole

Recorded here rather than left implied. Three milestones in the Phase 1 Warrants
are **not met**, and they are carried forward as scope of the alpha Warrants
below rather than quietly dropped:

- ~~**OW-WAR-0002 M2** — §91.2 test 12 (composition cycle) has unit tests but no
  plant against the shipped binary.~~ **CLOSED 2026-08-19** by the
  `composition cycle (self-parent)` plant.
- ~~**OW-WAR-0005 M2** — §91.2 test 10 (a generated atom cannot be edited through
  an authored-source command) is not implemented.~~ **CLOSED 2026-08-22** —
  implemented as three rules in `war check` and planted three ways. Its first run
  against the corpus found all six ADR atoms declaring `authored` where §16.1
  places the `adr` role under `bound`, so a Warrant binding a decision claimed
  the right to rewrite it. OW-WAR-0005's OBL-002 claim about test 10 is now TRUE
  rather than amended.
- **OW-WAR-0005 M4** — bootstrap closure was declared, then the very next unit of
  work (the ADR Overview, commit `3678455`) shipped with no Warrant. That is
  untracked work under §95, committed against this repository's own obligation.

## Alpha, Beta and Release Warrants

This document used to carry four tables of Warrants with a hand-maintained
status column. They are gone. OW-WAR-0032 was marked "resolved" here while
its record said otherwise (see OW-WAR-0032's manifest) — the schema pack it names was never built — and a
status column nobody reads at gate time drifts exactly that way.

What exists, and what state it is in, is compiled from the records:

- `docs/warrants/generated/CORPUS_STATUS.md` — every Warrant's rung (draft,
  ready to resolve, would satisfy, resolved), every Objective's ladder, what
  is next actionable, and the caveats the projection carries about itself.
- `docs/warrants/generated/WARRANT_OVERVIEW.md` — the Warrants themselves,
  compiled.
- `war status`, `war next`, `war sign --list` — the same facts, live.

The alpha set (§106's requirements, one Warrant each), the nine beta Warrants
authored 2026-08-20 around §98's phase Exit criteria, and the release set
(Phase 9 signatures and custody, security boundary hardening, Phase 10's
contractor profile, the relicense) are all in the corpus under those names;
`roadmap://` refs in manifests resolve to §98's phases, not to this file.

### Adequacy reviews

The count of controlled Warrants with and without a §39.2 outcome is in
`CORPUS_STATUS.md` under each Warrant's thirteen checks. This section held a
number from 2026-08-19 that was false by the time alpha closed, which is the
failure mode the whole document is about; it is not restated here. The
reasoning behind the rule is, because it is not in any projection:

Missing attacks stay a WARN rather than an ERROR: §39.3 says attacks SHOULD be
run "where economical", and a SHOULD promoted to a hard failure is the first
rule anyone disables.

The rule was written to catch this repository first, and it did — twice.
OW-WAR-0023 had passed RQ-055 for two weeks with a section that never asked
§39.1's question, because the old check was a substring search for the word
"adequacy". Then the replacement's own absence-phrase allowlist was wrong three
times, hiding warnings for the Warrants being most honest, until attacks were
counted structurally instead.

### What "resolved" means for a module the corpus cannot yet exercise

Several alpha Warrants deliver machinery that `war check` does not yet consult,
because the record it would govern does not exist in this repository. Independence
declarations, authorization records, resolutions, disputes and monitors all arrive
as DATA with the local journal (OW-WAR-0031) and Knowledge Fabric federation
(OW-WAR-0028).

Those Warrants are resolved in the alpha sense — the capability exists, is typed,
and is unit-tested against the SAS's own vocabulary — and are NOT resolved in the
sense of being exercised on real records. The plant battery is the honest measure
here: a rule with no plant is a rule nothing has tried to break. Current plants
cover the record-level rules (§16, §19, §23, §28, §38, §39, §43, §44); the
execution-plane rules are unit-tested only.

Beta is where these meet real data. Until then, no gate count should be read as
evidence that resolution semantics have been exercised end to end.

## How this document stays true

It is a source atom, not a projection: it is hand-maintained. The generated
Warrant Overview (`docs/warrants/generated/WARRANT_OVERVIEW.md`) is compiled from
the Warrants themselves and is the authority on what exists and what state it is
in. Where the two disagree, the Overview is right and this file is stale.
