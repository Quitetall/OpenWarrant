# Read-only board and batch questions

Candidate work, not independent verification or Warrant resolution.

`war board` assembles the configured program name, corpus objectives and Warrant
records, complete stage frontier, open questions and numbered approval commands.
`war board --json` uses the existing report envelope with a draft board payload.
`war board --html > board.html` produces a self-contained, escaped snapshot with
no scripts, forms, signing controls or external resources. Existing status,
frontier and signing evaluators retain their semantics. Legacy record status is
explicitly distinct from implementation completion.

The `war-grill` skill has a conditional batch path covering every open question,
with durable identity and unresolved items retained. Legacy human-only answers
remain requests for the human; the skill cannot impersonate a responder.

Validation so far: Rust 1.97.1 `cargo check -p openwarrant-cli` and initial public
CLI integration test passed. The test observes unchanged fixture bytes, equality
with console approvals and frontier rows, offline HTML and corrupt-question
refusal. Extended signing-list comparison and HTML injection tests also passed (2 tests,
0 failures).

`cargo clippy -p openwarrant-cli --all-targets -- -D warnings` passed.

Remaining: final hosted integration checks, offline-file rendering inspection,  exact delivery/evidence reconciliation for all
OW-WAR-0069 obligations, and independent verification. Signed atoms unchanged.

The reference workbench now reads `/api/board` through the authenticated local API.
Nine HTTP tests passed; browser observations are in `board-browser-qa.md`.

The full gate at 9161f97 failed three steps because D-004 in unresolved OW68
still named the prior skill bytes; generated corpus and related plants reflected
that mismatch. The prior manifest and skill are now preserved under OW68
`attempts/batch-grill-20260918/`, with a new unverified candidate digest. Tests were
not weakened. A fresh exact-subject gate remains required.

All 47 reference-web tests pass, including the execution fixture's expected board
refusal without an OpenWarrant corpus. New unsigned OW104 binds this continuation's
CI scope; signed OW69 remains intact.

Fresh isolated gate at 7ba66ae passed all 14 steps and 308 planted controls,
including the question/performer/signing refusal cases. Terminal exit 0 observed.
Retained log: `docs/warrants/OW-WAR-0104/implementation/gate-7ba66ae.log.gz`.

Legacy OW69 still has unanswered Q-001 through Q-006 records. No answer was forged.
This delivery provides the reference workbench board; the historical question
proposing a separate Pages board is not silently treated as answered or accepted.
