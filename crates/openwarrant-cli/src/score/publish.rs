// SPDX-License-Identifier: Apache-2.0
//! What the score publishes (OW-WAR-0148 M17, decision 23): the in-toto
//! statement, the badge, the report page and the trend, written by `war admin
//! compile` under `.openwarrant/score/` (which the directory ignores).
//!
//! # The in-toto statement
//!
//! An in-toto Statement v1 (`_type` `https://in-toto.io/Statement/v1`) whose
//! one subject is the repository at the measured commit
//! (`digest: {gitCommit: <sha>}`, the subject form SLSA Source tooling reads)
//! and whose `predicateType` is
//! `https://openwarrant.dev/attestation/work-score/v1`. The predicate is the
//! score: weights version, score, level, and every dimension with its counts
//! and points. [`verify`] checks a statement against that documented shape
//! and recomputes every number from the counts, so an UNKNOWN dimension that
//! claims points, or a score that is not the sum, is refused. The statement
//! is unsigned; signing it (a DSSE envelope) is the publisher's step.

use camino::Utf8PathBuf;
use serde_json::{Value, json};

use super::{PREDICATE_TYPE, STATEMENT_TYPE, Score, Status, WEIGHTS, WEIGHTS_SCHEMA};
use crate::repo::{RepoError, Repository};
use crate::vfs as fs;

/// Where compile writes, under the repository root.
pub const DIR: &str = ".openwarrant/score";

/// An in-toto Statement v1 carrying the score (`score-statement` in the
/// schema pack).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Statement {
    #[serde(rename = "_type")]
    pub type_: String,
    pub subject: Vec<Subject>,
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    pub predicate: Predicate,
}

/// The repository at the measured commit.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Subject {
    pub name: String,
    /// `gitCommit` → the full commit id.
    pub digest: std::collections::BTreeMap<String, String>,
}

/// The score, as the predicate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Predicate {
    pub weights: String,
    pub score: u32,
    pub level: super::Level,
    pub dimensions: Vec<super::Dimension>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<super::Window>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer: Option<Producer>,
}

/// What produced the statement.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Producer {
    pub name: String,
    pub version: String,
}

/// The in-toto statement, when the score has a commit to name.
#[must_use]
pub fn statement(repo: &Repository, score: &Score) -> Option<Value> {
    let sha = score.commit.as_ref()?;
    let name = repo
        .config
        .project
        .repository_url
        .clone()
        .unwrap_or_else(|| format!("git+local:{}", repo.config.project.namespace.as_str()));
    let st = Statement {
        type_: STATEMENT_TYPE.to_owned(),
        subject: vec![Subject {
            name,
            digest: [("gitCommit".to_owned(), sha.clone())].into(),
        }],
        predicate_type: PREDICATE_TYPE.to_owned(),
        predicate: Predicate {
            weights: score.weights.clone(),
            score: score.score,
            level: score.level.clone(),
            dimensions: score.dimensions.clone(),
            window: Some(score.window.clone()),
            producer: Some(Producer {
                name: "war".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            }),
        },
    };
    serde_json::to_value(st).ok()
}

