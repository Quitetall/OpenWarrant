---
schema: oh.war/atom/v1
warrant_uuid: 01a0650e-5702-7f52-ba66-dbaee871efba
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.3, sha256 `742dfd066b8df579116ebbd36e19a4b57dc08` (prefix; the
  revision record carries the full digest).
- §19.3 ADR supersession; §28.3 revision immutability; §34.1 stable
  requirement identifiers; §34.2 contributions; §39.3 executed attacks;
  §71.7 output shape; §92 conformance.
- RQ-020, RQ-022.

## Measured on 2026-09-03

- Unestablished obligations after round two: 63, across 36 Warrants.
- `war check` accepted `implements` refs naming §106 rows that do not exist
  (`sas://WAR-SAS-RQ-999` parsed as well-formed and was never looked up).
- `AdrRecord` had `status` and `governs`, and no `supersedes` or
  `superseded_by`.
- Commit `3678455` (OW-WAR-0039's known untracked case) is not in `main`'s
  history: `git log --format=%h | grep -c 3678455` → 0.
- The committed §94 baseline no longer verified against HEAD (`war
  telemetry --verify`: "the committed baseline differs from a fresh one").

## Measured during execution

- The renderer's source contains " / " inside its inlined JavaScript (a
  path label), so a "no division" test has to distinguish an operator from
  text; it now skips comment and string lines.

## Assumptions carried in

- A test that reads the committed corpus (every `WAR.json`) is evidence
  about the corpus as committed, and moves with it.
- Re-taking the telemetry baseline at a named commit supersedes the earlier
  one; the earlier artifact is in history, not deleted.
