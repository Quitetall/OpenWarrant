# The compliance score

`war status` ends with a score from 1 to 1000: how much of this repository's
work is tracked, tested, verified and approved, read from the repository
alone (OW-WAR-0148 M17, decisions 21–23). Beside it are a scorecard by
dimension, a level derived from the score, and the next steps that would
raise it most.

```sh
war status                       # the score block follows the projection
war status --json                # result.compliance: oh.war/score/v1
war admin score                  # the score alone
war admin score --weights        # the published weights, oh.war/score-weights/v1
war admin score --in-toto        # an in-toto Statement v1 carrying it
war admin score --verify FILE    # check a statement against the shape below
war admin score --at main        # score a commit, in a throwaway worktree
war check --floor main           # refuse when the level drops below main's
```

## The weights: `oh.war/score-weights/v1`

Each dimension earns `weight × numerator ÷ denominator` points, rounded down
to a whole point. The score is the sum, at least 1.

**A dimension that cannot be measured reads UNKNOWN and earns 0.** It is
never counted as a pass: a shallow clone, a repository with no merged PR
yet, or one with no done tick yet loses those points until they can be
read.

| dimension | weight | numerator / denominator | UNKNOWN when |
|---|---|---|---|
| `commits` | 200 | commits in the window that cite a Warrant this repository holds (`Warrant: <id>` trailer) / commits in the window (merges left out) | no commit, a shallow clone, not a git checkout |
| `prs` | 150 | merged PRs that cite a Warrant, in the merge message or any commit the merge brought in / merged PRs in the window (`Merge pull request #N` merges and `(#N)` squash commits) | no merged PR in the window, a shallow clone |
| `ledger` | 100 | files changed in the window, still in HEAD, that have a ledger atom (docs/LEDGER.md) / those files | no file changed, a shallow clone |
| `documents` | 100 | development documents a type governs / documents indexed (`document_coverage`, M18) | no document indexed |
| `tested` | 150 | done ticks at observed or above / done ticks | no done tick |
| `verified` | 150 | done ticks at independent or above / done ticks | no done tick |
| `approved` | 150 | done ticks signed, plus directory Warrants with an authorization record / done ticks plus directory Warrants | neither exists |

The weights sum to 1000. A tick's level is the one the tick ladder believes
(docs/TICKETS.md): a marker no record backs reads one level down. "Cites a
Warrant" means a `Warrant:` (or `Warrants:`) line whose id names a directory
Warrant or a ticket in this checkout; an id that names nothing earns
nothing.

The window is the last `[score] window` commits (500 by default) reachable
from HEAD; with `[adoption] baseline`, commits before the baseline are not
counted, since nothing governed them.

## Levels

| level | name | least score |
|---|---|---|
| 1 | untracked | 1 |
| 2 | tracked | 200 |
| 3 | tested | 400 |
| 4 | verified | 600 |
| 5 | approved | 800 |

## Versioning

The weights are versioned like a schema. Changing a weight, adding or
removing a dimension, or moving a level's threshold is
`oh.war/score-weights/v2`, published beside v1; v1 is never edited. Every
score names the version it was computed with (`weights`). A unit test holds
this page's two tables to the constants in `crates/openwarrant-cli/src/score/mod.rs`.

## Where the score lives

Never in a committed projection. The `commits` and `prs` dimensions move with
every commit, so a score committed under `generated/` would be stale the
moment it was committed, and `war check --generated` would fail on every
branch. Instead:

- `war status` computes it on read (`compliance` in `--json`).
- `war admin compile` writes, under `.openwarrant/score/` (ignored by the
  directory's own `.gitignore`):
  - `badge.svg`, and `badge.json` in the shields.io endpoint form;
  - `report.md` and `report.html`, the shareable report page;
  - `statement.intoto.json`, the in-toto statement;
  - `score.json`, the score itself;
  - `trend.jsonl`, one line per compile that saw a new commit or a new score:
    `{at, commit, score, level}`. `war status` prints the trend's first and
    last points.
- The GitHub Action (`.github/actions/openwarrant-check`) adds the score and
  report to the job summary and sets `score` and `level` outputs; publish
  `badge.svg` or `badge.json` from there to wherever your README reads it.

`[score] compile = false` turns the compile step off.

## The CI floor: never drop a level

Opt-in. With `[score] floor = true` in openwarrant.toml **on the base
branch**, `war check --pr <n>` (and so the Action) also scores the PR's base
commit in a throwaway worktree and refuses the PR when the PR's level is
below the base's (`score.floor`). Reading the flag from the base means a PR
cannot turn its own floor off. `war check --floor <rev>` runs the same
comparison locally against any commit.

A base that cannot be scored (the commit is not in the clone; use
`fetch-depth: 0`), or a dimension that was measured at the base and is
UNKNOWN at the head, makes the floor UNKNOWN: never a pass.

## The in-toto statement

`war admin score --in-toto` prints an unsigned
[in-toto Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md):

```json
{
  "_type": "https://in-toto.io/Statement/v1",
  "subject": [
    {"name": "<[project] repository_url, or git+local:<namespace>>",
     "digest": {"gitCommit": "<the measured commit, 40 hex>"}}
  ],
  "predicateType": "https://openwarrant.dev/attestation/work-score/v1",
  "predicate": {
    "weights": "oh.war/score-weights/v1",
    "score": 612,
    "level": {"number": 4, "name": "verified"},
    "dimensions": [
      {"id": "commits", "weight": 200, "status": "measured",
       "numerator": 41, "denominator": 50, "points": 164,
       "detail": "41 of 50 commit(s) cite a Warrant"},
      {"id": "prs", "weight": 150, "status": "unknown", "points": 0,
       "detail": "no merged PR found in the window ..."}
    ],
    "window": {"limit": 500},
    "producer": {"name": "war", "version": "1.0.0-alpha.2"}
  }
}
```

The shape, which `war admin score --verify FILE` checks and the schema pack
publishes as `schemas/oh.war/score-statement/v1.json`:

- `_type` is `https://in-toto.io/Statement/v1`;
- `subject` is a non-empty list, each with a `name` and a 40-hex
  `digest.gitCommit` (the subject form SLSA Source tooling reads for a
  revision);
- `predicateType` is `https://openwarrant.dev/attestation/work-score/v1`;
- `predicate.weights` is `oh.war/score-weights/v1`, and `dimensions` lists
  exactly the seven dimensions above, in that order, with their weights;
- an `unknown` dimension has no counts and 0 points; a `measured` one has
  `numerator ≤ denominator` and exactly `weight × numerator ÷ denominator`
  points;
- `score` is the sum (at least 1), and `level` is the level that score
  reaches.

A statement that departs from any of these is refused, by line.

**How Scorecard v6 and SLSA Source tooling consume it.** Both read in-toto
Statements, and a custom `predicateType` is how in-toto carries evidence no
predicate in its registry describes. Sign the statement with your
attestation tooling (a DSSE envelope, e.g. `cosign attest-blob` or GitHub
artifact attestations) and store it beside your Scorecard or SLSA Source
attestations for the same commit. What was not established when this was
written: the exact predicate type Scorecard v6 uses for its own evidence,
and whether its policy engine reads foreign predicates. This statement does
not claim to be a Scorecard check or a SLSA level.