/// Every way `v` departs from the documented statement shape; empty when it
/// conforms.
#[must_use]
pub fn verify(v: &Value) -> Vec<String> {
    let mut bad = Vec::new();
    if v.get("_type").and_then(Value::as_str) != Some(STATEMENT_TYPE) {
        bad.push(format!("`_type` is not {STATEMENT_TYPE}"));
    }
    match v.get("subject").and_then(Value::as_array) {
        Some(s) if !s.is_empty() => {
            for (n, sub) in s.iter().enumerate() {
                if sub
                    .get("name")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
                {
                    bad.push(format!("subject {n} has no name"));
                }
                let ok = sub
                    .pointer("/digest/gitCommit")
                    .and_then(Value::as_str)
                    .is_some_and(|d| d.len() == 40 && d.chars().all(|c| c.is_ascii_hexdigit()));
                if !ok {
                    bad.push(format!("subject {n} has no 40-hex `digest.gitCommit`"));
                }
            }
        }
        _ => bad.push("`subject` is not a non-empty list".to_owned()),
    }
    if v.get("predicateType").and_then(Value::as_str) != Some(PREDICATE_TYPE) {
        bad.push(format!("`predicateType` is not {PREDICATE_TYPE}"));
    }
    let Some(p) = v.get("predicate") else {
        bad.push("no `predicate`".to_owned());
        return bad;
    };
    if p.get("weights").and_then(Value::as_str) != Some(WEIGHTS_SCHEMA) {
        bad.push(format!("`predicate.weights` is not {WEIGHTS_SCHEMA}"));
    }
    let dims: Vec<super::Dimension> = match p
        .get("dimensions")
        .cloned()
        .map(serde_json::from_value::<Vec<super::Dimension>>)
    {
        Some(Ok(d)) => d,
        _ => {
            bad.push("`predicate.dimensions` does not read as the scorecard".to_owned());
            return bad;
        }
    };
    let ids: Vec<&str> = dims.iter().map(|d| d.id.as_str()).collect();
    let want: Vec<&str> = WEIGHTS.iter().map(|w| w.id).collect();
    if ids != want {
        bad.push(format!("the dimensions are {ids:?}, not {want:?}"));
    }
    let mut sum = 0u32;
    for d in &dims {
        let Some(w) = WEIGHTS.iter().find(|w| w.id == d.id) else {
            continue;
        };
        if d.weight != w.weight {
            bad.push(format!("{} weighs {}, not {}", d.id, d.weight, w.weight));
        }
        match d.status {
            Status::Unknown => {
                if d.points != 0 || d.numerator.is_some() || d.denominator.is_some() {
                    bad.push(format!(
                        "{} is UNKNOWN yet claims counts or {} point(s): unknown earns 0",
                        d.id, d.points
                    ));
                }
            }
            Status::Measured => match (d.numerator, d.denominator) {
                (Some(n), Some(den)) if den > 0 && n <= den => {
                    let want = super::points(w.weight, n, den);
                    if d.points != want {
                        bad.push(format!(
                            "{} claims {} point(s); {n}/{den} of {} is {want}",
                            d.id, d.points, w.weight
                        ));
                    }
                }
                _ => bad.push(format!(
                    "{} is measured without a numerator ≤ denominator",
                    d.id
                )),
            },
        }
        sum += d.points;
    }
    let score = p.get("score").and_then(Value::as_u64);
    if score != Some(u64::from(sum.clamp(1, 1000))) {
        bad.push(format!(
            "`predicate.score` is {score:?}; the dimensions sum to {}",
            sum.clamp(1, 1000)
        ));
    }
    let level = p.pointer("/level/number").and_then(Value::as_u64);
    let want = super::level_of(sum.clamp(1, 1000));
    if level != Some(u64::from(want.number))
        || p.pointer("/level/name").and_then(Value::as_str) != Some(want.name.as_str())
    {
        bad.push(format!(
            "the level is not {} ({}), the level {} reaches",
            want.number,
            want.name,
            sum.clamp(1, 1000)
        ));
    }
    bad
}

fn color(level: u8) -> &'static str {
    match level {
        5 => "#2e7d32",
        4 => "#558b2f",
        3 => "#9e9d24",
        2 => "#ef6c00",
        _ => "#9e9e9e",
    }
}

/// A flat badge: `openwarrant | 612 verified`.
#[must_use]
pub fn badge_svg(score: &Score) -> String {
    let right = format!("{} {}", score.score, score.level.name);
    let lw = 82;
    let rw = 10 + 7 * right.chars().count();
    let w = lw + rw;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"20\" role=\"img\" \
         aria-label=\"openwarrant: {right}\"><title>openwarrant: {right}</title>\
         <rect width=\"{lw}\" height=\"20\" fill=\"#555\"/>\
         <rect x=\"{lw}\" width=\"{rw}\" height=\"20\" fill=\"{c}\"/>\
         <g fill=\"#fff\" font-family=\"Verdana,DejaVu Sans,sans-serif\" font-size=\"11\">\
         <text x=\"6\" y=\"14\">openwarrant</text><text x=\"{tx}\" y=\"14\">{right}</text></g></svg>\n",
        c = color(score.level.number),
        tx = lw + 5
    )
}

