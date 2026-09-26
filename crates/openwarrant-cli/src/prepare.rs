// SPDX-License-Identifier: Apache-2.0
//! `war prepare <alias>... | --all` — take Warrants to their sign-off
//! unattended (t-cee5).
//!
//! The owner signs once, at the end, in one sitting (docs/SIGNING.md, "One
//! sitting"). Getting a Warrant ready for it was a procedure an agent
//! improvised, and three unattended runs on 2026-09-25/26 each fell into a
//! different gap in it: delivery never recorded, a stage's dispatch-bound
//! receipt never made, steps in an order that staled the ones before. This is
//! the procedure as one command.
//!
//! # The order, and why
//!
//! 1. **deliver** — `war deliver`: provenance on each declared deliverable.
//!    `deliverables.toml` is source to the reuse rule (it is not an evidence,
//!    verification, authority or ticket record), so it moves the tree every
//!    tree-bound receipt names: it goes FIRST, for every Warrant, before any
//!    evidence anywhere.
//! 2. **run** — `war run <alias> <STAGE>` for each stage whose executor is a
//!    gate (`executor_kind: service`, `executor_ref: gate://…`). An
//!    obligation may need that stage's dispatch-bound receipt, which
//!    `war evidence record` does not make (OW-WAR-0066). Its `dispatches/`
//!    and `submissions/` also move the tree, so this is before evidence too.
//!    A Dispatch projects an AUTHORIZED contract (§47): an unauthorized
//!    Warrant's stages wait for the sitting's authorization, and prepare says
//!    so and stops that Warrant before verification.
//! 3. **commit** — the source 1 and 2 wrote, and nothing else of source. A
//!    receipt minted over a dirty tree names no source (`worktree:dirty`,
//!    reuse UNKNOWN), so the evidence must run over a committed one. Anything
//!    else dirty refuses the whole run before step 1. `--no-commit` stops here
//!    instead. With it, after `war compile`, go every record the tree rule
//!    skips and every projection: a battery runs in a clone of HEAD, and HEAD's
//!    projections must be the ones its records compile to (t-88d2).
//! 4. **evidence** — `war evidence record <alias> --gate <key>` for each cited
//!    gate that has no admissible run for the contract as it compiles now.
//! 5. **verify** — `war verify <alias> --run`: the configured INDEPENDENT
//!    verifier reads a bundle carrying the receipts from 2 and 4 and the
//!    delivered bytes from 1. Skipped when every obligation already has an
//!    admissible `established` verdict (`--reverify` asks again). A verdict
//!    is the verifier's; prepare writes none.
//! 6. **document gates** — last, because `document.review@1.0.0` needs the
//!    verifications 5 writes.
//!
//! Evidence, verification and journal records are skipped by the tree rule
//! (t-22fd, t-fed6, t-5d82: `gate_cmd::source::Exclusions::excludes_from_tree`),
//! so 5 and 6 leave 4's receipts standing.
//!
//! # What runs at once
//!
//! A gate whose argv starts with `war` (`war check --generated`,
//! `war document review`) reads the compiled projections, and every receipt
//! changes them: it runs alone, with `war compile` immediately before it. Every
//! other gate — the battery runs in a disposable clone (t-d052) — may run
//! beside others, `--jobs` at a time, one Warrant per job; so may the verifier.
//!
//! # What it never does
//!
//! Sign, write a disposition, or ask. Each step is the same `war` command an
//! agent would type, run as a child of this one with no ssh-agent in its
//! environment. A gate that ran and failed is a result: that Warrant stops,
//! named with the gate, and the exit is still 0. A step that could not be done
//! is an error, and a result nobody could obtain is UNKNOWN (Law 15); either
//! exits 2.

use std::collections::BTreeSet;
use std::sync::Mutex;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

pub const SCHEMA: &str = "oh.war/prepare/v1";

/// What the caller asked for.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub aliases: Vec<String>,
    pub all: bool,
    pub jobs: usize,
    pub commit: bool,
    pub reverify: bool,
    pub dry_run: bool,
}

/// How a gate is scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// Runs outside the working tree's projections: may run beside others.
    Isolated,
    /// Runs `war`, which reads the projections receipts change: alone, after
    /// `war compile`. Also every gate the registry does not know.
    InTree,
    /// A document gate (`document.*`): after verification, alone, after
    /// `war compile`.
    Document,
}

/// Classify a gate by its definition. `None` — not registered — is `InTree`:
/// the cautious schedule for a gate nothing is known about.
#[must_use]
pub fn class_of(def: Option<&openwarrant_core::GateDefinition>) -> Class {
    def.map_or(Class::InTree, |d| class_for(&d.gate_id, &d.argv))
}

/// [`class_of`] from the two fields it reads.
#[must_use]
pub fn class_for(gate_id: &str, argv: &[String]) -> Class {
    if gate_id.starts_with("document.") {
        return Class::Document;
    }
    let runs_war = argv
        .first()
        .and_then(|a| Utf8Path::new(a).file_name())
        .is_some_and(|n| n == "war");
    if runs_war {
        Class::InTree
    } else {
        Class::Isolated
    }
}

/// One step's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Done now.
    Done,
    /// Already current; nothing run.
    Current,
    /// Would run (`--dry-run`).
    Planned,
    /// Ran, and the gate failed: a result.
    Failed,
    /// Could not be answered (Law 15).
    Unknown,
    /// Could not be done.
    Error,
    /// Not reached, or not applicable; `detail` says which.
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub step: &'static str,
    pub subject: String,
    pub outcome: Outcome,
    pub detail: String,
}

