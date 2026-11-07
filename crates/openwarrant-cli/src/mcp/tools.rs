// SPDX-License-Identifier: Apache-2.0
//! The tool table. Three kinds: read-only, request halves (emit the document
//! a human signs; write nothing), and writes that an agent is allowed to make
//! (`war new`, evidence, compile, gate runs, journal backfill, a reviewed
//! `plan --apply`). No tool signs, ingests, proposes a SAS revision, or runs
//! the configured drafter; the tests in `mod.rs` grep this file for that.
//!
//! Every tool builds the `oh.war/report/v1` envelope the CLI prints under
//! `--json`, as structured content, and the text the CLI prints in a
//! terminal where that differs from the envelope. `mod.rs` hands the caller
//! the text, or the envelope when asked (`json: true`, `[mcp] structured`).

use std::collections::{BTreeMap, BTreeSet};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ErrorData as McpError};
use rmcp::{tool, tool_router};
use serde::Deserialize;

use super::WarServer;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::RepoError;

/// The tools served by default: the daily commands an agent runs, and the
/// context and note it needs. `war admin mcp --all` (or `[mcp] tools =
/// "all"`) serves the whole table.
pub const DEFAULT_TOOLS: &[&str] = &[
    "war_check",
    "war_context",
    "war_create",
    "war_done",
    "war_note",
    "war_start",
    "war_status",
];

// ---- parameter shapes -----------------------------------------------------
//
// A doc comment here is the field's description in `tools/list`, which every
// agent pays for on every session: say only what the name does not. `actor`
// is who is acting (the configured performer by default); it authorizes
// nothing (OW-WAR-0147).

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct CreateParams {
    pub title: String,
    /// One line, at most 200 bytes.
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub items: Vec<String>,
    /// Markdown description.
    #[serde(default)]
    pub body: Option<String>,
    /// 0 (most urgent) to 4; default 2.
    #[serde(default)]
    pub priority: Option<u8>,
    /// One of `[fields] types`.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    /// Parent Warrant (an epic).
    #[serde(default)]
    pub part_of: Option<String>,
    /// Paths or globs it may write.
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    /// Items or Warrants it waits on; `repo:t-abc` in another repository.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Shell commands; `war_done` finishes only when they pass.
    #[serde(default)]
    pub tests: Vec<String>,
    /// A person signs it off at the end (an Order).
    #[serde(default)]
    pub sign: bool,
    #[serde(default)]
    pub actor: Option<String>,
}