/// The shields.io endpoint form of the badge.
#[must_use]
pub fn badge_json(score: &Score) -> Value {
    json!({
        "schemaVersion": 1,
        "label": "openwarrant",
        "message": format!("{} {}", score.score, score.level.name),
        "color": color(score.level.number).trim_start_matches('#'),
    })
}

/// The report page, as Markdown.
#[must_use]
pub fn report_md(repo: &Repository, score: &Score) -> String {
    let mut md = format!("# OpenWarrant score: {}\n\n", repo.config.project.name);
    if let Some(c) = &score.commit {
        md.push_str(&format!("At commit `{c}`.\n\n"));
    }
    md.push_str(&score.render().replacen("## Score\n\n", "", 1));
    md.push_str(
        "\nHow it is computed: docs/SCORE.md. Each dimension earns weight × numerator ÷ \
         denominator; a dimension that cannot be measured reads UNKNOWN and earns 0.\n",
    );
    md
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The report page, as HTML.
#[must_use]
pub fn report_html(repo: &Repository, score: &Score) -> String {
    let mut rows = String::new();
    for d in &score.dimensions {
        let m = match d.status {
            Status::Measured => escape(&d.detail),
            Status::Unknown => format!("<strong>UNKNOWN</strong>: {}", escape(&d.detail)),
        };
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{m}</td></tr>\n",
            escape(&d.id),
            d.points,
            d.weight
        ));
    }
    let steps: String = score
        .next_steps
        .iter()
        .map(|s| format!("<li>{}</li>\n", escape(s)))
        .collect();
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>OpenWarrant score</title>\
         <style>body{{font-family:system-ui,sans-serif;max-width:46rem;margin:2rem auto;padding:0 1rem}}\
         table{{border-collapse:collapse;width:100%}}td,th{{border-bottom:1px solid #ddd;padding:.4rem;text-align:left}}</style>\
         </head><body>\n<h1>{name}</h1>\n<p><img src=\"badge.svg\" alt=\"openwarrant {score} {level}\"></p>\n\
         <p><strong>{score}</strong> / 1000, level {num} ({level}). Weights {weights}.{commit}</p>\n\
         <table><thead><tr><th>dimension</th><th>points</th><th>weight</th><th>measured</th></tr></thead><tbody>\n{rows}</tbody></table>\n\
         <h2>Next steps</h2>\n<ul>\n{steps}</ul>\n</body></html>\n",
        name = escape(&repo.config.project.name),
        score = score.score,
        level = escape(&score.level.name),
        num = score.level.number,
        weights = WEIGHTS_SCHEMA,
        commit = score
            .commit
            .as_ref()
            .map(|c| format!(" At commit <code>{c}</code>."))
            .unwrap_or_default(),
    )
}

/// One trend line: when, at which commit, what score and level.
#[must_use]
pub fn trend_line(score: &Score, at: &str) -> Value {
    json!({
        "at": at,
        "commit": score.commit,
        "score": score.score,
        "level": score.level.number,
    })
}

/// The trend journal's lines, oldest first.
#[must_use]
pub fn trend(root: &camino::Utf8Path) -> Vec<Value> {
    fs::read_to_string(root.join(DIR).join("trend.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn write_if_changed(path: &Utf8PathBuf, text: &str) -> Result<bool, RepoError> {
    if fs::read_to_string(path).ok().as_deref() == Some(text) {
        return Ok(false);
    }
    fs::write(path, text).map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })?;
    Ok(true)
}