/// Where a Warrant ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Every step done or current; what remains is a human's.
    Prepared,
    /// Would be prepared by the plan shown (`--dry-run`).
    Planned,
    /// A gate failed, the verifier did not establish something, or the run
    /// stopped before evidence for want of a commit: a result.
    Stopped,
    /// Stopped before verification: a gate-executed stage needs the
    /// authorization first.
    AwaitsAuthorization,
    /// A result nobody could obtain.
    Unknown,
    /// A step could not be done.
    Error,
    /// Not prepared, by name (resolved).
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct WarrantResult {
    pub alias: String,
    pub state: State,
    pub steps: Vec<Step>,
    /// §56.1 requirements unmet after the run, by name.
    pub unmet_requirements: Vec<String>,
    /// Why it stopped, when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Prepared {
    pub schema: &'static str,
    pub warrants: Vec<WarrantResult>,
    /// The commit made before the evidence, when one was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// `war sign --list` lines for these Warrants: the acts only a human does.
    pub human_acts: Vec<String>,
}

/// A Warrant as the run holds it.
struct Work {
    alias: String,
    dir: Utf8PathBuf,
    contract: String,
    authorized: bool,
    /// Gate-executed stages: (stage id, gate key, class).
    stages: Vec<(String, String, Class)>,
    /// Cited gates, in citation order, deduplicated: (key, class).
    gates: Vec<(String, Class)>,
    steps: Vec<Step>,
    halt: Option<(State, String)>,
    awaiting: Vec<String>,
}

impl Work {
    fn live(&self) -> bool {
        self.halt.is_none()
    }
    fn push(&mut self, step: &'static str, subject: &str, outcome: Outcome, detail: String) {
        self.steps.push(Step {
            step,
            subject: subject.to_owned(),
            outcome,
            detail,
        });
    }
    fn stop(&mut self, state: State, why: String) {
        if self.halt.is_none() {
            self.halt = Some((state, why));
        }
    }
}

/// `war prepare`.
pub fn run(repo: &Repository, opts: &Options) -> Result<(Report, Prepared), RepoError> {
    let mut report = Report::default();
    let mut skipped: Vec<WarrantResult> = Vec::new();
    let mut work: Vec<Work> = Vec::new();
    let registry = crate::check::load_gate_registry(repo, &mut Report::default());

    for (alias, why_skip) in select(repo, opts)? {
        if let Some(why) = why_skip {
            skipped.push(WarrantResult {
                alias,
                state: State::Skipped,
                steps: vec![],
                unmet_requirements: vec![],
                stopped_at: Some(why),
            });
            continue;
        }
        match plan(repo, &alias, &registry) {
            Ok(w) => work.push(w),
            Err(why) => skipped.push(WarrantResult {
                alias,
                state: State::Error,
                steps: vec![],
                unmet_requirements: vec![],
                stopped_at: Some(why),
            }),
        }
    }
    let selected: Vec<String> = work.iter().map(|w| w.alias.clone()).collect();

    // Before anything is written: the only dirty paths a run may start from
    // are the ones it writes itself (an interrupted run's, which step 3
    // commits). Anything else would be in the tree the evidence names.
    let mut commit = None;
    if !opts.dry_run && !work.is_empty() {
        match foreign_dirt(repo, &selected) {
            Ok(foreign) if foreign.is_empty() => {}
            Ok(foreign) => {
                let why = format!(
                    "the working tree has changes prepare did not make ({}); a receipt over a \
                     dirty tree names no source, so commit or stash them first — nothing was run",
                    list(&foreign, 5)
                );
                for w in &mut work {
                    w.stop(State::Error, why.clone());
                }
            }
            Err(why) => {
                for w in &mut work {
                    w.stop(
                        State::Unknown,
                        format!("the working tree cannot be named: {why}"),
                    );
                }
            }
        }
    }

    // 1. deliver
    for w in work.iter_mut().filter(|w| w.live()) {
        deliver(repo, w, opts.dry_run);
    }
    // 2. run gate-executed stages
    // Isolated gates in one phase; everything else, one at a time.
    let wanted = |c: Class, k: Class| {
        if c == Class::Isolated {
            k == c
        } else {
            k != Class::Isolated
        }
    };
    let stage_tasks = |c: Class| {
        move |w: &Work| -> Vec<String> {
            w.steps
                .iter()
                .filter(|s| s.step == "run" && s.outcome == Outcome::Planned)
                .filter(|s| {
                    w.stages
                        .iter()
                        .any(|(id, _, k)| id == &s.subject && wanted(c, *k))
                })
                .map(|s| s.subject.clone())
                .collect()
        }
    };
    for w in work.iter_mut().filter(|w| w.live()) {
        plan_stages(w);
    }
    if !opts.dry_run {
        phase(
            repo,
            &mut work,
            opts.jobs,
            false,
            "run",
            &stage_tasks(Class::Isolated),
            run_stage,
        );
        phase(
            repo,
            &mut work,
            1,
            true,
            "run",
            &stage_tasks(Class::InTree),
            run_stage,
        );
    }
    // 3. commit what 1 and 2 wrote
    if !opts.dry_run && work.iter().any(Work::live) {
        commit = commit_records(repo, &mut work, &selected, opts.commit);
    }
    // 4. evidence: every cited gate but the document gates
    for w in work.iter_mut().filter(|w| w.live()) {
        plan_evidence(repo, w, false);
    }
    let gate_tasks = |c: Class| {
        move |w: &Work| -> Vec<String> {
            w.steps
                .iter()
                .filter(|s| {
                    (s.step == "evidence" || s.step == "document") && s.outcome == Outcome::Planned
                })
                .filter(|s| w.gates.iter().any(|(k, g)| k == &s.subject && *g == c))
                .map(|s| s.subject.clone())
                .collect()
        }
    };
    if !opts.dry_run {
        phase(
            repo,
            &mut work,
            opts.jobs,
            false,
            "evidence",
            &gate_tasks(Class::Isolated),
            record_gate,
        );
        phase(
            repo,
            &mut work,
            1,
            true,
            "evidence",
            &gate_tasks(Class::InTree),
            record_gate,
        );
    }
    // 5. verify
    for w in work.iter_mut().filter(|w| w.live()) {
        if !w.awaiting.is_empty() {
            let why = format!(
                "{} wait(s) on the authorization: a Dispatch projects an authorized contract \
                 (§47). After the sitting's authorize, `war prepare {}` again",
                w.awaiting.join(", "),
                w.alias
            );
            w.push("verify", &w.alias.clone(), Outcome::Skipped, why.clone());
            w.stop(State::AwaitsAuthorization, why);
            continue;
        }
        plan_verify(repo, w, opts.reverify);
    }
    if !opts.dry_run {
        let verify_tasks = |w: &Work| -> Vec<String> {
            w.steps
                .iter()
                .filter(|s| s.step == "verify" && s.outcome == Outcome::Planned)
                .map(|s| s.subject.clone())
                .collect()
        };
        phase(
            repo,
            &mut work,
            opts.jobs,
            false,
            "verify",
            &verify_tasks,
            run_verify,
        );
        for w in work.iter_mut().filter(|w| w.live()) {
            judge_verdicts(repo, w);
        }
    }
    // 6. document gates
    for w in work.iter_mut().filter(|w| w.live()) {
        plan_evidence(repo, w, true);
    }
    if !opts.dry_run {
        phase(
            repo,
            &mut work,
            1,
            true,
            "document gate",
            &gate_tasks(Class::Document),
            record_gate,
        );
        // The projections as the records now stand, so `war check
        // --generated` reads no drift after a run.
        if let Err(why) = compile(repo) {
            report.note(format!("the closing `war compile` failed: {why}"));
        }
    }

    // What is left, and whose.
    let human_acts = human_acts(repo, &selected);
    let mut results = Vec::new();
    for w in work {
        let unmet = unmet_requirements(repo, &w.dir);
        let (state, stopped_at) = match w.halt {
            Some((s, why)) => (s, Some(why)),
            None if opts.dry_run => (State::Planned, None),
            None => (State::Prepared, None),
        };
        results.push(WarrantResult {
            alias: w.alias,
            state,
            steps: w.steps,
            unmet_requirements: unmet,
            stopped_at,
        });
    }
    results.extend(skipped);
    results.sort_by(|a, b| a.alias.cmp(&b.alias));
    for r in &results {
        report.push(diagnostic_of(r));
    }
    if results.is_empty() {
        report.push(Diagnostic::pass(
            "prepare.nothing",
            "no Warrant is unresolved and authorized or awaiting authorization".to_owned(),
        ));
    }
    Ok((
        report,
        Prepared {
            schema: SCHEMA,
            warrants: results,
            commit,
            human_acts,
        },
    ))
}

