---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-602f-7321-ad70-864c543b2927
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `docs/adr/atoms/OW-ADR-0025-assurance-mark.md` (new; next free number
   at delivery). Proposed, then accepted by the owner. It records:
   - Q-001's answer (issued or derived) and the rejected options;
   - baseline v1: each requirement, the record that evidences it, and
     what `UNKNOWN` means for it;
   - what a mark binds: baseline version, resolution digest, attestation
     digest, locator commit, contract digest, obligation ids in scope;
   - strengthening: a repository adds requirements; removing one is a
     different mark, and says so.
2. `docs/assurance/baseline-v1.toml` (new): the baseline as data, one
   entry per requirement, so `war mark` reads it rather than hard-coding
   it.
3. `crates/openwarrant-cli/src/mark.rs` (new) and
   `crates/openwarrant-cli/src/lib.rs`:
   - `war mark <alias> [--baseline v1]` evaluates each requirement: met,
     unmet (named), or `UNKNOWN` (named);
   - it emits `oh.war/mark/v1` only when every requirement is met;
   - `war mark <alias> --verify` recomputes every binding against the tree
     today and names what moved;
   - it writes nothing unless Q-001 selects (c).
4. `conformance/plants.d/57-assurance-mark.sh` (new): the plants in
   Assurance, on a scratch corpus.
5. `docs/ASSURANCE_MARK.md` (new): what the mark says (one accepted
   result, this revision, this scope, this baseline) and what it does not
   (the codebase, a release, every line read).

M2 does not start until the ADR is accepted.

## Frozen Surfaces

- `oh.war/resolution/v1`, the attestation format (OW-ADR-0015), and the
  four human act kinds, unless Q-001 selects (b), which is escalated, not
  built here.
- `war check`'s findings. The mark is its own command.

## Autonomy and Escalation

Tier T2 for M2; M1 is the owner's decision. Escalate on:
- any requirement that needs a new signed act;
- any mark that could exist without a human-signed resolution under it;
- a baseline requirement no record can answer (report it `UNKNOWN`; do not
  drop it).

## Rollback

Remove `war mark` and the baseline file. The ADR stays as the record of
the decision and can be superseded. No resolution or attestation is
touched.
