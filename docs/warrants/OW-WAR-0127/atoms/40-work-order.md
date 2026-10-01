---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f55-7171-906f-45d79a408ce3
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `docs/adr/atoms/OW-ADR-0026-native-held-deliverables.md`, status
   `proposed`, `governs` this Warrant. It records the reference form the
   owner chose (Q-001), the rejected forms, A-002, and Q-002 and U-003 as
   open.
2. `crates/openwarrant-core/src/deliverable.rs`:
   - one function that classifies a `target_ref` as a repository path or a
     native reference, in the chosen form;
   - validation that refuses a native reference with:
     - no provenance, or `provenance_required = false`;
     - a Source Holder that is empty or `git`;
     - `content_addressed = true` and no content digest.
   - Each refusal is a named `DeliverableError` variant.
3. `crates/openwarrant-cli/src/resolve.rs`: §56.1 requirements 2 and 3
   never read a native reference from disk. Each native deliverable leaves
   the requirement unmet with an UNKNOWN diagnostic naming the holder.
4. `crates/openwarrant-cli/src/check.rs`: drift checking skips native
   references and says so once per Warrant. Validation surfaces item 2's
   refusals as errors.
5. `crates/openwarrant-cli/src/pins.rs`: `war pins --refresh` never reads or
   rewrites a native reference's digest. `war pins` lists it as held by its
   system, not as pinned.
6. `crates/openwarrant-cli/src/correct.rs`: a correction request for a
   native reference is refused, naming the holder, and writes nothing.
7. `crates/openwarrant-cli/src/bundle.rs`: the verification bundle carries a
   native deliverable's reference, holder and recorded digest, marked as
   not read, instead of bytes.
8. `crates/openwarrant-cli/src/status.rs`: digest state for a native
   reference is "held by <system>", not `TargetUnreadable`.
9. `conformance/plants.d/61-native-authority.sh`, on a scratch corpus. See
   the obligations for each planted case.

## Frozen Surfaces

- `ResolutionChecks` and §56.1's thirteen booleans.
- The ownership set digest (`ownership::set_digest`) for every existing
  Warrant.
- Every record schema except the `target_ref` values `deliverables.toml`
  accepts. No network access anywhere.

## Autonomy and Escalation

Tier T2. The performer starts only after Q-001 is answered. Escalate rather
than decide:

- any change that alters how an existing repository-path deliverable
  parses, pins, owns or resolves;
- any read site not listed above that joins `target_ref` to the root;
- any wish to fetch or verify a native artifact.

## Rollback

Revert the seven code files and remove the plant and the ADR. No existing
deliverable uses a native reference, so no record needs repair. A Warrant
drafted with one in the meantime is refused by `war check` after the
revert, by name.
