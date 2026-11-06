# Who writes the spec: a second study

Status: draft for the owner's review, 2026-11-06. Nothing here has run.

## The question

The adoption study gives every arm the same expert backlog, so it measures what a
tool costs to use. It can't say whether a tool helps turn a request into good work.
So "DeepSeek plus OpenWarrant" has really meant "DeepSeek plus an expert spec,
delivered through OpenWarrant".

This study separates the two:

1. How much of a weak worker's result comes from who wrote the spec?
2. Does a spec tool lift a weak author, or only carry a strong author's spec?

## Prediction, stated before any run

The author matters more than the tool, and a tool helps a weak author somewhat. This
is a guess. It's written down so the result can contradict it.

| Result | What it would mean |
|---|---|
| The author dominates, the tool barely matters | Strong model plans, cheap agents execute. OpenWarrant's job is to carry a spec faithfully and cheaply. |
| A tool lifts weak authors | Structure substitutes for intelligence. The strongest case for OpenWarrant. |
| Weak authors fail under every tool | A person or a strong model has to stay at the planning step. |

## Factors

| Factor | Levels |
|---|---|
| Author | `opus` (Opus 5.5), `deepseek` (V4.1 Flash), `backlog` (the study's existing text, the control linking this to the adoption study) |
| Tool | `plain`, `plans`, `speckit`, `beads`, `openwarrant` |
| Worker | `deepseek` in the pilot; `glm` joins in the full study |
| Feature | From the existing directed items, the hard ones first: T8, T9, T10, K7, K8, K9 |

Every author uses its tool's own standard path:

| Tool | Spec path |
|---|---|
| `plain` | A written description in the instruction file |
| `plans` | `PLAN.md` |
| `speckit` | `/specify`, then its plan and tasks steps |
| `beads` | Issues, with acceptance and dependencies |
| `openwarrant` | `war plan`, or the `/war-spec` flow, ending in a ticket |

## The brief: the design's main risk

The hidden tests check exact details, such as a block header of exactly
`YYYY-MM-DD..YYYY-MM-DD`. No author could guess those from a paragraph, so a
one-paragraph brief would fail every cell and measure nothing.

So each feature gets a **brief** with two parts:

- **The request:** one paragraph, what a product owner would write. Why it's wanted
  and what it does. No implementation guidance, sequencing or edge-case decisions.
- **The contract:** only the externally observable facts a test depends on: command
  names, flags, output formats. This is the minimum needed for a correct solution to
  be possible, and every arm gets it verbatim.

Briefs are derived from `backlog.toml`: the request from `summary` and `description`,
the contract from `acceptance` and `notes`, with design advice removed. They're frozen
and reviewed before any author runs. **Each one gets a sufficiency check:** a
reference solution must be possible from the brief alone. A strong reader, in a fresh
session, marks any hidden-test assertion the brief doesn't determine. An undetermined
assertion is either added to the contract or excluded from grading, and every such
decision is listed.

Edge cases are where authors should differ. An author may ask questions, and a
scripted customer answers from the item's existing `question` field. It answers only
what is asked. Questions asked are themselves a measure.

## Isolation

- An author sees the brief, the seed repository and its tool: never `backlog.toml`,
  `hidden/`, `reference/`, the other arms or any earlier spec.
- Opus authors run in fresh sessions in the same sandbox. The session designing this
  study has seen the tests and writes no spec.
- Workers see only the finished spec in that tool's own form, plus the instruction
  file. They aren't told who wrote it.
- Specs are stored by digest. A worker cell records which spec it ran.

## Worker settings

- Turn cap 60, not 25. Every recent opencode session hit the 24-turn cap, which hides
  real differences. The `backlog` control reruns at 60, so nothing compares across caps.
- Same prompt in every arm: "Do backlog item <id>."
- Sessions start from the seed plus the reference solutions of earlier items, as now.

## Measures, fixed before any result

**Primary:** the item's hidden tests pass.

**Secondary:**

- Total cost: spec phase plus worker phase, in dollars at list price, with Opus,
  DeepSeek and GLM each counted. Free endpoints are still priced, or the cheap arm
  flatters itself.
- Turns and tokens before first code edit.
- False completion: the tool says done and the hidden tests fail.
- Regressions.
- Spec size, questions asked, and how much of the contract the spec carried.

No measure changes after results are seen. An added measure is labelled exploratory.

## Replication

Spec quality varies by draft, so repeating one spec under-counts variance. Each
author × tool × feature gets **2 independently written specs**, each run by **2
workers**. Analysis treats the spec, not the worker run, as the unit of author effect.

## Size and stages

| Stage | Cells | Specs | Purpose |
|---|---|---|---|
| Dry run | Fixture agent | None | Harness works, isolation holds, no model called |
| Pilot | 3 authors × 5 tools × 2 features (T8, K8) × 2 specs × 2 workers = 120 | 40 | Is the signal big enough, and do the briefs work? |
| Full | The same × 6 features, adding GLM | 120 | Only if the pilot shows a usable signal |

The 40 pilot specs are all written by models: 20 by each of `opus` and `deepseek`. The
`backlog` control has one fixed text per tool and feature, so it runs 4 workers on that
text, which keeps it at the same 40 cells as each model author.

The pilot reports effect sizes with intervals, not significance claims.

**Stop rule:** if the pilot's `backlog` control passes under 50%, or if `opus` and
`deepseek` can't be told apart on any measure, stop and redesign rather than scale up.

## Cost

Workers cost nothing on the LithosAI endpoint today. Opus authoring is the only cash
spend: 20 pilot specs, at a guessed $0.50 to $1.50 each, is $10 to $30. The pilot
measures the real figure before the full stage is proposed. Hard cap for the pilot:
$30, from what remains of the $200 budget. API spend stops at the cap.

Elapsed time matters more: 14 opencode cells took about 50 minutes at 3 jobs, so 120
cells at the higher cap are a day or more of wall clock, with the machine shared.

## Harness work needed

1. A `spec` phase in `run.py`: run an author in the sandbox, capture what it wrote in
   the tool's own form, store it by digest.
2. A `seed` path taking a stored spec in place of `backlog.toml`, for each of the 5 tools.
3. Brief writing, with the sufficiency check, for T8 and K8.
4. The scripted customer.
5. Reinstall Beads 1.3.1 and Spec Kit v1.2.2 at their pins. `run.py prepare` doesn't.
6. Opus through the API, Claude Code as the author harness. DeepSeek through opencode.
7. Self-checks: isolation (an author can't reach `hidden/`), a fixture author, and a
   spec that fails on purpose to prove grading catches it.

## Threats to validity

- **Contract leakage:** the contract gives every author the hard facts, which narrows
  the gap between them. That makes this a conservative test of the author effect.
- **Author and worker share a model:** in `deepseek` × `deepseek`, a spec may suit its
  own reader. The full study's GLM worker addresses this.
- **Tool fit:** Spec Kit and `war plan` are built for agents with skills loaded. Under
  opencode they may behave differently than under Claude Code.
- **Two repositories:** results may not generalise.
- **One author per model:** Opus 5.5 stands for strong models and DeepSeek for weak
  ones. This doesn't describe the whole range.
- **The designer has seen the tests:** briefs are reviewed and sufficiency-checked in a
  fresh session, but they're still derived by someone who has.

## Decisions for the owner

1. Is the brief design acceptable: request plus contract, with exact formats given?
2. Is the $30 pilot cap right?
3. Should a person review the frozen briefs before any author runs? Recommended.
