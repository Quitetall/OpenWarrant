// SPDX-License-Identifier: Apache-2.0
//! `war admin ledger …` (OW-WAR-0148 M17): record, prune, import, export.

use camino::Utf8PathBuf;
use serde_json::{Value, json};

use super::{Commit, Config, Entry, Mode, interop};
use crate::diagnostic::{Diagnostic, Report};
use crate::go::git::git;
use crate::repo::Repository;

/// What one ledger command answers.
pub struct Answer {
    pub report: Report,
    pub human: String,
    pub result: Value,
}

impl Answer {
    fn refused(rule: &str, file: &str, message: String) -> Self {
        let mut report = Report::default();
        report.push(Diagnostic::error(rule, file.to_owned(), message.clone()));
        Self {
            report,
            human: message,
            result: Value::Null,
        }
    }
}

fn config(repo: &Repository) -> Result<Config, Box<Answer>> {
    Config::read(&repo.root)
        .map(|c| c.scoped(repo))
        .map_err(|e| {
            Box::new(Answer::refused(
                "ledger.config",
                crate::init::CONFIG_FILE,
                e,
            ))
        })
}

/// `war admin ledger record`'s arguments.
#[derive(Debug, Clone, Default)]
pub struct RecordArgs {
    /// A commit, or a range `a..b`. Absent: HEAD, or with `--why` the
    /// uncommitted changes when there are any.
    pub rev: Option<String>,
    pub mode: Option<String>,
    pub why: Option<String>,
    pub warrant: Option<String>,
}

fn today() -> String {
    crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs())[..10].to_owned()
}

/// The files that differ from HEAD, tracked or not, deletions left out.
fn uncommitted(repo: &Repository, cfg: &Config) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for args in [
        &["diff", "--name-only", "-z", "--diff-filter=d", "HEAD"][..],
        &["ls-files", "--others", "--exclude-standard", "-z"][..],
    ] {
        let r = git(&repo.root, args);
        if r.ok {
            out.extend(
                r.stdout
                    .split('\0')
                    .filter(|p| !p.is_empty() && !super::excluded(cfg, p))
                    .map(str::to_owned),
            );
        }
    }
    out.sort();
    out.dedup();
    out
}

fn commits_of(repo: &Repository, cfg: &Config, rev: &str) -> Result<Vec<Commit>, String> {
    let shas: Vec<String> = if rev.contains("..") {
        let r = git(
            &repo.root,
            &["rev-list", "--reverse", "--no-merges", rev, "--"],
        );
        if !r.ok {
            return Err(format!("git could not list {rev:?}: {}", r.why()));
        }
        r.stdout.lines().map(str::to_owned).collect()
    } else {
        vec![rev.to_owned()]
    };
    shas.iter()
        .map(|s| super::read_commit(&repo.root, cfg, s))
        .collect()
}

