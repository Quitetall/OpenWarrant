---
schema: oh.war/atom/v1
adr_uuid: cffb01bd-2bbe-4b27-9994-043cccf6f228
local_alias: OW-ADR-0006
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
decided: 2026-08-19
governs:
  - "war://OW-WAR-0020"
---

# ADR OW-0006: Execution status and migration class are two vocabularies, and the verdict keeps its unknown

## Status

`Accepted 2026-08-19`

Governs OW-WAR-0020. Records a conflict between that Warrant's authored text and
the SAS it implements, and settles it in favour of the SAS.

## Context

OW-WAR-0020 was drafted before §44 was read closely, and its atoms say two things
the specification does not.

**First: it names the wrong vocabulary.** Its Intent describes "§44.2's execution
statuses" as a ten-item list — `malformed, foreign_working_directory,
missing_tool, missing_script, missing_crate, mutating, timeout, failed, passed,
not_run` — and its Work Order requires "all ten execution statuses, none
collapsible". §44.2 defines **six**:

```text
not_run  completed  timeout  infrastructure_error  cancelled  invalid
```

The ten belong to **§96.4**, which governs *migration*: what a legacy corpus's
gate outcomes must preserve when they are brought forward. §44.4's own worked
example shows where they actually live at runtime:

```yaml
askability: "not_askable"
execution_status: "not_run"
verdict: "unknown"
reason_code: "missing_tool"
```

`missing_tool` is a reason code accompanying a §44.2 status, not a status.

**Second: it forbids something §44 requires.** Its Premade Instructions read:

> Model the verdict as absent when unaskable. An `Option<Verdict>` where `None`
> means 'not asked' is the whole design; a `Verdict::Unknown` variant would let a
> caller treat it as a result.

But §44.3's verdict vocabulary is `pass`, `fail`, `unknown`, and both of §44.4's
unaskable examples record `verdict: "unknown"` explicitly. An `Option<Verdict>`
with no `Unknown` variant cannot represent the specification's own examples.

## Decision

**The specification governs. The Warrant's text is amended to match it.**

### 1. Both vocabularies are implemented, and neither is folded into the other

`ExecutionStatus` has §44.2's six. `ReasonCode` has §96.4's ten plus
`zero_selected_tests`, which §44.4 uses and §96.4 does not enumerate.
`MIGRATION_CLASSES` is exactly §96.4's ten, in its order.

`ReasonCode::migration_target` maps each legacy class to the §44 triple it
becomes. It is a total match, so adding a class without deciding its migration
fails to compile rather than defaulting to `failed` — which is the collapse
§96.4 forbids.

### 2. `Verdict::Unknown` exists, and criterion 19 is enforced by conjunction

§99 criterion 19 — "unaskable gates cannot pass" — is what the Warrant's
instruction was reaching for, and §44.5 already delivers it without deleting a
variant the specification defines. Only one triple satisfies a required pass:

```yaml
askability: "askable"
execution_status: "completed"
verdict: "pass"
```

`satisfies_required_pass()` is that three-way conjunction, and the test
enumerates **all 36** triples rather than sampling, asserting that exactly one
passes. `Verdict::Unknown` is therefore safe: a caller cannot treat it as a
result, because no caller consults the verdict alone.

Incoherent combinations are refused at validation — `not_askable` with a verdict
of pass or fail, `not_askable` with an execution status that implies it ran,
`completed` with `unknown`, and an unaskable run carrying no reason code.

### 3. Askability is decided before any process is spawned

`askability_of` completes before `Command::output`, and it is the only source of
`not_askable`. After a spawn the code cannot reach `missing_tool`. This is
structural rather than disciplinary: deciding askability from a non-zero exit
code is exactly how "could not ask" becomes "failed".

## Rationale

**The specification is the contract; the Warrant is a plan for meeting it.** When
they disagree, amending the plan is the only move that does not quietly change
what was agreed. Implementing the Warrant's ten "execution statuses" would have
produced a system that reads §96.4's migration classes as runtime states, and
then cannot express `infrastructure_error` or `cancelled` at all.

**The instruction was right about the danger and wrong about the mechanism.** A
`Verdict::Unknown` that a caller can treat as a result IS a hazard. The defence
is that no caller consults the verdict alone — not that the variant is unspeakable.
Deleting it would have made §44.4's examples unrepresentable, which is a larger
failure than the one being avoided, and would have left the required-pass check
resting on a type rather than on §44.5's stated conjunction.

**Enumerating all 36 triples is the point.** Criterion 19 is a claim about a whole
space. Tested on three examples it is a claim about three examples. The test also
asserts the space is still 36 in size, so widening a vocabulary forces criterion
19 to be re-argued rather than silently re-run over a bigger space.

**`missing_crate` is honestly narrow.** It is reachable only when cargo or rustc
itself is absent. A gate invoking a crate that does not exist gets as far as
running cargo, so it returns `completed` + `fail` — correctly, since cargo was
asked and answered. Detecting a missing crate inside a successful cargo
invocation means parsing cargo's output, which is a gate's work, not the runner's.
This is recorded rather than papered over with a heuristic that would sometimes
report a real failure as an unknown.

## Alternatives Considered

- **Implement the Warrant as written.** Rejected: it contradicts §44.2 and §44.3,
  and would lose `infrastructure_error` and `cancelled` entirely.
- **Treat §96.4's ten as execution statuses and drop §44.2's six.** Rejected for
  the same reason, and it would make the runtime model unable to express a
  cancelled run.
- **Keep `Option<Verdict>` and map `unknown` to `None` at the edges.** Rejected:
  two representations of one state, with a lossy conversion between them, and
  §44.4's examples would round-trip incorrectly.
- **Amend the SAS instead of the Warrant.** Not mine to do. The SAS is the
  governing source; a disagreement with it is a finding to report, not a licence
  to edit.
- **Sample the triple space in tests.** Rejected. Sampling is how a second passing
  combination survives a refactor.

## Consequences

**Good.** Both vocabularies are total and checkable. "Could not ask" cannot become
"failed", and there is a test that says so per class. Criterion 19 holds over the
whole space, not over examples.

**Bad.** OW-WAR-0020's authored atoms had to be amended after drafting, which is
a contract revision and should be visible as one. The Warrant is `draft`, so no
authorization was invalidated — had it been authorized, this would have required
§28.4 re-authorization rather than an edit.

**Unchanged.** §44.5 remains the sole definition of a required pass.

## Validation

Watch for: a caller that reads `verdict` without the other two fields, which is
the hazard the original instruction correctly feared; `missing_crate` being
widened by a heuristic that guesses from cargo's exit code; the 36-triple test
being changed to a sample when a vocabulary grows; and any future Warrant text
that cites "the ten execution statuses", which is the phrase that started this.
