---
name: war-spec
description: Turn the current conversation into an oh.war/draft-proposal/v2 and validate it, with no interview, only synthesis of what was already discussed. Use after a grilling session, or when the user says to-spec, write it up, make a Warrant from this.
disable-model-invocation: true
---

# war-spec

After mattpocock/skills `engineering/to-spec` (3cca18b, MIT). His spec goes
to an issue tracker; ours becomes a Warrant's atoms through the §74.4
gauntlet. Do not interview; synthesize.

## Process

1. Read `CONTEXT.md` and use its words. Read the ADRs in the area
   (`docs/adr/atoms/`) and respect them. Explore the code you will touch.
2. **Seams.** Name the seams the work will be tested at, existing seams
   first, the highest seam possible, ideally one. Confirm them with the user
   in one message; they become the obligations' scopes.
3. Write the proposal as one JSON file, `oh.war/draft-proposal/v2`, with
   these operations and nothing else:

| his template | our atom |
|---|---|
| Problem Statement, Solution | `10-intent.md`: Problem, Desired Outcome, Scope, Non-goals, SAS and Roadmap Traceability |
| User Stories | the Desired Outcome as numbered stories; every story an obligation can point at |
| Implementation Decisions | `40-work-order.md`: Deliverables (each a path), Frozen Surfaces, Premade Instructions; a durable choice is `propose_adr`, never a paragraph |
| Testing Decisions | `60-assurance.md`: one `OBL-nnn` per seam with its gate and evidence; Gate Adequacy with the adversarial question |
| Out of Scope | Non-goals in the intent |
| (implicit) | `45-milestones.yaml`: stages with `context_sections` and `budget_tokens`; `add_relation` for `roadmap://` |

No file paths in prose that will go stale, except the deliverables list,
which is what gets pinned. A snippet from a prototype that encodes a decision
more precisely than prose may be inlined, trimmed to the decision.

4. Validate, and only validate:

```bash
war plan --proposal <file>
```

Fix what steps 1 to 4 refuse. `plan.applicable` is the end of this skill.

5. Hand over the path. `--apply --reviewed` is the human's step (§74.4 5 to 6).

## Never

A skill never signs, and never claims a step it did not run.
