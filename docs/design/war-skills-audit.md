# War skill and planning audit

Scope: RC.3 footer adoption, unsigned OW-WAR-0075–0091 scope migration, a compact
live overview command and the adapted skill suite. Work is unverified; this is
not a signed SAS adoption, independent review verdict or Stable release.

## What changed

- Human-readable SAS and format contract now precede compact metadata footers.
  RC.3 has an explicit wire identifier, boundaries, title agreement and byte-span
  rules. RC.2 examples and signed historical subjects remain unchanged.
- All 17 unsigned planned contracts use revised SDK/provider ownership and input
  prerequisites. OW-WAR-0077/0078/0081 expose Phase 1 SDK subsets before Phase 2
  integration; CLI parity 0087 precedes Phase 1 exit 0085. Signed 0074 is unchanged.
- `war overview` / `war progress` derives a compact live list from existing status
  computation. Default shows unresolved records; `--all` includes resolutions.
  JSON names the legacy model and does not claim new Verified qualification.
- One `war` router selects progress, grill, spec, tickets, map, execution or review.
  The former compulsory seam/granularity/shared-understanding approval loops are
  removed. Settled answers are reused; material scope decisions still escalate.
- Four methods no longer disable model invocation. Seven project-local Codex
  discovery links and the Claude plugin share the same files; root AGENTS.md
  provides the conditional entry pointer. Discovery remains harness-dependent.
- Review requires exact code, fixtures and evidence in an independent workspace;
  a bundle-only observer cannot claim rerun results. Prototype completion and
  common assurance remain separate. Human signing keys remain human-controlled.
- Stop hook reports corpus defects without treating every harness stop as completed
  work or blocking an unrelated task. The pin guard and actual action gates remain.
- Full upstream revision, method paths and Matt Pocock's MIT notice now travel with
  the suite. Existing historical skill/context walkthroughs are preserved separately.

## Evidence and limits

- Footer documentation witness: six tests, including valid LF/CRLF, compact
  dependency equivalence, fenced marker text and 16 malformed-input mutations.
  These exercise documentation framing, not the unbuilt production SDK.
- Source-set checker: 40 files, 105 requirements (all 92 historical IDs retained),
  56 old cases, 18 mapped Warrants, 24 SDK cases, eight context cases, eight footer
  cases and 16 byte-identical historical example files.
- Live CLI integration tests: remaining rows agree with live source records;
  progress alias gives the same JSON; all view includes recorded resolutions;
  outside a repository refuses rather than reporting zero work.
- Structural skill audit checks enabled descriptions, file links, short entry
  bodies, project discovery links and license notice. Main skill bodies total
  1,446 words versus 2,416 before, despite adding the war router. This is a word
  count, not a measured model-token or task-success improvement.
- Twelve fixed prompts in `conformance/skills/prompts.json` cover positive routes,
  settled-answer reuse, explicit gates, self-qualification refusal and non-triggers.
  They are evaluation inputs. Paid model evaluation and fresh Codex/Claude harness
  activation have NOT RUN; no success-rate claim follows from static checks.

`conformance/skills/check_skills.py` deliberately labels its result structural.
Use the future S08/provider harness to run the prompt set under an exact model and
budget, retain tool traces, and measure artifacts, wrong acts, extra questions,
constraint loss, cost and token usage. Do not score a string matcher as an LLM.

## Planning readiness

Phase 1 has enough decisions and bounded work to begin OW-WAR-0075. No new general
approval policy or product interview is needed. Each implementation slice prepares
its concrete fixtures and validates assumptions against real interfaces before
claiming completion. This planning pass does not mean all future API details are
known or that future integration contracts have already been accepted.

Phase 3 still needs its concrete webapp/user-study implementation Warrants when
Phase 2 exposes actual integration behavior. Existing Phase 4 plans explicitly
require that workflow evidence. The roadmap does not pretend those later Warrants
already exist. Shared cross-project integration still needs exact participant
contracts; no document here authorizes edits in another project.

## Legacy record and publication limits

Current corpus reports the existing README correction mismatch plus the changed xtask reference-count test pin and seven
OW-WAR-0068 delivery-digest mismatches for changed skills/glossary/docs. The old
manifest and independent evidence remain intact. The refreshed source bytes do
not inherit that Warrant's old delivery evidence. Resolve this through explicit
successor/delivery migration, not by rewriting old hashes or claiming qualification.
These facts do not create a universal pre-start signing requirement for unverified
SDK work. Existing signed correction and formal publication acts remain distinct.

No new paid model call or signing action was performed. Final command/gate results
are reported separately when observed; this note does not predict their verdict.

The xtask skill test assumed exactly four reference files. It now checks that
the inventory is nonempty and that every reference is linked/present; its missing
link and dangling-link refusal tests remain. This test-only edit changes a file
pinned by OW-WAR-0060/D-002, so its historical correction remains a publication
requirement. The old signed digest has not been rewritten.

