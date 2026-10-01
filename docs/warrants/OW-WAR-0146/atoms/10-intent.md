---
schema: oh.war/atom/v1
warrant_uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The blind verifier (OW-WAR-0117) decides from the verification bundle
alone. The bundle carries each gate run's record and receipt, but not what
the gate printed: `gate-runs/<gate>.stdout.txt` and `.stderr.txt` sit beside
the receipt and are never read into it. On 2026-09-24 the verifier's first
run (OW-WAR-0004) returned `not_established` for all three obligations,
each naming the same gap — "its stdout is not included", "no test result is
in the bundle". Every obligation whose evidence is a gate's output is
unestablishable by construction, however good the work.

## Desired Outcome

- Each bundled gate run carries its stdout and stderr: the tail up to the
  repository's `max_excerpt_bytes`, the full sha256 of each file, and
  `truncated: true` when cut — the same rule deliverables already follow.
- A captured file whose digest disagrees with the one the receipt records
  (when it records one) is bundled with `mismatch: true` and the verifier
  is told, rather than silently trusted.
- A run with no captured output says so (`captured: false`), never an empty
  string that reads as "printed nothing".
- The bundle stays deterministic: two bundles of one tree share one digest.

## Non-goals

- Changing any gate, what it runs, or what a receipt records.
- Adding test results a gate did not produce: an obligation whose cited
  gate does not run the tests it names stays unestablishable, and the
  verifier will say so. That is a finding about those contracts.
- `docs/VERIFICATION.md`: OW-WAR-0117's delivered file, left alone until it
  resolves.