/// The Warrants to prepare, each with a reason when it is skipped by name.
fn select(repo: &Repository, opts: &Options) -> Result<Vec<(String, Option<String>)>, RepoError> {
    let resolved = |dir: &Utf8Path| dir.join("resolution.toml").is_file();
    if !opts.all {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for alias in &opts.aliases {
            if !seen.insert(alias.clone()) {
                continue;
            }
            let dir = repo.warrant_dir(alias)?;
            let skip = resolved(&dir).then(|| {
                format!(
                    "{alias} is resolved: its records are history, bound by its resolution, and \
                     nothing here may move them"
                )
            });
            out.push((alias.clone(), skip));
        }
        return Ok(out);
    }
    let pending: BTreeSet<String> = crate::sign::list(repo)?
        .into_iter()
        .filter_map(|p| match p {
            crate::sign::Pending::Authorize { alias, .. } => Some(alias),
            _ => None,
        })
        .collect();
    let mut out = Vec::new();
    for dir in repo.warrant_dirs()? {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        if resolved(&dir) {
            continue;
        }
        let authorized = repo
            .load_authorization(&dir)?
            .is_some_and(|a| a.revision.state == openwarrant_core::RevisionState::Authorized);
        if authorized || pending.contains(&alias) {
            out.push((alias, None));
        }
    }
    out.sort();
    Ok(out)
}

/// Read what a Warrant asks of the run from its atoms.
fn plan(
    repo: &Repository,
    alias: &str,
    registry: &openwarrant_core::GateRegistry,
) -> Result<Work, String> {
    let dir = repo.warrant_dir(alias).map_err(|e| e.to_string())?;
    let one = repo.load_warrant(&dir).map_err(|e| e.to_string())?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(format!(
            "{alias}: the manifest does not validate, so there is no contract"
        ));
    };
    let contract = openwarrant_compiler::lower(basis, validated)
        .and_then(|ir| ir.contract_digest())
        .map_err(|e| format!("{alias}: the contract does not compile: {e}"))?;
    let authorized = repo
        .load_authorization(&dir)
        .ok()
        .flatten()
        .is_some_and(|a| a.revision.state == openwarrant_core::RevisionState::Authorized);
    let mut stages = Vec::new();
    for atom in basis.atoms.iter().filter(|a| a.role == "milestones") {
        let Ok(graph) = openwarrant_core::milestones::parse(&String::from_utf8_lossy(&atom.bytes))
        else {
            return Err(format!("{alias}: the milestones atom does not parse"));
        };
        for s in graph.stages {
            if s.executor_kind != openwarrant_core::milestones::ExecutorKind::Service {
                continue;
            }
            let Some(key) = s
                .executor_ref
                .as_deref()
                .and_then(|r| r.strip_prefix("gate://"))
            else {
                continue;
            };
            stages.push((s.id.clone(), key.to_owned(), class_of(registry.get(key))));
        }
    }
    let mut gates: Vec<(String, Class)> = Vec::new();
    for key in crate::resolve::cited_gate_keys(&one) {
        if !gates.iter().any(|(k, _)| k == &key) {
            let c = class_of(registry.get(&key));
            gates.push((key, c));
        }
    }
    Ok(Work {
        alias: alias.to_owned(),
        dir,
        contract,
        authorized,
        stages,
        gates,
        steps: vec![],
        halt: None,
        awaiting: vec![],
    })
}

