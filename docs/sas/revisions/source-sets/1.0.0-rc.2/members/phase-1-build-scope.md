# RC.2 testable build scope

Status: implementation plan for [SAS 1.0.0-rc.2](WAR_Software_Architecture_Specification.md),
2026-09-14. Planning and reference-example work only. No production RC.2 parser,
compiler, or workflow is claimed implemented by this document.

## Phase boundary

Phase 1 delivers directly callable primitives and a small executable test driver.
Phase 2 composes them into the supported CLI/compiler with ergonomic authoring.
Phase 3 implements stateful workflows, starting with Knowledge Fabric Compiler.
The compiler library can contain primitives in Phase 1; the distinction is tested
pieces first, supported composition second, not a prohibition on its crate name.

Terminology follows normative SAS §3 definitions. New modules use source unit,
snapshot, projection, packet, observation record and check definition. Existing
legacy types keep their original names and bytes; any adapter mapping is explicit.

First Phase 1 slice: **F01, parse a minimal Warrant and reject malformed framing**.
Do not start with a TUI, database, agent launcher, or whole-corpus migration.
Each feature has one owner module, a narrow public boundary, and fixtures proving
behavior through that boundary. Proposed module names below may change without
changing wire semantics. Approved source/digest semantics require an ADR to change.

## Existing implementation to reuse or adapt

Inspected checkout: `f22ef2f7282e8b5c72f2c4323b300f3b5e16102c`, branch
`feat/battery-split`. Paths below exist at that checkout; future entry points are
explicitly marked proposed. Re-inspect before implementation because the working
tree also contains unrelated edits.

| Existing module | Useful foundation | RC.2 gap / treatment |
| --- | --- | --- |
| `crates/openwarrant-core/src/frontmatter.rs`, `structured.rs`, `manifest.rs` | Deliberate legacy grammars, role/ordinal validation | Keep legacy grammar unchanged; separate TOML document adapter |
| `crates/openwarrant-core/src/identity.rs`, `contract.rs` | UUID/alias distinctions, immutable contract basis | Preserve old identities; support local document IDs and explicit new digest domains |
| `crates/openwarrant-core/src/sections.rs`, `normative.rs` | Heading navigation and extracted-sentence views | Navigation only; stable marker units and dependency closure are new |
| `crates/openwarrant-core/src/context.rs` | Context roles, provenance, omissions, trust, conflicts | Extend through a versioned IR; do not silently reinterpret old context schemas |
| `crates/openwarrant-compiler/src/lower.rs`, `ir.rs` | Explicit CompilationBasis and lowering | Add new document/assembly IR boundary while preserving old canonical outputs |
| `crates/openwarrant-compiler/src/canonical.rs`, `digest.rs` | RFC 8785 canonicalization and domain-separated hashes | Reuse tested functions; register RC.2 domains explicitly |
| `crates/openwarrant-compiler/src/dispatch.rs` | Pure `compile_dispatch` and required-source checks | New unit selection, role packets, and final-entry budget accounting |
| `crates/openwarrant-cli/src/context_select.rs`, `dispatch.rs`, `perform.rs` | Existing repository selection and performer handoff | Required atoms currently whole; context emission optional; not portable RC.2 proof |
| `crates/openwarrant-core/src/verification.rs`, `independence.rs`, `gate_run.rs`, `attestation.rs` | Bounded evidence, independent roles, truthful verdicts | Reuse semantics; keep new record eligibility separate from issuing marks |
| `crates/openwarrant-cli/src/bundle.rs`, `export.rs` | Verification and preservation exports | Do not equate existing export with the new task package |

## Feature inventory and runnable surface

Every proposed command in this table is a **future acceptance command**, not an
installed command or a check claimed to have run. The new lightweight driver
`rc2_probe` belongs under `crates/openwarrant-compiler/examples/rc2_probe.rs` and
calls library functions directly. It reads fixture inputs, emits structured
results, and checks no behavior hidden only in CLI state. It is not the production
compiler or a second implementation of the algorithms.

Each driver case SHALL exit 0 when its observed result matches its expected
success/refusal and nonzero on a mismatch. Thus an expected refusal can pass a
test without presenting the underlying invalid input as valid. No no-op cases,
placeholder outputs, or all-tests-skipped success are permitted.

