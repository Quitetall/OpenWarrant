// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war telemetry` — §94's baseline, §95's untracked-work candidates, §100's metrics.
//!
//! # Why a measure may not be zero
//!
//! §94 lists eighteen measures and `TELEMETRY_MEASURES` transcribes them. Ten of
//! them cannot be taken from a git repository, because they are properties of an
//! authoring SESSION that nothing instruments: how many minutes a human spent,
//! how many clarifying questions were asked, how long the wall clock ran.
//!
//! Recording those as `0` would be the worst available answer. Zero is a
//! measurement — it says the thing was looked for and not found — and a reader
//! summing "clarification count: 0" across a corpus would conclude the process
//! needs no clarification. So an unmeasurable measure carries
//! `not_measurable_yet` and a reason, and OW-WAR-0041's OBL-002 asks for exactly
//! that: "a grep for `= 0` across the recorded baseline returns no measure that
//! was never actually taken."
//!
//! # Why no §100 deltas
//!
//! §100 is a list of things the system succeeds by REDUCING or INCREASING. A
//! delta needs two measurements. This is the first, so every §100 metric records
//! a baseline value or `no baseline`, and the artifact states no direction of
//! travel at all. Claiming an improvement from one sample is the substitution
//! §40.7 forbids, and it is the specific temptation a success-metrics section
//! creates.

use std::collections::BTreeMap;

use openwarrant_core::lifecycle::{DERIVED_METRICS, TELEMETRY_MEASURES};

use crate::repo::{RepoError, Repository};

/// One §94 measure: a number, or a stated reason it cannot be taken.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Measure {
    Taken {
        value: u64,
        method: String,
    },
    /// Deliberately not `value: 0`. See the module docs.
    NotYet {
        not_measurable_yet: String,
    },
}

impl Measure {
    fn taken(value: u64, method: &str) -> Self {
        Self::Taken {
            value,
            method: method.to_owned(),
        }
    }
    fn not_yet(reason: &str) -> Self {
        Self::NotYet {
            not_measurable_yet: reason.to_owned(),
        }
    }
}

/// §94 + §95 + §100 at one commit. The whole artifact.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Baseline {
    pub schema: String,
    /// The commit this was taken at. OBL-001: "a baseline taken after tuning is
    /// not a baseline", so the commit is what makes it checkable.
    pub commit: String,
    pub measures: BTreeMap<String, Measure>,
    pub derived: BTreeMap<String, Measure>,
    /// §95 candidates. Diagnostic only — §95 says this "SHALL not fabricate a
    /// relationship after the fact without review".
    pub untracked_work_candidates: Vec<String>,
    /// §100's sixteen. Every entry is a baseline or the string `no baseline`;
    /// none is a delta.
    pub success_metrics: BTreeMap<String, String>,
}

/// §100's sixteen, verbatim: nine reduced, seven increased.
const SUCCESS_METRICS: [&str; 16] = [
    "context lost between humans and agents",
    "clarification turns",
    "untracked implementation decisions",
    "stale gate commands",
    "false completion claims",
    "manual document synchronization",
    "repeated context assembly",
    "human time per accepted unit",
    "post-resolution surprises",
    "stage completion rate",
    "evidence reuse",
    "gate reuse",
    "repair success",
    "traceability from SAS to artifact",
    "truthful unknown reporting",
    "ability to hand work between agents and humans",
];

/// Why each unmeasurable measure cannot be taken, stated once so the artifact
/// and this file cannot drift apart.
fn unmeasurable(measure: &str) -> Option<&'static str> {
    Some(match measure {
        "human authoring minutes" | "wall time" | "time to first usable artifact" => {
            "no instrumented authoring session; a git repository records commits, not elapsed \
             human or wall time"
        }
        "interview questions" | "clarification count" => {
            "§74.6's interview has never run — `war plan` has no agent on the other side of \
             the §75.2 seam, so no question has been asked to count"
        }
        "escalation count and class" => {
            "no §31 escalation has occurred; the count is unknown rather than zero because \
             nothing records escalations even when they do"
        }
        "replay, repair, restart" => {
            "no execution has been replayed, repaired or restarted, and no receipt store \
             exists to have recorded one"
        }
        "gate failure cause" => {
            "gate runs mint receipts but nothing aggregates causes across them, so a cause \
             distribution cannot be reported"
        }
        "compute and model cost" => "no cost is metered for any run in this repository",
        "reopenings" => {
            "no Warrant has been resolved, so none can have been reopened. This is a \
             consequence of §56.1 requirement 10, not of missing instrumentation"
        }
        "post-resolution escapes" => {
            "no Warrant has been resolved; an escape after resolution is undefined while the \
             count of resolutions is zero"
        }
        _ => return None,
    })
}

