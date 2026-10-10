// SPDX-License-Identifier: Apache-2.0
//! The compliance score (OW-WAR-0148 M17, decisions 21–23; docs/SCORE.md).
//!
//! A number from 1 to 1000 for how much of this repository's work is
//! tracked, tested, verified and approved, read from the repository alone:
//! its records, its tickets' tick ladder, its file ledger and its git
//! history. Seven dimensions, each with a fixed published weight
//! ([`WEIGHTS`], versioned as `oh.war/score-weights/v1`); a dimension earns
//! `weight × numerator ÷ denominator` points, in whole points, rounded down.
//!
//! **Unknown is never a pass.** A dimension that cannot be measured (a
//! shallow clone, no merged PRs yet, no done ticks yet) reads UNKNOWN and
//! earns 0. The score is the sum, at least 1; the level is derived from it
//! ([`LEVELS`]).
//!
//! **Never in a committed projection.** The commit and PR dimensions move
//! with every commit, so a committed score would be stale the moment it was
//! committed. `war status` computes it on read; `war admin compile` writes
//! the badge, report, in-toto statement and trend under `.openwarrant/score/`,
//! which ignores itself.

pub mod floor;
pub mod publish;

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use crate::go::git::git;
use crate::repo::Repository;
use crate::vfs as fs;

/// The score's own schema family.
pub const SCHEMA: &str = "oh.war/score/v1";
/// The weights' version. Changing a weight, a dimension or a level threshold
/// is a new version (`v2`), never an edit of this one.
pub const WEIGHTS_SCHEMA: &str = "oh.war/score-weights/v1";
/// The in-toto predicate the statement carries.
pub const PREDICATE_TYPE: &str = "https://openwarrant.dev/attestation/work-score/v1";
/// in-toto Statement v1.
pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
/// Commits looked at, most recent first, unless `[score] window` says.
pub const DEFAULT_WINDOW: u32 = 500;

/// One dimension's published weight.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Weight {
    pub id: &'static str,
    pub weight: u32,
    pub measures: &'static str,
}

/// The weights, `oh.war/score-weights/v1`. They sum to 1000.
pub const WEIGHTS: [Weight; 7] = [
    Weight {
        id: "commits",
        weight: 200,
        measures: "commits that cite a Warrant this repository holds (`Warrant:` trailer)",
    },
    Weight {
        id: "prs",
        weight: 150,
        measures: "merged PRs whose merge, or any commit in them, cites a Warrant",
    },
    Weight {
        id: "ledger",
        weight: 100,
        measures: "files changed in the window that have a ledger atom",
    },
    Weight {
        id: "documents",
        weight: 100,
        measures: "development documents a type governs (typed / total)",
    },
    Weight {
        id: "tested",
        weight: 150,
        measures: "done ticks at observed or above",
    },
    Weight {
        id: "verified",
        weight: 150,
        measures: "done ticks at independent or above",
    },
    Weight {
        id: "approved",
        weight: 150,
        measures: "done ticks signed, and directory Warrants with an authorization record",
    },
];

/// `(level, name, least score)`, weakest first.
pub const LEVELS: [(u8, &str, u32); 5] = [
    (1, "untracked", 1),
    (2, "tracked", 200),
    (3, "tested", 400),
    (4, "verified", 600),
    (5, "approved", 800),
];

/// Whether a dimension was measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Measured,
    /// Could not be measured: earns 0, and is never a pass.
    Unknown,
}

/// One row of the scorecard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Dimension {
    pub id: String,
    pub weight: u32,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numerator: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denominator: Option<u64>,
    pub points: u32,
    /// What was counted, or why it could not be.
    pub detail: String,
}

/// The derived level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Level {
    pub number: u8,
    pub name: String,
}

/// What the commit and PR dimensions looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Window {
    /// At most this many commits, most recent first.
    pub limit: u32,
    /// `[adoption] baseline`: commits before it are not counted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
}