| ID | Primitive / proposed ownership | Direct operation and output | Dependencies | Required cases |
| --- | --- | --- | --- | --- |
| F01 | Document framing/parser, core `document.rs` | `parse_document(bytes)` → typed metadata, exact units, spans | None | T01–T06 |
| F02 | Document semantics/extension validation, core `document.rs` | `validate_document(doc)` → validity plus diagnostics | F01 | T07–T10 |
| F03 | Snapshot/capture and reference resolution, compiler `source_set.rs`; thin I/O driver | `capture_sources` then `resolve_refs` → immutable lock table and bound references | F01–F02 | T11–T15 |
| F04 | Conditions, core `condition.rs` | `evaluate_condition(condition, inputs)` → TRUE/FALSE/UNKNOWN plus reasons | F02 | T16–T20 |
| F05 | Dependency closure/conflicts, compiler `context_graph.rs` | `select_units` → exact membership, reasons, omissions, blockers | F03–F04 | T21–T26 |
| F06 | Master/role IR and rendering primitives, compiler `packet.rs` | `assemble_master`, `project_role`, `render_entry` → common IR and faithful views | F05 | T27–T31 |
| F07 | Offline package writer/reader, compiler `package.rs`; explicit filesystem shell | `build_package`, `check_package` → bytes, manifest/root, integrity/coverage report | F06 | T32–T37 |
| F08 | Budget/cache primitives, compiler `budget.rs`, `basis.rs` | `account_entry`, `basis_key` → estimates, refusal, invalidation identity | F03/F06 | T38–T41 |
| F09 | Supplied record/readiness/assurance facts, core `record.rs` | `check_records`, `evaluate_readiness`, `evaluate_assurance` → per-condition results | F02–F03 | T42–T48 |
| F10 | Legacy import and successor-history mapping, compiler `legacy_import.rs` | `import_legacy` → original bytes, mapping, unsupported facts; lineage check | F03/F09 | T49–T53 |
| F11 | Field-authoring primitive, core `document.rs` | `author_document(fields)` → source bytes, then F01/F02 validation | F01–F02 | T54–T56 |

For each Fnn, run:

```bash
# FUTURE: available only after the driver and corresponding primitive exist.
cargo run -p openwarrant-compiler --example rc2_probe -- --feature F01 --fixtures conformance/rc2
# FUTURE: all features must report observed cases; absent case is an error.
cargo run -p openwarrant-compiler --example rc2_probe -- --all --fixtures conformance/rc2
```

Each feature also has integration tests through its public boundary. Test output
must name case ID, exact fixture digest, observed result/diagnostic, and whether
expectation matched. Timing/count assertions alone are not semantic evidence.

## Acceptance matrix

All T cases below are **NOT IMPLEMENTED / NOT RUN for RC.2 production code** at
planning time. Existing tests may be reusable but do not count until mapped and
executed against the new contract. Case inputs and expected outputs must be
checked in before the corresponding feature is called complete.

