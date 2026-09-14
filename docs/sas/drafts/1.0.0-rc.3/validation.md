# RC.3 amendment validation

Observed 2026-09-14 in isolated checkout `/mnt/4tb/tmp/ow-sdk-standard-amendment`,
branch `codex/sdk-standard-amendment`, base `16f0559db9a267c2123b334224c1b1ab5e645b03`.
Scope: authored documentation and compatibility fixtures. No production SDK,
compiler integration, signed adoption, Warrant verification or release is claimed.

## Observed

- `python3 docs/sas/drafts/1.0.0-rc.3/check_draft.py --write-manifest` and the same
  command without the write flag: passed. Review manifest derived from actual
  files; normative companions, local links and declared source-unit dependencies
  resolve. The manifest is generated inventory, not approval.
- Requirement index: 105 unique IDs; all 92 RC.2 IDs retained; thirteen new IDs
  RQ-122–134 add SDK, skills, integration, webapp, human signature, hardening,
  context views, shared work, retention and stop/change requirements. Changed titles/owners are intentional successor meanings.
- Roadmap: all 11 RC.2 feature IDs, all 56 case IDs and all 18 planned Warrants
  OW-WAR-0074–0091 mapped. No record claims a historical state change.
- Sixteen RC.2 example files preserved byte-for-byte, including canonical
  preimages and packet blobs. The retained example audit passed: three Markdown
  sources, ten selected units, six package files. This proves only the fixed
  reference corpus, not a production parser or semantic compiler.
- Before the prompt-only clarification, disposable copies observed four refusals: changed source bytes without inventory
  refresh (`source inventory/hash mismatch`); missing case despite refreshed
  hashes (`case coverage`); missing dependency target (`missing dependency`);
  changed retained example despite refreshed hashes (`historical fixture changed`).
- Before the prompt-only clarification, the existing SAS diff command, run with the source-set implementation build,
  exited 0: 38 additions, 61 retitle warnings, zero errors against the configured
  historical baseline. This is a requirement-index check, not semantic review or
  acceptance. It does not compare every clause automatically.
- `git diff --exit-code 16f0559 -- docs/sas/drafts/1.0.0-rc.2 docs/warrants
  .claude/skills AGENTS.md CONTEXT.md docs/SKILLS.md`: exit 0. Those historical and
  installed guidance paths remain unchanged in this drafting checkout.

Current consolidation refusal checks used disposable copies: removing SDK-24
with refreshed inventory failed `SDK case coverage`; removing CTX-08 failed
`context case coverage`; editing the stop companion without inventory refresh
failed `source inventory/hash mismatch`. The original source set was untouched
by those mutations. SDK-01–SDK-24 and CTX-01–CTX-08 are now checked explicitly.

## Manual consistency review

Ownership table and SDK contract distinguish single-document parsing/record checks
from provider-owned source resolution, closure, projection, packaging and budgets.
Integrity-only validation explicitly reports semantic coverage as not evaluated.
Phase plan moves CLI parity to Phase 1, compiler interoperability to Phase 2,
real webapp and first-party/user evidence to Phase 3, and hardening/Stable promotion
to Phase 4. First-party integration inventory must be confirmed before freezing
Phase 3; it cannot silently omit a requested app or claim unavailable support.

Ungated prompt-only work needs no prior OpenWarrant authorization or signing.
Qualification-only findings remain separate from execution, while explicit Warrant
start/signoff gates are enforced. Later review
can qualify prototype results; explicit pre-work conditions cannot be backdated.
Release acceptance can bind several exact subjects in one human signing ceremony,
while each qualification still requires its own evidence and profile conditions.

The consolidated source set contains 34 files and 105 requirement IDs. Nine
prototype/release prose cases, SDK-09–SDK-24, eight CTX cases and the stop scenario
matrix cover prompt-only work,
later qualification, joint-project timing, batch acceptance, full completion,
tracker handoff/recovery, explicit action gates, every work-stop scope, configurable
brief pointers, shared contracts, source-owned context views, evidence deletion,
work/harness updates and starting the next Warrant. Documentation and
retained-example checks passed; these new behavior cases are not executable tests
and do not establish that the current CLI/harness supports the proposed flows.

Agent authorization/completion is separated from human acceptance and qualification.
Existing RC.2 wire identifiers and example bytes remain compatibility inputs. New
agent acts and RC.3 assurance have separate proposed identifiers. A human signature
alone does not establish the mark; an agent signature never replaces the required
human signing act. Prototype mode does not permit effective-policy self-escalation.

War-skill adaptations are attributed to Matt Pocock, with recorded upstream source,
artifact mappings, context pointers and observed completion criteria. The exact
upstream MIT license at recorded revision 3cca18b was read during this amendment;
this is not an audit that all existing distributions carry correct notices.

## Remaining work and limits

No deployed behavior changed. Agent-act JSON Schema/codec/signing fixtures, SDK
APIs/CLI, actual skill migration, provider integration and workflow transports
remain implementation tasks in their assigned phases. Existing installed signing
and skill rules still apply until scoped migration. These document checks do not establish source-set acceptance, an independent
verifier disposition, a commit review, publication or a public release. Commit
and publication evidence belongs to the resulting Git history and review receipt.

Review/adopt the complete successor subject through the applicable authority
process. Rebase or succeed affected Warrant scope explicitly; this amendment does
not reuse OW-WAR-0074's signature for RC.3. Preserve its existing uncommitted
implementation and human corrections in their original checkout.