/// Take the baseline.
pub fn take(repo: &Repository, commit: &str) -> Result<Baseline, RepoError> {
    let dirs = repo.warrant_dirs()?;
    let loaded: Vec<_> = dirs
        .iter()
        .filter_map(|d| repo.load_warrant(d).ok())
        .collect();

    let amendments = dirs
        .iter()
        .filter_map(|d| d.join("amendments").read_dir_utf8().ok())
        .map(|e| e.filter_map(Result::ok).count() as u64)
        .sum();

    // An adequacy counterexample is an executed attack, and the count comes from
    // `adequacy::parse` — the SAME parser `war check` uses to decide whether a
    // review ran any. Counting bullets here with a private rule would have been
    // a second answer to one question, and a telemetry baseline that disagrees
    // with the checker about how many attacks exist is worse than no baseline.
    let counterexamples = loaded
        .iter()
        .filter_map(|l| l.basis.as_ref())
        .flat_map(|b| b.atoms.iter().filter(|a| a.role == "assurance"))
        .map(|a| {
            openwarrant_core::adequacy::parse(&String::from_utf8_lossy(&a.bytes))
                .executed_attacks
                .len() as u64
        })
        .sum();

    let gates_dir = repo.root.join(&repo.config.paths.gates);
    let gates_defined = gates_dir
        .read_dir_utf8()
        .map(|e| e.filter_map(Result::ok).count() as u64)
        .unwrap_or(0);
    let gate_citations = loaded
        .iter()
        .filter_map(|l| l.basis.as_ref())
        .flat_map(|b| b.atoms.iter())
        .map(|a| {
            String::from_utf8_lossy(&a.bytes)
                .lines()
                .filter(|l| l.contains("gate://"))
                .count() as u64
        })
        .sum();

    let untracked = untracked_candidates(repo)?;

    let mut measures = BTreeMap::new();
    for name in TELEMETRY_MEASURES {
        let m = match name {
            "amendments" => Measure::taken(
                amendments,
                "amendment records under docs/warrants/*/amendments/",
            ),
            "adequacy counterexamples" => Measure::taken(
                counterexamples,
                "executed attacks, via the same adequacy::parse `war check` uses",
            ),
            "untracked commits or artifacts" | "work completed outside WAR" => Measure::taken(
                untracked.len() as u64,
                "commits whose message cites no Warrant alias (§95 candidates)",
            ),
            "evidence and gate reuse" => {
                Measure::taken(gate_citations, "gate:// citations across all atoms")
            }
            "gate library reuse" => Measure::taken(
                gates_defined,
                "gate definitions in the registry; reuse is citations over definitions",
            ),
            "auto-authorizable fraction" => Measure::taken(
                0,
                "Warrants authorizable without human action: zero, and MEASURED zero — \
                 §56.1 requirement 10 is unmet for every Warrant, so none is auto-authorizable",
            ),
            other => match unmeasurable(other) {
                Some(reason) => Measure::not_yet(reason),
                // A measure this function does not classify is not silently
                // zeroed either: it is unmeasured until someone says how.
                None => Measure::not_yet(
                    "not classified by this build; no method has been stated for taking it",
                ),
            },
        };
        measures.insert(name.to_owned(), m);
    }

    // Every derived metric is a ratio over measures, and a ratio whose numerator
    // or denominator is unmeasurable is unmeasurable. None is asserted.
    let derived = DERIVED_METRICS
        .iter()
        .map(|d| {
            (
                (*d).to_owned(),
                Measure::not_yet(
                    "derived from measures that are not all takeable yet; a ratio over an \
                     unmeasured term is not a ratio",
                ),
            )
        })
        .collect();

    let success_metrics = SUCCESS_METRICS
        .iter()
        .map(|m| ((*m).to_owned(), "no baseline".to_owned()))
        .collect();

    Ok(Baseline {
        schema: "oh.war/telemetry-baseline/v1".to_owned(),
        commit: commit.to_owned(),
        measures,
        derived,
        untracked_work_candidates: untracked,
        success_metrics,
    })
}

