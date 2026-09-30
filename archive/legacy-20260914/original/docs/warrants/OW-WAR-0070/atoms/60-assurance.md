---
schema: oh.war/atom/v1
warrant_uuid: 01a09762-3fa2-7dd0-b794-0190b44fe879
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — Every Warrant state classifies to exactly one next act

- **scope:** `src/inbox/classify.rs`, `tests/inbox_classify.rs`
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** test run output showing one passing case per state; compiler rejects the match without a wildcard arm (no `_ =>` present, confirmed by grep in the evidence note).

### OBL-002 — `war inbox` lists precisely the human-waiting Warrants

- **scope:** `src/cli/inbox.rs`, `conformance/fixtures/inbox/`, `tests/inbox_cli.rs`
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** integration test asserting the exact alias set against the fixture; captured stdout of `war inbox` and `war inbox --json` over the fixture; exit code 0 on an empty fixture.

### OBL-003 — Command is documented and the change is recorded

- **scope:** `docs/cli/inbox.md`, `CHANGELOG.md`
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the doc page contains the state-to-act table identical to the classifier match arms; CHANGELOG entry present under Unreleased.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** The gate proves the code compiles, tests pass, and the record is well-formed, but does it prove that the classification table is *right*, that is, that the SAS actually assigns each listed act to a human? It does not; a test only pins whatever table the agent wrote. A wrong table would list Warrants that are not waiting on the human, or hide ones that are, and the gate would still pass.

- **outcome:** gap_accepted

## Residual Risk

- The state-to-act table encodes an interpretation of the SAS; a misreading silently misfiles Warrants. Mitigation: the table is a single, reviewable match in one file and reproduced verbatim in the doc for human eyes.
- Wait age depends on transition timestamps being present in records; older records may show `unknown`.
- If the JSON format ADR is later revised, downstream consumers of `oh.war/inbox/v1` must migrate; the version field exists for that reason.
- Sixty-plus records are loaded on every invocation; acceptable now, may need a cache at a much larger record count.