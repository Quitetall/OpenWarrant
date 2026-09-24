---
schema: oh.war/atom/v1
warrant_uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a bundled gate run carries its output
- **scope:** `war verify <alias> --bundle` on a scratch Warrant with one
  recorded `software.repo.war-check` run.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - the bundle's run entry has `stdout.text` equal to the tail of
    `gate-runs/<gate>.stdout.txt`, and `stdout.sha256` equal to that
    file's sha256;
  - with `max_excerpt_bytes` set below the file's size, `truncated` is
    true and the digest is still of the whole file;
  - refusal: with the stdout file removed, `captured` is false and `text`
    is absent, never an empty string.

### OBL-002 — output that disagrees with its receipt is flagged
- **scope:** the same scratch Warrant, the stdout file edited after the
  receipt was minted, where the receipt records a stdout digest.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the entry carries `mismatch: true`; unedited, `mismatch` is
  absent or false. Where the receipt records no stdout digest, the plant
  says so and asserts only that the field is absent.

### OBL-003 — the bundle stays deterministic and verifiable
- **scope:** two `--bundle` runs over one tree, and `war verify --run` with
  the fake-`claude` verifier of `62-verifier.sh`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** one bundle file after two runs (one digest), and the fake
  verifier's response is ingested as before.

## Gate Adequacy

Required at `basic`. OBL-001's "never an empty string" refusal is the
load-bearing one: an absent capture that read as silent output would let a
verifier conclude a gate printed nothing wrong.
