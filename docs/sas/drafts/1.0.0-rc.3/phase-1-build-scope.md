# Phase 1: testable SDK and standard scope

Normative companion to SAS RC.3. All entries are planned, not production evidence.
The standard and SDK are OpenWarrant's deliverables. LAMU owns semantic compiler
implementation. The direct SDK boundary precedes CLI exposure for each feature.

## Inventory and cases

| Feature | Public seam / artifact | Positive and refusal proof |
| --- | --- | --- |
| S01 Documents | `parse_document`, `validate_document`, `unit` | T01–T10 from RC.2: exact minimal document and byte spans, UTF-8/CRLF, fences, missing/duplicate fields/units, malformed framing, limits, kind rules, optional/required extensions and valid-unready draft |
| S02 Authoring | `author_document`, `edit_document` | T54–T56: valid typed-field round trip, preserved untouched bytes, delimiter/marker/shell text escaped or refused, cancelled save preserves old source |
| S03 Standard codecs | Canonical JSON/digests, pointer/condition syntax, source/packet/record types | Independent canonical vectors, matching declared ranges; refuse duplicate fields, wrong schema/domain/digest/range, unsafe target syntax, unsupported operator; no resolution inferred from syntax |
| S04 Record evaluation | `check_records`, `evaluate_readiness`, `evaluate_assurance` | T42–T48 plus agent-act examples: claims without observations, UNKNOWN vs FAIL, self-verifier, missing isolation, spoofed human, changed candidate, true fixture timing, delegated governance and forbidden effective-policy escalation |
| S05 Integrity and preservation | Supplied-byte manifest checks, explicit legacy import/export, successor lineage | T49–T53 and integrity part of T33/T35: preserve original bytes/states/signature subjects; refuse corrupt/missing/escaping bytes, unsupported meaning; no inferred RC.3 mark or blanket old-file correction fanout |
| S06 CLI parity | Explicit I/O shell over S01–S05 | Each operation runnable through typed library input, direct driver and noninteractive CLI; one JSON result, documented nonzero refusal, no prompt, no hidden signing/model call; write failure/cancel preserves prior destination |
| S07 Consumer contract | Typed adapter interfaces and reference consumer | Serialize/decode declared profile types; mismatch/unsupported/unavailable cases explicit; integrity-only result cannot masquerade as semantic coverage; no provider required for local document APIs |
| S08 Adapted skills | War skill contracts and evaluation fixtures | Each declared invocation produces correct typed artifact, preserves settled decisions and unknowns, loads required context, records actual acts; refuses fabricated authority, malformed outputs and unsupported profile |

S03–S08 additional acceptance scenarios (not aliases for RC.2 tests):

- SDK-01: stable literal canonical vector passes; altered byte or wrong domain fails.
- SDK-02: available local unit resolves exactly; unresolved external pointer stays
  syntactically valid and explicitly unresolved. SDK performs no network request.
- SDK-03: permitted agent authorization/completion is attributable with no mark;
  human-kind impersonation, missing policy or forged attestation does not establish
  permission. Unsigned draft remains distinct from an authenticated act.
- SDK-04: independent passing findings alone and secure human signature alone each
  fail qualification; exact required evidence plus secure human acceptance can be
  eligible. An agent-controlled human key is not evidence of a human act.
- SDK-05: noninteractive CLI parity covers every scoped operation and both success
  and refusal; failures/cancellation leave previous file bytes unchanged.
- SDK-06: provider version mismatch, malformed response and wrong basis refuse;
  fake-provider success reports only transport proof, not integration qualification.
- SDK-07: war-grill records questions/answers and drafts the bounded outcome;
  war-spec emits a valid Warrant/ADR proposal; stage/review/map cases use their
  declared artifacts; no free-form HTML or tracker issue substitutes for the record.
- SDK-08: a proposed policy change remains a proposal; prototype configuration
  cannot authorize its own escalation or waive the mark's human signature.
- SDK-09: human/agent unverified completion without a prior OpenWarrant authorization
  or signature remains representable; missing approval is a qualification gap, not
  a blanket execution refusal. Null-policy unverified agent completion makes no
  authority claim. Full no-prompt execution is observed in Phase 3.
- SDK-10: later final-result review can qualify exact prototype results without
  pre-work approval; a later signature cannot satisfy a condition explicitly
  requiring approval before work. Historical timing remains unchanged.
- SDK-11: joint-project start approval, independent starts and mixed stage conditions
  produce profile-specific findings. An unmet qualification-only condition leaves
  unverified execution available; an explicit Warrant action gate blocks its named
  action. Missing runtime access remains a separate fact.
- SDK-12: one securely signed review manifest covers multiple exact result subjects;
  each eligible member qualifies separately. Altered, added or uncovered subjects
  and members missing required evidence refuse the claimed qualification. Plain
  release-name text and a legacy single-result signature cannot imply batch coverage.