/// §95 — commits touching tracked scope with no WAR relation.
///
/// Candidates, emphatically. §95 calls this "a diagnostic and governance
/// signal", and the sentence immediately after is the one that matters: it
/// "SHALL not fabricate a relationship after the fact without review". So this
/// lists and does not attach.
fn untracked_candidates(repo: &Repository) -> Result<Vec<String>, RepoError> {
    // UNBOUNDED on purpose. A `-n 400` ceiling was here and is a silent
    // truncation: past 400 commits the candidate count would quietly shrink,
    // `--verify` would fail against the committed baseline, and nothing would
    // say why. A measurement that gets smaller as the history grows is worse
    // than an expensive one.
    let out = std::process::Command::new("git")
        .args(["log", "--format=%h %s"])
        .current_dir(repo.root.as_std_path())
        .output()
        .map_err(|e| RepoError::Message(format!("git log: {e}")))?;
    if !out.status.success() {
        return Err(RepoError::Message(
            "git log failed; refusing to report zero untracked candidates from a failed \
             command, which would look identical to a clean history"
                .to_owned(),
        ));
    }
    // Matched against the SUBJECT, not the whole line, and against the alias
    // shape rather than a bare substring. `l.contains("OW-WAR-")` would exclude
    // a genuinely untracked commit whose subject merely quotes the prefix —
    // `fix: handle "OW-WAR-" in the parser` is untracked work about aliases, not
    // work under a Warrant.
    let cites_warrant = |subject: &str| {
        subject.split_whitespace().any(|w| {
            let w = w.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
            w.starts_with("OW-WAR-") && w.len() > "OW-WAR-".len()
                || w.starts_with("war://") && w.len() > "war://".len()
        })
    };
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| {
            let subject = l.split_once(' ').map_or("", |(_, s)| s);
            !cites_warrant(subject)
        })
        .map(ToOwned::to_owned)
        .collect())
}

/// §95 — attach a Warrant relation to an untracked-work candidate, with review.
///
/// The refusal is the point. §95 says this "SHALL not fabricate a relationship
/// after the fact without review", and `UntrackedWork::attach_relation` enforces
/// it by refusing an empty reviewer — a rule that shipped in alpha and that no
/// command has ever called, so the refusal had never run outside a unit test.
///
/// Attributing an orphan commit to whichever Warrant happened to be open would
/// turn a diagnostic into a fabrication, which is why the reviewer is required
/// rather than defaulted to the acting user.
pub fn attach(scope: &str, warrant: &str, reviewer: &str) -> Result<String, RepoError> {
    let mut work = openwarrant_core::lifecycle::UntrackedWork {
        scope: scope.to_owned(),
        carried_identifiers: Vec::new(),
        related_warrant: String::new(),
        reviewed_by: String::new(),
    };
    work.attach_relation(warrant, reviewer)
        .map_err(|e| RepoError::Message(format!("{e}")))?;
    Ok(format!(
        "{} related to {} (reviewed by {})",
        work.scope, work.related_warrant, work.reviewed_by
    ))
}

/// Render deterministically. Two runs at one commit must agree BYTE FOR BYTE —
/// which is checkable only because `--verify` compares exactly rather than
/// trimming. The maps are `BTreeMap`, so key order is the ordering of the keys
/// and not of a hash; switching to `HashMap` would break this silently.
pub fn render(b: &Baseline) -> Result<String, RepoError> {
    serde_json::to_string_pretty(b).map_err(|e| RepoError::Message(format!("{e}")))
}
