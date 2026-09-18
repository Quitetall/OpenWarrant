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

Remaining: full isolated gate, rendered HTML inspection, integration
with the existing platform board, exact delivery/evidence reconciliation for all
OW-WAR-0069 obligations, and independent verification. Signed atoms unchanged.