fn deliver(repo: &Repository, w: &mut Work, dry_run: bool) {
    let declared = repo
        .load_deliverables(&w.dir)
        .map(|s| !s.records.is_empty() || !s.failures.is_empty())
        .unwrap_or(true);
    if !declared {
        w.push(
            "deliver",
            &w.alias.clone(),
            Outcome::Skipped,
            "no deliverable declared".to_owned(),
        );
        return;
    }
    let result = crate::deliver::run_with(
        repo,
        &w.alias,
        &crate::deliver::Options {
            dry_run,
            ..Default::default()
        },
    );
    match result {
        Ok((report, delivered)) if report.is_ready() => {
            let recorded = delivered
                .outcomes
                .iter()
                .filter(|(_, o)| *o == crate::deliver::Outcome::Recorded)
                .count();
            let current = delivered.outcomes.len() - recorded;
            let outcome = match (recorded, dry_run) {
                (0, _) => Outcome::Current,
                (_, true) => Outcome::Planned,
                (_, false) => Outcome::Done,
            };
            w.push(
                "deliver",
                &w.alias.clone(),
                outcome,
                format!("{recorded} recorded, {current} current"),
            );
        }
        Ok((report, _)) => {
            let why = first_problem(&report);
            w.push("deliver", &w.alias.clone(), Outcome::Error, why.clone());
            w.stop(State::Error, format!("deliver: {why}"));
        }
        Err(e) => {
            w.push("deliver", &w.alias.clone(), Outcome::Error, e.to_string());
            w.stop(State::Error, format!("deliver: {e}"));
        }
    }
}

/// Mark each gate-executed stage current, planned, or waiting.
fn plan_stages(w: &mut Work) {
    for (stage, key, _) in w.stages.clone() {
        if stage_current(&w.dir, &stage, &w.contract) {
            w.push(
                "run",
                &stage,
                Outcome::Current,
                format!("gate://{key} passed under a dispatch of this contract"),
            );
        } else if !w.authorized {
            w.push(
                "run",
                &stage,
                Outcome::Skipped,
                format!("gate://{key} waits on the authorization (§47)"),
            );
            w.awaiting.push(stage);
        } else {
            w.push("run", &stage, Outcome::Planned, format!("gate://{key}"));
        }
    }
}

/// A submission for `stage` under this contract that asked to be verified,
/// with a passing receipt beside its run.
fn stage_current(dir: &Utf8Path, stage: &str, contract: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir.join("submissions")) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|e| {
        let Ok(text) = std::fs::read_to_string(e.path()) else {
            return false;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            return false;
        };
        let s = |k: &str| v[k].as_str().unwrap_or_default().to_owned();
        if s("stage_id") != stage
            || s("requested_next_action") != "verify"
            || s("contract_digest") != contract
        {
            return false;
        }
        let runs = dir.join("gate-runs").join(s("dispatch_id"));
        std::fs::read_dir(&runs).is_ok_and(|rs| {
            rs.filter_map(Result::ok).any(|r| {
                r.file_name().to_string_lossy().ends_with(".receipt.json")
                    && std::fs::read_to_string(r.path())
                        .ok()
                        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                        .is_some_and(|rv| rv["verdict"] == "pass")
            })
        })
    })
}

fn run_stage(repo: &Repository, alias: &str, stage: &str) -> (Outcome, String) {
    match war(repo, &["run", alias, stage]) {
        Err(e) => (Outcome::Error, e),
        Ok(env) => {
            if let Some(d) = env.find("run.passed") {
                (Outcome::Done, d.message.clone())
            } else if let Some(d) = env.find("run.failed").or_else(|| env.find("run.timeout")) {
                (Outcome::Failed, d.message.clone())
            } else {
                (Outcome::Error, env.first_error())
            }
        }
    }
}

/// Mark each cited gate (document gates only, or all others) current or
/// planned.
fn plan_evidence(repo: &Repository, w: &mut Work, documents: bool) {
    let evidence = crate::evidence::load(repo, &w.dir).unwrap_or_default();
    let step = if documents { "document" } else { "evidence" };
    for (key, class) in w.gates.clone() {
        if (class == Class::Document) != documents {
            continue;
        }
        let current = evidence.iter().any(|e| {
            e.run.gate == key && crate::evidence::admissibility(e, Some(&w.contract)).is_ok()
        });
        if current {
            w.push(
                step,
                &key,
                Outcome::Current,
                "an admissible run is on file for this contract and source".to_owned(),
            );
        } else {
            w.push(
                step,
                &key,
                Outcome::Planned,
                format!("{class:?}").to_lowercase(),
            );
        }
    }
}

fn record_gate(repo: &Repository, alias: &str, key: &str) -> (Outcome, String) {
    match war(repo, &["evidence", "record", alias, "--gate", key]) {
        Err(e) => (Outcome::Error, e),
        Ok(env) => {
            if let Some(d) = env.find("gate-run.fail") {
                (Outcome::Failed, d.message.clone())
            } else if let Some(d) = env
                .find("gate-run.unaskable")
                .or_else(|| env.find("gate-run.no-result"))
            {
                (Outcome::Unknown, d.message.clone())
            } else if env.errors().next().is_some() {
                (Outcome::Error, env.first_error())
            } else if let Some(d) = env
                .find("gate-run.pass")
                .or_else(|| env.find("evidence.replayed"))
            {
                (Outcome::Done, d.message.clone())
            } else {
                (
                    Outcome::Unknown,
                    format!("`war evidence record` reported no result for {key}"),
                )
            }
        }
    }
}

