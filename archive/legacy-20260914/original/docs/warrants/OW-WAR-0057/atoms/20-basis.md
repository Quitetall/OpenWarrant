---
schema: oh.war/atom/v1
warrant_uuid: 01a0603f-d2c2-7ad1-a9f7-aa223a6d6559
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
- §13 jurisdiction — a generated atom is not directly editable; §17.1 the
  generated header; §17.5 read projections; §77.3 consumers of generated
  schemas.
- RQ-012, RQ-075.

## Depends on

OW-WAR-0055. This Warrant reads `CorpusStatus` and the canonical JSON that
`war status --json` emits; it adds a third rendering and changes nothing in
the first two.

## Measured on 2026-09-02

- `CORPUS_STATUS.json` is 135 KB of canonical JSON, byte-identical across
  runs, drift-checked. It carries: caveats, next_actionable, objectives (12,
  including `unassigned`), release, requirements (57), warrants (53).
- No HTTP server exists in the workspace; the only network dependency is a
  blocking client used by one command that requires `--confirm-write`.
- The committed projections are emitted by one loop in `compile.rs` and
  drift-checked by one function in `check.rs`; a third file joins both.

## Assumptions carried in

- Inlining the JSON is acceptable at this size. At ten times the corpus the
  page would be a megabyte; still one file, still no fetch, and still faster
  than a round trip. Recorded as a residual risk with the number.
- A browser is the reader. The Markdown projection remains for everyone else.