/// Write an entry for each file each commit touched.
#[must_use]
pub fn record(repo: &Repository, args: &RecordArgs) -> Answer {
    let cfg = match config(repo) {
        Ok(c) => c,
        Err(a) => return *a,
    };
    let mode = match args.mode.as_deref().map(Mode::parse).transpose() {
        Ok(m) => m.unwrap_or(cfg.mode),
        Err(e) => return Answer::refused("ledger.mode", "war admin ledger record", e),
    };
    let mode = if args.why.is_some() {
        Mode::Agent
    } else {
        mode
    };
    let known = super::warrant_titles(repo);
    if let Some(w) = args.warrant.as_deref()
        && !known.contains_key(w)
    {
        return Answer::refused(
            "ledger.warrant-unknown",
            "war admin ledger record",
            format!(
                "{w} names no Warrant in this repository; `war view warrants` lists them, or \
                 leave --warrant out"
            ),
        );
    }
    let commits = match args.rev.as_deref() {
        Some(r) => commits_of(repo, &cfg, r),
        None => {
            let pending = if args.why.is_some() {
                uncommitted(repo, &cfg)
            } else {
                Vec::new()
            };
            if pending.is_empty() {
                commits_of(repo, &cfg, "HEAD")
            } else {
                Ok(vec![Commit {
                    sha: String::new(),
                    date: today(),
                    subject: String::new(),
                    body: String::new(),
                    files: pending,
                }])
            }
        }
    };
    let commits = match commits {
        Ok(c) => c,
        Err(e) => return Answer::refused("ledger.rev", "war admin ledger record", e),
    };
    let mut written: Vec<String> = Vec::new();
    let mut unchanged = 0usize;
    let mut asked: Vec<String> = Vec::new();
    let mut report = Report::default();
    for c in &commits {
        let warrant = args
            .warrant
            .clone()
            .or_else(|| super::cited_warrant(&c.body, &known));
        for f in &c.files {
            let Some((source, why)) =
                super::why_for(mode, c, f, warrant.as_deref(), &known, args.why.as_deref())
            else {
                asked.push(f.clone());
                continue;
            };
            let entry = Entry {
                date: c.date.clone(),
                commit: (!c.sha.is_empty()).then(|| c.sha.clone()),
                warrant: warrant.clone(),
                source,
                why,
            };
            match super::add(repo, &cfg, f, entry) {
                Ok(true) => written.push(f.clone()),
                Ok(false) => unchanged += 1,
                Err(e) => report.push(Diagnostic::error(
                    "ledger.atom",
                    repo.relative(&super::atom_path(repo, &cfg, f)),
                    format!("{e}; fix the atom, then record again"),
                )),
            }
        }
    }
    if !asked.is_empty() {
        report.push(Diagnostic::error(
            "ledger.why-needed",
            "war admin ledger record".to_owned(),
            format!(
                "the ledger is in agent mode: say in a line why {} file(s) changed ({}), with \
                 `war admin ledger record --why \"...\"`",
                asked.len(),
                asked.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    let human = if written.is_empty() {
        format!("ledger: nothing new ({unchanged} entr(ies) already recorded)")
    } else {
        format!(
            "ledger: {} file(s) recorded ({} mode): {}\n`war admin compile` regenerates {}",
            written.len(),
            mode.as_str(),
            written.join(", "),
            super::JSONL
        )
    };
    Answer {
        report,
        human,
        result: json!({
            "mode": mode.as_str(),
            "commits": commits.iter().filter(|c| !c.sha.is_empty()).map(|c| c.sha.clone()).collect::<Vec<_>>(),
            "written": written,
            "unchanged": unchanged,
            "asked": asked,
        }),
    }
}

/// Fold every atom to `[ledger] keep` (or its token budget).
#[must_use]
pub fn prune(repo: &Repository) -> Answer {
    let cfg = match config(repo) {
        Ok(c) => c,
        Err(a) => return *a,
    };
    let mut report = Report::default();
    let mut folded = Vec::new();
    for (rel, atom) in super::load_all(repo, &cfg) {
        match atom {
            Ok(mut a) => {
                a.prune(&cfg);
                match super::store(repo, &cfg, &a) {
                    Ok(true) => folded.push(a.path.clone()),
                    Ok(false) => {}
                    Err(e) => report.push(Diagnostic::error("ledger.atom", rel, e.to_string())),
                }
            }
            Err(e) => report.push(Diagnostic::error(
                "ledger.atom",
                rel,
                format!("this ledger atom does not read ({e}); fix the line by hand"),
            )),
        }
    }
    Answer {
        human: format!("ledger: {} atom(s) folded", folded.len()),
        report,
        result: json!({"folded": folded}),
    }
}

/// Write each foreign entry into its file's atom.
fn take_in(
    repo: &Repository,
    cfg: &Config,
    entries: Vec<(String, Entry)>,
    written: &mut Vec<String>,
    report: &mut Report,
) {
    for (path, e) in entries {
        if super::excluded(cfg, &path)
            || path.starts_with('/')
            || path.split('/').any(|p| p == "..")
        {
            report.push(Diagnostic::warn(
                "ledger.import",
                path.clone(),
                "a path outside what the ledger records was skipped".to_owned(),
            ));
            continue;
        }
        match super::add(repo, cfg, &path, e) {
            Ok(true) => written.push(path),
            Ok(false) => {}
            Err(err) => report.push(Diagnostic::error("ledger.atom", path, err.to_string())),
        }
    }
}

/// `war admin ledger import --agent-trace <FILE>` or `--git-ai`.
#[must_use]
pub fn import(repo: &Repository, agent_trace: Option<&Utf8PathBuf>, git_ai: bool) -> Answer {
    let cfg = match config(repo) {
        Ok(c) => c,
        Err(a) => return *a,
    };
    let mut report = Report::default();
    let mut written = Vec::new();
    let mut read = 0usize;
    if let Some(path) = agent_trace {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                return Answer::refused(
                    "ledger.agent-trace",
                    path.as_str(),
                    format!("could not read {path}: {e}"),
                );
            }
        };
        let records = match interop::read_records(&text) {
            Ok(r) => r,
            Err(e) => return Answer::refused("ledger.agent-trace", path.as_str(), e),
        };
        for (n, r) in records.iter().enumerate() {
            match interop::agent_trace_in(r) {
                Ok((_, entries)) => {
                    read += 1;
                    take_in(repo, &cfg, entries, &mut written, &mut report);
                }
                Err(e) => report.push(Diagnostic::error(
                    "ledger.agent-trace",
                    path.to_string(),
                    format!("record {}: not an Agent Trace v0.1 record: {e}", n + 1),
                )),
            }
        }
    }
    if git_ai {
        let known = super::warrant_titles(repo);
        for sha in interop::noted_commits(&repo.root) {
            let Some(text) = interop::read_note(&repo.root, &sha) else {
                continue;
            };
            let note = match interop::Note::parse(&text) {
                Ok(n) => n,
                Err(e) => {
                    report.push(Diagnostic::warn(
                        "ledger.git-ai",
                        sha.clone(),
                        format!("this commit's git-ai note does not read as v3 ({e}); skipped"),
                    ));
                    continue;
                }
            };
            let c = super::read_commit(&repo.root, &cfg, &sha).ok();
            let date = c.as_ref().map_or_else(today, |c| c.date.clone());
            let warrant = c
                .as_ref()
                .and_then(|c| super::cited_warrant(&c.body, &known));
            match interop::git_ai_in(&note, &sha, &date, warrant.as_deref()) {
                Ok(entries) => {
                    read += 1;
                    take_in(repo, &cfg, entries, &mut written, &mut report);
                }
                Err(e) => report.push(Diagnostic::warn("ledger.git-ai", sha.clone(), e)),
            }
        }
    }
    written.sort();
    written.dedup();
    Answer {
        human: format!(
            "ledger: {read} record(s) read, {} file(s) recorded",
            written.len()
        ),
        report,
        result: json!({"read": read, "written": written}),
    }
}

/// `war admin ledger export --agent-trace` (JSON lines, to stdout or
/// `--out`) or `--git-ai` (notes under `refs/notes/ai`).
#[must_use]
pub fn export(
    repo: &Repository,
    agent_trace: bool,
    out: Option<&Utf8PathBuf>,
    git_ai: bool,
) -> Answer {
    let cfg = match config(repo) {
        Ok(c) => c,
        Err(a) => return *a,
    };
    let atoms = interop::atoms(repo, &cfg);
    let mut report = Report::default();
    let mut human = Vec::new();
    let mut result = json!({});
    if agent_trace {
        let records = interop::agent_trace_out(&repo.root, &atoms);
        let lines: String = records
            .iter()
            .filter_map(|r| serde_json::to_string(r).ok())
            .map(|s| s + "\n")
            .collect();
        match out {
            Some(p) => {
                if let Err(e) = std::fs::write(p, &lines) {
                    return Answer::refused(
                        "ledger.agent-trace",
                        p.as_str(),
                        format!("could not write {p}: {e}"),
                    );
                }
                human.push(format!(
                    "{} Agent Trace record(s) written to {p}",
                    records.len()
                ));
            }
            None => human.push(lines.trim_end().to_owned()),
        }
        result["agent_trace"] = Value::Array(records);
    }
    if git_ai {
        let (written, refused) = interop::git_ai_out(&repo.root, &atoms);
        for (sha, why) in &refused {
            report.push(Diagnostic::unknown(
                "ledger.git-ai",
                sha.clone(),
                why.clone(),
            ));
        }
        human.push(format!(
            "{} git-ai note(s) written under {}",
            written.len(),
            interop::GIT_AI_REF
        ));
        result["git_ai"] = json!({"written": written, "ref": interop::GIT_AI_REF});
    }
    Answer {
        report,
        human: human.join("\n"),
        result,
    }
}