fn plan_verify(repo: &Repository, w: &mut Work, reverify: bool) {
    let (established, declared) = verdicts(repo, &w.dir);
    let alias = w.alias.clone();
    if declared.is_empty() {
        w.push(
            "verify",
            &alias,
            Outcome::Skipped,
            "no obligation declared".to_owned(),
        );
        return;
    }
    if !reverify && established.len() == declared.len() {
        w.push(
            "verify",
            &alias,
            Outcome::Current,
            format!("{0}/{0} established", declared.len()),
        );
        return;
    }
    w.push(
        "verify",
        &alias,
        Outcome::Planned,
        format!(
            "{}/{} established on file",
            established.len(),
            declared.len()
        ),
    );
}

fn run_verify(repo: &Repository, alias: &str, _: &str) -> (Outcome, String) {
    let performer = repo.performer();
    match war(repo, &["verify", alias, "--performer", &performer, "--run"]) {
        Err(e) => (Outcome::Error, e),
        Ok(env) if env.errors().next().is_some() => (Outcome::Error, env.first_error()),
        Ok(env) => (
            Outcome::Done,
            env.find("verify.bundle")
                .map(|d| d.message.clone())
                .unwrap_or_else(|| "the verifier answered".to_owned()),
        ),
    }
}

/// After the verifier: stop a Warrant whose obligations are not all
/// established. The verdicts are the verifier's; this only reads them.
fn judge_verdicts(repo: &Repository, w: &mut Work) {
    let (established, declared) = verdicts(repo, &w.dir);
    if let Some(step) = w
        .steps
        .iter_mut()
        .find(|s| s.step == "verify" && s.outcome == Outcome::Done)
    {
        step.detail = format!("{}/{} established", established.len(), declared.len());
    }
    if declared.is_empty() || established.len() == declared.len() {
        return;
    }
    let missing: Vec<String> = declared
        .into_iter()
        .filter(|o| !established.contains(o))
        .collect();
    w.stop(
        State::Stopped,
        format!(
            "the verifier did not establish {} — its verdicts are in verifications/",
            missing.join(", ")
        ),
    );
}

/// (established, declared) obligation ids.
fn verdicts(repo: &Repository, dir: &Utf8Path) -> (Vec<String>, Vec<String>) {
    let Ok(one) = repo.load_warrant(dir) else {
        return (vec![], vec![]);
    };
    let declared = crate::resolve::declared_obligations(&one);
    let established = crate::resolve::assess(repo, &one)
        .map(|a| a.established)
        .unwrap_or_default();
    (established, declared)
}

/// Run one phase: for each live Warrant, the tasks `tasks` names, in order,
/// stopping that Warrant at the first that does not succeed. With `serial`,
/// one task at a time across the corpus, each after `war compile`.
fn phase(
    repo: &Repository,
    work: &mut [Work],
    jobs: usize,
    serial: bool,
    label: &str,
    tasks: &dyn Fn(&Work) -> Vec<String>,
    act: fn(&Repository, &str, &str) -> (Outcome, String),
) {
    let batches: Vec<(usize, String, Vec<String>)> = work
        .iter()
        .enumerate()
        .filter(|(_, w)| w.live())
        .map(|(i, w)| (i, w.alias.clone(), tasks(w)))
        .filter(|(_, _, t)| !t.is_empty())
        .collect();
    if batches.is_empty() {
        return;
    }
    let results: Mutex<Vec<(usize, String, Outcome, String)>> = Mutex::new(Vec::new());
    let one = |(i, alias, tasks): (usize, String, Vec<String>)| {
        for task in tasks {
            if serial && let Err(why) = compile(repo) {
                push_result(
                    &results,
                    (i, task, Outcome::Error, format!("war compile: {why}")),
                );
                return;
            }
            eprintln!("prepare: {alias}: {label} {task} …");
            let started = std::time::Instant::now();
            let (outcome, detail) = act(repo, &alias, &task);
            eprintln!(
                "prepare: {alias}: {label} {task}: {} in {}s",
                outcome_word(outcome),
                started.elapsed().as_secs()
            );
            let stop = outcome != Outcome::Done;
            push_result(&results, (i, task, outcome, detail));
            if stop {
                return;
            }
        }
    };
    let width = if serial { 1 } else { jobs.max(1) };
    in_parallel(width, batches, one);
    for (i, task, outcome, detail) in results.into_inner().unwrap_or_default() {
        let w = &mut work[i];
        let Some(step) = w
            .steps
            .iter_mut()
            .find(|s| s.subject == task && s.outcome == Outcome::Planned)
        else {
            continue;
        };
        step.outcome = outcome;
        step.detail = detail.clone();
        let what = format!("{} {task}", step.step);
        match outcome {
            Outcome::Done | Outcome::Current | Outcome::Planned | Outcome::Skipped => {}
            Outcome::Failed => w.stop(State::Stopped, format!("{what} failed: {detail}")),
            Outcome::Unknown => w.stop(State::Unknown, format!("{what} UNKNOWN: {detail}")),
            Outcome::Error => w.stop(State::Error, format!("{what}: {detail}")),
        }
    }
    // A task never reached because an earlier one stopped the Warrant.
    for w in work.iter_mut() {
        if w.halt.is_some() {
            for s in w.steps.iter_mut().filter(|s| s.outcome == Outcome::Planned) {
                s.outcome = Outcome::Skipped;
                s.detail = "not reached: an earlier step stopped this Warrant".to_owned();
            }
        }
    }
}

