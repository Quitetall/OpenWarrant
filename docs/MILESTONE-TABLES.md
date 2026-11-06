# Import milestone tables

Run `war admin import table roadmap.md --section "Queue"`. The source must be
inside the repository. The importer preserves its full contents under
`docs/imports/table/<source-digest>/sources/<original-path>` and creates one
single-file Warrant for each row. Originals stay where they are.

A table needs one stable `ID`, `Package`, or `Milestone` column, plus one
`Title`, `Outcome`, or `Description` column. Select a heading with `--section`
when several tables match. Duplicate keys, ambiguous columns, malformed rows
and unresolved dependencies refuse the import before any output is written.
Escape a literal pipe in a cell as `\|`.

Optional `Depends on`, `depends_on`, or `After` cells name row keys, separated
by commas or semicolons. Row order and phase labels create no dependency.
Optional `Status`, `State`, or `Engineering state` cells mark exact `done`,
`complete`, `completed`, or `implemented` values as imported completion claims.
Other wording remains visible as source text and imports open. An imported
claim is unverified; it supplies no checks, signatures or assurance mark.

Identity comes from source path, section and row key. Keep these stable.
A repeated unchanged row does not write its Warrant again. A changed row
returns a digest-bound proposal with the existing ID, prior text and proposed
text. It preserves local edits. The maintained Warrant is the work record;
its original table is retained provenance, not a second live tracker.

The initial importer reports proposals; automatic application of changed rows
is not yet implemented. Review and merge proposed text through the normal
unsigned authoring workflow. Keep the source pointer and revision marker when
the imported row is adopted, so later imports can identify its baseline.
