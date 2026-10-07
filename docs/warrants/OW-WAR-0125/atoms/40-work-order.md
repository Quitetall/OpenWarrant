---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f26-7ba0-a64b-2618b983ba43
role: work_order
jurisdiction: authored
order: 40
classification: internal
---


# Work Order

## Deliverables

1. `docs/adr/atoms/OW-ADR-0028-sas-sections-are-atoms.md` (proposed):
   - a section's identity is `<NS>-SAS-<n>` or `<NS>-SAS-<n>.<m>`, its
     number as written in the accepted revision, and is never renumbered;
   - step one derives sections from the pinned document (this Warrant);
     step two makes them the authored source (a later Warrant);
   - proposed SAS text: `sas://<NS>-SAS-<section>` added to §105, marked
     as a proposal for a separate SAS revision.
2. `crates/openwarrant-core/src/sas_sections.rs` and its export from
   `crates/openwarrant-core/src/lib.rs`:
   - `split(bytes)` returns sections in order: id, title, byte range,
     sha256, and subsection ids;
   - `join(sections)` returns the input byte for byte;
   - fences are respected;
   - no I/O.
3. `crates/openwarrant-cli/src/compile.rs`: `docs/sas/generated/SECTIONS.json`
   (canonical JSON) and `SECTIONS.md`, from the document in force, stating
   its sha256 and matching revision like `NORMATIVE.json`, and covered by
   `check --generated`.
4. `crates/openwarrant-cli/src/check.rs`, two rules on each amendment's
   `governing_adr_or_policy` when it is a `sas://<NS>-SAS-<section>`
   reference:
   - `sas.section-ref`: the section or subsection exists in the revision
     the Warrant is pinned to (the latest recorded one if unpinned); an
     error otherwise;
   - `sas.section-current`: its digest at the pinned revision equals its
     digest at the latest accepted revision (pass), differs (warning,
     naming both revisions), or cannot be read (UNKNOWN, saying why).
5. `crates/openwarrant-cli/src/sas.rs`: `war sas diff <candidate>` also
   lists sections added, removed and changed, by id.
6. `conformance/plants.d/56-sas-sections.sh`:
   - `join(split(x))` has the sha256 of revision `1.1.0`, via a test hook
     or `war sas diff` against the document itself reporting no change;
   - a planted `## 999. Fake` inside a fenced block is not a section;
   - a hand edit to `SECTIONS.json` fails `check --generated`;
   - an amendment citing `sas://WAR-SAS-999` fails with `sas.section-ref`;
     `sas://WAR-SAS-43.9` (no such subsection) fails the same way;
   - the sixteen existing section references pass both rules;
   - a candidate SAS with one word changed in §62.3 makes `war sas diff`
     name section 62 and nothing else;
   - in a `git clone --depth 1` copy, `sas.section-current` for a Warrant
     pinned to `0.1.0-draft.1` is UNKNOWN.

- (AM-002) `.github/workflows/ci.yml`: the `gate` job's checkout fetches full
  history (`fetch-depth: 0`), so `sas.section-current` is answered in CI
  rather than read UNKNOWN from a shallow clone.

## Frozen Surfaces

- The SAS document's bytes and every revision record.
- `oh.war/sas-revision/v1`, every record schema and the schema pack.
- `NORMATIVE.md` and `NORMATIVE.json`.
- Every signed amendment.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any existing section reference that fails `sas.section-ref`;
- any change that would move the SAS's source or its bytes;
- the ADR number, if 0028 is taken.

## Rollback

Remove the two rules, the generated index and the diff lines; delete the
module. The ADR stays as proposed, or is withdrawn by the owner. No record
changes.
