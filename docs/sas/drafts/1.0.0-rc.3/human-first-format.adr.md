# Human-first document source format

Status: proposed RC.3 implementation decision, selected by the owner in this
conversation. This draft has no allocated ADR identifier or acceptance signature.

The owner approved readable narrative first and compact metadata last. Adopt
`oh.war/document/1.0.0-rc.3` footer framing in the normative format contract, with
exact title agreement, stable unit markers and whole-source digest coverage.
Compact TOML inline arrays preserve repeated unit/target pairs and their order.
HTML details wrapping is optional presentation, with exact parser boundaries.

Preserve RC.2 framing under its existing identifier. Retain all historical example,
packet and signed bytes; converted source is a new subject. This avoids silently
changing digest or span meaning. Moving metadata helps human reading; exact task
projections reduce agent context. Native harness context files keep their formats.

Alternatives: keeping a long front header burdens readers; repeated keys in one
TOML table are invalid; using a dictionary keyed by unit loses repeated targets
or ordering; metadata in a separate sidecar complicates single-file portability.

Consequences and testable expectations: F1/F2 in format-contract.md,
FOOTER-01–FOOTER-08 in phase-1-build-scope.md, and separate examples-footer/.
Production parsing and author/edit round trips remain OW-WAR-0075/0076 work.