- SDK-13: unsigned unverified work and verified work share the same complete state.
  Supplied tracker records count both as completed and expose qualification
  separately. Missing human acceptance cannot demote unverified completion to pending.
- SDK-14: work-stop response carries configured safeword, exact result/event/scope,
  pointer to a tracker-produced overview with revision/as-of identity, notes,
  document trail and next steps. A link to a view missing the named completion,
  an unsupported tracker-origin claim, or a word alone cannot establish a confirmed handoff.
- SDK-15: delivery/synchronization state is separate from completed work. Repeated
  delivery uses the same completion identity without changing its result. Actual
  tracker write/read recovery and duplicate prevention are exercised in Phase 3.
- SDK-16: explicit start/signoff gates retain exact action scope and prerequisites;
  unverified mode cannot bypass them. Absence of a gate does not invent one.
- SDK-17: feature, integration, Warrant and program work stops all produce a scoped
  response. Feature completion does not finish its parent; a harness interruption
  cannot emit the completion signal. Mentioning the safeword in chat is inert.
- SDK-18: minimal and richer response profiles reference the same deterministic
  overview and event. Required state, notes, document trail and next steps remain
  available through pointers; shortening chat cannot fabricate metrics or delete
  required information. Actual rendering/metrics and token measurements are Phase 3.
- SDK-19: supplied context-view metadata distinguishes document maturity, work
  completion, qualification and code-derived versus approved contract facts.
- SDK-20: typed task/context requests preserve Warrant, participant bases, role,
  limits and selection inputs; a query string is not accepted as authority or
  proof of complete context. SDK validation performs no database query.
- SDK-21: shared-contract references keep one revision and each participant's
  scope; mismatched local copies, unsupported required conditions and invalid
  timing refuse the associated claim without inventing a universal start gate.
- SDK-22: supplied deletion/availability records preserve historical signatures
  and identify affected assurance; absence, deletion, retained exact duplicate
  and restored evidence yield distinct findings. No SDK call deletes storage.
- SDK-23: supplied stop records preserve class, scope, cause and observed worker
  state; a timeout cannot establish stopped writers, and partial completion
  cannot complete a parent. Replayed event identity does not mean a new completion.
- SDK-24: work-change and harness-update records retain old/new basis, explicit
  choice, affected scope, limits and required resume evidence. Missing writer
  fencing or required context cannot establish readiness to resume.

[Prototype and release examples](prototype-and-release-cases.md) make these cases
concrete. They are expected behavior, not observed runtime results.
The [context companion](context-views-and-shared-work.md) and
[stop contract](work-stop-contract.md) add required provider/workflow scenarios.
Phase 1 checks their supplied representations; actual storage, cancellation,
querying and rendering remain integration or workflow work.

The [SDK contract](sdk-contract.md) fixes boundaries and the new agent-act envelope.
Wire schemas and executable expectations must be reviewed and checked in before
their production behavior is called complete. Prepare one vertical slice at a
time; these case descriptions are scope, not a pretense that all tests exist.

## Reuse and ownership

Reuse `openwarrant-core` types, original source parsers where their dialect matches,
and the tested canonicalization/digest implementation. Preserve legacy meanings.
SDK module/dependency layout may be refined within the approved boundary; do not
write a second TOML or canonical JSON engine merely to preserve an incidental
crate layout. Any dependency change receives compatibility/license review.

One Warrant owns each implementation slice/worktree; shared exports and CLI are
serialized at integration. Future Rust example name: `sdk_probe`. It must dispatch
real public operations, never a fixture-name answer table. Proposed invocation:

```text
cargo run -p <sdk-package> --example sdk_probe -- --feature S01 --fixtures <path>
```

This is intentionally a proposed form: package/path/flags are finalized with the
first implementation Warrant. Current repository does not supply this command.
Each observed result records case ID, input/build/profile identity, actual output
or diagnostic and expectation match. Absent cases fail the phase-exit inventory.

## Excluded and reassigned

Cross-source capture/resolution, condition evaluation, dependency closure, master
assembly, projection, context ranking, package construction and budget/cache
orchestration are external compiler work qualified in Phase 2. SDK may represent
and validate their input/output structure, not secretly implement them in helpers.
Database, service, TUI/webapp, orchestration, signing UI and merge/deploy enforcement
belong to workflow apps in Phase 3. No assurance runtime is implied by S04's pure
evaluation of supplied facts.

The complete RC.2 F01–F11/T01–T56 reassignment is retained in migration-map.md and
roadmap.json. No case disappears merely because ownership changed. Old receipts
remain evidence only for their original code, fixtures and subject.

## Exit

Every scoped primitive and S/SDK/T assignment has callable library and CLI proof,
admitted/refused inputs, named diagnostics and bounded claims. Linux/macOS checks,
legacy compatibility and required repository gates pass on declared toolchains.
Independent review is separate from performer test logs. No required UNKNOWN is
counted as a pass. SDK users can author/inspect documents offline without LAMU;
full compiler-profile support is not claimed until Phase 2 integration evidence.
