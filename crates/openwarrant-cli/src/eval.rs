// SPDX-License-Identifier: Apache-2.0

//! `war eval` (1.0 plan F1): the agent loop, measured.
//!
//! A task is one vague sentence and what the loop around it should produce.
//! For each task the harness scaffolds a throwaway program (`war init
//! --program`), lets the configured drafter draft it (`war plan --draft
//! --apply`), checks it, compiles the first stage's Dispatch, hands that
//! Dispatch back to the same drafter in "perform" mode (or runs the stage,
//! for the run kind), records evidence, runs the blind verifier over a
//! bundle, and asks what a resolution WOULD say (§38.6).
//!
//! What it never does: authorize, resolve, sign. Those are human acts; the
//! score is `would_resolve_satisfied` under the blind verifier, and the
//! record says that requirement 1 of the thirteen (an authorization) is
//! unmet in every scratch program. A score is a ladder rung — would-satisfy,
//! partial, over-budget, refused, errored — and `war eval verify` reports a
//! delta per task, never a percentage.
//!
//! Every step runs as a child `war --json` in the scratch program, so the
//! harness measures exactly what a user gets. Timings go beside the result,
//! not in it: the result of the fixture drafter is byte-deterministic and a
//! plant proves it.

use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const TASK_SCHEMA: &str = "oh.war/eval-task/v1";
pub const PERFORM_SCHEMA: &str = "oh.war/eval-perform/v1";
pub const RESULT_SCHEMA: &str = "oh.war/eval-result/v1";
pub const TIMING_SCHEMA: &str = "oh.war/eval-timing/v1";
pub const DEFAULT_TASKS_DIR: &str = "evals/tasks";
pub const DEFAULT_BASELINE: &str = "evals/baseline.json";
pub const SCRATCH_NAMESPACE: &str = "EV";

/// One task file: `evals/tasks/<id>/task.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub schema: String,
    /// The vague request, verbatim — what a person would type.
    pub sentence: String,
    /// code | document | run.
    pub kind: String,
    /// The gate the proposal is expected to cite (a `<id>@<version>` key).
    pub expected_gate: String,
    /// Estimated tokens the whole loop may spend (request + Dispatch + bundle).
    pub max_tokens: u64,
    pub max_wall_secs: u64,
    /// The rung this task is expected to reach with the fixture drafter.
    #[serde(default = "default_expected")]
    pub expected: String,
}

fn default_expected() -> String {
    "would_satisfy".to_owned()
}

/// The ladder. Order matters only for reading; `verify` compares against
/// each task's own expectation, never a rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Score {
    /// §38.6: every declared obligation established by an admissible,
    /// independent verification.
    WouldSatisfy,
    /// The loop ran to the end; something is unestablished.
    Partial,
    /// The loop's estimated tokens exceeded the task's `max_tokens`.
    OverBudget,
    /// A named refusal stopped the loop (the tool did its job).
    Refused,
    /// A step failed for a reason that is not a refusal.
    Errored,
}

impl Score {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::WouldSatisfy => "would_satisfy",
            Self::Partial => "partial",
            Self::OverBudget => "over_budget",
            Self::Refused => "refused",
            Self::Errored => "errored",
        }
    }
}

/// Refusal rules the loop can hit. A step that ends in one of these is
/// scored `refused`, which for some tasks is the expected rung.
const REFUSALS: &[&str] = &[
    "plan.drafter-wrote-files",
    "plan.drafter-timeout",
    "plan.drafter-failed",
    "plan.v1-has-no-payloads",
    "plan.interview-required",
    "dispatch.over-budget",
    "dispatch.section-missing",
    "dispatch.unknown-atom",
    "dispatch.artifact-missing",
    "verify.inadmissible",
    "verify.no-verifier",
    "run.not-a-service",
    "run.unknown-gate",
    "submission.self-completion",
    "submission.unknown-dispatch",
];