fn push_result(
    m: &Mutex<Vec<(usize, String, Outcome, String)>>,
    r: (usize, String, Outcome, String),
) {
    m.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(r);
}

/// Run `f` over `items`, at most `width` at a time.
fn in_parallel<T: Send>(width: usize, items: Vec<T>, f: impl Fn(T) + Sync) {
    if width <= 1 || items.len() <= 1 {
        items.into_iter().for_each(f);
        return;
    }
    let queue = Mutex::new(items.into_iter());
    std::thread::scope(|scope| {
        for _ in 0..width {
            scope.spawn(|| {
                loop {
                    let next = queue
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .next();
                    let Some(item) = next else { break };
                    f(item);
                }
            });
        }
    });
}

/// Is `path` (repository-relative) one this run writes before the evidence?
fn prepare_writes(path: &str, warrants: &str, aliases: &[String]) -> bool {
    aliases.iter().any(|a| {
        let base = format!("{}/{a}/", warrants.trim_end_matches('/'));
        path.strip_prefix(&base).is_some_and(|rest| {
            rest == "deliverables.toml"
                || rest.starts_with("dispatches/")
                || rest.starts_with("submissions/")
        })
    })
}

/// Dirty paths the tree rule sees (outside every excluded record).
fn dirt(repo: &Repository) -> Result<Vec<String>, String> {
    use crate::gate_cmd::source;
    let tree = source::head_tree(&repo.root)?;
    source::moved_since(&repo.root, &tree, &source::Exclusions::of(repo))
}

/// Dirty paths prepare would not have written.
fn foreign_dirt(repo: &Repository, aliases: &[String]) -> Result<Vec<String>, String> {
    let warrants = repo.config.paths.warrants.to_string();
    Ok(dirt(repo)?
        .into_iter()
        .filter(|p| !prepare_writes(p, &warrants, aliases))
        .collect())
}

/// Changed paths the tree rule skips, which a commit must still carry: every
/// record (evidence, a signature already made, a verification, a ticket) and
/// every compiled projection. The projections are compiled from all of them,
/// so HEAD is consistent only if it holds what they were compiled from.
/// Committing a signed record is not signing it; none of these moves the tree
/// a receipt names.
fn carried_records(repo: &Repository) -> Result<Vec<String>, String> {
    let ex = crate::gate_cmd::source::Exclusions::of(repo);
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.root.as_str())
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .output()
        .map_err(|e| format!("git status: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    let mut paths = Vec::new();
    let mut entries = out.stdout.split(|b| *b == 0).filter(|e| !e.is_empty());
    while let Some(entry) = entries.next() {
        let entry = String::from_utf8_lossy(entry);
        let (xy, path) = entry.split_at(3.min(entry.len()));
        // A rename or copy is followed by its source path.
        if xy.starts_with('R') || xy.starts_with('C') {
            entries.next();
        }
        if ex.excludes_from_tree(path) {
            paths.push(path.to_owned());
        }
    }
    Ok(paths)
}

/// Step 3: commit the records steps 1 and 2 wrote, and the projections
/// compiled from them.
fn commit_records(
    repo: &Repository,
    work: &mut [Work],
    aliases: &[String],
    commit: bool,
) -> Option<String> {
    let warrants = repo.config.paths.warrants.to_string();
    let dirty = match dirt(repo) {
        Ok(d) => d,
        Err(why) => {
            for w in work.iter_mut().filter(|w| w.live()) {
                w.stop(
                    State::Unknown,
                    format!("the working tree cannot be named: {why}"),
                );
            }
            return None;
        }
    };
    let (ours, foreign): (Vec<String>, Vec<String>) = dirty
        .into_iter()
        .partition(|p| prepare_writes(p, &warrants, aliases));
    if !foreign.is_empty() {
        let why = format!(
            "the tree moved outside prepare's records while the stages ran ({}); a receipt over \
             it would name no source",
            list(&foreign, 5)
        );
        for w in work.iter_mut().filter(|w| w.live()) {
            w.stop(State::Error, why.clone());
        }
        return None;
    }
    if ours.is_empty() {
        return None;
    }
    if !commit {
        let why = format!(
            "--no-commit: {} record(s) to commit before the evidence ({}); commit them and run \
             prepare again",
            ours.len(),
            list(&ours, 5)
        );
        for w in work.iter_mut().filter(|w| w.live()) {
            w.push("commit", &w.alias.clone(), Outcome::Skipped, why.clone());
            w.stop(State::Stopped, why.clone());
        }
        return None;
    }
    // The projections are compiled from every record, the stage runs and
    // journals the tree rule skips included. A commit of `ours` alone leaves
    // HEAD's projections stale against HEAD's own records, and every gate that
    // runs in a clone of HEAD (the battery) reads drift. So every record and
    // the projections compiled from them go in too. All are outside the tree
    // rule: carrying them moves no receipt.
    if let Err(why) = compile(repo) {
        let why = format!("`war compile` before the commit failed: {why}");
        for w in work.iter_mut().filter(|w| w.live()) {
            w.push("commit", &w.alias.clone(), Outcome::Error, why.clone());
            w.stop(State::Error, why.clone());
        }
        return None;
    }
    let carried = match carried_records(repo) {
        Ok(c) => c,
        Err(why) => {
            for w in work.iter_mut().filter(|w| w.live()) {
                w.stop(
                    State::Unknown,
                    format!("the working tree cannot be named: {why}"),
                );
            }
            return None;
        }
    };
    let ours: Vec<String> = ours
        .into_iter()
        .chain(carried)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let touched: Vec<&str> = aliases
        .iter()
        .filter(|a| {
            ours.iter()
                .any(|p| p.starts_with(&format!("{}/{a}/", warrants.trim_end_matches('/'))))
        })
        .map(String::as_str)
        .collect();
    let message = format!(
        "prepare: delivery provenance and service-stage runs\n\n{}\n\nWritten by `war prepare` \
         before recording evidence, so each receipt names a committed tree. Nothing here is a \
         signature.\n",
        touched.join(", ")
    );
    let git = |args: &[&str]| -> Result<String, String> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(repo.root.as_str())
            // An unattended commit never reaches a signing key.
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .env_remove("SSH_AUTH_SOCK")
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("git: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
        }
    };
    let mut add = vec!["add", "-A", "--"];
    add.extend(ours.iter().map(String::as_str));
    let mut ci = vec!["commit", "-q", "-m", &message, "--"];
    ci.extend(ours.iter().map(String::as_str));
    let result = git(&add)
        .and_then(|_| git(&ci))
        .and_then(|_| git(&["rev-parse", "--short", "HEAD"]));
    match result {
        Ok(sha) => {
            for w in work
                .iter_mut()
                .filter(|w| touched.contains(&w.alias.as_str()))
            {
                w.push(
                    "commit",
                    &w.alias.clone(),
                    Outcome::Done,
                    format!("{sha}: delivery and stage records"),
                );
            }
            Some(sha)
        }
        Err(why) => {
            for w in work.iter_mut().filter(|w| w.live()) {
                w.push("commit", &w.alias.clone(), Outcome::Error, why.clone());
                w.stop(State::Error, format!("commit: {why}"));
            }
            None
        }
    }
}

