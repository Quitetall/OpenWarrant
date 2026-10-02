---
schema: oh.war/atom/v1
warrant_uuid: 01a0feb6-ab80-73a5-abcb-7ade9cfaaeaa
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — one compiled model, and every client's output is unchanged
- **scope:**
  - this repository and the synthetic 1,000-Warrant corpus from
    `tools/scale/synth-corpus.sh --n 1000 --resolved 500`;
  - the commands `check`, `check --json`, `check --generated`,
    `status --json`, `next`, `console --json`, `sign --list` and
    `pins --resolved-only`.
  - No claim about commands not listed.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: the pre-M1 and post-M2 binaries print byte-identical output
    for every listed command on both corpora. The only exception is the
    named state fix; for that Warrant, status and the warrant overview now
    agree.
  - Accepting: `tools/scale/budget.sh` still holds on the 1,000-Warrant
    corpus.
  - Refusing: a doctored output (one byte changed) is reported as a
    difference by the same comparison.

### OBL-002 — `war model --json` is the shared model, drift-checked
- **scope:** a scratch program with Warrants, a ticket and a roadmap
  (`72-model.sh`), and this repository's model.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: `oh.war/model/v1` names every Warrant, obligation,
    deliverable, stage, question, phase, ticket and item as a record. It
    names each parent, supersedes, roadmap, implements, blocker and
    `promoted_to` as a relation.
  - Accepting: two runs over the same tree give the same `basis_digest` and
    bytes, and the schema validates it. `war schemas --check` passes with the
    pack version 0.2.0.
  - Refusing: a record whose relation names an unknown target is a
    diagnostic, not a dropped edge.

### OBL-003 — a type's capabilities decide what applies to it
- **scope:** profiles on a scratch program (`73-capabilities.sh`): the four
  shipped profiles, and test profiles.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: a decision Warrant reaches `would_satisfy` with all its
    obligations established. Check 12 reads "not applicable: no `stages`
    capability", by name.
  - Refusing: a profile declaring `verification` without `evidence` is
    refused `profile.capability-prerequisite`. So is one naming an unknown
    capability.
  - Refusing: `war sign --dry-run` on a kind without `authorization` is
    refused by name, and `war next` offers no authorize act for it.
  - Refusing: a §56.1 check is never reported as met merely because its
    capability is absent. It reads "not applicable", and a resolution of a
    kind lacking `resolution` is refused.

### OBL-004 — behaviour once chosen by profile name is profile data
- **scope:** outcome word, falsifiability, standing coverage and promotion
  target; a scratch extension profile that sets each.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: each of the four reads the profile's value, and delivery
    keeps today's values byte for byte (OBL-001).
  - Refusing: a standing class applied to a profile whose data does not
    allow standing coverage is refused, as delivery-only is refused today.

### OBL-005 — the signature covers the type
- **scope:** Warrants made by `war new` after this Warrant, on a scratch
  program. Pre-existing manifests are covered only by OBL-001's
  unchanged-output claim.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: a new manifest carries `profile_digest` equal to the profile
    file's sha256. The Warrant authorizes with a throwaway signer, and
    `war check` is clean.
  - Refusing: one byte changed in the profile file afterwards gives
    `profile.pin-drift`, an error naming the profile and both digests. The
    same edit before authorization gives a warning.
  - Accepting: a manifest without `profile_digest` checks exactly as before.

### OBL-006 — records carry identity and their own revision
- **scope:** `oh.war/records/v1` atoms on a scratch program
  (`74-records.sh`), including the password-reset records.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: each `## <ID> · <type>` becomes a record. Editing one
    record's text moves only that record's revision.
  - Refusing: a duplicate id, a type the governing profile does not
    declare, and a relation of an undeclared core kind are each refused by
    rule.
  - Accepting: a namespaced kind is carried and changes no state, readiness
    or check.