/// The requirements of the thirteen that are answered from the authority
/// context — roles, an authorization, judgments, a resolver — and therefore
/// cannot hold in a scratch program no human has touched. Everything else
/// (deliverables, digests, dispositions, gate results, independence, receipts)
/// must hold for `would_satisfy`.
const HUMAN_DEPENDENT: &[&str] = &[
    "exact authorized Contract Revision",
    "no required unknown remains",
    "no blocker remains",
    "required judgments exist",
    "residual risks have sufficient authority",
    "resolver holds the role",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    pub exit_code: i64,
    pub verdict: String,
    /// The rule of the first error diagnostic, when there was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tokens {
    pub request: u64,
    pub dispatch: u64,
    pub bundle: u64,
    pub total: u64,
    pub method: String,
    /// False for the run kind: its bundle carries a receipt, and a receipt
    /// carries wall-clock durations, so the estimate moves by a token or two
    /// between identical runs. A comparison should drop the numbers then.
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub id: String,
    pub kind: String,
    pub sentence: String,
    pub score: Score,
    pub expected: String,
    pub as_expected: bool,
    pub expected_gate: String,
    pub expected_gate_cited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warrant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// The §74.4 gauntlet as `plan.apply` reported it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gauntlet: Option<serde_json::Value>,
    pub tokens: Tokens,
    pub max_tokens: u64,
    pub obligations_established: usize,
    pub obligations_total: usize,
    /// The thirteen that are unmet at the end. An authorization is always
    /// among them: the harness fabricates no human act.
    pub requirements_unmet: Vec<String>,
    /// The unmet requirements that do NOT depend on a human act — what the
    /// loop itself left undone. Empty for `would_satisfy`.
    pub requirements_unmet_by_the_loop: Vec<String>,
    pub refusals: Vec<String>,
    pub steps: Vec<Step>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub schema: String,
    pub war_version: String,
    pub drafter: String,
    pub verifier: String,
    pub tasks: Vec<TaskResult>,
    /// Tasks per rung.
    pub ladder: BTreeMap<String, usize>,
    pub as_expected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timing {
    pub schema: String,
    pub started_at: String,
    pub tasks: Vec<TaskTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTiming {
    pub id: String,
    pub wall_secs: f64,
    pub max_wall_secs: u64,
    pub over_wall_time: bool,
}

pub struct Options {
    pub tasks_dir: Utf8PathBuf,
    pub only: Option<String>,
    pub drafter: Vec<String>,
    pub verifier: Vec<String>,
    pub out: Option<Utf8PathBuf>,
    pub keep: bool,
}

/// Load every task under the tasks directory, sorted by id.
pub fn load_tasks(
    repo: &Repository,
    dir: &Utf8Path,
    only: Option<&str>,
) -> Result<Vec<(String, Utf8PathBuf, Task)>, RepoError> {
    let abs = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        repo.root.join(dir)
    };
    let entries = std::fs::read_dir(&abs).map_err(|source| RepoError::Io {
        context: format!("could not read the tasks directory {abs}"),
        source,
    })?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RepoError::Io {
            context: format!("could not read {abs}"),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|p| RepoError::Message(format!("non-UTF-8 path under {abs}: {p:?}")))?;
        let task_file = path.join("task.toml");
        if !task_file.is_file() {
            continue;
        }
        let id = path.file_name().unwrap_or_default().to_owned();
        if only.is_some_and(|o| o != id) {
            continue;
        }
        let text = std::fs::read_to_string(&task_file).map_err(|source| RepoError::Io {
            context: format!("could not read {task_file}"),
            source,
        })?;
        let task: Task = toml::from_str(&text)
            .map_err(|e| RepoError::Message(format!("{task_file}: not an eval task: {e}")))?;
        if task.schema != TASK_SCHEMA {
            return Err(RepoError::Message(format!(
                "{task_file}: schema is {}, not {TASK_SCHEMA}",
                task.schema
            )));
        }
        if !matches!(task.kind.as_str(), "code" | "document" | "run") {
            return Err(RepoError::Message(format!(
                "{task_file}: kind is {}; code, document or run",
                task.kind
            )));
        }
        out.push((id, path, task));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    if out.is_empty() {
        return Err(RepoError::Message(match only {
            Some(o) => format!("no task named {o} under {abs}"),
            None => format!("no tasks under {abs}"),
        }));
    }
    Ok(out)
}

/// One parsed `oh.war/report/v1` envelope from a child `war --json`.
struct Envelope {
    exit_code: i64,
    verdict: String,
    diagnostics: Vec<(String, String, String)>,
    result: Option<serde_json::Value>,
}

impl Envelope {
    fn first_error_rule(&self) -> Option<String> {
        self.diagnostics
            .iter()
            .find(|(sev, _, _)| sev == "error")
            .map(|(_, rule, _)| rule.clone())
    }

    /// The refusal this envelope names, if any: a diagnostic rule from the
    /// list, or a bare `cli.error` whose message starts with one.
    fn refusal(&self) -> Option<String> {
        for (sev, rule, message) in &self.diagnostics {
            if sev != "error" {
                continue;
            }
            if REFUSALS.contains(&rule.as_str()) {
                return Some(rule.clone());
            }
            if rule == "cli.error" {
                for r in REFUSALS {
                    if message.starts_with(r) {
                        return Some((*r).to_owned());
                    }
                }
            }
        }
        None
    }

    fn step(&self, name: &str) -> Step {
        Step {
            name: name.to_owned(),
            exit_code: self.exit_code,
            verdict: self.verdict.clone(),
            rule: self.first_error_rule(),
        }
    }
}

fn parse_envelope(stdout: &str) -> Option<Envelope> {
    // A printer may put human lines before the envelope; the envelope is the
    // suffix from the first line that is exactly `{`.
    let start = stdout
        .lines()
        .scan(0usize, |offset, line| {
            let at = *offset;
            *offset += line.len() + 1;
            Some((at, line))
        })
        .find(|(_, line)| *line == "{")
        .map(|(at, _)| at)?;
    let v: serde_json::Value = serde_json::from_str(&stdout[start..]).ok()?;
    Some(Envelope {
        exit_code: v
            .get("exit_code")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(-1),
        verdict: v
            .get("verdict")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned(),
        diagnostics: v
            .get("diagnostics")
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|d| {
                        let s = |k: &str| {
                            d.get(k)
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("")
                                .to_owned()
                        };
                        (s("severity"), s("rule"), s("message"))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        result: v.get("result").cloned().filter(|r| !r.is_null()),
    })
}

struct Scratch {
    root: Utf8PathBuf,
    exe_dir: Utf8PathBuf,
    task_dir: Utf8PathBuf,
    keep: bool,
}

impl Scratch {
    fn war(&self, args: &[&str]) -> Result<Envelope, RepoError> {
        let exe = std::env::current_exe()
            .map_err(|e| RepoError::Message(format!("cannot locate the war binary: {e}")))?;
        let out = Command::new(exe)
            .arg("--json")
            .args(args)
            .current_dir(&self.root)
            .env("PATH", self.path_env())
            .env("EVAL_TASK_DIR", &self.task_dir)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| RepoError::Message(format!("cannot run war {}: {e}", args.join(" "))))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        parse_envelope(&stdout).ok_or_else(|| {
            RepoError::Message(format!(
                "war {} printed no envelope (exit {}): {}",
                args.join(" "),
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        })
    }

    /// The scaffold's gates say `war`, not a build path: the binary that runs
    /// the harness is the one they find.
    fn path_env(&self) -> String {
        let inherited = std::env::var("PATH").unwrap_or_default();
        format!("{}:{inherited}", self.exe_dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

fn copy_tree(from: &Utf8Path, to: &Utf8Path) -> Result<(), RepoError> {
    let io = |context: String| move |source| RepoError::Io { context, source };
    std::fs::create_dir_all(to).map_err(io(format!("could not create {to}")))?;
    for entry in std::fs::read_dir(from).map_err(io(format!("could not read {from}")))? {
        let entry = entry.map_err(io(format!("could not read {from}")))?;
        let src = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|p| RepoError::Message(format!("non-UTF-8 path {p:?}")))?;
        let dst = to.join(src.file_name().unwrap_or_default());
        if src.is_dir() {
            copy_tree(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(io(format!("could not copy {src} to {dst}")))?;
        }
    }
    Ok(())
}

fn make_scratch(
    id: &str,
    task_dir: &Utf8Path,
    drafter: &[String],
    verifier: &[String],
    keep: bool,
) -> Result<Scratch, RepoError> {
    let exe = std::env::current_exe()
        .map_err(|e| RepoError::Message(format!("cannot locate the war binary: {e}")))?;
    let exe_dir = Utf8PathBuf::from_path_buf(
        exe.parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default(),
    )
    .map_err(|p| RepoError::Message(format!("non-UTF-8 binary path {p:?}")))?;
    let base = Utf8PathBuf::from_path_buf(std::env::temp_dir())
        .map_err(|p| RepoError::Message(format!("non-UTF-8 temp dir {p:?}")))?;
    // Not the pid alone: pids recycle, and a run inside another run (or a
    // container sharing a namespace) must never remove a sibling's scratch.
    static SCRATCH_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let root = base.join(format!(
        "war-eval-{}-{}-{nanos}-{id}",
        std::process::id(),
        SCRATCH_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|source| RepoError::Io {
        context: format!("could not create {root}"),
        source,
    })?;
    let scratch = Scratch {
        root: root.clone(),
        exe_dir,
        task_dir: task_dir.to_path_buf(),
        keep,
    };
    // The drafter's "wrote files" refusal is a `git status` diff, so the
    // scratch is a repository.
    let git = Command::new("git")
        .args(["init", "-q", "."])
        .current_dir(&root)
        .output()
        .map_err(|source| RepoError::Io {
            context: "could not run git init".to_owned(),
            source,
        })?;
    if !git.status.success() {
        return Err(RepoError::Message(format!(
            "git init failed in {root}: {}",
            String::from_utf8_lossy(&git.stderr).trim()
        )));
    }
    crate::init::run_program(&format!("Eval {id}"), SCRATCH_NAMESPACE, Some(root.clone()))
        .map_err(|e| RepoError::Message(format!("could not scaffold {root}: {e}")))?;
    let fixture = task_dir.join("fixture");
    if fixture.is_dir() {
        copy_tree(&fixture, &root)?;
    }
    // The seams' other sides, in the scratch's own configuration.
    let repo = Repository::discover(Some(root.clone()))?;
    let mut config = repo.config.clone();
    config.plan.drafter_argv = drafter.to_vec();
    config.plan.drafter_name = program_name(drafter);
    config.verify.verifier_argv = verifier.to_vec();
    let rendered = toml::to_string_pretty(&config)
        .map_err(|e| RepoError::Message(format!("could not render the scratch config: {e}")))?;
    std::fs::write(root.join(crate::init::CONFIG_FILE), rendered).map_err(|source| {
        RepoError::Io {
            context: format!("could not write {root}/openwarrant.toml"),
            source,
        }
    })?;
    Ok(scratch)
}

fn program_name(argv: &[String]) -> String {
    argv.first()
        .map(|p| Utf8Path::new(p).file_name().unwrap_or(p).to_owned())
        .unwrap_or_else(|| "none".to_owned())
}

/// Absolute argv: the scratch's cwd is not the repository's.
fn absolutize(repo: &Repository, argv: &[String]) -> Vec<String> {
    let mut out = argv.to_vec();
    if let Some(first) = out.first_mut() {
        let p = Utf8Path::new(first);
        if !p.is_absolute() && (first.contains('/') || repo.root.join(p).is_file()) {
            *first = repo.root.join(p).to_string();
        }
    }
    out
}

fn read_json(path: &Utf8Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn first_stage(warrant_dir: &Utf8Path, kind: &str) -> Option<String> {
    let text = std::fs::read_to_string(warrant_dir.join("atoms/45-milestones.yaml")).ok()?;
    let graph = openwarrant_core::milestones::parse(&text).ok()?;
    let want_service = kind == "run";
    graph
        .stages
        .iter()
        .find(|s| {
            !want_service
                || matches!(
                    s.executor_kind,
                    openwarrant_core::milestones::ExecutorKind::Service
                )
        })
        .map(|s| s.id.clone())
}

fn cites_gate(warrant_dir: &Utf8Path, key: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(warrant_dir.join("atoms")) else {
        return false;
    };
    entries.flatten().any(|e| {
        std::fs::read_to_string(e.path())
            .map(|t| t.contains(&format!("gate://{key}")))
            .unwrap_or(false)
    })
}

/// The perform half: the drafter again, told to perform, with the Dispatch
/// on stdin. It may write into the scratch — that is the work — and its
/// stdout is recorded, not parsed.
fn perform(
    scratch: &Scratch,
    drafter: &[String],
    doc: &serde_json::Value,
    max_wall: u64,
) -> Result<(i64, String), RepoError> {
    let Some(program) = drafter.first() else {
        return Err(RepoError::Message("no drafter to perform with".to_owned()));
    };
    let mut child = Command::new(program)
        .args(&drafter[1..])
        .arg("perform")
        .current_dir(&scratch.root)
        .env("PATH", scratch.path_env())
        .env("EVAL_TASK_DIR", &scratch.task_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| RepoError::Io {
            context: format!("could not run the performer {program}"),
            source,
        })?;
    {
        let mut stdin = child.stdin.take().expect("piped");
        let _ = stdin.write_all(
            serde_json::to_string_pretty(doc)
                .unwrap_or_default()
                .as_bytes(),
        );
    }
    let started = Instant::now();
    let deadline = Duration::from_secs(max_wall.max(1));
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut o = String::new();
        let _ = stdout.read_to_string(&mut o);
        let mut e = String::new();
        let _ = stderr.read_to_string(&mut e);
        (o, e)
    });
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|source| RepoError::Io {
            context: "could not wait for the performer".to_owned(),
            source,
        })? {
            break Some(s);
        }
        if started.elapsed() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let Some(status) = status else {
        // A killed performer's grandchildren may still hold the pipes; the
        // reader thread is left to finish on its own rather than joined here.
        return Ok((-1, format!("killed after {}s", deadline.as_secs())));
    };
    let (out, err) = reader.join().unwrap_or_default();
    let tail = |s: &str| {
        let mut start = s.len().saturating_sub(2000);
        while start > 0 && !s.is_char_boundary(start) {
            start -= 1;
        }
        s[start..].to_owned()
    };
    Ok((
        i64::from(status.code().unwrap_or(-1)),
        if status.success() {
            tail(&out)
        } else {
            tail(&err)
        },
    ))
}

fn run_task(
    id: &str,
    task_dir: &Utf8Path,
    task: &Task,
    drafter: &[String],
    verifier: &[String],
    keep: bool,
) -> Result<(TaskResult, TaskTiming), RepoError> {
    let started = Instant::now();
    let mut r = TaskResult {
        id: id.to_owned(),
        kind: task.kind.clone(),
        sentence: task.sentence.clone(),
        score: Score::Errored,
        expected: task.expected.clone(),
        as_expected: false,
        expected_gate: task.expected_gate.clone(),
        expected_gate_cited: false,
        warrant: None,
        stage: None,
        gauntlet: None,
        tokens: Tokens {
            method: openwarrant_core::tokens::METHOD.to_owned(),
            stable: task.kind != "run",
            ..Tokens::default()
        },
        max_tokens: task.max_tokens,
        obligations_established: 0,
        obligations_total: 0,
        requirements_unmet: Vec::new(),
        requirements_unmet_by_the_loop: Vec::new(),
        refusals: Vec::new(),
        steps: Vec::new(),
        notes: Vec::new(),
    };
    let finish = |r: &mut TaskResult, started: Instant| {
        r.as_expected = r.score.name() == r.expected;
        TaskTiming {
            id: r.id.clone(),
            wall_secs: started.elapsed().as_secs_f64(),
            max_wall_secs: task.max_wall_secs,
            over_wall_time: started.elapsed().as_secs() > task.max_wall_secs,
        }
    };

    let scratch = make_scratch(id, task_dir, drafter, verifier, keep)?;
    if keep {
        r.notes.push(format!("scratch kept at {}", scratch.root));
    }

    // 1. Draft and apply. `--reviewed` is the harness standing in for §74.4
    // steps 5–6 inside a throwaway program; the record says so.
    r.notes.push(
        "§74.4 steps 5–6 (human review) waived by the harness inside the scratch program"
            .to_owned(),
    );
    let plan = scratch.war(&["plan", &task.sentence, "--draft", "--apply", "--reviewed"])?;
    r.steps.push(plan.step("plan.apply"));
    // Precedence at every step: a named refusal scores `refused`; any other
    // failure falls through to `errored` (a crash is not the tool doing its
    // job, even when a refusal diagnostic sits beside it).
    if let Some(rule) = plan.refusal() {
        r.refusals.push(rule);
        r.score = Score::Refused;
        let t = finish(&mut r, started);
        return Ok((r, t));
    }
    let Some(alias) = plan
        .result
        .as_ref()
        .and_then(|v| v.get("alias"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
    else {
        r.notes.push(format!(
            "plan.apply produced no alias: {}",
            plan.first_error_rule().unwrap_or_default()
        ));
        let t = finish(&mut r, started);
        return Ok((r, t));
    };
    r.gauntlet = plan
        .result
        .as_ref()
        .and_then(|v| v.get("pipeline"))
        .cloned();
    r.warrant = Some(alias.clone());
    let warrant_dir = scratch.root.join("docs/warrants").join(&alias);
    r.expected_gate_cited = cites_gate(&warrant_dir, &task.expected_gate);
    if let Some(req) = read_json(&warrant_dir.join("plan/request.json")) {
        r.tokens.request = openwarrant_core::tokens::estimate(req.to_string().len() as u64);
    }

    // 2. Check.
    let check = scratch.war(&["check", &alias])?;
    r.steps.push(check.step("check"));

    // 3. The first stage's Dispatch — the unit of agent context.
    let Some(stage) = first_stage(&warrant_dir, &task.kind) else {
        r.notes.push(if task.kind == "run" {
            "no service stage to run".to_owned()
        } else {
            "no stage to dispatch".to_owned()
        });
        let t = finish(&mut r, started);
        return Ok((r, t));
    };
    r.stage = Some(stage.clone());
    let dispatch_path = scratch.root.join("eval-dispatch.json");
    let dispatch = scratch.war(&["dispatch", &alias, &stage, "--emit", dispatch_path.as_str()])?;
    r.steps.push(dispatch.step("dispatch"));
    if let Some(rule) = dispatch.refusal() {
        r.refusals.push(rule);
        r.score = Score::Refused;
        let t = finish(&mut r, started);
        return Ok((r, t));
    }
    let dispatch_doc = read_json(&dispatch_path);
    r.tokens.dispatch = dispatch_doc
        .as_ref()
        .and_then(|d| d.get("tokens"))
        .and_then(|t| t.get("estimated_tokens"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let _ = std::fs::remove_file(&dispatch_path);

    // 4–7. Perform, compile, record evidence, verify blind. The order differs
    // by kind: a code or document deliverable must exist before the gate
    // runs; a run's deliverable IS the receipt, so the performer pins it after
    // the evidence step has written the final one.
    let perform_step =
        |r: &mut TaskResult, dispatch_doc: &Option<serde_json::Value>| -> Result<bool, RepoError> {
            let doc = serde_json::json!({
                "schema": PERFORM_SCHEMA,
                "task": id,
                "kind": task.kind,
                "warrant": alias,
                "stage": stage,
                "repo_root": scratch.root.as_str(),
                "dispatch": dispatch_doc.clone().unwrap_or(serde_json::Value::Null),
            });
            let (code, tail) = perform(&scratch, drafter, &doc, task.max_wall_secs)?;
            r.steps.push(Step {
                name: "perform".to_owned(),
                exit_code: code,
                verdict: if code == 0 { "performed" } else { "failed" }.to_owned(),
                rule: None,
            });
            if code != 0 {
                r.notes.push(format!("performer exited {code}: {tail}"));
            }
            Ok(code == 0)
        };
    if task.kind == "run" {
        let run = scratch.war(&["run", &alias, &stage])?;
        r.steps.push(run.step("run"));
        if let Some(rule) = run.refusal() {
            r.refusals.push(rule);
            r.score = Score::Refused;
            let t = finish(&mut r, started);
            return Ok((r, t));
        }
        let compile = scratch.war(&["compile"])?;
        r.steps.push(compile.step("compile"));
        let evidence = scratch.war(&["evidence", "record", &alias])?;
        r.steps.push(evidence.step("evidence.record"));
        if !perform_step(&mut r, &dispatch_doc)? {
            let t = finish(&mut r, started);
            return Ok((r, t));
        }
        let compile = scratch.war(&["compile"])?;
        r.steps.push(compile.step("compile.after-perform"));
    } else {
        if !perform_step(&mut r, &dispatch_doc)? {
            let t = finish(&mut r, started);
            return Ok((r, t));
        }
        // The deliverable is pinned now, and the scaffold's gate
        // (`war check --generated`) needs the projections to exist.
        let compile = scratch.war(&["compile"])?;
        r.steps.push(compile.step("compile"));
        let evidence = scratch.war(&["evidence", "record", &alias])?;
        r.steps.push(evidence.step("evidence.record"));
    }
    let verify = scratch.war(&["verify", &alias, "--performer", "eval-performer", "--run"])?;
    r.steps.push(verify.step("verify.run"));
    if let Some(rule) = verify.refusal() {
        r.refusals.push(rule);
        r.score = Score::Refused;
        let t = finish(&mut r, started);
        return Ok((r, t));
    }
    if let Ok(entries) = std::fs::read_dir(warrant_dir.join("verifications")) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("bundle-") && name.ends_with(".json") {
                let p = Utf8PathBuf::from_path_buf(e.path()).unwrap_or_default();
                if let Some(t) = read_json(&p).and_then(|b| {
                    b.get("estimated_tokens")
                        .and_then(serde_json::Value::as_u64)
                }) {
                    r.tokens.bundle = r.tokens.bundle.max(t);
                }
            }
        }
    }
    if task.kind == "document" {
        // Its gate wants the review to exist (C4a rule 3).
        let again = scratch.war(&["evidence", "record", &alias])?;
        r.steps.push(again.step("evidence.record.after-review"));
    }

    // 8. What a resolution would say — §38.6, in-process, nothing signed.
    let scratch_repo = Repository::discover(Some(scratch.root.clone()))?;
    let one = scratch_repo.load_warrant(&warrant_dir)?;
    let assessment = crate::resolve::assess(&scratch_repo, &one)?;
    r.requirements_unmet = assessment
        .checks
        .unmet()
        .into_iter()
        .map(str::to_owned)
        .collect();
    r.requirements_unmet_by_the_loop = r
        .requirements_unmet
        .iter()
        .filter(|u| !HUMAN_DEPENDENT.contains(&u.as_str()))
        .cloned()
        .collect();
    r.obligations_established = assessment.established.len();
    r.obligations_total = assessment.established.len() + assessment.unestablished.len();
    if r.obligations_total == 0 {
        r.obligations_total = crate::resolve::declared_obligations(&one).len();
    }
    r.tokens.total = r.tokens.request + r.tokens.dispatch + r.tokens.bundle;
    r.score = if r.tokens.total > task.max_tokens {
        Score::OverBudget
    } else if assessment.would_resolve_satisfied == Some(true)
        && r.requirements_unmet_by_the_loop.is_empty()
    {
        Score::WouldSatisfy
    } else {
        Score::Partial
    };
    let t = finish(&mut r, started);
    Ok((r, t))
}

fn short_sha(repo: &Repository) -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(&repo.root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "nogit".to_owned())
}

/// `war eval run`.
pub fn run(
    repo: &Repository,
    opts: &Options,
) -> Result<(Report, EvalResult, Utf8PathBuf), RepoError> {
    let mut report = Report::default();
    let drafter = if opts.drafter.is_empty() {
        repo.config.plan.drafter_argv.clone()
    } else {
        opts.drafter.clone()
    };
    if drafter.is_empty() {
        report.push(Diagnostic::error(
            "eval.no-drafter",
            "openwarrant.toml".to_owned(),
            "no drafter: pass --drafter <argv element>... or set [plan] drafter_argv. The \
             fixture drafter is evals/fixtures/fixture-agent.sh"
                .to_owned(),
        ));
        return Err(RepoError::Message(
            "eval.no-drafter: nothing to evaluate with".to_owned(),
        ));
    }
    let verifier = if opts.verifier.is_empty() {
        repo.config.verify.verifier_argv.clone()
    } else {
        opts.verifier.clone()
    };
    let drafter = absolutize(repo, &drafter);
    let verifier = absolutize(repo, &verifier);
    let tasks = load_tasks(repo, &opts.tasks_dir, opts.only.as_deref())?;

    let started_at = crate::gate_cmd::receipt::now_rfc3339_public();
    let mut results = Vec::new();
    let mut timings = Vec::new();
    for (id, dir, task) in &tasks {
        let (r, t) = run_task(id, dir, task, &drafter, &verifier, opts.keep)?;
        let line = format!(
            "{id} [{}]: {} (expected {}); {}/{} obligations established; {} tokens of {}; {:.1}s",
            r.kind,
            r.score.name(),
            r.expected,
            r.obligations_established,
            r.obligations_total,
            r.tokens.total,
            r.max_tokens,
            t.wall_secs
        );
        if r.as_expected {
            report.push(Diagnostic::pass("eval.task", line));
        } else {
            report.push(Diagnostic::warn(
                "eval.task.unexpected",
                format!("{}/{id}/task.toml", opts.tasks_dir),
                format!(
                    "{line}; unmet by the loop: {}; refusals: {}; notes: {}",
                    if r.requirements_unmet_by_the_loop.is_empty() {
                        "nothing".to_owned()
                    } else {
                        r.requirements_unmet_by_the_loop.join(", ")
                    },
                    if r.refusals.is_empty() {
                        "none".to_owned()
                    } else {
                        r.refusals.join(", ")
                    },
                    r.notes.join(" | ")
                ),
            ));
        }
        if t.over_wall_time {
            report.push(Diagnostic::warn(
                "eval.over-wall-time",
                format!("{}/{id}/task.toml", opts.tasks_dir),
                format!(
                    "{id}: {:.1}s against max_wall_secs {}",
                    t.wall_secs, t.max_wall_secs
                ),
            ));
        }
        results.push(r);
        timings.push(t);
    }
    let mut ladder = BTreeMap::new();
    for r in &results {
        *ladder.entry(r.score.name().to_owned()).or_insert(0) += 1;
    }
    let result = EvalResult {
        schema: RESULT_SCHEMA.to_owned(),
        war_version: env!("CARGO_PKG_VERSION").to_owned(),
        drafter: program_name(&drafter),
        verifier: program_name(&verifier),
        as_expected: results.iter().filter(|r| r.as_expected).count(),
        tasks: results,
        ladder,
    };
    let out = opts.out.clone().unwrap_or_else(|| {
        repo.root
            .join("evals/results")
            .join(format!("{}-{}.json", short_sha(repo), result.drafter))
    });
    let out = if out.is_absolute() {
        out
    } else {
        repo.root.join(out)
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    let canonical = serde_jcs::to_string(&result)
        .map_err(|e| RepoError::Message(format!("could not canonicalize the result: {e}")))?;
    std::fs::write(&out, format!("{canonical}\n").as_bytes()).map_err(|source| RepoError::Io {
        context: format!("could not write {out}"),
        source,
    })?;
    let timing = Timing {
        schema: TIMING_SCHEMA.to_owned(),
        started_at,
        tasks: timings,
    };
    let timing_path = out.with_extension("timing.json");
    std::fs::write(
        &timing_path,
        serde_json::to_string_pretty(&timing).unwrap_or_default(),
    )
    .map_err(|source| RepoError::Io {
        context: format!("could not write {timing_path}"),
        source,
    })?;
    report.note(format!(
        "{} task(s): {}; {} as expected. Result {} (canonical, timings beside it in {})",
        result.tasks.len(),
        result
            .ladder
            .iter()
            .map(|(k, v)| format!("{v} {k}"))
            .collect::<Vec<_>>()
            .join(", "),
        result.as_expected,
        repo.relative(&out),
        timing_path.file_name().unwrap_or_default()
    ));
    report.note(
        "no Warrant was authorized or resolved: requirement 1 of the thirteen is unmet in every \
         scratch program by design, and would_resolve_satisfied is the score"
            .to_owned(),
    );
    Ok((report, result, out))
}

/// `war eval verify`: a result against the committed baseline, per task.
pub fn verify(
    repo: &Repository,
    result_path: &Utf8Path,
    baseline_path: &Utf8Path,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let load = |p: &Utf8Path| -> Result<EvalResult, RepoError> {
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            repo.root.join(p)
        };
        let text = std::fs::read_to_string(&abs).map_err(|source| RepoError::Io {
            context: format!("could not read {abs}"),
            source,
        })?;
        let v: EvalResult = serde_json::from_str(&text)
            .map_err(|e| RepoError::Message(format!("{abs}: not an eval result: {e}")))?;
        if v.schema != RESULT_SCHEMA {
            return Err(RepoError::Message(format!(
                "{abs}: schema {} is not {RESULT_SCHEMA}",
                v.schema
            )));
        }
        Ok(v)
    };
    let now = load(result_path)?;
    let base = load(baseline_path)?;
    let base_by: BTreeMap<&str, &TaskResult> =
        base.tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let now_by: BTreeMap<&str, &TaskResult> =
        now.tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    for (id, b) in &base_by {
        let Some(n) = now_by.get(id) else {
            report.push(Diagnostic::error(
                "eval.missing-task",
                baseline_path.to_string(),
                format!("{id}: in the baseline, not in the result"),
            ));
            continue;
        };
        let file = result_path.to_string();
        let tokens = format!("tokens {} → {}", b.tokens.total, n.tokens.total);
        match (b.score == n.score, b.as_expected, n.as_expected) {
            (true, _, _) => report.push(Diagnostic::pass(
                "eval.same",
                format!("{id}: {} both times; {tokens}", n.score.name()),
            )),
            (false, true, false) => report.push(Diagnostic::error(
                "eval.regressed",
                file,
                format!(
                    "{id}: {} → {} (expected {}); {tokens}",
                    b.score.name(),
                    n.score.name(),
                    n.expected
                ),
            )),
            (false, false, true) => report.push(Diagnostic::pass(
                "eval.improved",
                format!(
                    "{id}: {} → {} (now as expected); {tokens}",
                    b.score.name(),
                    n.score.name()
                ),
            )),
            (false, _, _) => report.push(Diagnostic::warn(
                "eval.moved",
                file,
                format!(
                    "{id}: {} → {} (expected {}, neither run met it); {tokens}",
                    b.score.name(),
                    n.score.name(),
                    n.expected
                ),
            )),
        }
    }
    for id in now_by.keys() {
        if !base_by.contains_key(id) {
            report.push(Diagnostic::warn(
                "eval.new-task",
                result_path.to_string(),
                format!("{id}: in the result, not in the baseline; add it to {baseline_path} deliberately"),
            ));
        }
    }
    report.note(format!(
        "baseline {} ({}, {} as expected) vs result {} ({}, {} as expected)",
        base.drafter,
        ladder_line(&base.ladder),
        base.as_expected,
        now.drafter,
        ladder_line(&now.ladder),
        now.as_expected
    ));
    Ok(report)
}

fn ladder_line(ladder: &BTreeMap<String, usize>) -> String {
    ladder
        .iter()
        .map(|(k, v)| format!("{v} {k}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_in_a_bare_error_is_named() {
        let e = parse_envelope(
            r#"compiled
{
  "schema": "oh.war/report/v1", "command": "error",
  "diagnostics": [{"severity":"error","rule":"cli.error","file":null,"message":"plan.drafter-wrote-files: the drafter changed the working tree"}],
  "notes": [], "counts": {"pass":0,"warn":0,"unknown":0,"error":1,"worst":"error"},
  "verdict": "not_ready", "verdict_line": "NOT READY", "exit_code": 2
}"#,
        )
        .expect("envelope");
        assert_eq!(e.refusal().as_deref(), Some("plan.drafter-wrote-files"));
        assert_eq!(e.exit_code, 2);
    }

    #[test]
    fn score_names_round_trip_through_the_expected_field() {
        for s in [
            Score::WouldSatisfy,
            Score::Partial,
            Score::OverBudget,
            Score::Refused,
            Score::Errored,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            assert_eq!(json.trim_matches('"'), s.name());
        }
    }

    #[test]
    fn the_human_dependent_requirements_are_real_ones() {
        for name in HUMAN_DEPENDENT {
            assert!(
                openwarrant_core::resolution::RESOLUTION_REQUIREMENTS.contains(name),
                "{name} is not one of the thirteen"
            );
        }
        // Never waived: what the loop itself must achieve.
        for kept in [
            "required deliverables exist",
            "artifact digests verify",
            "every required gate has admissible result",
            "independence requirements are met",
        ] {
            assert!(openwarrant_core::resolution::RESOLUTION_REQUIREMENTS.contains(&kept));
            assert!(!HUMAN_DEPENDENT.contains(&kept));
        }
    }

    #[test]
    fn every_refusal_rule_is_one_the_tool_emits() {
        // The list is a contract with the rules' spellings elsewhere in this
        // crate; a typo here would score a real refusal as `errored`.
        let src = concat!(
            include_str!("plan.rs"),
            include_str!("dispatch.rs"),
            include_str!("context_select.rs"),
            include_str!("bundle.rs"),
            include_str!("verify.rs"),
            include_str!("run_cmd.rs"),
            include_str!("main.rs"),
        );
        for rule in REFUSALS {
            assert!(src.contains(rule), "{rule} is not a rule this crate emits");
        }
    }
}