fn compile(repo: &Repository) -> Result<(), String> {
    let env = war(repo, &["compile"])?;
    match env.errors().next() {
        None => Ok(()),
        Some(_) => Err(env.first_error()),
    }
}

/// §56.1 requirements unmet, by name.
fn unmet_requirements(repo: &Repository, dir: &Utf8Path) -> Vec<String> {
    repo.load_warrant(dir)
        .ok()
        .and_then(|one| crate::resolve::assess(repo, &one).ok())
        .map(|a| a.checks.unmet().into_iter().map(str::to_owned).collect())
        .unwrap_or_else(|| vec!["(could not be assessed)".to_owned()])
}

/// The signing queue's lines for these Warrants.
fn human_acts(repo: &Repository, aliases: &[String]) -> Vec<String> {
    let Ok(pending) = crate::sign::list(repo) else {
        return vec!["`war sign --list` could not be read".to_owned()];
    };
    pending
        .iter()
        .filter(|p| {
            let alias = match p {
                crate::sign::Pending::Authorize { alias, .. }
                | crate::sign::Pending::Resolve { alias, .. }
                | crate::sign::Pending::Correct { alias, .. } => alias,
                _ => return false,
            };
            aliases.contains(alias)
        })
        .map(crate::sign::line)
        .collect()
}

