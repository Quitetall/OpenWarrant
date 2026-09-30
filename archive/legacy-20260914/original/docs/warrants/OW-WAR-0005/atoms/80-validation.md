---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-71ab-a5db-0f2c062305af
role: validation
jurisdiction: authored
order: 80
classification: internal
---

# Ongoing Validation

Distinct from the acceptance obligations (§57). Those prove the work shipped;
this watches whether it stays right.

## What would show this decision was wrong later

- **The conformance suite passes for a month without a single planted violation
  being updated.** Either nothing changed, or the plants stopped tracking the
  controls they were written against. Fixtures rot silently, and a rotted plant
  is a green light over an untested rule.

- **`war check` is run and its output is not read.** The signal is a commit that
  changes a Warrant and does not change any verdict. Ceremony is the failure
  mode this whole system was built against; it is not immune to it.

- **`UNKNOWN` never appears in any verdict.** It should appear whenever a check
  cannot be performed. A vocabulary entry that never occurs in practice usually
  means the code collapsed it into PASS or ERROR at some call site, which is
  exactly the "could not ask became failed" substitution §96.4 forbids.

- **A Warrant is authorized without `war check` having been run on it**, or with
  ERROR diagnostics outstanding. The check would then be advisory, which is what
  it already is until branch protection makes the gate a required status check.

- **Bootstrap closure is claimed but the next unit of work does not open as a
  Warrant.** OBL-005 is satisfiable on paper by working code. It is only true
  when the process actually changes, and the honest test is what the sixth
  Warrant looks like — or whether there is one.

## Metrics worth watching

- Count of `UNKNOWN` diagnostics per run, over time.
- Time between a control changing and its planted violation being updated.
- Number of Warrants in `docs/warrants/` versus number of units of work
  undertaken in the same period. Divergence measures untracked work (§95).
