# OW-WAR-0075 parser and validator candidate

Status: implemented candidate, unverified. No signature, independent disposition,
SAS adoption, legacy resolution or common assurance mark is claimed here.
Scope: public document parsing/validation/unit-access seams from RC.3 F1–F4.
The copied candidate uses source-set digest
`217bea8600fd34405b0a521454b84bdffd0c06f05a9bd5b15a2b18168e47be7a`.
The implementation worktree is `codex/ow-war-0075-sdk`; copied draft records were
already dirty before code work. Historical parsers and digest domains are unchanged.

## Implemented behavior

- `document::parse_document(bytes, Dialect, ParseLimits)` explicitly selects RC.3
  footer or retained RC.2 header grammar. The document borrows original bytes and
  stores immutable unit ranges. Neither normalization nor source replacement occurs.
- Plain/wrapped footers, compact/repeated-table dependencies, fenced inert markers,
  LF/CRLF and UTF-8 byte offsets are handled without interpreting rendered Markdown.
- `validate_document` checks declared fields, identity/state/title, nonempty kind
  requirements, local pointer/dependency/conflict/condition syntax and extensions.
  It returns document validity separately from semantic extension support;
  context resolution and readiness are explicitly not evaluated.
- `document.unit(id)` returns the exact original source slice. Required external
  sources may be unresolved in a valid document; no readiness or permission follows.
- Resource checks refuse rather than truncate. Default source/metadata/unit bounds
  are 8 MiB/1 MiB/65,536. Metadata uses an additional conservative depth bound of 64,
  configurable from 1 through 128: dotted keys add to enclosing container depth;
  table-header segments count twice to cover prior array-of-tables ancestors.
  This is an implementation resource bound, not a new source-schema field.

## Dependency and safety decisions

The SDK permits a proper TOML parser. Workspace `toml` accepts TOML 1.1 whereas this
wire profile names TOML 1.0. Core therefore uses `toml10`, an exact dependency alias
for `toml=0.8.23` with parsing only. It and its transitive parser dependencies use
permitted licenses. The existing workspace dependency and legacy parsers remain
unchanged. A narrow lexical preflight rejects multiline strings and bounds nested
structure before TOML allocation; the library owns all TOML grammar. Parsed values
then reject floats, timestamps, negative/unsafe integers and unsupported types.

Independent review found a real stack-overflow case: 63 nested inline tables each
with 63 dotted key segments bypassed separate dot/container counters. The process
aborted on an 8,873-byte source. Red evidence records that failure; combined depth
preflight and 8/32/63 nesting regressions now return `resource-limit`. A separate
review finding showed NUL diagnostics pointing to byte 0 instead of the offending
byte; the corrected diagnostic is checked after multibyte text and CRLF.

F2 is interpreted literally: Context requires a nonempty binding unit; background
alone does not meet its minimum. Any future exception needs an explicit spec edit.
Unknown extension semantics are never treated as implemented merely because source
syntax passes. `ValidationOptions.supported_extensions` is a caller capability
assertion, not an extension implementation, trust decision or authority grant.

## Observed validation

Rust 1.97.1 commands and results:

- `cargo test -p openwarrant-core --all-features`: 493 existing tests plus 13 new
  public document tests passed. No test was ignored.
- `cargo run -p openwarrant-core --example sdk_probe -- --scope 75 --fixtures conformance/sdk`: 22 actual-file cases passed.
- A new unseen fixture filename and ID passed; mutating its source bytes to add
  trailing content produced exit 1. This checks driver behavior beyond known names.
- `cargo clippy -p openwarrant-core --all-targets -- -D warnings`: passed.
- `cargo fmt --package openwarrant-core --check`: passed.
- Additional all-features clippy surfaced an existing `identity.rs:304`
  `items_after_test_module` violation. That unrelated historical file was not edited.

Evidence logs are in `../evidence/sdk-parser/`. Sequential red/green logs retain
first failures instead of presenting only successful reruns. The final source
inventory records exact current implementation/fixture bytes.

## Remaining obligations

Independent reviewer must recheck final fixes and the integrated subject. Parent
integration still needs aggregate repository checks, qualification-only obligations,
formal review of the exact commit and any applicable human acceptance. Old corpus
conflicts are outside this parser implementation. No legacy disposition was changed.

FOOTER-08 author/edit is OW-WAR-0076 work; parsing is ready as its read-side seam.
SDK record/assurance operations, context resolution/compiler behavior and CLI parity
remain their separate Warrants. These results do not complete those scopes or
publish Stable 1.0.
