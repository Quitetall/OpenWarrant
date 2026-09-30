# Frontier integrity hardening

OW68 deliverable 4 requires frontier to report open, unblocked stages. The previous
implementation silently skipped load errors and treated failed journal reads as an
empty claim history. Damaged history could therefore produce an OPEN row.

The frontier now propagates failed basis, milestone, assessment and journal reads.
A malformed journal or non-file journal container cannot become an empty history.
A genuinely absent journal remains an empty history, and valid profiles without
a milestones declaration remain outside the stage frontier.

The pinned legacy journal loader and signed OW68 contract remain unchanged.
This is an implementation contribution, not independent verification, completion
of all OW68 skill/runtime obligations, or a unified execution permission model.

Validation: public CLI regression covers readable positive fixture, malformed
journal, directory container, absent journal and malformed milestones. Focused
unit test and Rust 1.97.1 Clippy also pass. Full gate pending at this checkpoint.

Initial full gate failed on stale D-002 delivery digest and dependent corpus tests. D-002 now records the new unverified implementation, preserving the old digest as an input; prior full record remains in parent Git history. Non-file journals are rejected before reading to avoid special-file hangs. Full gate rerun required.

Corrected full gate passed at 325ce50e242d2e13fc003b5e0dc40d5339a79af3: 14/14 steps, 308 plants. Exact transcripts retained under OW99 implementation/evidence. This does not complete other OW68 obligations.