| Case | Input or change | Required observation |
| --- | --- | --- |
| T01 | Minimal Warrant example | Parse title, kind, identity and three exact unit ranges; no permission inferred |
| T02 | Duplicate header key / missing closing delimiter | `source-invalid`; no partial valid document |
| T03 | Marker-like text inside fenced code | Remains literal text; no phantom unit |
| T04 | Duplicate unit ID / malformed marker | `unit-duplicate` / `source-invalid` with location |
| T05 | UTF-8 non-ASCII, LF and CRLF sources | Exact slices and original digests retained; offsets count bytes |
| T06 | Invalid UTF-8, unclosed fence, oversized source/header | Named refusal, no panic, no partial output |
| T07 | Missing outcome / empty scope / unknown core field | Invalid document; distinguish each cause |
| T08 | Optional unknown extension, then required unknown extension | Preserve/diagnose first; `extension-required` blocks second |
| T09 | ADR/context kinds, and wrong required units | Kind-specific validation; proposed ADR grants no accepted authority |
| T10 | Minimal valid draft with unresolved context | Validity valid; complete task context/readiness blocked where required |
| T11 | Warrant + ADR + opaque fixture capture | Every source pin resolves to bytes and correct unit spans |
| T12 | Renamed heading, same ID | Pointer still finds same unit ID at new snapshot; old approval not reused |
| T13 | Missing target, ambiguous identity, bad digest | Specific target/digest diagnostic; no complete packet |
| T14 | `../`, absolute path, symlink, source changed during capture | Refuse path/snapshot violation; preserve old output |
| T15 | Reordered input enumeration | Canonical source lock and output bytes identical |
| T16 | Stage/subsystem/path positive and negative cases | Exact anchored matching, no filesystem-dependent results |
| T17 | Missing stage/subsystem/path for required rule | UNKNOWN + rule and dependency inclusion; condition remains visible |
| T18 | Explicit empty list, false AND unknown, all true | FALSE, FALSE, TRUE respectively; field reasons retained |
| T19 | Unsupported operator, empty condition, malformed glob | `condition-invalid`, never UNKNOWN fallback |
| T20 | `src/**`, `src/*.rs`, case difference, Unicode path | Published segment semantics including zero-segment `**` |
| T21 | Rule requires definition, exception, schema in other units | Full transitive closure, exact wording including no-keyword text |
| T22 | Duplicate roots and shared dependencies | One copy per exact unit; all distinct inclusion reasons retained |
| T23 | Self-cycle and multi-unit cycle | Terminates; complete consistent closure plus cycle warning |
| T24 | Missing cycle member, explicit conflict, required competing revisions | Refusal instead of selecting arbitrary winner |
| T25 | Governing pointer outside the Warrant | Evaluated from captured source set; no need to duplicate it in Warrant |
| T26 | Optional unavailable target, then required target | Labeled absent optional reference; required case blocks |
| T27 | Master assembly then each of four role views | Same source basis and common constraints; role-specific brief/output |
| T28 | Background summary from tainted/restricted sources | Inherited labels/provenance; no replacement of exact rule |
| T29 | Performer suggested verdict in inputs | Labeled claim/background; cannot become verification disposition |
| T30 | Full evidence behind displayed excerpt | Excerpt labeled; full required bytes accessible in package |
| T31 | Render human and machine views | Same membership/basis; exact unit slices, no paraphrase or omitted exception |
| T32 | Read packet after original repository unavailable, network disabled | All required context readable using only package |
| T33 | Delete required blob / alter one byte / mismatch range | Package check refuses each case |
| T34 | Change blob and all simple hashes but omit declared required unit | Semantic membership check refuses; hash consistency alone insufficient |
| T35 | Package traversal, symlink, extra file, duplicate normalized path | Safe refusal before accessing/writing outside destination |
| T36 | Rebuild identical request twice, including fresh output directory | Identical per-file bytes and root digest; no timestamp/path noise |
| T37 | Full-source access denied though selected slice looks public | `access-denied`; no disallowed source blob leaks |
| T38 | Required entry just below, at, and above token estimate budget | Inclusive limit passes; above refuses; no binding content removed |
| T39 | Large render framing plus small selected text | Estimate covers final ENTRY bytes, not only selected text |
| T40 | Source/policy/role/selection/version/budget changes | Relevant basis key changes; old packet remains intact |
| T41 | Previously omitted routing source becomes applicable | Invalidate and include newly required rule |
| T42 | Claim says all checks passed, no observations | Cannot establish verification or assurance eligibility |
| T43 | Unavailable/error check versus observed violation | UNKNOWN versus FAIL; required result blocks eligibility |
| T44 | Performer as verifier, missing isolation evidence | Ineligible/unknown with reason; cannot clear independent requirement |
| T45 | Automated disposition / fabricated human-kind field | No human acceptance/mark; authenticity must be externally established |
| T46 | Trusted human acceptance + qualifying exact evidence, then changed candidate | Eligible first; new candidate cannot reuse old eligibility |
| T47 | Later-added fixtures versus fixtures-before-work profile | Final-result baseline can qualify; stricter history claim remains unmet |
| T48 | Delegated SAS adoption / indirect effective-policy change | Delegation never earns mark; effective-policy edit still requires human authority |
| T49 | Legacy signed fixture import, including a resolved Warrant | Preserve bytes, digest domains, UUIDs, original states and signature subjects; report legacy resolution without automatically minting RC.2 acceptance or qualification |
| T50 | Legacy unsupported semantics / missing historical artifact bytes / unproved acceptance meaning | Diagnostic; cannot claim complete preservation, infer a mark or fabricate history |
| T51 | Successor Warrant changes old delivered pathname | Both versions recoverable; lineage, no blanket old-record correction fanout |
| T52 | Unrelated unresolved Warrant, then relevant broken prerequisite | First does not block; second does |
| T53 | Export/import/export new and legacy preservation payloads | Same semantic identity and original immutable bytes |
| T54 | Author minimum fields through typed input | Valid source round trip; no hand-maintained digest/index fields |
| T55 | User text contains delimiter, marker, or shell syntax | Header escaping or explicit refusal; no field injection or execution |
| T56 | Edit one field and cancel interrupted authoring | Valid new draft or unchanged prior file; no damaged partially written source |