## Final observed validation (2026-09-14)

Rust 1.97.1: aggregate gate rerun in an independent Git clone reached 12/14.
All 727 Rust tests passed; formatting, clippy, schema, license, skill and
attestation steps passed. Corpus failed with nine legacy digest mismatches:
README correction, xtask delivery pin and seven OW-WAR-0068 delivery digests.
The planted-violation battery refused dirty source inputs to avoid discarding
work during its restoration step; no full battery pass is claimed.

Final footer wrapper/Unicode/comment refinements then passed all six documentation
witness tests and the source-set checker. Current generated views were regenerated
and still expose the same nine corpus errors. No production Rust change followed
the successful Rust test run. Hook probes observed generated-edit refusal,
unpinned-edit allowance and diagnostic-only stopping on the actual red corpus.

Live `war progress --json` returned in 0.35 seconds here: 87 records, 29 recorded
resolutions and 58 unresolved records. Timing is one local observation, not a
benchmark guarantee. `/tmp/ow-current-overview.md` is the generated snapshot.
Source-set SHA-256: `661a1260ebc33d22cc8b9bf4ed3f9798df63457ef3093912ef7aca359c2faf2f`.

Changes remain uncommitted in the amendment checkout. No new commit, human
signature, independent LAMU verdict, merge, model evaluation or Stable publication
is asserted by this planning/skill pass.

## Migration and context follow-up

For corpus adoption, read [full-provenance migration](full-provenance-migration.md):
the current ADR importer does not migrate Warrant history, and distinct signed
records remain in other worktrees. Local discovery on 2026-09-14 covered 16
registered worktrees and 19,184 current document-file occurrences, including
duplicates. No file changed during its individual hash read; this is not an
atomic snapshot, backup or signature verification. The session manifest is
`/tmp/openwarrant-migration-discovery-20260914.json`, SHA-256
`aa5fa3e1b2c386ef1d7c343608a077f3e62c832d4af8d09bcef09ba20dbec3af`.

For provider design and evaluation, read the [current context-management
comparison](../research/context-management-2026-09.md). Its recommendations are
research guidance. They do not alter the pinned SAS source set or prove runtime
context quality.

## Document migration skill follow-up

`war migrate` now routes to the original `war-migrate` skill. It covers existing
system specifications, ADRs, plans and tickets as well as older OpenWarrant
records. It selects an exact edition, preserves original sources and historical
authority, maps document kinds and reports supported conversion results. Native
agent context files retain their roles. The legacy ADR shell importer remains a
narrow tool that the skill may use; it is not the complete skill workflow.

Structural audit now passes for eight skills, 34 links and 17 prepared prompt
cases. Skill bodies total 2,062 words after adding migration. The five new cases
cover ordinary documents, signed candidate migration, native context preservation,
read-only explanation and unsupported targets. Model evaluation and fresh harness
activation remain NOT RUN. SAS documentation checks and six footer witnesses pass.
All 17 unsigned implementation bases now reference source-set SHA-256
`0076942ef6a9cc376ab273daeee99fc73791fb4f96bd131204f5867486277451`.
The compiler regenerated affected views successfully. Corpus checking still
reports 841 passes, 88 warnings and the same nine legacy digest errors; no new
error category was introduced. No human act or completed corpus migration is claimed.

## Preservation and migration clarification

The owner clarified that `war migrate` preserves nonconforming originals unchanged
in their original relative structure, then authors new conforming OpenWarrant
documents from their content. The skill, SAS and migration guidance now state
that archive-only preservation or a wrapper is not completed conversion.
Candidate source-set SHA-256 is now
`217bea8600fd34405b0a521454b84bdffd0c06f05a9bd5b15a2b18168e47be7a`;
all 17 unsigned basis references were refreshed and projections regenerated.
Structural skill checks and SAS documentation witnesses passed; model invocation
remains untested. The [remaining-work record](remaining-work.md) separates actual
legacy state, archival evidence, implementation scope and publication work.

## Amendment review and candidate reconciliation

Independent documentation/asset review found no false release or qualification
claim. The migration cutover checklist now requires validated new documents and
exact source-section mappings. Archive verification now uses explicit runtime
refusals; eight normal/optimized Python cases pass, and restoring captured Git
refs and 1,850 baseline files into an empty repository passes.

The seven OW-WAR-0068 delivery mismatches are now recorded as a new candidate
attempt. The prior manifest is retained byte-for-byte; each changed entry records
the known authorization contract digest and the prior file digest as an input.
Independent review confirmed this provenance. Candidate recording is not reused
verification, contract fulfillment or resolution. The corpus now reports two
errors, both signed correction chains: README and xtask. Exact human requests
are in the [amendment review packet](amendment-review/README.md).

Parser implementation has started in its own worktree. Its test and review
results belong to OW-WAR-0075; earlier documentation witnesses do not establish
parser completion. No main-branch publication or SAS acceptance has occurred.