fn diagnostic_of(r: &WarrantResult) -> Diagnostic {
    let mut parts: Vec<String> = Vec::new();
    for step in ["deliver", "run", "commit", "evidence", "verify", "document"] {
        let of: Vec<&Step> = r.steps.iter().filter(|s| s.step == step).collect();
        if of.is_empty() {
            continue;
        }
        let summary = match step {
            "deliver" | "verify" | "commit" => of
                .iter()
                .map(|s| match s.outcome {
                    Outcome::Done | Outcome::Current | Outcome::Planned => {
                        format!("{} ({})", outcome_word(s.outcome), s.detail)
                    }
                    _ => outcome_word(s.outcome).to_owned(),
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => of
                .iter()
                .map(|s| format!("{} {}", s.subject, outcome_word(s.outcome)))
                .collect::<Vec<_>>()
                .join(", "),
        };
        parts.push(format!("{step} {summary}"));
    }
    let requirements = if r.unmet_requirements.is_empty() {
        "§56.1 all 13 met".to_owned()
    } else {
        format!("§56.1 unmet: {}", r.unmet_requirements.join("; "))
    };
    let mut message = format!("{}: ", r.alias);
    if !parts.is_empty() {
        message.push_str(&parts.join(" · "));
        message.push_str(" · ");
    }
    if r.state != State::Skipped {
        message.push_str(&requirements);
    }
    if let Some(why) = &r.stopped_at {
        message.push_str(&format!(" — stopped: {why}"));
    }
    let severity = match r.state {
        State::Prepared | State::Planned => Severity::Pass,
        State::Stopped | State::AwaitsAuthorization | State::Skipped => Severity::Warn,
        State::Unknown => Severity::Unknown,
        State::Error => Severity::Error,
    };
    let rule = match r.state {
        State::Prepared => "prepare.prepared",
        State::Planned => "prepare.planned",
        State::Stopped => "prepare.stopped",
        State::AwaitsAuthorization => "prepare.awaits-authorization",
        State::Skipped => "prepare.skipped",
        State::Unknown => "prepare.unknown",
        State::Error => "prepare.error",
    };
    // No file: one line per Warrant.
    Diagnostic::new(severity, rule, None, message)
}

fn outcome_word(o: Outcome) -> &'static str {
    match o {
        Outcome::Done => "done",
        Outcome::Current => "current",
        Outcome::Planned => "planned",
        Outcome::Failed => "FAILED",
        Outcome::Unknown => "UNKNOWN",
        Outcome::Error => "ERROR",
        Outcome::Skipped => "skipped",
    }
}

/// One line per Warrant, the human's acts, and a summary.
#[must_use]
pub fn render(report: &Report, result: &Prepared) -> String {
    let mut out = String::new();
    for d in &report.diagnostics {
        out.push_str(&format!("{d}\n"));
    }
    if let Some(sha) = &result.commit {
        out.push_str(&format!(
            "\ncommitted {sha}: the delivery and stage records, before the evidence\n"
        ));
    }
    out.push_str("\nLEFT FOR A HUMAN (war sign --list):\n");
    if result.human_acts.is_empty() {
        out.push_str("  nothing awaits a signature for these Warrants\n");
    }
    for line in &result.human_acts {
        out.push_str(&format!("  {line}\n"));
    }
    out.push_str(
        "  sign in one sitting, authorizations then resolutions (docs/SIGNING.md \"One sitting\")\n",
    );
    let count = |s: State| result.warrants.iter().filter(|w| w.state == s).count();
    out.push_str(&format!(
        "\n{} prepared · {} planned · {} stopped · {} await authorization · {} skipped · {} unknown · {} error\n",
        count(State::Prepared),
        count(State::Planned),
        count(State::Stopped),
        count(State::AwaitsAuthorization),
        count(State::Skipped),
        count(State::Unknown),
        count(State::Error),
    ));
    out
}

fn list(paths: &[String], n: usize) -> String {
    let mut s = paths.iter().take(n).cloned().collect::<Vec<_>>().join(", ");
    if paths.len() > n {
        s.push_str(&format!(", … {} more", paths.len() - n));
    }
    s
}

fn first_problem(report: &Report) -> String {
    report
        .diagnostics
        .iter()
        .find(|d| d.severity.blocks_readiness())
        .map(|d| format!("{} {}", d.rule, d.message))
        .unwrap_or_else(|| "refused".to_owned())
}

/// One diagnostic from a child's envelope.
#[derive(Debug, Clone)]
struct Wire {
    severity: String,
    rule: String,
    message: String,
}

#[derive(Debug, Clone)]
struct Envelope {
    diagnostics: Vec<Wire>,
}

impl Envelope {
    fn find(&self, rule: &str) -> Option<&Wire> {
        self.diagnostics.iter().find(|d| d.rule == rule)
    }
    fn errors(&self) -> impl Iterator<Item = &Wire> {
        self.diagnostics.iter().filter(|d| d.severity == "error")
    }
    fn first_error(&self) -> String {
        self.errors()
            .next()
            .map(|d| format!("{} {}", d.rule, d.message))
            .unwrap_or_else(|| "no result".to_owned())
    }
}

/// Run `war --root <root> --json <args>` — this binary — and read its
/// envelope. The child has no ssh-agent: nothing it runs can sign.
fn war(repo: &Repository, args: &[&str]) -> Result<Envelope, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this executable: {e}"))?;
    let out = std::process::Command::new(exe)
        .arg("--root")
        .arg(repo.root.as_str())
        .arg("--json")
        .args(args)
        .current_dir(repo.root.as_std_path())
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_AGENT_PID")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("could not run war {}: {e}", args.join(" ")))?;
    // The envelope is the last value on stdout: a command that also prints
    // its progress (`war compile`) prints it first.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let body = if stdout.starts_with('{') {
        &stdout[..]
    } else {
        stdout
            .rfind("\n{")
            .map_or(&stdout[..], |i| &stdout[i + 1..])
    };
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        format!(
            "war {} printed no envelope ({e}; exit {})",
            args.join(" "),
            out.status
        )
    })?;
    let diagnostics = v["diagnostics"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|d| Wire {
                    severity: d["severity"].as_str().unwrap_or_default().to_owned(),
                    rule: d["rule"].as_str().unwrap_or_default().to_owned(),
                    message: d["message"].as_str().unwrap_or_default().to_owned(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Envelope { diagnostics })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(id: &str, argv: &[&str]) -> Class {
        let argv: Vec<String> = argv.iter().map(|s| (*s).to_owned()).collect();
        class_for(id, &argv)
    }

    #[test]
    fn a_gate_that_runs_war_is_in_tree_and_a_battery_is_not() {
        assert_eq!(
            class("software.repo.war-check", &["./target/debug/war", "check"]),
            Class::InTree
        );
        assert_eq!(class("x", &["war", "check"]), Class::InTree);
        assert_eq!(
            class(
                "ops.conformance.plants",
                &["bash", "conformance/plant-isolated.sh"]
            ),
            Class::Isolated
        );
        assert_eq!(class("ops.echo", &["true"]), Class::Isolated);
        assert_eq!(
            class(
                "document.review",
                &["./target/debug/war", "document", "review"]
            ),
            Class::Document
        );
        // Unknown to the registry: the cautious schedule.
        assert_eq!(class_of(None), Class::InTree);
        // A name that merely ends in "war" is not war.
        assert_eq!(class("x", &["./hotwar"]), Class::Isolated);
    }

    #[test]
    fn only_the_records_prepare_writes_may_be_dirty() {
        let a = vec!["X-WAR-0001".to_owned()];
        let w = "docs/warrants";
        assert!(prepare_writes(
            "docs/warrants/X-WAR-0001/deliverables.toml",
            w,
            &a
        ));
        assert!(prepare_writes(
            "docs/warrants/X-WAR-0001/dispatches/d.json",
            w,
            &a
        ));
        assert!(prepare_writes(
            "docs/warrants/X-WAR-0001/submissions/s.json",
            w,
            &a
        ));
        assert!(!prepare_writes(
            "docs/warrants/X-WAR-0002/deliverables.toml",
            w,
            &a
        ));
        assert!(!prepare_writes(
            "docs/warrants/X-WAR-0001/atoms/10-intent.md",
            w,
            &a
        ));
        assert!(!prepare_writes("src/lib.rs", w, &a));
    }

    #[test]
    fn in_parallel_runs_every_item_once() {
        let seen = Mutex::new(Vec::new());
        in_parallel(3, (0..10).collect(), |i| seen.lock().unwrap().push(i));
        let mut v = seen.into_inner().unwrap();
        v.sort_unstable();
        assert_eq!(v, (0..10).collect::<Vec<_>>());
    }
}
