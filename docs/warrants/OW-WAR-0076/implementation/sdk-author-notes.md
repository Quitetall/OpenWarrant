# SDK author/edit implementation

Scope: OW-WAR-0076, S02, T54–T56 and FOOTER-01–08. Owner requested
unverified implementation; this record does not authorize, resolve or qualify work.
Predecessor: published main `8280bb94b8bcfb921bb7a14000504a2b15faa41a`.

The pure `document::author_document` API takes typed metadata values and exact
Markdown units. It emits readable RC.3 units before compact TOML footer metadata.
Presentation options select LF/CRLF framing and the optional details wrapper.
Required schema, identity and revision are explicit inputs. Unit text includes
its heading and terminal newline; the library does not normalize it.

`edit_document` replaces named metadata fields or existing unit text. Identity
and revision change only when explicitly edited. A title metadata edit also
updates the first RC.3 heading, preserving its newline. An explicitly supplied
first-unit edit must agree with the title; conflicting inputs refuse. Unit IDs
and kinds remain unchanged. Adding/removing/reordering units is done by explicitly
authoring a new document from the desired unit list, not by an implicit edit.

No-op edits preserve all original bytes. Unit-only edits preserve metadata bytes;
metadata edits preserve unit bytes except the explicitly synchronized title.
Editing metadata reserializes that metadata span (comments/layout may change),
while preserving parsed optional extensions, order within arrays, existing footer
wrapper and all surrounding bytes. Explicit RC.2 edits retain RC.2 framing.
No approval or assurance follows changed bytes.

Both APIs return complete validated bytes or a named diagnostic. Candidate
validation uses the public parser and validator, then checks exact unit identity,
kind and content to detect injected markers. Resource limits bound serialization and aggregate encoded edit payloads before
copying caller edits (both use the source-byte limit);
unsupported profile values and duplicate fields/edits refuse. Diagnostics from
candidate validation refer to candidate offsets, not old source offsets.

There is no implicit save, execution or I/O helper. Cancellation means discarding
returned bytes; the caller's original source remains unchanged. This phase does
not claim an atomic filesystem save API. External context availability, required
extension semantics, readiness and assurance remain separate evaluations.

Current checks: six public author/edit tests, one allocation regression, 22
parser cases and 10 author/edit cases pass. A mutated positive fixture produces
exit 1 and its named failure. Workspace/all-targets clippy passes. Independent
Spec and Standards reviews pass after aggregate-allocation and quadratic-lookup
fixes. See [evidence](../evidence/sdk-author/independent-review.md).
Aggregate gate passed on `e3e16813a9aa8a917dfd2cbf285a94856cedc3ec`:
14/14 steps, 747 Rust tests and 308 mutation checks. LAMU returned PASS WITH NITS;
all three findings were checked false positives. See [review response](../evidence/sdk-author/review-disposition.md),
[gate log](../evidence/sdk-author/gate.log), [completion evidence](../evidence/sdk-author/completion.json)
and [live CLI overview snapshot](../evidence/sdk-author/progress.json).

This bounded implementation is complete, unverified. The overview uses legacy
record state and does not infer this new work-state model from unsigned notes.
Human acceptance and legacy resolution remain outstanding. Next: review this
candidate, then request OW-WAR-0077's Phase 1 source/snapshot/reference type slice;
provider acquisition/resolution remains a separate Phase 2 responsibility.
