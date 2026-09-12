# Release 1.0 — SAS §99 against the record

§99 lists twenty-five acceptance criteria. This table says, for each, which
Warrant delivered it and what proves it today — a plant, a test or a command.
Rows 13, 15, 16 and 17 are the federation kernels; the 1.0 scope is
**standalone** (git + `war` + MCP + skill + ssh signatures), so those rows
are stated as not exercised rather than claimed. Nothing here is a status
claim: `docs/warrants/generated/CORPUS_STATUS.md` is where standing lives.

Twenty-one rows name a proof that runs today; four (13, 15, 16, 17) are
scoped out of standalone 1.0 and say so.

| # | Criterion | Warrant | Proof |
|---:|---|---|---|
| 1 | a human can create a draft from one sentence | OW-WAR-0042 (draft), A4/A8 | `war plan "<sentence>" --draft --apply --reviewed`; plants 71-plan.sh |
| 2 | the planner asks only unresolved high-value questions | OW-WAR-0042 | `plan.interview-required` refuses unanswered blockers; plants 71-plan.sh |
| 3 | the agent outputs structured proposals | OW-WAR-0042, OW-ADR-0013 | `oh.war/draft-proposal/v2` validated by the §74.4 gauntlet; plants 71-plan.sh |
| 4 | authored atoms remain the editable sources | OW-WAR-0002 | `war check` composition rules; plants 00-corpus.sh (manifest.*) |
| 5 | the parent is one complete generated document | OW-WAR-0004 | `war compile`; `generated.drift`; plants 00-corpus.sh |
| 6 | every generated section has provenance | OW-WAR-0004 | WAR.json `source_and_composition`; test `lowering_is_deterministic` |
| 7 | every normative decision is an ADR | OW-WAR-0036 | `adr.*` rules; plants 00-corpus.sh (local choice needs no ADR) |
| 8 | all ADRs compile into the audit overview | OW-WAR-0006 | `docs/adr/generated/ADR_OVERVIEW.md`, drift-checked |
| 9 | WARs reference SAS and Roadmap | OW-WAR-0013 | `traceability.*`, `roadmap.wrong-namespace`; plants 00-corpus.sh, 76-pinned-batch.sh |
| 10 | child WARs inherit exact parent context without rewriting it | OW-WAR-0006 | `relations.rs`; plants 00-corpus.sh (parent refs) |
| 11 | superseded WARs remain honest history | OW-WAR-0006, OW-WAR-0059 | `supersedes` relations; resolution history plants 00-corpus.sh |
| 12 | local drafting works offline | OW-WAR-0001 | no network in any command but `war kf` |
| 13 | KF registration adds global authority without stealing Git source authority | OW-WAR-0029 | standalone 1.0: not exercised — the KF adapter has its own profile and stays open past 1.0 |
| 14 | a stateless actor can execute one Dispatch | OW-WAR-0047, OW-WAR-0066 | `war dispatch` → `war run` / `war submit`; plants 82-context.sh, 86-run.sh |
| 15 | Katana authority is not duplicated | — | standalone 1.0: not exercised; no duplication exists (no Katana code in this repository) |
| 16 | BLUT authority is not duplicated | OW-WAR-0027 | standalone 1.0: not exercised beyond the PlanSpec lowering; no duplication exists |
| 17 | Liminal authority is not duplicated | — | standalone 1.0: not exercised; no duplication exists (§82.2 names no command) |
| 18 | performer claims cannot become independent evidence | OW-WAR-0021, OW-WAR-0046 | `verify.inadmissible`, §46 independence fields; plants 84-verify-bundle.sh |
| 19 | unaskable gates cannot pass | OW-WAR-0020 | `not_askable` pairs with `not_run`; plants 00-corpus.sh (gate runs) |
| 20 | the assurance case separates evidence, observation, inference, judgment, and resolution | OW-WAR-0017 | `epistemic.rs` vocabularies; plants 00-corpus.sh |
| 21 | a material amendment never changes prior attempt basis | OW-WAR-0010 | `attempt_basis_digest` pinned per attempt; plants 00-corpus.sh (amendments) |
| 22 | resolution requires the exact authorized contract | OW-WAR-0059 | `resolution.stale`, requirement 1 of the thirteen; plants 00-corpus.sh |
| 23 | dispute and annulment preserve history | OW-WAR-0046 | `ResolutionStanding`; OBL-004 of OW-WAR-0046 |
| 24 | one canonical JSON export preserves the full Warrant | OW-WAR-0030 | `war export --round-trip --reconnect`; plants 00-corpus.sh |
| 25 | basic WAR overhead is low enough that bypass is irrational | OW-WAR-0041, F1 | `artifacts/telemetry-baseline.json`; `war eval`: twelve tasks, about 1.5k tokens and under a second each with the fixture agent |

## The two acts before the tag

1. **Relicense to Apache-2.0** — `RELICENSING.md`'s preconditions hold
   (`cargo deny check licenses` is green; contribution terms are in place).
   The owner changes `license` in `Cargo.toml` and the `LICENSE` file in one
   commit. `release.yml`'s crates job refuses to publish under any other
   string.
2. **Accept SAS 1.0.0** — `war sign 1.0.0 --ssh-sign` with OW-ADR-0016; its
   acceptance is the first attested SAS act.

Then `git tag v1.0.0` and push: `release.yml` runs the gate, builds both
hosts, proves the IR is byte-identical across them (§91.1 test 1), creates the
release, and publishes `openwarrant-core`, `openwarrant-agent`,
`openwarrant-compiler`, `openwarrant-cli` in that order.
