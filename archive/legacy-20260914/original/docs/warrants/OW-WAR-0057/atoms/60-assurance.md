---
schema: oh.war/atom/v1
warrant_uuid: 01a0603f-d2c2-7ad1-a9f7-aa223a6d6559
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the page is byte-deterministic and self-contained
- **scope:** §17.5, RQ-075.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two consecutive `war compile` runs leave `CORPUS_STATUS.html`
  unchanged; the file contains no `fetch(`, no `<script src=`, no `<link
  href=` to any host, and no `http` URL other than the SAS's own.

### OBL-002 — the page shows the ladder and never a percentage
- **scope:** the projection's rule, inherited.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the renderer source contains no division; the rendered page
  carries the provenance banner before the title and the string `0
  satisfied` while nothing is resolved.

### OBL-003 — a hand-edited page is caught
- **scope:** RQ-075, §13.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant editing one byte of `CORPUS_STATUS.html` is rejected
  as `corpus-status.drift`.

## Gate Adequacy

Not required at `basic`. Asked anyway: could the page show a number the JSON
does not contain? Only by computing one in the renderer, which is the one
thing the renderer is forbidden to do; a reviewer reading the inline script
for a `/` operator is the check.

## Residual Risk

- The JSON is inlined. At 135 KB today the page is fine; at ten times the
  corpus it is a megabyte, still one file. The number is recorded so the
  decision can be revisited with it rather than remembered without it.
- A page is read by whoever opens it, and a reader who scrolls past the
  banner reads `draft` as a verdict. The banner is first; nothing further can
  be done about scrolling.