/// The score, `oh.war/score/v1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Score {
    pub schema: String,
    pub weights: String,
    pub score: u32,
    pub level: Level,
    pub dimensions: Vec<Dimension>,
    /// What would raise the score most, first.
    pub next_steps: Vec<String>,
    /// HEAD when the score was measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub window: Window,
}

/// `[score]` in openwarrant.toml.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub window: u32,
    /// The CI floor: `war check --pr` refuses a PR that lowers the level.
    pub floor: bool,
    /// `war admin compile` writes `.openwarrant/score/`.
    pub compile: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window: DEFAULT_WINDOW,
            floor: false,
            compile: true,
        }
    }
}

impl Config {
    /// # Errors
    /// A `[score]` table that does not read.
    pub fn read(root: &Utf8Path) -> Result<Self, String> {
        match fs::read_to_string(root.join(crate::init::CONFIG_FILE)) {
            Ok(t) => Self::from_text(&t),
            Err(_) => Ok(Self::default()),
        }
    }

    /// # Errors
    /// See [`Self::read`].
    pub fn from_text(text: &str) -> Result<Self, String> {
        let doc: toml::Table = text
            .parse()
            .map_err(|e| format!("openwarrant.toml does not parse: {e}"))?;
        let mut out = Self::default();
        let Some(t) = doc.get("score") else {
            return Ok(out);
        };
        let t = t
            .as_table()
            .ok_or_else(|| "[score] is not a table".to_owned())?;
        for (k, v) in t {
            match k.as_str() {
                "window" => {
                    let n = v
                        .as_integer()
                        .ok_or_else(|| "[score] window is not a number".to_owned())?;
                    if !(1..=100_000).contains(&n) {
                        return Err(format!(
                            "[score] window = {n}: it is 1 to 100000 commits; the default is {DEFAULT_WINDOW}"
                        ));
                    }
                    out.window = u32::try_from(n).unwrap_or(DEFAULT_WINDOW);
                }
                "floor" => {
                    out.floor = v
                        .as_bool()
                        .ok_or_else(|| "[score] floor is not true or false".to_owned())?;
                }
                "compile" => {
                    out.compile = v
                        .as_bool()
                        .ok_or_else(|| "[score] compile is not true or false".to_owned())?;
                }
                other => {
                    return Err(format!(
                        "[score] {other} is not a key war reads: window, floor, compile"
                    ));
                }
            }
        }
        Ok(out)
    }
}

// ---- inputs -------------------------------------------------------------------------

/// What the git dimensions counted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    pub head: String,
    pub commits: u64,
    pub commits_citing: u64,
    pub prs: u64,
    pub prs_citing: u64,
    pub files: u64,
    pub files_with_ledger: u64,
}

/// Every count the score is computed from. [`compute`] is a pure function
/// of this, so the same repository always scores the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inputs {
    pub history: Result<History, String>,
    pub documents: Option<(u64, u64)>,
    /// `(claimed, observed, independent, signed)` over every done tick.
    pub ticks: (u64, u64, u64, u64),
    /// Directory Warrants, and those with an authorization record.
    pub warrants: (u64, u64),
    pub window: Window,
}

