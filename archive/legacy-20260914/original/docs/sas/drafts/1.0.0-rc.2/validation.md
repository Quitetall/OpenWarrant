# RC.2 draft validation

Date: 2026-09-14. Checkout: `feat/battery-split` at
`f22ef2f7282e8b5c72f2c4323b300f3b5e16102c`. This report concerns the unaccepted
RC.2 source set, not a production compiler, Warrant verification, or release gate.

## Observed checks

- `python3 docs/sas/drafts/1.0.0-rc.2/examples/check_examples.py`: exit 0. Three
  authored Markdown examples, ten selected required units, six package files;
  exact binding slices, dependency definition, optional-reference labeling,
  source hashes, stored canonical preimages, and size accounting agree.
- JSON Schema Draft 2020-12 schema validation and reference-packet validation:
  passed with Python `jsonschema`. This establishes structural types, not trust,
  semantic coverage, or actual compiler behavior.
- Copied only the package into a temporary directory. An isolated Python child
  read all required units/fixture bytes there, without reading the repository or
  contacting a service. This demonstrates reference-package readability, not a
  sandbox/network-enforcement product feature.
- Altered one blob in a disposable copy: the example audit exited 1 with
  `file mismatch`. The positive audit is capable of refusing corrupted evidence.
- Parsed TOML in both normative source documents and all literal JSON examples.
  The SAS and format contract include explicit dependency metadata for context
  projections. General production framing/condition tests remain planned.
- Requirement inventory: 92 unique IDs; all 61 active-baseline IDs retained;
  all 53 interview decisions, 11 features, and 56 test-case IDs accounted for.
- `./target/debug/war sas diff docs/sas/drafts/1.0.0-rc.2/WAR_Software_Architecture_Specification.md --json`:
  exit 0, 31 pass, 61 warnings, zero errors. Additions and retitles are intentional
  architecture changes. The command checks legacy requirement-index stability,
  not all prose, links, runtime behavior, or human acceptance.
- Recorded SHA-256 values for 26 protected/history/unrelated files still match:
  active SAS, original revision record, ADR atoms, SAS projections, AGENTS.md,
  and the existing README edit. No production source, signature, or registered
  Warrant state was changed by this task.
- All 45 explicit normative-source dependency targets resolve; 99 relative
  Markdown links resolve. Source and format have unique structural unit IDs.
  The source-set manifest binds 23 exact files.
- The active SAS loader still sees exactly one top-level Markdown file in
  `docs/sas/`. RC.2 lives in its own draft subdirectory.

Expected JSON bytes were produced using the existing repository's
`openwarrant_compiler::canonical::to_canonical_bytes` through a temporary helper,
compiled offline with Rust `1.97.1 (8bab26f4f 2026-07-14)`. No placeholder
canonicalizer or new production RC.2 compiler was introduced.

## Independent specification review

The attached LAMU `review_diff` call returned `Transport closed`. A fresh local
LAMU MCP connection successfully ran external review with automatic model
selection. The first verdict was NEEDS CHANGES:

- Packet structural types and a full example were missing at that review point.
  Added a concrete packet, JSON Schema, and explicit object/array definitions.
- Budget check/publication order was insufficiently explicit. It now renders
  within resource limits, checks final entry size, and emits no package on refusal.
- The claimed circular manifest digest was a false positive: the reviewed text
  already excluded manifest.json from its own files list. That rule was made
  explicit with SHALL NOT; no digest algorithm was changed to fix a nonexistent bug.

A follow-up review was requested after these changes but returned no verdict
within 240 seconds; its local MCP process was stopped. Final external approval
is therefore unestablished. Local checks verified the fixes, source dependencies,
packet schema and examples. External document review is not a Warrant gate
clearance or a human acceptance record.

## Existing repository failures

`./target/debug/war check --generated --json`: exit 2, **714 pass, 88 warnings,
5 errors**. These match the preexisting unsigned README correction and derived
status drift:

- `correction.new-digest-mismatch` for OW-WAR-0062 / D-003.
- Three `corpus-status.drift` errors for Markdown/JSON/HTML projections.
- One `corpus-pending.drift` error.

No generated records were rewritten to conceal those failures. This source set
does not claim a green repository. The full `cargo xtask gate`, production
T01–T56 tests, agent efficiency trials, live workflow proof, signatures, adoption,
and Stable release checks were not performed in this documentation task.

## Later consolidation and commit review

The record above preserves the initial candidate work. The later commit
2bca43c received external review_commit PASS WITH NITS. Verified findings and
clarifications are recorded in the roadmap validation record. The owner then
confirmed binding canonical definitions/advisory usage and successor preparation
for the three signed legacy Warrants. Those decisions are now consolidated in
the candidate, with source-set manifest SHA-256 `201b707e9657a3c6674ea113a018a34a54b5ab425f37c417c8968af9f4d2cf31`.
No production conformance or human acceptance is implied by those document edits.