/// `war view warrants` filters; none lists every Warrant.
#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct TicketsParams {
    /// A light type (bug) or a profile (delivery).
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    /// open, in_progress, done, or a declared state.
    #[serde(default)]
    pub state: Option<String>,
    /// A phrase, case-insensitive.
    #[serde(default)]
    pub text: Option<String>,
    /// Word prefixes, any order.
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub epic: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ActorParams {
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ClaimParams {
    /// An item, a Warrant, or words of its title.
    #[serde(alias = "ticket", alias = "id")]
    pub target: String,
    /// Write scopes for collision admission; not permissions.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Take a claim older than its TTL.
    #[serde(default)]
    pub steal: bool,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct StartParams {
    /// Id or title words; omit for the next ready one.
    #[serde(default, alias = "ticket", alias = "id")]
    pub target: Option<String>,
    /// Claim `target` only, without the brief (`war start --claim`).
    #[serde(default)]
    pub claim: bool,
    /// With `claim`: write scopes for collision admission.
    #[serde(default)]
    pub paths: Vec<String>,
    /// With `claim`: take a claim older than its TTL.
    #[serde(default)]
    pub steal: bool,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct HeartbeatParams {
    /// One claim; omit for all you hold.
    #[serde(default, alias = "ticket", alias = "id")]
    pub target: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct DoneParams {
    /// A Warrant (all its items) or one item; omit for the one you hold.
    #[serde(default, alias = "ticket", alias = "id")]
    pub target: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Refused `work.stale-revision` unless still at this revision.
    #[serde(default)]
    pub if_rev: Option<String>,
    /// Run its tests and KPIs first.
    #[serde(default)]
    pub check: bool,
    /// Give the claim back unfinished.
    #[serde(default)]
    pub release: bool,
    /// Untick a done item; needs `reason`.
    #[serde(default)]
    pub reopen: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct AddParams {
    /// The Warrant (`t-...`).
    #[serde(alias = "target", alias = "id")]
    pub ticket: String,
    pub text: String,
    /// Items or Warrants it waits on.
    #[serde(default)]
    pub after: Vec<String>,
    /// Refused `work.stale-revision` unless still at this revision.
    #[serde(default)]
    pub if_rev: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct NoteParams {
    /// A Warrant or item.
    #[serde(alias = "ticket", alias = "id")]
    pub target: String,
    /// Markdown; with `answer`, the answer.
    #[serde(default)]
    pub text: String,
    /// Ask this question instead.
    #[serde(default)]
    pub ask: Option<String>,
    /// Answer this question id with `text`.
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub blocking: bool,
    #[serde(default)]
    pub needs_human: bool,
    /// Refused `work.stale-revision` unless still at this revision.
    #[serde(default)]
    pub if_rev: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct PrimeParams {
    /// One Warrant in full; omit for every open one.
    #[serde(default, alias = "target", alias = "id")]
    pub ticket: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct NoParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ContextParams {
    /// An id, item, path, `FILE.md#unit`, or search words.
    pub target: String,
    /// Bytes; default 16000.
    #[serde(default)]
    pub budget: Option<usize>,
    /// Tokens (4 bytes each) when `budget` is absent.
    #[serde(default)]
    pub tokens: Option<usize>,
    /// Relation steps; default 3.
    #[serde(default)]
    pub depth: Option<usize>,
    /// Refuse rather than list a record by summary only.
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct AliasParams {
    pub alias: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct CheckParams {
    /// One Warrant; omit for the corpus.
    #[serde(default, alias = "alias")]
    pub target: Option<String>,
    /// Also drift-check generated files.
    #[serde(default)]
    pub generated: bool,
    /// Every finding, passes included.
    #[serde(default)]
    pub verbose: bool,
}

/// `war status`, with its folds: `--next`, `--ready`, `<id> --view`.
#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct StatusParams {
    /// One Warrant; omit for the corpus.
    #[serde(default, alias = "alias", alias = "id")]
    pub target: Option<String>,
    /// Whose step is next, agent or person.
    #[serde(default)]
    pub next: bool,
    /// What can start now.
    #[serde(default)]
    pub ready: bool,
    /// With `target`: a view (`full_warrant`, `status`, ...).
    #[serde(default)]
    pub view: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ShowParams {
    /// An alias, `t-...`, or `openspec:...`/`speckit:...`.
    pub alias: String,
    #[serde(default = "default_view")]
    pub view: String,
}

fn default_view() -> String {
    "full_warrant".to_owned()
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct DiffParams {
    pub alias: String,
    /// A git ref; default the committed IR.
    #[serde(default)]
    pub from: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct GateRunParams {
    /// Omit for every gate.
    #[serde(default)]
    pub gate: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct PinsParams {
    /// Only files resolved Warrants pin.
    #[serde(default)]
    pub resolved_only: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct DispatchParams {
    pub alias: String,
    pub stage: String,
    /// `initial` or `repair`.
    #[serde(default = "default_attempt")]
    pub attempt_kind: String,
    /// For a repair.
    #[serde(default)]
    pub prior_failure: Vec<String>,
}

fn default_attempt() -> String {
    "initial".to_owned()
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequestParams {
    pub alias: String,
    /// Default: the configured performer.
    #[serde(default)]
    pub performer: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct VersionParams {
    pub version: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct PlanRequestParams {
    /// The request; empty with an issue.
    #[serde(default)]
    pub sentence: String,
    /// `gh issue view --json` output, repository-relative.
    #[serde(default)]
    pub issue_file: Option<String>,
    /// Fetched through `[intake] fetch_argv`.
    #[serde(default)]
    pub issue: Option<String>,
    /// `delivery` or `decision`.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// `basic`, `standard` or `high`.
    #[serde(default = "default_assurance")]
    pub assurance: String,
    /// By question id.
    #[serde(default)]
    pub answers: BTreeMap<String, String>,
}

fn default_profile() -> String {
    "delivery".to_owned()
}
fn default_assurance() -> String {
    "basic".to_owned()
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct PlanProposalParams {
    /// An `oh.war/draft-proposal/v2` document as JSON text.
    pub proposal_json: String,
    /// The request; empty with an issue.
    #[serde(default)]
    pub sentence: String,
    /// `gh issue view --json` output, repository-relative.
    #[serde(default)]
    pub issue_file: Option<String>,
    /// Fetched through `[intake] fetch_argv`.
    #[serde(default)]
    pub issue: Option<String>,
    /// `delivery` or `decision`.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// `basic`, `standard` or `high`.
    #[serde(default = "default_assurance")]
    pub assurance: String,
    /// By question id.
    #[serde(default)]
    pub answers: BTreeMap<String, String>,
    /// A person reviewed it; needed to apply.
    #[serde(default)]
    pub reviewed: bool,
}

/// OW-WAR-0141: the intake input of a war_plan_* call, read as `war plan`
/// reads it. An issue file is resolved against the repository root.
fn intake_of(
    repo: &crate::repo::Repository,
    sentence: &str,
    issue: Option<&str>,
    issue_file: Option<&str>,
) -> Result<Option<crate::plan::Intake>, RepoError> {
    let file = issue_file.map(|f| {
        let f = camino::Utf8Path::new(f);
        if f.is_absolute() {
            f.to_owned()
        } else {
            repo.root.join(f)
        }
    });
    crate::plan::resolve_intake(repo, sentence, issue, file.as_deref())
}

/// The sentence and answers a call drafts from: the issue's, when it names
/// one, with the answers already given to that input's intake questions.
fn drafted_from(
    intake: Option<&crate::plan::Intake>,
    sentence: &str,
    answers: &BTreeMap<String, String>,
) -> (String, BTreeMap<String, String>) {
    let mut answers = answers.clone();
    let Some(i) = intake else {
        return (sentence.to_owned(), answers);
    };
    for (id, text) in &i.answers {
        answers.entry(id.clone()).or_insert_with(|| text.clone());
    }
    (i.sentence.clone(), answers)
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct AskParams {
    pub alias: String,
    pub stage: String,
    pub question: String,
    #[serde(default)]
    pub recommend: String,
    #[serde(default)]
    pub blocking: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct QuestionsParams {
    #[serde(default)]
    pub alias: Option<String>,
    /// Only unanswered.
    #[serde(default)]
    pub open: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct AnswersParams {
    pub alias: String,
    #[serde(default)]
    pub stage: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct NewParams {
    pub title: String,
    /// `delivery` or `decision`.
    #[serde(default = "default_profile")]
    pub profile: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct EvidenceParams {
    pub alias: String,
    /// Omit for every cited gate.
    #[serde(default)]
    pub gate: Option<String>,
    /// `file:<path>#sha256:<digest>`, for the Bonsai gate.
    #[serde(default)]
    pub evidence_ref: Option<String>,
    /// Claimed attribution only.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct DeliverParams {
    pub alias: String,
    /// `D-001`, ...; omit for all.
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct CompileParams {
    /// Omit for the corpus.
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct SignShowParams {
    /// As `war sign --list` names it.
    pub target: String,
}

// ---- result shaping -------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct StandingShowParams {
    /// Omit for every class.
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct StandingApplyParams {
    pub alias: String,
    /// `standing://<id>@<rev>`; default `[standing] ref`.
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

type ToolResult = Result<CallToolResult, McpError>;

fn envelope(command: &str, report: &Report, result: Option<serde_json::Value>) -> ToolResult {
    let text = crate::output::envelope(command, report, result);
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| McpError::internal_error(format!("envelope is not JSON: {e}"), None))?;
    lean(&mut value);
    Ok(CallToolResult::structured(value))
}

/// [`envelope`] with every finding: what `verbose: true` asks for.
fn envelope_whole(command: &str, report: &Report, result: Option<serde_json::Value>) -> ToolResult {
    let text = crate::output::envelope(command, report, result);
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| McpError::internal_error(format!("envelope is not JSON: {e}"), None))?;
    Ok(CallToolResult::structured(value))
}

/// A result this small is returned whole: its passes are the answer (a
/// request emitted, a view rendered), not noise around one.
const LEAN_ABOVE: usize = 25;

/// Warnings of one rule an agent is shown before the rest become a count.
const WARNINGS_PER_RULE: usize = 3;

/// What an agent's context receives: every error and unknown, a few
/// warnings per rule, no passes. `counts` still totals every finding, and
/// `folded` says what was left out by rule, so nothing is hidden, only not
/// repeated. A clean check of 147 Warrants was 711 KB of envelope, 1,891
/// PASS rows of it; the CLI's `--json` stays complete for programs.
fn lean(value: &mut serde_json::Value) {
    let Some(all) = value
        .get_mut("diagnostics")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    if all.len() <= LEAN_ABOVE {
        return;
    }
    let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut folded: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    all.retain(|d| {
        let severity = d["severity"].as_str().unwrap_or("");
        let rule = d["rule"].as_str().unwrap_or("").to_owned();
        let keep = match severity {
            "pass" => false,
            "warn" => {
                let n = seen.entry(rule.clone()).or_default();
                *n += 1;
                *n <= WARNINGS_PER_RULE
            }
            _ => true,
        };
        if !keep && severity == "warn" {
            *folded.entry(rule).or_default() += 1;
        }
        keep
    });
    if !folded.is_empty() {
        value["folded"] = serde_json::json!({ "warn": folded });
    }
}

/// A refusal is a tool OUTCOME, not a protocol failure: the agent reads the
/// rule and the message the CLI would have printed, in the same envelope.
fn refused(command: &str, err: &RepoError) -> ToolResult {
    let mut report = Report::default();
    if let RepoError::ObservationUnavailable { rule, message } = err {
        report.push(Diagnostic::unknown(*rule, String::new(), message));
        return envelope(command, &report, None);
    }
    report.push(Diagnostic::error(
        "cli.error",
        String::new(),
        err.to_string(),
    ));
    let text = crate::output::envelope(command, &report, None);
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| McpError::internal_error(format!("envelope is not JSON: {e}"), None))?;
    Ok(CallToolResult::structured_error(value))
}

fn report_of(command: &str, r: Result<Report, RepoError>) -> ToolResult {
    match r {
        Ok(report) => envelope(command, &report, None),
        Err(e) => refused(command, &e),
    }
}

fn value_of<T: serde::Serialize>(
    command: &str,
    r: Result<T, RepoError>,
    human: &str,
) -> ToolResult {
    match r {
        Ok(v) => {
            let mut report = Report::default();
            report.push(Diagnostic::pass(format!("{command}.emitted"), human));
            envelope(command, &report, Some(crate::output::value(&v)))
        }
        Err(e) => refused(command, &e),
    }
}

/// Run this same binary with `--json`, stdout captured. Used only where the
/// command's printer lives in a pinned file and cannot be routed elsewhere.
fn self_exec(repo_root: &camino::Utf8Path, args: &[&str]) -> Result<(String, i32), RepoError> {
    let exe = std::env::current_exe()
        .map_err(|e| RepoError::Message(format!("cannot locate the war binary: {e}")))?;
    let out = std::process::Command::new(exe)
        .arg("--json")
        // `--root` is what actually names the repository; `current_dir` stays
        // because the child's OWN subprocesses — a gate's argv — resolve their
        // relative paths against it. Before the flag existed, moving the
        // process was the only way to say which tree this was about.
        .arg("--root")
        .arg(repo_root.as_str())
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| RepoError::Message(format!("cannot run war {}: {e}", args.join(" "))))?;
    Ok((
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
    ))
}

fn passthrough(command: &str, r: Result<(String, i32), RepoError>) -> ToolResult {
    match r {
        // The child's exit code is deliberately unused: the envelope it
        // printed carries `exit_code`, and that is what the agent reads.
        Ok((stdout, _exit_code)) => {
            // The child printed one envelope (or an error envelope); pass it
            // through verbatim so the agent sees exactly the CLI's report.
            // A printer in a pinned file may put human lines BEFORE the
            // envelope (`war compile` says "compiled X"). The envelope is the
            // suffix starting at the first line that is exactly `{`, which is
            // how `output::envelope` pretty-prints it; a `{` inside a human
            // line is not at column 0 on a line of its own.
            let json_start = stdout
                .lines()
                .scan(0usize, |offset, line| {
                    let at = *offset;
                    *offset += line.len() + 1;
                    Some((at, line))
                })
                .find(|(_, line)| *line == "{")
                .map_or(0, |(at, _)| at);
            let value: serde_json::Value = serde_json::from_str(stdout[json_start..].trim()).unwrap_or_else(|_| {
                serde_json::json!({
                    "schema": "oh.war/report/v1",
                    "command": command,
                    "diagnostics": [{"severity": "error", "rule": "mcp.child-output", "file": null,
                        "message": format!("child `war {command}` did not print one JSON envelope")}],
                    "notes": [], "counts": {"pass": 0, "warn": 0, "unknown": 0, "error": 1, "worst": "error"},
                    "verdict": "not_ready", "verdict_line": "NOT READY", "exit_code": 1,
                    "result": {"stdout": stdout}
                })
            });
            let is_error = value.get("exit_code").and_then(serde_json::Value::as_u64) != Some(0);
            Ok(if is_error {
                CallToolResult::structured_error(value)
            } else {
                CallToolResult::structured(value)
            })
        }
        Err(e) => refused(command, &e),
    }
}

// ---- the table ------------------------------------------------------------

#[tool_router(router = tool_router, vis = "pub")]
impl WarServer {
    // -- read-only --

    #[tool(
        name = "war_check",
        description = "Check the records of one Warrant or the corpus. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_check(&self, Parameters(p): Parameters<CheckParams>) -> ToolResult {
        let r = crate::check::run_with(
            &crate::corpus::held(&self.repo),
            p.target.as_deref(),
            p.generated,
        );
        match r {
            // The text is what `war check` prints: the brief, or every
            // finding under `verbose`.
            Ok(report) if p.verbose => {
                let text: Vec<String> =
                    report.diagnostics.iter().map(ToString::to_string).collect();
                Ok(with_text(
                    envelope_whole("check", &report, None)?,
                    text.join("\n"),
                ))
            }
            Ok(report) => {
                let text = crate::output::brief_with(&report, false);
                Ok(with_text(envelope("check", &report, None)?, text))
            }
            Err(e) => refused("check", &e),
        }
    }

    #[tool(
        name = "war_status",
        description = "Where the work stands, or one Warrant (`target`, `view`); `next`: whose step; `ready`: what can start. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_status(&self, Parameters(p): Parameters<StatusParams>) -> ToolResult {
        if p.next {
            return self.next_answer();
        }
        if p.ready {
            return self.ticket("ready", p.actor.as_deref(), crate::ticket::ready);
        }
        let Some(target) = p.target else {
            // The text is `war status`'s brief; the envelope keeps the
            // corpus projection it always carried.
            let brief = crate::status_brief::Brief::read(&self.repo);
            let mut text = brief.render();
            for why in &brief.unknown {
                text.push_str(&format!("UNKNOWN (status.unreadable): {why}\n"));
            }
            let r = value_of(
                "status",
                crate::corpus::held(&self.repo).status().cloned(),
                "corpus status built",
            )?;
            return Ok(with_text(r, text));
        };
        if let Some(view) = p.view {
            return self.show_view(&target, &view);
        }
        match crate::warrants::kind_of(&target) {
            crate::warrants::IdKind::Light => {
                self.ticket("status", None, |s| crate::ticket::show(s, &target))
            }
            crate::warrants::IdKind::ReadInPlace => {
                let repo = self.repo.clone();
                self.ticket("status", None, move |_| {
                    Ok(crate::interop::adapters::show(&repo, &target))
                })
            }
            crate::warrants::IdKind::Directory => {
                match crate::show::run(&self.repo, &target, "status") {
                    Ok(rendered) => {
                        let r = value_of(
                            "status",
                            Ok(serde_json::json!({"alias": target, "rendered": rendered})),
                            "status view rendered",
                        )?;
                        Ok(with_text(r, rendered))
                    }
                    Err(e) => refused("status", &e),
                }
            }
        }
    }

    #[tool(
        name = "war_show",
        description = "One Warrant as a person reads it (`war status <id> --view`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_show(&self, Parameters(p): Parameters<ShowParams>) -> ToolResult {
        self.show_view(&p.alias, &p.view)
    }

    #[tool(
        name = "war_journal",
        description = "A Warrant's journal. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_journal(&self, Parameters(p): Parameters<AliasParams>) -> ToolResult {
        value_of(
            "journal",
            crate::journal_cmd::show(&self.repo, &p.alias)
                .map(|rendered| serde_json::json!({"alias": p.alias, "rendered": rendered})),
            "journal rendered",
        )
    }

    #[tool(
        name = "war_diff",
        description = "Semantic diff of a Warrant's compiled IR against the committed one or a git ref. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_diff(&self, Parameters(p): Parameters<DiffParams>) -> ToolResult {
        let from = p.from.map(camino::Utf8PathBuf::from);
        report_of(
            "diff",
            crate::show::diff(&self.repo, &p.alias, from.as_ref()),
        )
    }

    #[tool(
        name = "war_gate_list",
        description = "The registered gates, not run. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_gate_list(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        report_of(
            "gate",
            crate::gate_cmd::run(&self.repo, false, None, false, &[], &[], None),
        )
    }

    #[tool(
        name = "war_sas_status",
        description = "The accepted SAS revision and what pins to it. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_sas_status(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        report_of("sas.status", crate::sas::status(&self.repo))
    }

    #[tool(
        name = "war_resolve_dry_run",
        description = "The thirteen resolution requirements, assessed; resolves nothing. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_resolve_dry_run(&self, Parameters(p): Parameters<AliasParams>) -> ToolResult {
        report_of("resolve.dry_run", crate::resolve::run(&self.repo, &p.alias))
    }

    #[tool(
        name = "war_sign_list",
        description = "Acts waiting for a person's signature. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_sign_list(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        value_of(
            "sign.list",
            crate::sign::list(&self.repo).and_then(|pending| {
                let offs = crate::ticket::acts::awaiting_in(&self.repo)?;
                let lines: Vec<String> = pending
                    .iter()
                    .map(crate::sign::line)
                    .chain(offs.iter().map(crate::ticket::acts::SignOff::line))
                    .collect();
                Ok(serde_json::json!({"pending": lines, "count": lines.len(),
                    "how": "a human runs `war sign <target> --ssh-sign` at a terminal"}))
            }),
            "pending acts listed",
        )
    }

    #[tool(
        name = "war_sign_show",
        description = "What one pending act would sign, as the person sees it. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_sign_show(&self, Parameters(p): Parameters<SignShowParams>) -> ToolResult {
        passthrough(
            "sign.show",
            self_exec(&self.repo.root, &["sign", &p.target, "--show"]),
        )
    }

    #[tool(
        name = "war_pins",
        description = "Files Warrants pin, with state and digest. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_pins(&self, Parameters(p): Parameters<PinsParams>) -> ToolResult {
        value_of(
            "pins",
            crate::pins::list(&self.repo, p.resolved_only),
            "pins listed",
        )
    }

    #[tool(
        name = "war_standing_show",
        description = "Standing authorizations: state, signer, expiry, use, matches. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_standing_show(&self, Parameters(p): Parameters<StandingShowParams>) -> ToolResult {
        match crate::standing_cmd::show(&self.repo, p.id.as_deref()) {
            Ok((report, views)) => {
                envelope("standing", &report, Some(crate::output::value(&views)))
            }
            Err(e) => refused("standing", &e),
        }
    }

    #[tool(
        name = "war_next",
        description = "What is ready and whose step each is (`war status --next`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_next(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        self.next_answer()
    }

    // read_only_hint stays true: the journal line a compile appends is the
    // Warrant's own history of being compiled, not a change to its records.
    #[tool(
        name = "war_dispatch",
        description = "Compile a stage's Dispatch packet; journals `dispatch.compiled`, writes nothing else.",
        annotations(read_only_hint = true)
    )]
    fn war_dispatch(&self, Parameters(p): Parameters<DispatchParams>) -> ToolResult {
        let kind = match p
            .attempt_kind
            .parse::<openwarrant_core::execution::AttemptKind>()
        {
            Ok(k) => k,
            Err(e) => return refused("dispatch", &RepoError::Message(e.to_string())),
        };
        // The packet printer lives in a pinned file and prints to stdout,
        // which is the transport's. Emit to a scratch file and read it back.
        // The name is built from the alias and stage only after they are
        // reduced to [A-Za-z0-9_.-]: a `/` or `..` in a tool argument must not
        // become a path component. A nanosecond stamp keeps two calls apart.
        let safe = |s: &str| -> String {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
                .collect()
        };
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let scratch = std::env::temp_dir().join(format!(
            "war-mcp-dispatch-{}-{}-{}-{stamp}.json",
            std::process::id(),
            safe(&p.alias),
            safe(&p.stage)
        ));
        let Some(scratch) = camino::Utf8PathBuf::from_path_buf(scratch).ok() else {
            return refused(
                "dispatch",
                &RepoError::Message("temp dir is not UTF-8".to_owned()),
            );
        };
        let report = crate::dispatch::run(
            &self.repo,
            &p.alias,
            &p.stage,
            crate::dispatch::Options {
                attempt_kind: kind,
                prior_failure_evidence: &p.prior_failure,
                emit_to: Some(&scratch),
                emit_context_to: None,
                // An agent cannot decide to work under no authority (§27.2).
                prototype: false,
            },
        );
        let packet = std::fs::read_to_string(&scratch).ok();
        let _ = std::fs::remove_file(&scratch);
        match report {
            Ok(report) => {
                let packet: Option<serde_json::Value> =
                    packet.and_then(|t| serde_json::from_str(&t).ok());
                envelope("dispatch", &report, packet)
            }
            Err(e) => refused("dispatch", &e),
        }
    }

    // -- request halves: emit the document a human signs; write nothing --

    #[tool(
        name = "war_authorize_request",
        description = "Draft the authorization request a person signs. Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_authorize_request(&self, Parameters(p): Parameters<AliasParams>) -> ToolResult {
        value_of(
            "authorize.request",
            crate::authorize::request(&self.repo, &p.alias),
            "authorization request emitted; a human signs it with `war sign`",
        )
    }

    #[tool(
        name = "war_resolve_request",
        description = "Draft the resolution request a person signs. Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_resolve_request(&self, Parameters(p): Parameters<AliasParams>) -> ToolResult {
        value_of(
            "resolve.request",
            crate::resolution_cmd::request(&self.repo, &p.alias),
            "resolution request emitted; a human signs it with `war sign`",
        )
    }

    #[tool(
        name = "war_verify_request",
        description = "Draft the request for an independent verifier. Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_verify_request(&self, Parameters(p): Parameters<VerifyRequestParams>) -> ToolResult {
        value_of(
            "verify.request",
            crate::verify::request(
                &self.repo,
                &p.alias,
                p.performer.as_deref().unwrap_or(&self.repo.performer()),
            ),
            "verification request emitted for an independent verifier",
        )
    }

    #[tool(
        name = "war_sas_accept_request",
        description = "Draft the SAS acceptance request a person signs. Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_sas_accept_request(&self, Parameters(p): Parameters<VersionParams>) -> ToolResult {
        value_of(
            "sas.accept.request",
            crate::sas::accept_request(&self.repo, &p.version),
            "SAS acceptance request emitted; a human signs it with `war sign`",
        )
    }

    #[tool(
        name = "war_plan_request",
        description = "Draft Request for a sentence or an issue. Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_plan_request(&self, Parameters(p): Parameters<PlanRequestParams>) -> ToolResult {
        let intake = match intake_of(
            &self.repo,
            &p.sentence,
            p.issue.as_deref(),
            p.issue_file.as_deref(),
        ) {
            Ok(i) => i,
            Err(e) => return refused("plan.request", &e),
        };
        let (sentence, answers) = drafted_from(intake.as_ref(), &p.sentence, &p.answers);
        if sentence.trim().is_empty() {
            return refused(
                "plan.request",
                &RepoError::Message(
                    "war_plan_request needs a sentence, an issue_file or an issue".to_owned(),
                ),
            );
        }
        value_of(
            "plan.request",
            crate::plan::request(&self.repo, &sentence, &p.profile, &p.assurance, &answers),
            "draft request emitted; answer it with an `oh.war/draft-proposal/v2` document",
        )
    }

    #[tool(
        name = "war_plan_validate",
        description = "Run a Draft Proposal through `--apply`'s checks. Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_plan_validate(&self, Parameters(p): Parameters<PlanProposalParams>) -> ToolResult {
        let intake = match intake_of(
            &self.repo,
            &p.sentence,
            p.issue.as_deref(),
            p.issue_file.as_deref(),
        ) {
            Ok(i) => i,
            Err(e) => return refused("plan.validate", &e),
        };
        let (_, answers) = drafted_from(intake.as_ref(), &p.sentence, &p.answers);
        let answered: BTreeSet<String> = answers.keys().cloned().collect();
        let review = match crate::plan::review_of(&self.repo, intake.as_ref(), false, p.reviewed) {
            Ok(r) => r,
            Err(e) => return refused("plan.validate", &e),
        };
        let known = match crate::plan::known_refs(&self.repo) {
            Ok(k) => k,
            Err(e) => return refused("plan.validate", &e),
        };
        match crate::plan::validate_v2(
            &p.proposal_json,
            review.completes_the_step(),
            &answered,
            &known,
        ) {
            Ok((_, pipeline)) => {
                let mut report = Report::default();
                report.push(Diagnostic::pass(
                    "plan.applicable",
                    "the proposal passed every validation step",
                ));
                envelope(
                    "plan.validate",
                    &report,
                    Some(crate::output::value(&pipeline)),
                )
            }
            Err(e) => refused("plan.validate", &e),
        }
    }

    #[tool(
        name = "war_questions",
        description = "Questions asked of a person, blocking and open first. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_questions(&self, Parameters(p): Parameters<QuestionsParams>) -> ToolResult {
        match crate::questions::list(&self.repo, p.alias.as_deref(), p.open) {
            Ok((report, list)) => {
                let mut result = envelope("questions", &report, Some(crate::output::value(&list)))?;
                if !report.is_ready() {
                    result.is_error = Some(true);
                }
                Ok(result)
            }
            Err(e) => refused("questions", &e),
        }
    }

    #[tool(
        name = "war_answers",
        description = "A stage's answers from a person. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_answers(&self, Parameters(p): Parameters<AnswersParams>) -> ToolResult {
        value_of(
            "answers",
            crate::questions::answers_for(&self.repo, &p.alias, p.stage.as_deref()),
            "answers listed",
        )
    }

    // -- writes an agent may make --

    // Asking and answering are `war_note`'s (OW-ADR-0034): who may answer
    // follows the ladder there. No `war_answer` tool is served.
    #[tool(
        name = "war_ask",
        description = "Ask a person a question that blocks a stage; a person answers.",
        annotations(read_only_hint = false)
    )]
    fn war_ask(&self, Parameters(p): Parameters<AskParams>) -> ToolResult {
        report_of(
            "ask",
            crate::questions::ask(
                &self.repo,
                &p.alias,
                &crate::questions::Asking {
                    stage: &p.stage,
                    question: &p.question,
                    recommended: &p.recommend,
                    blocking: p.blocking,
                    ..crate::questions::Asking::default()
                },
            ),
        )
    }

    #[tool(
        name = "war_new",
        description = "Create a directory Warrant with stub atoms (`war plan new`).",
        annotations(read_only_hint = false)
    )]
    fn war_new(&self, Parameters(p): Parameters<NewParams>) -> ToolResult {
        // The title is written into manifest.toml as a quoted TOML string by
        // a pinned file; a quote, backslash or control character in it would
        // be a TOML injection from a tool argument. Refuse here, by name.
        if p.title
            .chars()
            .any(|c| c == '"' || c == '\\' || c.is_control())
        {
            return refused(
                "new",
                &RepoError::Message(
                    "new.title-unsafe: a title may not contain a double quote, a backslash \
                     or a control character (it is written into manifest.toml verbatim)"
                        .to_owned(),
                ),
            );
        }
        let profile = match p.profile.parse::<openwarrant_core::Profile>() {
            Ok(pr) => pr,
            Err(e) => return refused("new", &RepoError::Message(e.to_string())),
        };
        value_of(
            "new",
            crate::new::run(&self.repo, &p.title, profile)
                .map(|dir| serde_json::json!({"dir": self.repo.relative(&dir)})),
            "Warrant created; fill its atoms, then `war check`",
        )
    }

    // -- the ticket loop (OW-WAR-0147): no signature, no human act --

    #[tool(
        name = "war_prime",
        description = "What is open, one line each; or one Warrant in full. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_prime(&self, Parameters(p): Parameters<PrimeParams>) -> ToolResult {
        self.ticket("prime", None, |s| {
            if let Some(outcome) = p
                .ticket
                .as_deref()
                .and_then(|t| crate::pinned_target(&self.repo, t))
            {
                return Ok(outcome);
            }
            crate::ticket::prime(&self.repo, s, p.ticket.as_deref())
        })
    }

    #[tool(
        name = "war_context",
        description = "The records a task needs, cited by id, within a byte budget. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_context(&self, Parameters(p): Parameters<ContextParams>) -> ToolResult {
        let corpus = crate::corpus::held(&self.repo);
        let mut opts = crate::context::Options::with(
            crate::context::Preset::builtin(crate::context::DEFAULT_PRESET)
                .unwrap_or_else(|| unreachable!("the default preset is built in")),
            p.budget,
            p.tokens,
            p.depth,
        );
        opts.strict = p.strict;
        // W6: another repository's target reads through its pin, or says why
        // it could not.
        let pinned = match crate::repos::classify(&self.repo.root, &p.target) {
            crate::repos::Target::Local => None,
            crate::repos::Target::Undeclared(name) => {
                Some(Err(format!("{name} is not declared in [repos]")))
            }
            crate::repos::Target::Named(name, rest) => Some(Ok((name, rest))),
        };
        if let Some(pinned) = pinned {
            let mut report = crate::diagnostic::Report::default();
            let mut value = None;
            let read = pinned.and_then(|(name, rest)| {
                opts.exact = true;
                let located = crate::repos::locate(&self.repo.root, name)?;
                let also = rest.split('#').next().filter(|p| p.contains('/'));
                let commit = located.pin.commit.clone();
                let packet = crate::repos::read_at(&located, &commit, also, |other| {
                    let corpus = crate::corpus::Corpus::new(other);
                    let graph =
                        crate::context::context_graph(&corpus).map_err(|e| e.to_string())?;
                    let target = crate::context::relative_target(other, rest);
                    match crate::context::answer_with(&corpus, &graph, &target, &opts) {
                        crate::context::Answered::Units(p) => {
                            let mut v = crate::output::value(&p);
                            v["markdown"] = serde_json::json!(p.markdown);
                            Ok(v)
                        }
                        crate::context::Answered::Records(Ok(p)) => {
                            let mut v = crate::output::value(&p);
                            v["markdown"] = serde_json::json!(p.markdown);
                            Ok(v)
                        }
                        crate::context::Answered::Records(Err(r)) => Err(r.to_string()),
                    }
                })??;
                report.note(crate::repos::read_line(&located));
                value = Some(packet);
                Ok(())
            });
            if let Err(why) = read {
                report.push(crate::diagnostic::Diagnostic::unknown(
                    crate::repos::UNKNOWN_RULE,
                    p.target.clone(),
                    why,
                ));
            }
            let text = crate::output::envelope("context", &report, value);
            let mut value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                McpError::internal_error(format!("envelope is not JSON: {e}"), None)
            })?;
            lean(&mut value);
            return Ok(if report.is_ready() {
                CallToolResult::structured(value)
            } else {
                CallToolResult::structured_error(value)
            });
        }
        let graph = match crate::context::context_graph(&corpus) {
            Ok(g) => g,
            Err(e) => return refused("context", &e),
        };
        let target = crate::context::relative_target(&self.repo, &p.target);
        let result = match crate::context::answer_with(&corpus, &graph, &target, &opts) {
            crate::context::Answered::Records(r) => r,
            crate::context::Answered::Units(packet) => {
                let report = crate::context::units::report_of(&packet);
                let mut value = crate::output::value(&packet);
                if let Some(o) = value.as_object_mut() {
                    o.insert("markdown".to_owned(), serde_json::json!(packet.markdown));
                }
                let text = crate::output::envelope("context", &report, Some(value));
                let mut value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                    McpError::internal_error(format!("envelope is not JSON: {e}"), None)
                })?;
                lean(&mut value);
                return Ok(CallToolResult::structured(value));
            }
        };
        let report = crate::context::report_of(&result);
        let value = result.ok().map(|packet| {
            let mut v = crate::output::value(&packet);
            if let Some(o) = v.as_object_mut() {
                o.insert("markdown".to_owned(), serde_json::json!(packet.markdown));
            }
            v
        });
        let text = crate::output::envelope("context", &report, value);
        let mut value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| McpError::internal_error(format!("envelope is not JSON: {e}"), None))?;
        lean(&mut value);
        Ok(if report.is_ready() {
            CallToolResult::structured(value)
        } else {
            CallToolResult::structured_error(value)
        })
    }

    #[tool(
        name = "war_ready",
        description = "What can start now (`war status --ready`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_ready(&self, Parameters(p): Parameters<ActorParams>) -> ToolResult {
        self.ticket("ready", p.actor.as_deref(), crate::ticket::ready)
    }

    #[tool(
        name = "war_tickets",
        description = "Every Warrant with type, state and progress, filtered. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_tickets(&self, Parameters(p): Parameters<TicketsParams>) -> ToolResult {
        let filter = crate::ticket::Filter {
            kind: p.kind,
            labels: p.labels,
            state: p.state,
            text: p.text,
            search: p.search,
            epic: p.epic,
        };
        self.ticket("tickets", None, |s| {
            let work = crate::work::Work::read_with(&self.repo, s);
            crate::ticket::list(s, &work, &filter)
        })
    }

    #[tool(
        name = "war_create",
        description = "Create a Warrant (a ticket); returns its id. `sign` asks for a person's sign-off at the end.",
        annotations(read_only_hint = false)
    )]
    fn war_create(&self, Parameters(p): Parameters<CreateParams>) -> ToolResult {
        let args = crate::ticket::CreateArgs {
            title: p.title,
            summary: p.summary,
            items: p.items,
            body: p.body,
            priority: p.priority,
            kind: p.kind,
            labels: p.labels,
            part_of: p.part_of,
            issue: None,
            scope: crate::ticket::Scope {
                depends_on: p.depends_on,
                change: None,
                clauses: Vec::new(),
                paths: p.paths,
                tools: p.tools,
                commands: p.commands,
            },
            tests: p.tests,
        };
        self.ticket("create", p.actor.as_deref(), |s| {
            let created = crate::ticket::create(s, &args)?;
            if p.sign {
                crate::ticket::parts::signed(s, created)
            } else {
                Ok(created)
            }
        })
    }

    #[tool(
        name = "war_claim",
        description = "Claim an item or Warrant (`war start --claim`).",
        annotations(read_only_hint = false)
    )]
    fn war_claim(&self, Parameters(p): Parameters<ClaimParams>) -> ToolResult {
        self.ticket("claim", p.actor.as_deref(), |s| {
            crate::ticket::claim_scoped(s, &p.target, p.steal, &p.paths)
        })
    }

    #[tool(
        name = "war_start",
        description = "Take the next ready Warrant, or `target`, and return its brief; `claim` only claims.",
        annotations(read_only_hint = false)
    )]
    fn war_start(&self, Parameters(p): Parameters<StartParams>) -> ToolResult {
        if p.claim {
            // `war start <id> --claim`: the claim half alone.
            return self.ticket("claim", p.actor.as_deref(), |s| match p.target.as_deref() {
                Some(t) => crate::ticket::claim_scoped(s, t, p.steal, &p.paths),
                None => Ok(crate::ticket::Outcome::refused(
                    "start.claim-needs-target",
                    String::new(),
                    "`claim` needs a `target`",
                )),
            });
        }
        self.ticket("start", p.actor.as_deref(), |s| {
            let mut outcome = crate::ticket::start_cmd(s, p.target.as_deref())?;
            crate::session_usage::after_start(&self.repo.root, s, &mut outcome);
            Ok(outcome)
        })
    }

    #[tool(
        name = "war_heartbeat",
        description = "Renew the lease on your claims.",
        annotations(read_only_hint = false)
    )]
    fn war_heartbeat(&self, Parameters(p): Parameters<HeartbeatParams>) -> ToolResult {
        self.ticket("heartbeat", p.actor.as_deref(), |s| {
            crate::ticket::heartbeat(s, p.target.as_deref())
        })
    }

    #[tool(
        name = "war_done",
        description = "Close the Warrant you hold, or one item; `release` gives it back, `reopen` unticks.",
        annotations(read_only_hint = false)
    )]
    fn war_done(&self, Parameters(p): Parameters<DoneParams>) -> ToolResult {
        // `war done <id> --release` and `--reopen --reason`: the other
        // halves of done, each under its own command name.
        if p.release || p.reopen {
            let command = if p.release { "release" } else { "reopen" };
            return self.ticket(command, p.actor.as_deref(), |s| {
                let Some(target) = p.target.as_deref() else {
                    return Ok(crate::ticket::Outcome::refused(
                        "done.needs-target",
                        String::new(),
                        format!("`{command}` needs a `target`"),
                    ));
                };
                if p.release {
                    return crate::ticket::release(s, target, p.if_rev.as_deref());
                }
                match p.reason.as_deref() {
                    Some(reason) => crate::ticket::reopen(s, target, reason, p.if_rev.as_deref()),
                    None => Ok(crate::ticket::Outcome::refused(
                        "done.reopen-needs-reason",
                        String::new(),
                        "`reopen` needs a `reason`",
                    )),
                }
            });
        }
        self.ticket("done", p.actor.as_deref(), |s| {
            let target = match p.target.clone() {
                Some(t) => t,
                None => match crate::ticket::held_target(s)? {
                    Ok(t) => t,
                    Err(refusal) => return Ok(refusal),
                },
            };
            let mut outcome = crate::ticket::done_with(
                s,
                &target,
                p.note.as_deref(),
                p.if_rev.as_deref(),
                p.check,
            )?;
            crate::session_usage::after_done(&self.repo.root, s, &mut outcome, None);
            Ok(outcome)
        })
    }

    #[tool(
        name = "war_add",
        description = "Append an item to a Warrant's checklist.",
        annotations(read_only_hint = false)
    )]
    fn war_add(&self, Parameters(p): Parameters<AddParams>) -> ToolResult {
        self.ticket("add", p.actor.as_deref(), |s| {
            crate::ticket::add(s, &p.ticket, &p.text, &p.after, p.if_rev.as_deref())
        })
    }

    #[tool(
        name = "war_note",
        description = "Append a dated note to a Warrant or item, or ask or answer a question.",
        annotations(read_only_hint = false)
    )]
    fn war_note(&self, Parameters(p): Parameters<NoteParams>) -> ToolResult {
        self.ticket("note", p.actor.as_deref(), |s| {
            let act = match (&p.ask, &p.answer) {
                (Some(q), _) => crate::hotline::NoteAct::Ask(crate::questions::Asking {
                    question: q,
                    blocking: p.blocking,
                    needs_human: p.needs_human,
                    ..crate::questions::Asking::default()
                }),
                (None, Some(id)) => crate::hotline::NoteAct::Answer { id, text: &p.text },
                (None, None) => {
                    return crate::ticket::note(s, &p.target, &p.text, p.if_rev.as_deref());
                }
            };
            crate::hotline::note(&self.repo, &p.target, act, p.actor.as_deref())
        })
    }

    #[tool(
        name = "war_evidence_record",
        description = "Run the gates a Warrant cites and record their receipts.",
        annotations(read_only_hint = false)
    )]
    fn war_evidence_record(&self, Parameters(p): Parameters<EvidenceParams>) -> ToolResult {
        report_of(
            "evidence",
            crate::evidence::record_as(
                &self.repo,
                &p.alias,
                p.gate.as_deref(),
                p.evidence_ref.as_deref(),
                p.actor.as_deref(),
            ),
        )
    }

    #[tool(
        name = "war_deliver",
        description = "Record a Warrant's deliverables at their bytes on disk; run before `war_evidence_record`.",
        annotations(read_only_hint = false)
    )]
    fn war_deliver(&self, Parameters(p): Parameters<DeliverParams>) -> ToolResult {
        report_of(
            "deliver",
            crate::deliver::run(
                &self.repo,
                &p.alias,
                &crate::deliver::Options {
                    ids: &p.ids,
                    dry_run: p.dry_run,
                    ..Default::default()
                },
            ),
        )
    }

    #[tool(
        name = "war_standing_apply",
        description = "Check a Warrant against a standing authorization a person signed.",
        annotations(read_only_hint = false)
    )]
    fn war_standing_apply(&self, Parameters(p): Parameters<StandingApplyParams>) -> ToolResult {
        report_of(
            "standing",
            crate::standing_cmd::apply(&self.repo, &p.alias, p.class.as_deref(), p.dry_run),
        )
    }

    #[tool(
        name = "war_compile",
        description = "Compile Warrants to their generated projections.",
        annotations(read_only_hint = false)
    )]
    fn war_compile(&self, Parameters(p): Parameters<CompileParams>) -> ToolResult {
        let mut args = vec!["compile"];
        if let Some(alias) = p.alias.as_deref() {
            args.push(alias);
        }
        passthrough("compile", self_exec(&self.repo.root, &args))
    }

    #[tool(
        name = "war_gate_run",
        description = "Run registered gates without recording.",
        annotations(read_only_hint = false)
    )]
    fn war_gate_run(&self, Parameters(p): Parameters<GateRunParams>) -> ToolResult {
        report_of(
            "gate",
            crate::gate_cmd::run(&self.repo, true, p.gate.as_deref(), false, &[], &[], None),
        )
    }

    #[tool(
        name = "war_journal_backfill",
        description = "Backfill a Warrant's journal from its records.",
        annotations(read_only_hint = false)
    )]
    fn war_journal_backfill(&self, Parameters(p): Parameters<AliasParams>) -> ToolResult {
        report_of(
            "journal",
            crate::journal_cmd::backfill(&self.repo, &p.alias),
        )
    }

    #[tool(
        name = "war_plan_apply",
        description = "Apply a reviewed Draft Proposal v2; authorizes nothing.",
        annotations(read_only_hint = false)
    )]
    fn war_plan_apply(&self, Parameters(p): Parameters<PlanProposalParams>) -> ToolResult {
        // OW-WAR-0141: an issue in place of the sentence, read before anything
        // is written; a refusal here leaves the tree as it was.
        let intake = match intake_of(
            &self.repo,
            &p.sentence,
            p.issue.as_deref(),
            p.issue_file.as_deref(),
        ) {
            Ok(i) => i,
            Err(e) => return refused("plan.apply", &e),
        };
        let (sentence, answers) = drafted_from(intake.as_ref(), &p.sentence, &p.answers);
        let review = match crate::plan::review_of(&self.repo, intake.as_ref(), false, p.reviewed) {
            Ok(r) => r,
            Err(e) => return refused("plan.apply", &e),
        };
        if !review.completes_the_step() {
            return refused(
                "plan.apply",
                &RepoError::Message(
                    "§74.4 step 6: a proposal is applied only after a human reviewed it; \
                     pass reviewed=true once that happened"
                        .to_owned(),
                ),
            );
        }
        let answered: BTreeSet<String> = answers.keys().cloned().collect();
        let known = match crate::plan::known_refs(&self.repo) {
            Ok(k) => k,
            Err(e) => return refused("plan.apply", &e),
        };
        let request =
            match crate::plan::request(&self.repo, &sentence, &p.profile, &p.assurance, &answers) {
                Ok(r) => r,
                Err(e) => return refused("plan.apply", &e),
            };
        let (proposal, mut pipeline) =
            match crate::plan::validate_v2(&p.proposal_json, true, &answered, &known) {
                Ok(v) => v,
                // A thin issue: its question waits under docs/intake/, as on
                // the CLI path, and no alias is allocated.
                Err(e) => match crate::plan::record_questions(
                    &self.repo,
                    intake.as_ref(),
                    None,
                    &request,
                    &p.proposal_json,
                    &e,
                ) {
                    Ok(Some((report, asked))) => {
                        return envelope(
                            "plan.interview",
                            &report,
                            Some(crate::output::value(&asked)),
                        );
                    }
                    Ok(None) => return refused("plan.apply", &e),
                    Err(e) => return refused("plan.apply", &e),
                },
            };
        let record = intake.as_ref().and_then(crate::plan::Intake::record);
        match crate::plan::apply_with(
            &self.repo,
            &proposal,
            &mut pipeline,
            &request,
            None,
            &p.proposal_json,
            review,
            record.as_ref(),
        ) {
            Ok((applied, report)) => {
                envelope("plan.apply", &report, Some(crate::output::value(&applied)))
            }
            Err(e) => refused("plan.apply", &e),
        }
    }
}