Each control needs both admitted and refused input. Where a case names multiple
mutations, test each; passing one cannot stand in for the whole row. Source
fixtures must also include real retained legacy records copied into a disposable
test corpus, with private material excluded and historical identity clearly labeled.

## Work order and phase-exit gate

Order: F01 → F02/F11 → F03/F04 → F05 → F06 → F07/F08 → F09 → F10.
F09 may start after F03; its interfaces must not couple parsing to a running
service. Each slice ends with its direct example, integration tests, refusal
case, and recorded output. Independent review remains separate from performer
self-checks and does not get a fabricated Warrant verification disposition.

Phase 1 exit requires:

- All F01–F11 callable without a service or model; all T01–T56 observed, none skipped.
- Production primitives, not a fixture lookup table, produce expected behavior.
- Contract fixtures and snapshots agree with the adopted source-set manifest.
- No regressions in legacy parser/canonicalization/signed-payload behavior.
- Focused tests plus the repository's required aggregate gate, on its pinned
  toolchain, pass for the implementation change. Relevant unresolved integrity or
  authority blockers remain reported separately; a baseline failure is not a pass.
- A reviewer can reproduce each direct-driver result from documented inputs.

Existing contributor gate is `cargo xtask gate` on Rust **1.97.1**. It includes
planted checks and may touch the corpus; run in an isolated implementation checkout
with the actual authorized basis. Do not edit AGENTS.md or signed deliveries to
force this planning checkout green. Current unsigned README correction is separate
repository bookkeeping, not permission to rewrite historical records.

## Phase 2 scope and Stable gate

The production CLI command names are reserved here under a distinct namespace
so old `war compile` behavior is not silently changed:

```text
war document new/edit/check       # manual fields; --json for noninteractive use
war context capture/master       # explicit root/source list, then assembly
war context project              # role/stage/options over captured inputs
war context export/check         # directory packet and offline validation
```

Exact flags/help are delivered in Phase 2, generated from one typed request model.
Noninteractive mode must never prompt; stdout contains one JSON report, diagnostics
on stderr, exit 0 for successful operation and nonzero for refusal/failure. A
check of a valid draft reports validity separately from readiness. Interactive
mode selects a field, edits it, shows validation, and saves atomically; cancellation
preserves the prior draft. Automated tests exercise both modes without an AI call.
CLI spelling is reference-tool surface, not part of basic document compatibility.

Phase 2 release gate requires T01–T56 through the integrated path, exact rendering
and JSON golden vectors, fresh offline consumers on Linux/macOS, reproducible
capture/project/export, resource-limit and write-failure cases, plus the repository
aggregate checks and reviewed docs/examples. Phase 1 does not claim these command
surfaces exist. The standard, format schemas, fixture version and compiler build
are published together; no Stable tag before the required observed results.

Deterministic context benchmark: at least 12 fixed tasks across SAS, ADR, Warrant,
and evidence sources; 100% expected required-unit coverage; zero extra authority
or unsupported completion claims; zero binding-byte changes. Include at least
three tasks needing definitions/exceptions without SHALL, three UNKNOWN-condition
tasks, and three expected refusals (categories may overlap).

Agent evaluation: compare full-source, existing keyword-only projection, and RC.2
packets on those same tasks, three trials per runnable task per representation,
with identical recorded agent/harness versions and limits. Report failures and
refusals, total model tokens (including retrieval), elapsed time, human assistance,
and task success. Model availability is not a dependency for compilation or its
release gate. No fixed speedup is promised; an efficiency claim requires measured
improvement without increased binding-rule violations, and includes sample counts
and uncertainty. Offline deterministic coverage is the mandatory compiler gate;
agent trials qualify performance claims, not basic format validity.

## Deferred to Phase 3

Knowledge Fabric Compiler transport, local database/service, TUI/browser, authenticated
sessions, policy administration, agent launcher/skills, worktree writer fencing,
hotline, retry/rebuttal budgets, verification execution, human acceptance/mark
issuance, merge/deployment, hosted multi-user operation, and named sibling adapters.
Each requires its own runnable acceptance plan before implementation. F09 checks
supplied facts; it is not a substitute for those real-world controls.

## Runnable now: reference-example audit

```bash
python3 docs/sas/drafts/1.0.0-rc.2/examples/check_examples.py
./target/debug/war sas diff docs/sas/drafts/1.0.0-rc.2/WAR_Software_Architecture_Specification.md --json
```

The first checks authored example syntax, exact slices, expected membership, and
package byte integrity. It is intentionally not a general parser/compiler or an
independent verifier. The second checks the legacy requirement index diff only.
Neither executes the planned production conformance suite.