### OBL-007 — `war impact` names what a change affects
- **scope:** the password-reset scratch program: records, a ticket, a
  roadmap phase and a Warrant draft.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: after REQ-pr1 changes, `war impact REQ-pr1` lists the ticket
    items that implement it, the Warrant that names it, the obligation that
    evaluates it, the phase's progress, and the projections that select
    those records.
  - Refusing: `war impact` on an unknown id is refused by name.
  - Accepting: evidence recorded before the change stays, bound to the old
    revision.

### OBL-008 — declared states only refine the fixed states
- **scope:** a scratch profile declaring `in_review` (refines `in_progress`)
  and `signed_off` (refines `verified`) (`75-states.sh`).
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: `war state <item> in_review` on a claimed item is journaled
    and shown. When the item is done, `in_review` no longer holds.
  - Refusing:
    - `in_review` on an unclaimed or a done item;
    - `signed_off` on an obligation not verified;
    - a declared state whose `refines` is unknown.
  - Refusing: no declared state ever satisfies a §56.1 check.

### OBL-009 — tickets run on the kernel, unchanged on disk
- **scope:** `45-tickets.sh` and `tests/tickets_cli.rs` as they stand, plus
  a copy of a ticket written before this Warrant.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: every existing ticket check passes. A pre-Warrant ticket
    reads, claims and finishes, and its files differ only in the lines the
    act wrote.
  - Accepting: items appear in `war model` as records, and blockers as
    `depends_on`.
  - Refusing: a blocked item is still refused claim (`ticket.blocked`).

### OBL-010 — ticket features
- **scope:** a scratch program (`76-ticket-features.sh`).
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: `war tickets --type`, `--label`, `--state`, `--text` and
    `--search` each return exactly the planted set. An epic lists its
    tickets through `part_of`.
  - Refusing: a label outside a closed label set declared by the profile is
    refused, and a filter naming an unknown state is refused.

### OBL-011 — GitHub issues in, write-back opt-in, and the demonstration
- **scope:**
  - a fake `gh` on PATH that records its argv and environment, on a scratch
    program;
  - the password-reset demonstration (`77-typed-demo.sh`).
  - No claim about the real GitHub service.
- **gate:** `gate://ops.conformance.plants@1.1.0`
- **evidence:**
  - Accepting: `war create --issue 12` makes a ticket from one read. With
    `[intake] writeback` set, `war done` on its last item runs one comment
    and one close, with the configured argv.
  - Refusing: without `writeback`, nothing but the read runs. A
    `fetch_argv` that writes is still refused before it starts. A failing
    write leaves the ticket done and reports the issue UNKNOWN.
  - Accepting: the demonstration compiles a ticket, a roadmap view, a
    Warrant draft and an agent packet from one set of records. After
    REQ-pr1 changes, `war impact` names each affected output.

## Gate Adequacy

The level is basic. Every obligation is observed by the battery on scratch
programs, with a refusal observed beside each acceptance. The claims are
structural or differential, so a plant that runs the commands is the right
observer.

**How could this look done and not be?**
- A capability-gated rule could simply never run: "not applicable" could
  hide an unwired check. OBL-003 requires the check to be named, and requires
  a resolution of a kind lacking `resolution` to be refused, not waved
  through.
- The model could disagree with the clients' own readings. OBL-001's
  differential runs over two corpora, including 1,000 Warrants.
- The pin could be decorative. OBL-005 changes one byte and requires an
  error.
- Declared states could leak authority. OBL-008 requires that a refinement
  of `verified` cannot be entered without `verified`, and that no declared
  state meets a §56.1 check.

## Residual Risk

- OBL-001's equality holds for the eight commands only. Other commands are
  covered by their existing plants, not by the differential.
- The real GitHub service is not exercised. A fake `gh` stands in, so the
  write-back's real-world behaviour (rate limits, permissions) is observed
  only when it is used.
- Accepted by the owner at authorization.