/// `r` with `text` as the text the model reads; the envelope stays its
/// structured content, for a caller that asks for it.
fn with_text(mut r: CallToolResult, text: String) -> CallToolResult {
    r.content = vec![ContentBlock::text(text)];
    r
}

impl WarServer {
    /// `war status --next`: the text is what the CLI prints.
    fn next_answer(&self) -> ToolResult {
        match crate::next::run_with(&crate::corpus::held(&self.repo)) {
            Ok(n) => {
                let text = crate::next::render(&n).trim_end().to_owned();
                Ok(with_text(
                    value_of("next", Ok(n), "next action derived")?,
                    text,
                ))
            }
            Err(e) => refused("next", &e),
        }
    }

    /// `war status <id> --view <view>`, the one `war view show` was.
    fn show_view(&self, alias: &str, view: &str) -> ToolResult {
        match crate::warrants::kind_of(alias) {
            crate::warrants::IdKind::Light => {
                return self.ticket("show", None, |s| crate::ticket::show(s, alias));
            }
            crate::warrants::IdKind::ReadInPlace => {
                let alias = alias.to_owned();
                let repo = self.repo.clone();
                return self.ticket("show", None, move |_| {
                    Ok(crate::interop::adapters::show(&repo, &alias))
                });
            }
            crate::warrants::IdKind::Directory => {}
        }
        match crate::show::run(&self.repo, alias, view) {
            Ok(rendered) => {
                let r = value_of(
                    "show",
                    Ok(serde_json::json!({"alias": alias, "view": view, "rendered": rendered})),
                    "view rendered",
                )?;
                Ok(with_text(r, rendered))
            }
            Err(e) => refused("show", &e),
        }
    }

