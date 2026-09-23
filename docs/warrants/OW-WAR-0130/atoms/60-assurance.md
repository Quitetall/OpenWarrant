---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fa2-7203-a94c-690e843d1319
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a batch killed mid-record is found and undone
- **scope:** `war sign --batch` step 6, on a scratch corpus with a test
  signing key, the process killed with SIGKILL after its second of three
  records. No claim about a power loss during `--recover` itself (R-001).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - After the kill, `war check` reports ERROR `batch.interrupted` naming
    the batch, and a new `war sign --batch --dry-run` refuses the same.
  - `war sign --batch --recover <id>` leaves every path listed in the
    marker byte-identical to its prior sha256, or absent as before, and
    `.refused.json` exists.
  - The same three acts, not killed, record n of n with no marker left:
    the marker is not left behind on success.

### OBL-002 — an equivalent retry replays, and a conflicting one is refused
- **scope:** `war ask`, `war submit`, `war verify --response`,
  `war evidence record`, and the single-act `war sign` ingest, each run
  twice with identical input on a scratch corpus. Only acts the inventory
  lists; an act it does not cover is named in the inventory, not claimed.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - For each act, the second run exits 0 with a replay diagnostic, and a
    sha256 of the Warrant directory is unchanged between the two runs.
  - Before this change the same plant fails on at least `war verify
    --response`: non-zero exit after the file is rewritten. That shows the
    plant tests something.
  - The same event appended with a different `actor_ref` is refused
    `journal.idempotency-conflict`, and the journal's line count is
    unchanged.

### OBL-003 — an older war refuses a newer repository, and a newer record is UNKNOWN
- **scope:** `compat.rs` under option B of U-001, over `openwarrant.toml`
  and the frozen record types in docs/COMPATIBILITY.md. No claim about
  unknown optional fields (option C).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `requires_war = ">=99"` makes `war status` exit 2 with
    `compat.war-too-old`, naming both versions, before any Warrant finding
    is printed.
  - `requires_war` equal to the running version, and no key at all, both
    pass unchanged.
  - A verification record rewritten to `oh.war/verification/v9` is
    reported UNKNOWN `compat.newer-record` by `war check`, never PASS and
    never ERROR.

## Gate Adequacy

Required at `controlled`: this changes how signed human acts are recorded.
The load-bearing obligation is OBL-001. A batch recorded with a hole is a
list the signer did not sign, and the byte-identity check over every listed
path is what shows the hole cannot remain. OBL-002's "before" run is the
plant's own negative control.

**Adversarial question:** could a batch pass every plant here while still
recording a hole?

- Yes, if the only induced failure were an ingest refusal. The rollback
  would then be tested on the one path the dry run already covers. So
  OBL-001 induces three failure kinds, including an unwritable target,
  which no dry run sees.
- Yes, if "byte-identical" were checked only on the failing act. So it
  is checked on every path the marker lists.
- A crash mid-rollback remains unplanted (R-001). It is named, not
  claimed.

- **outcome:** obligation_narrowed, gate_strengthened

This is the drafter's own pass, which narrowed OBL-001 and OBL-002 to named
acts and failure kinds. It is not the blind adversarial review §39.4
requires at `controlled`, which remains to be done by someone who did not
write this Warrant.

**Executed attacks:** none yet; the plants above are the attacks, run at
evidence time.

Limitation: the plants exercise `war`'s own files on one machine. They say
nothing about concurrent writers on a shared filesystem.
