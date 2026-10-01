---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e81-73d3-b28f-cedb19f93c32
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- `docs/design/openwarrant-product-spec.md`, "Friction targets and proof":
  the two targets, and the rule that human effort is reported as
  administration, substantive review, and waiting, separately and in total.
- The same document, "Engineering contracts still to specify": "the
  real-work baseline and measurement procedure, including what evidence
  establishes the setup and per-Warrant friction targets on each supported
  OS."
- SAS §94 (wall time and human authoring minutes are measured) and §100
  (success is a measured reduction, so a baseline must exist first).
- SAS §98 Phase 0, whose `open` list in `docs/roadmap/atoms/20-phases.yaml`
  names `friction-baseline`. OW-WAR-0114's gap table places it there.
- `QUICKSTART.md`, "Governed legacy workflow": the step sequence the script
  follows, and which steps are a human's.
- `conformance/plants.d/96-batch.sh`: the throwaway-key and ssh-agent
  pattern the script reuses. No owner key is ever used.
- The RC.3 draft SAS §16 repeats the targets
  (`docs/sas/drafts/1.0.0-rc.3/`). It is a draft, not adopted, and is cited
  only as the same wording.

## Assumptions

- A-001: a throwaway key in an ssh-agent without `-c` signs exactly the
  bytes a human's confirmed key would sign, so the tool time of `war sign
  --ssh-sign` is measured correctly. What is not measured is the human's
  confirmation. Confidence: high. The 2026-09-23 probe recorded an
  authorization this way.
- A-002: wall time on one machine is noisy. The script repeats each step
  (default 5 runs) and records the median and the maximum. Confidence:
  medium. A busy machine can still skew a run, so the record names the
  load average at start.
- A-003: the release build is the one users run, so the baseline is taken
  with it. A debug build is accepted by the script but labeled as such in
  the record. Confidence: high.

## Unknowns

- U-001 (non-blocking): which acts count as "routine Warrant
  administration". This draft uses: `new`, `check <alias>`, `compile`,
  `authorize` (the request), `sign --ssh-sign` (tool part), `next`,
  `status`, and `resolve --dry-run`. The owner may amend the list; the
  script takes it from one table.
- U-002 (non-blocking): the human time of setup. Only a human can measure
  it. The doc gives the protocol; whether anyone runs it is not this
  Warrant's evidence.
- U-003 (non-blocking): the 2026-09-23 probe showed `war status` listing
  the freshly authorized Warrant as `draft` while `war next` treated it as
  authorized. The script defines "authorized" by `war next` and
  `authorization.toml`, and records the `status` row as observed. Whether
  that is a defect belongs to another Warrant.

## Residual risks

- R-001: a sub-second tool time can be read as "setup takes seconds".
  Every summary line in the record and the doc puts the human steps beside
  the number, as counted and unmeasured.
- R-002: the baseline goes stale as the tool changes. It carries the
  binary's version and commit, and the doc says a new baseline is a new
  file, never an edit of the old one.
