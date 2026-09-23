---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f6e-7d90-8eed-cf0953b77657
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — every §81 input and output is dispositioned
- **scope:** the request and result tables in
  `docs/integrations/kf-compiler.md`, against §81.1 and §81.2 of the SAS
  at the reviewed commit.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - the reviewer finds a row for each of the nine inputs and nine results;
  - each row says carried, deferred with a reason, or refused;
  - a table missing any one of the eighteen is refused.

### OBL-002 — the process contract names what it refuses
- **scope:** the process-contract section of `kf-compiler.md`.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - for a path given instead of bytes, an unknown protocol version, input
    over the bound, and a request needing the network, the document names
    the triggering input, the error code and the exit status;
  - the reviewer refuses a refusal stated without a code, or a sandbox
    condition stated only as "should".

### OBL-003 — nothing is invented about either side
- **scope:** every element of `kf-compiler.md` and OW-ADR-0027 that
  describes existing behaviour.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - each OpenWarrant element names its source file or says "to build";
  - each KF element cites a KF file at a named commit, or sits in "Open
    questions for KF";
  - the gate's `citation-missing` rule fires on a planted citation to a
    file that does not exist.

### OBL-004 — the examples parse and name their digests
- **scope:** the fenced JSON examples in `kf-compiler.md`.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - the reviewer extracts each block and `jq .` exits 0 on it;
  - every digest in them carries an algorithm and a domain (RQ-081);
  - the refused example's error code is one the process contract names.

### OBL-005 — the decision is a parsed, proposed ADR with its alternatives
- **scope:** `docs/adr/atoms/OW-ADR-0027-kf-compiler-interface.md`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - `war check` parses it with no `adr.malformed`;
  - its status is `proposed` and its `governs` names this Warrant's uuid;
  - it records the answers to Q-001 to Q-003 and the options not chosen.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-003. The failure
this Warrant guards against is an interface that describes a KF behaviour
nobody built. That reads as agreement between two systems when only one
side wrote it.