fn is_squash_pr(subject: &str) -> bool {
    subject
        .trim_end()
        .strip_suffix(')')
        .and_then(|s| s.rsplit_once("(#"))
        .is_some_and(|(_, n)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// Read the git dimensions' counts.
///
/// # Errors
/// Why the history cannot be read: not a git checkout, no commits, a
/// shallow clone (whose history is incomplete), or a hosted run.
pub fn history(
    repo: &Repository,
    cfg: &Config,
    known: &BTreeMap<String, String>,
) -> Result<History, String> {
    if fs::is_hosted() {
        return Err("a hosted run reads no git history".to_owned());
    }
    let root = &repo.root;
    let head = git(root, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]);
    if !head.ok {
        return Err("no commit yet, or not a git checkout".to_owned());
    }
    let head = head.stdout.trim().to_owned();
    let shallow = git(root, &["rev-parse", "--is-shallow-repository"]);
    if shallow.ok && shallow.stdout.trim() == "true" {
        return Err(
            "a shallow clone: its history is incomplete (`git fetch --unshallow` measures it)"
                .to_owned(),
        );
    }
    let range = baseline(repo).map_or_else(|| "HEAD".to_owned(), |b| format!("{b}..HEAD"));
    let n = format!("-n{}", cfg.window);
    let log = git(
        root,
        &["log", &n, "--format=%H%x1f%P%x1f%B%x1e", &range, "--"],
    );
    if !log.ok {
        return Err(format!("git log did not answer: {}", log.why()));
    }
    let mut h = History {
        head,
        ..History::default()
    };
    for rec in log.stdout.split('\u{1e}') {
        let mut f = rec.trim_start_matches('\n').splitn(3, '\u{1f}');
        let (Some(_sha), Some(parents), Some(msg)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        let parents: Vec<&str> = parents.split_whitespace().collect();
        let subject = msg.lines().next().unwrap_or_default();
        let cites = crate::ledger::cited_warrant(msg, known).is_some();
        if parents.len() <= 1 {
            h.commits += 1;
            h.commits_citing += u64::from(cites);
            if is_squash_pr(subject) {
                h.prs += 1;
                h.prs_citing += u64::from(cites);
            }
        } else if subject.starts_with("Merge pull request #") {
            h.prs += 1;
            let inner = if cites {
                true
            } else {
                let r = git(
                    root,
                    &[
                        "log",
                        "-n500",
                        "--format=%B%x1e",
                        &format!("{}..{}", parents[0], parents[1]),
                        "--",
                    ],
                );
                r.ok && r
                    .stdout
                    .split('\u{1e}')
                    .any(|m| crate::ledger::cited_warrant(m, known).is_some())
            };
            h.prs_citing += u64::from(inner);
        }
    }
    // The files the window's commits changed, as HEAD still has them.
    let lcfg = crate::ledger::Config::read(root)
        .unwrap_or_default()
        .scoped(repo);
    let names = git(
        root,
        &[
            "-c",
            "core.quotePath=false",
            "log",
            &n,
            "--no-merges",
            "--name-only",
            "--diff-filter=d",
            "--format=",
            &range,
            "--",
        ],
    );
    let tracked = git(root, &["ls-files", "-z"]);
    if names.ok && tracked.ok {
        let at_head: BTreeSet<&str> = tracked.stdout.split('\0').collect();
        let changed: BTreeSet<&str> = names
            .stdout
            .lines()
            .map(str::trim)
            .filter(|p| !p.is_empty() && at_head.contains(p) && !crate::ledger::excluded(&lcfg, p))
            .collect();
        h.files = changed.len() as u64;
        h.files_with_ledger = changed
            .iter()
            .filter(|p| fs::is_file(crate::ledger::atom_path(repo, &lcfg, p)))
            .count() as u64;
    }
    Ok(h)
}

/// `[adoption] baseline`, when it is an ancestor of HEAD.
fn baseline(repo: &Repository) -> Option<String> {
    let b = repo.config.adoption.as_ref()?.baseline.clone();
    git(&repo.root, &["merge-base", "--is-ancestor", &b, "HEAD"])
        .ok
        .then_some(b)
}

/// Read every input.
#[must_use]
pub fn measure(repo: &Repository) -> Inputs {
    let cfg = Config::read(&repo.root).unwrap_or_default();
    let known = crate::ledger::warrant_titles(repo);
    let corpus = crate::corpus::Corpus::new(repo);
    let docs = corpus.documents().coverage();
    let ticks = crate::ticket::Store::open(repo, None)
        .ok()
        .and_then(|s| crate::ticket::ladder::tracker(&s).ok())
        .map_or((0, 0, 0, 0), |t| {
            (
                t.counts.claimed as u64,
                t.counts.observed as u64,
                t.counts.independent as u64,
                t.counts.signed as u64,
            )
        });
    let dirs = repo.warrant_dirs().unwrap_or_default();
    let approved = dirs
        .iter()
        .filter(|d| repo.load_authorization(d).ok().flatten().is_some())
        .count() as u64;
    Inputs {
        history: history(repo, &cfg, &known),
        documents: Some((docs.typed as u64, docs.total as u64)),
        ticks,
        warrants: (dirs.len() as u64, approved),
        window: Window {
            limit: cfg.window,
            baseline: baseline(repo),
        },
    }
}

// ---- the score ----------------------------------------------------------------------

fn points(weight: u32, num: u64, den: u64) -> u32 {
    if den == 0 {
        return 0;
    }
    u32::try_from(u64::from(weight) * num.min(den) / den).unwrap_or(weight)
}

/// The level a score reaches.
#[must_use]
pub fn level_of(score: u32) -> Level {
    let (n, name, _) = LEVELS
        .iter()
        .rev()
        .find(|(_, _, least)| score >= *least)
        .copied()
        .unwrap_or(LEVELS[0]);
    Level {
        number: n,
        name: name.to_owned(),
    }
}

/// The score over `inputs`.
#[must_use]
pub fn compute(inputs: &Inputs) -> Score {
    let measured = |id: &str, num: u64, den: u64, detail: String| -> Dimension {
        let w = WEIGHTS.iter().find(|w| w.id == id).map_or(0, |w| w.weight);
        if den == 0 {
            return Dimension {
                id: id.to_owned(),
                weight: w,
                status: Status::Unknown,
                numerator: None,
                denominator: None,
                points: 0,
                detail,
            };
        }
        Dimension {
            id: id.to_owned(),
            weight: w,
            status: Status::Measured,
            numerator: Some(num),
            denominator: Some(den),
            points: points(w, num, den),
            detail,
        }
    };
    let (c, o, i, s) = inputs.ticks;
    let ticks = c + o + i + s;
    let (dirs, authorized) = inputs.warrants;
    let mut dims = Vec::new();
    match &inputs.history {
        Ok(h) => {
            dims.push(measured(
                "commits",
                h.commits_citing,
                h.commits,
                if h.commits == 0 {
                    "no commit in the window".to_owned()
                } else {
                    format!(
                        "{} of {} commit(s) cite a Warrant",
                        h.commits_citing, h.commits
                    )
                },
            ));
            dims.push(measured(
                "prs",
                h.prs_citing,
                h.prs,
                if h.prs == 0 {
                    "no merged PR found in the window (a `Merge pull request #N` merge or a \
                     `(#N)` squash)"
                        .to_owned()
                } else {
                    format!("{} of {} merged PR(s) cite a Warrant", h.prs_citing, h.prs)
                },
            ));
            dims.push(measured(
                "ledger",
                h.files_with_ledger,
                h.files,
                if h.files == 0 {
                    "no file changed in the window".to_owned()
                } else {
                    format!(
                        "{} of {} changed file(s) have a ledger atom",
                        h.files_with_ledger, h.files
                    )
                },
            ));
        }
        Err(why) => {
            for id in ["commits", "prs", "ledger"] {
                dims.push(measured(id, 0, 0, why.clone()));
            }
        }
    }
    let (typed, total) = inputs.documents.unwrap_or((0, 0));
    dims.push(measured(
        "documents",
        typed,
        total,
        if total == 0 {
            "no development document indexed".to_owned()
        } else {
            format!("{typed} of {total} document(s) typed")
        },
    ));
    let no_ticks = || "no done tick yet".to_owned();
    dims.push(measured(
        "tested",
        o + i + s,
        ticks,
        if ticks == 0 {
            no_ticks()
        } else {
            format!("{} of {ticks} done tick(s) observed or above", o + i + s)
        },
    ));
    dims.push(measured(
        "verified",
        i + s,
        ticks,
        if ticks == 0 {
            no_ticks()
        } else {
            format!("{} of {ticks} done tick(s) independent or above", i + s)
        },
    ));
    dims.push(measured(
        "approved",
        s + authorized,
        ticks + dirs,
        if ticks + dirs == 0 {
            "no done tick and no directory Warrant".to_owned()
        } else {
            format!(
                "{s} of {ticks} done tick(s) signed; {authorized} of {dirs} directory Warrant(s) \
                 authorized"
            )
        },
    ));
    let sum: u32 = dims.iter().map(|d| d.points).sum();
    let score = sum.clamp(1, 1000);
    let level = level_of(score);
    let next_steps = next_steps(&dims);
    Score {
        schema: SCHEMA.to_owned(),
        weights: WEIGHTS_SCHEMA.to_owned(),
        score,
        level,
        dimensions: dims,
        next_steps,
        commit: inputs.history.as_ref().ok().map(|h| h.head.clone()),
        window: inputs.window.clone(),
    }
}

/// What to do next, most points first: at most five.
#[must_use]
pub fn next_steps(dims: &[Dimension]) -> Vec<String> {
    let mut order: Vec<&Dimension> = dims.iter().filter(|d| d.points < d.weight).collect();
    order.sort_by(|a, b| {
        (b.weight - b.points)
            .cmp(&(a.weight - a.points))
            .then(a.id.cmp(&b.id))
    });
    order
        .into_iter()
        .take(5)
        .map(|d| {
            let gain = d.weight - d.points;
            let how = match d.id.as_str() {
                "commits" => {
                    "cite the Warrant in each commit message with a `Warrant: <id>` trailer"
                }
                "prs" => "put a `Warrant: <id>` line in each pull request's description",
                "ledger" => {
                    "record why files changed after each commit: `war admin ledger record`"
                }
                "documents" => {
                    "give each untyped document a type: `war plan type <file> <type>`"
                }
                "tested" => "tick items with their checks: `war done <id> --check`",
                "verified" => {
                    "ask someone else to verify ticks: `war evidence verify <id>` writes the request"
                }
                _ => "a human signs off done ticks, a human's step: `war sign <id> --ssh-sign`",
            };
            let state = match d.status {
                Status::Unknown => format!("UNKNOWN, {}", d.detail),
                Status::Measured => d.detail.clone(),
            };
            format!("{}: {how} (up to +{gain}; {state})", d.id)
        })
        .collect()
}

impl Score {
    /// The block `war status` prints after the projection.
    #[must_use]
    pub fn render(&self) -> String {
        let mut md = format!(
            "## Score\n\n{} / 1000, level {} ({}). Weights {}; an UNKNOWN dimension earns 0.\n\n",
            self.score, self.level.number, self.level.name, self.weights
        );
        md.push_str("| dimension | points | weight | measured |\n|---|---:|---:|---|\n");
        for d in &self.dimensions {
            let m = match d.status {
                Status::Measured => d.detail.clone(),
                Status::Unknown => format!("UNKNOWN: {}", d.detail),
            };
            md.push_str(&format!(
                "| {} | {} | {} | {m} |\n",
                d.id, d.points, d.weight
            ));
        }
        if !self.next_steps.is_empty() {
            md.push_str("\nNext steps:\n\n");
            for s in &self.next_steps {
                md.push_str(&format!("- {s}\n"));
            }
        }
        md
    }
}

/// One line on the trend `war admin compile` journals, when it has two
/// points or more: first and last score, and how many compiles between.
#[must_use]
pub fn trend_summary(root: &Utf8Path) -> Option<String> {
    let t = publish::trend(root);
    let (first, last) = (t.first()?, t.last()?);
    if t.len() < 2 {
        return None;
    }
    let f = first.get("score")?.as_u64()?;
    let l = last.get("score")?.as_u64()?;
    let delta = i64::try_from(l).ok()? - i64::try_from(f).ok()?;
    Some(format!(
        "Trend: {f} -> {l} ({delta:+}) over {} compile(s), from {} (.openwarrant/score/trend.jsonl)",
        t.len(),
        first
            .get("at")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?")
    ))
}

/// The weights document, `oh.war/score-weights/v1`.
#[must_use]
pub fn weights_document() -> serde_json::Value {
    serde_json::json!({
        "schema": WEIGHTS_SCHEMA,
        "total": WEIGHTS.iter().map(|w| w.weight).sum::<u32>(),
        "dimensions": WEIGHTS,
        "levels": LEVELS.iter().map(|(n, name, least)| serde_json::json!({"number": n, "name": name, "least": least})).collect::<Vec<_>>(),
        "unknown": "a dimension that cannot be measured reads UNKNOWN and earns 0",
        "points": "weight * numerator / denominator, rounded down; the score is the sum, at least 1",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> Inputs {
        Inputs {
            history: Ok(History {
                head: "a".repeat(40),
                commits: 10,
                commits_citing: 5,
                prs: 0,
                prs_citing: 0,
                files: 4,
                files_with_ledger: 1,
            }),
            documents: Some((3, 4)),
            ticks: (2, 1, 1, 0),
            warrants: (0, 0),
            window: Window {
                limit: DEFAULT_WINDOW,
                baseline: None,
            },
        }
    }

    #[test]
    fn the_weights_sum_to_a_thousand_and_the_levels_climb() {
        assert_eq!(WEIGHTS.iter().map(|w| w.weight).sum::<u32>(), 1000);
        assert!(LEVELS.windows(2).all(|w| w[0].2 < w[1].2));
        assert_eq!(level_of(1).number, 1);
        assert_eq!(level_of(599).name, "tested");
        assert_eq!(level_of(1000).number, 5);
    }

    #[test]
    fn the_score_is_deterministic_and_an_unknown_dimension_earns_nothing() {
        let a = compute(&inputs());
        assert_eq!(a, compute(&inputs()));
        let prs = a.dimensions.iter().find(|d| d.id == "prs").unwrap();
        assert_eq!((prs.status, prs.points), (Status::Unknown, 0));
        // 100 + 0 + 25 + 75 + 75 + 37 + 0 (ticks: 4, approved 0 of 4).
        assert_eq!(a.score, 312);
        assert_eq!(a.level.name, "tracked");
        // Every dimension unknown is still a score of 1, never 0 and never more.
        let none = compute(&Inputs {
            history: Err("shallow".to_owned()),
            documents: None,
            ticks: (0, 0, 0, 0),
            warrants: (0, 0),
            window: Window::default(),
        });
        assert_eq!(none.score, 1);
        assert!(none.dimensions.iter().all(|d| d.status == Status::Unknown));
        assert!(none.next_steps.iter().all(|s| s.contains("UNKNOWN")));
    }

    #[test]
    fn the_published_table_matches_the_weights() {
        let doc = include_str!("../../../../docs/SCORE.md");
        for w in WEIGHTS {
            let row = format!("| `{}` | {} |", w.id, w.weight);
            assert!(doc.contains(&row), "docs/SCORE.md lacks {row}");
        }
        for (n, name, least) in LEVELS {
            let row = format!("| {n} | {name} | {least} |");
            assert!(doc.contains(&row), "docs/SCORE.md lacks {row}");
        }
    }

    #[test]
    fn a_squash_subject_is_a_pr() {
        assert!(is_squash_pr("Fix the thing (#42)"));
        assert!(!is_squash_pr("Fix (#) the thing"));
        assert!(!is_squash_pr("Fix (#4a)"));
    }

    #[test]
    fn a_bad_score_table_is_refused() {
        assert!(Config::from_text("[score]\nwindow = 0\n").is_err());
        assert!(Config::from_text("[score]\nfloor = \"yes\"\n").is_err());
        assert!(Config::from_text("[score]\nflor = true\n").is_err());
        assert!(Config::from_text("[score]\nfloor = true\n").unwrap().floor);
    }
}
