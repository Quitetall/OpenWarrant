# SDK artifacts

Use this branch for RC.3 source documents or supplied-record evaluation. Inspect
`war sdk --help`, then read the [CLI profile](../../../../conformance/sdk/cli/README.md)
for the selected operation's exact fields. Use `author` or `edit` for new document
bytes, then `validate`. These operations run offline and do not need a repository.
The CLI returns source text inside a JSON result; `--output` saves that JSON to a
new file. It does not replace an existing source or approve the work.

Keep settled decisions and exact required constraints in named binding units.
Record unresolved questions separately. Use Warrant kind for bounded work, ADR
kind for architecture proposals and context kind for discovery. Drafting does not
complete implementation. For stages and reviews, retain the existing milestone
and record profiles; a generic context document does not replace executable stage
or independent-observation records.

Exit zero means evaluation ran. Inspect validity, readiness and assurance fields;
caller-supplied trust facts remain assumptions. Human acceptance and independent
verification require their real actors and evidence. Legacy proposal transport
remains available through the [drafting guide](drafting.md).

For native host guidance, preserve exact host bytes and nested scope. Phase 1
[context-entry examples](../../../../conformance/sdk/skills/README.md) propose
additive edits only. An installer must resolve and recheck the real target and
permission before writing; a proposal itself grants none.