/// Write the published set into `dir`: `score.json`, the badge (SVG and
/// shields.io JSON), the report page (Markdown and HTML), and the in-toto
/// statement when there is a commit to name. The files whose bytes changed.
///
/// # Errors
/// A write.
pub fn write_set(
    repo: &Repository,
    score: &Score,
    dir: &camino::Utf8Path,
) -> Result<Vec<Utf8PathBuf>, RepoError> {
    fs::create_dir_all(dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let pretty = |v: &Value| serde_json::to_string_pretty(v).unwrap_or_default() + "\n";
    let mut files = vec![
        ("score.json", pretty(&crate::output::value(score))),
        ("badge.svg", badge_svg(score)),
        ("badge.json", pretty(&badge_json(score))),
        ("report.md", report_md(repo, score)),
        ("report.html", report_html(repo, score)),
    ];
    if let Some(s) = statement(repo, score) {
        files.push(("statement.intoto.json", pretty(&s)));
    }
    let mut written = Vec::new();
    for (name, text) in files {
        let p = dir.join(name);
        if write_if_changed(&p, &text)? {
            written.push(p);
        }
    }
    Ok(written)
}

/// `war admin compile`'s step: measure, write the set under
/// `.openwarrant/score/`, and journal a trend line when the commit or the
/// score moved. The files written.
///
/// # Errors
/// A write.
pub fn compile(repo: &Repository) -> Result<Vec<Utf8PathBuf>, RepoError> {
    let cfg = super::Config::read(&repo.root).unwrap_or_default();
    if !cfg.compile || fs::is_hosted() {
        return Ok(Vec::new());
    }
    let score = super::compute(&super::measure(repo));
    crate::ledger::ensure_ignored(&repo.root)?;
    let dir = repo.root.join(DIR);
    let mut written = write_set(repo, &score, &dir)?;
    // The trend: one line per compile that saw a new commit or a new score.
    let last = trend(&repo.root).pop();
    let moved = last.as_ref().is_none_or(|l| {
        l.get("commit") != Some(&json!(score.commit)) || l.get("score") != Some(&json!(score.score))
    });
    if moved {
        let at = crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs());
        let p = dir.join("trend.jsonl");
        let mut text = fs::read_to_string(&p).unwrap_or_default();
        text.push_str(&serde_json::to_string(&trend_line(&score, &at)).unwrap_or_default());
        text.push('\n');
        fs::write(&p, text).map_err(|source| RepoError::Io {
            context: format!("could not write {p}"),
            source,
        })?;
        written.push(p);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::super::{History, Inputs, Window, compute};
    use super::*;

    fn score() -> Score {
        compute(&Inputs {
            history: Ok(History {
                head: "c".repeat(40),
                commits: 4,
                commits_citing: 3,
                prs: 0,
                prs_citing: 0,
                files: 2,
                files_with_ledger: 2,
            }),
            documents: Some((1, 2)),
            ticks: (1, 1, 0, 0),
            warrants: (1, 1),
            window: Window::default(),
        })
    }

    fn statement_of(s: &Score) -> Value {
        json!({
            "_type": STATEMENT_TYPE,
            "subject": [{"name": "git+local:T", "digest": {"gitCommit": s.commit}}],
            "predicateType": PREDICATE_TYPE,
            "predicate": {"weights": s.weights, "score": s.score, "level": s.level, "dimensions": s.dimensions},
        })
    }

    #[test]
    fn a_statement_conforms_and_each_tampering_is_refused() {
        let s = score();
        let good = statement_of(&s);
        assert!(verify(&good).is_empty(), "{:?}", verify(&good));
        // An UNKNOWN dimension that claims points.
        let mut bad = good.clone();
        let i = s.dimensions.iter().position(|d| d.id == "prs").unwrap();
        bad["predicate"]["dimensions"][i]["points"] = json!(150);
        bad["predicate"]["score"] = json!(s.score + 150);
        let why = verify(&bad);
        assert!(why.iter().any(|w| w.contains("UNKNOWN")), "{why:?}");
        // A score that is not the sum.
        let mut bad = good.clone();
        bad["predicate"]["score"] = json!(999);
        assert!(!verify(&bad).is_empty());
        // The wrong predicate type, and a subject with no commit.
        let mut bad = good.clone();
        bad["predicateType"] = json!("https://slsa.dev/provenance/v1");
        assert!(!verify(&bad).is_empty());
        let mut bad = good;
        bad["subject"][0]["digest"] = json!({"sha256": "00"});
        assert!(!verify(&bad).is_empty());
    }

    #[test]
    fn the_badge_names_the_score_and_level() {
        let s = score();
        assert!(badge_svg(&s).contains(&format!("{} {}", s.score, s.level.name)));
        assert_eq!(badge_json(&s)["schemaVersion"], 1);
    }
}