    /// Run one ticket command against this repository's ticket store and
    /// answer with its envelope as structured content; a refusal is a tool
    /// outcome, as elsewhere. The text the model reads is what the CLI prints
    /// (its findings, then its answer), not the envelope again: a brief is
    /// read once, in Markdown, and `war_done` answers in one line.
    fn ticket(
        &self,
        command: &str,
        actor: Option<&str>,
        run: impl FnOnce(&crate::ticket::Store) -> Result<crate::ticket::Outcome, RepoError>,
    ) -> ToolResult {
        // M11: every ticket tool call renews the acting agent's leases.
        let outcome = crate::ticket::Store::open(&self.repo, actor).and_then(|s| {
            s.renew_all();
            let answer = run(&s);
            // W3: a write looks for changes under running agents.
            crate::ticket::live::after_served_write(&s, command);
            answer
        });
        match outcome {
            Ok(o) => {
                let mut said = cli_text(&o);
                let text = crate::output::envelope(command, &o.report, Some(o.result));
                let mut value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                    McpError::internal_error(format!("envelope is not JSON: {e}"), None)
                })?;
                lean(&mut value);
                // A notice the envelope carried is read in the text too.
                if let Some(line) = value["notes"].as_array().and_then(|n| {
                    n.iter()
                        .filter_map(serde_json::Value::as_str)
                        .find(|l| l.starts_with("notice: "))
                }) {
                    said = format!("{}\n{line}", said.trim_end());
                }
                let mut r = if o.report.is_ready() {
                    CallToolResult::structured(value)
                } else {
                    CallToolResult::structured_error(value)
                };
                if !said.trim().is_empty() {
                    r.content = vec![ContentBlock::text(said)];
                }
                Ok(r)
            }
            Err(e) => refused(command, &e),
        }
    }
}

/// What `war <ticket command>` prints for `o` in a terminal: each finding as
/// `refused (rule): ...`, then the answer unless the command was refused.
fn cli_text(o: &crate::ticket::Outcome) -> String {
    use crate::diagnostic::Severity;
    let mut lines: Vec<String> = Vec::new();
    for d in &o.report.diagnostics {
        let tag = match d.severity {
            Severity::Error => "refused",
            Severity::Warn => "warning",
            Severity::Unknown => "UNKNOWN",
            _ => continue,
        };
        lines.push(format!("{tag} ({}): {}", d.rule, d.message));
    }
    let happened = o.report.count(Severity::Error) == 0 && !o.result.is_null();
    if !o.is_refused() || happened {
        lines.push(o.human.trim_end().to_owned());
    }
    lines.join("\n")
}
