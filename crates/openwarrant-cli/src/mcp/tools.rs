// SPDX-License-Identifier: Apache-2.0
//! The tool table. Three kinds: read-only, request halves (emit the document
//! a human signs; write nothing), and writes that an agent is allowed to make
//! (`war new`, evidence, compile, gate runs, journal backfill, a reviewed
//! `plan --apply`). No tool signs, ingests, proposes a SAS revision, or runs
//! the configured drafter; the tests in `mod.rs` grep this file for that.
//!
//! Every tool returns the same `oh.war/report/v1` envelope the CLI prints
//! under `--json`, as structured content, so an agent that learned the CLI
//! learns nothing new here.

use std::collections::{BTreeMap, BTreeSet};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ErrorData as McpError};
use rmcp::{tool, tool_router};
use serde::Deserialize;

use super::WarServer;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::RepoError;

// ---- parameter shapes -----------------------------------------------------

/// Who a ticket tool acts as. A name for coordination between agents; it
/// authorizes nothing (OW-WAR-0147).
#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct CreateParams {
    /// What this work accomplishes, in one sentence.
    pub title: String,
    /// Checklist items, one line each. May be empty: `war_add` adds later.
    #[serde(default)]
    pub items: Vec<String>,
    /// Context, decisions, links: Markdown for the Warrant's description.
    #[serde(default)]
    pub body: Option<String>,
    /// 0 (most urgent) to 4; default 2.
    #[serde(default)]
    pub priority: Option<u8>,
    /// One of the working form's `[fields] types` (OW-WAR-0148 M5).
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    /// Labels; refused outside a closed label set.
    #[serde(default)]
    pub labels: Vec<String>,
    /// The Warrant (an epic) this one is part of.
    #[serde(default)]
    pub part_of: Option<String>,
    /// Who is acting; defaults to the repository's configured performer.
    #[serde(default)]
    pub actor: Option<String>,
}

/// `war warrants` filters (OW-WAR-0148 M5); none given lists every Warrant.
#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct TicketsParams {
    /// Only Warrants of this type: a light one's type (bug), or a profile (delivery, openspec).
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    /// Only Warrants carrying every one of these labels.
    #[serde(default)]
    pub labels: Vec<String>,
    /// open, in_progress, done, or a declared state (in_review).
    #[serde(default)]
    pub state: Option<String>,
    /// A phrase anywhere in the Warrant's text (case-insensitive).
    #[serde(default)]
    pub text: Option<String>,
    /// Words, each beginning a word of the Warrant's text, any order.
    #[serde(default)]
    pub search: Option<String>,
    /// Only the Warrants part of this one (an epic).
    #[serde(default)]
    pub epic: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ActorParams {
    /// Who is acting; defaults to the repository's configured performer.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ClaimParams {
    /// An item (`i-...`, `t-.../i-...`) or a whole light Warrant (`t-...`); a unique prefix works.
    pub target: String,
    /// Take a claim older than the TTL (`[tickets] claim_ttl_minutes`). Journalled.
    #[serde(default)]
    pub steal: bool,
    /// Who is acting; defaults to the repository's configured performer.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct HeartbeatParams {
    /// One claimed item or ticket; omit to renew every claim the actor holds.
    #[serde(default)]
    pub target: Option<String>,
    /// Who is acting; defaults to the repository's configured performer.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct DoneParams {
    /// The claimed item (or a Warrant with nothing left open).
    pub target: String,
    /// What was done, written on the item's line for the next reader.
    #[serde(default)]
    pub note: Option<String>,
    /// Write only if the target is still at this revision (from `war_show`'s
    /// `revision`, or an item's); a stale one is refused
    /// `warrant.stale-revision`, naming the current one.
    #[serde(default)]
    pub if_rev: Option<String>,
    /// Run the item's tests and KPIs first and tick at `observed` only when
    /// they pass (`war done --check`); on a done item, raise its tick.
    #[serde(default)]
    pub check: bool,
    /// Who is acting; defaults to the repository's configured performer.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct AddParams {
    /// The Warrant (`t-...`).
    pub ticket: String,
    /// The item, one line.
    pub text: String,
    /// What the item waits on: items of this Warrant, Warrants, or `t-x/i-y`.
    #[serde(default)]
    pub after: Vec<String>,
    /// Write only if the target is still at this revision (from `war_show`'s
    /// `revision`, or an item's); a stale one is refused
    /// `warrant.stale-revision`, naming the current one.
    #[serde(default)]
    pub if_rev: Option<String>,
    /// Who is acting; defaults to the repository's configured performer.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct NoteParams {
    /// The Warrant (or one of its items).
    pub target: String,
    /// The note, Markdown.
    pub text: String,
    /// Write only if the target is still at this revision (from `war_show`'s
    /// `revision`, or an item's); a stale one is refused
    /// `warrant.stale-revision`, naming the current one.
    #[serde(default)]
    pub if_rev: Option<String>,
    /// Who is acting; defaults to the repository's configured performer.
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct PrimeParams {
    /// One Warrant in full; omit for every open one.
    #[serde(default)]
    pub ticket: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct NoParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct AliasParams {
    /// Local alias, e.g. `OW-WAR-0042`.
    pub alias: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct CheckParams {
    /// Local alias; omit to check the whole corpus.
    #[serde(default)]
    pub alias: Option<String>,
    /// Also drift-check the generated projections (`war check --generated`).
    #[serde(default)]
    pub generated: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct StatusParams {
    /// Local alias for one Warrant's status view; omit for the corpus.
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct ShowParams {
    /// A Warrant's id: an alias, a light one (`t-...`), or one read in place (`openspec:...`, `speckit:...`).
    pub alias: String,
    /// View name: `full_warrant` (default), `status`, or another `war show --view`.
    #[serde(default = "default_view")]
    pub view: String,
}

fn default_view() -> String {
    "full_warrant".to_owned()
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct DiffParams {
    /// Local alias.
    pub alias: String,
    /// Git ref to diff the compiled IR from; omit for the committed IR.
    #[serde(default)]
    pub from: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct GateRunParams {
    /// Gate key to run, e.g. `software.repo.war-check@1.0.0`; omit to run every registered gate.
    #[serde(default)]
    pub gate: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct PinsParams {
    /// Only files pinned by RESOLVED Warrants (a change to one is drafted with `war correct`).
    #[serde(default)]
    pub resolved_only: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct DispatchParams {
    /// Local alias.
    pub alias: String,
    /// Stage id, e.g. `STAGE-001`.
    pub stage: String,
    /// `initial` or `repair`.
    #[serde(default = "default_attempt")]
    pub attempt_kind: String,
    /// Evidence of a prior failure, for a repair attempt.
    #[serde(default)]
    pub prior_failure: Vec<String>,
}

fn default_attempt() -> String {
    "initial".to_owned()
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct VerifyRequestParams {
    /// Local alias.
    pub alias: String,
    /// Who performed the work (the verifier must be someone else). Defaults
    /// to the repository's configured performer.
    #[serde(default)]
    pub performer: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct VersionParams {
    /// SAS version, e.g. `0.1.0-draft.3`.
    pub version: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct PlanRequestParams {
    /// The vague request, verbatim — one sentence is enough. Empty when an
    /// issue is given instead.
    #[serde(default)]
    pub sentence: String,
    /// An issue file (`gh issue view <n> --json number,title,body,url`
    /// output), relative to the repository root (OW-WAR-0141). Its title and
    /// body are the request.
    #[serde(default)]
    pub issue_file: Option<String>,
    /// An issue number, fetched through `[intake] fetch_argv`. Refused,
    /// starting nothing, when no `[intake]` table is configured.
    #[serde(default)]
    pub issue: Option<String>,
    /// `delivery` or `decision`.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// `basic`, `standard` or `high`.
    #[serde(default = "default_assurance")]
    pub assurance: String,
    /// Interview answers by question id.
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
pub struct PlanProposalParams {
    /// The `oh.war/draft-proposal/v2` document, as JSON text.
    pub proposal_json: String,
    /// The sentence the proposal answers (recorded under plan/). Empty when
    /// an issue is given instead.
    #[serde(default)]
    pub sentence: String,
    /// The issue file the proposal answers, relative to the repository root
    /// (OW-WAR-0141). An applied Warrant records it in `plan/intake.json`.
    #[serde(default)]
    pub issue_file: Option<String>,
    /// The issue number the proposal answers, fetched through
    /// `[intake] fetch_argv`.
    #[serde(default)]
    pub issue: Option<String>,
    /// `delivery` or `decision`.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// `basic`, `standard` or `high`.
    #[serde(default = "default_assurance")]
    pub assurance: String,
    /// Interview answers by question id.
    #[serde(default)]
    pub answers: BTreeMap<String, String>,
    /// §74.4 step 6: a human has reviewed this proposal. Required true to
    /// apply, unless the proposal answers an issue and `[intake]
    /// policy_approval` is set, when the review is recorded as `policy`.
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
pub struct AskParams {
    /// Local alias.
    pub alias: String,
    /// The stage the answer unblocks.
    pub stage: String,
    /// The question, in full.
    pub question: String,
    /// Your recommended answer, so a human can reply in one word.
    #[serde(default)]
    pub recommend: String,
    /// The stage cannot proceed without the answer.
    #[serde(default)]
    pub blocking: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct QuestionsParams {
    /// One Warrant; omit for every Warrant.
    #[serde(default)]
    pub alias: Option<String>,
    /// Only what awaits an answer.
    #[serde(default)]
    pub open: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct AnswersParams {
    /// Local alias.
    pub alias: String,
    /// One stage; omit for every stage.
    #[serde(default)]
    pub stage: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct NewParams {
    /// Title of the new Warrant.
    pub title: String,
    /// `delivery` or `decision`.
    #[serde(default = "default_profile")]
    pub profile: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct EvidenceParams {
    /// Local alias.
    pub alias: String,
    /// One gate key to run; omit for every gate the assurance atom cites.
    #[serde(default)]
    pub gate: Option<String>,
    /// The Bonsai evidence document the Bonsai gate's receipt binds,
    /// `file:<path>#sha256:<digest>`; required for that gate only.
    #[serde(default)]
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct DeliverParams {
    /// Local alias.
    pub alias: String,
    /// Deliverable ids (`D-001`); omit for every deliverable declared.
    #[serde(default)]
    pub ids: Vec<String>,
    /// Report what would be recorded and write nothing.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct CompileParams {
    /// Local alias; omit to compile the whole corpus and its projections.
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct SignShowParams {
    /// A pending act as `war sign --list` names it: an alias, a SAS version, or `<alias>/<D-id>`.
    pub target: String,
}

// ---- result shaping -------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct StandingShowParams {
    /// A class id, `standing://<id>@<rev>` or `standing:<id>@<rev>`; omit for every class.
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema, Default)]
pub struct StandingApplyParams {
    /// Local alias of the Warrant to check against its class.
    pub alias: String,
    /// The class, `standing://<id>@<rev>`; defaults to the manifest's `[standing] ref`.
    #[serde(default)]
    pub class: Option<String>,
    /// Run every refusal and write nothing.
    #[serde(default)]
    pub dry_run: bool,
}

type ToolResult = Result<CallToolResult, McpError>;

fn envelope(command: &str, report: &Report, result: Option<serde_json::Value>) -> ToolResult {
    let text = crate::output::envelope(command, report, result);
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| McpError::internal_error(format!("envelope is not JSON: {e}"), None))?;
    Ok(CallToolResult::structured(value))
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
        description = "Run every Phase 1 check over one Warrant or the corpus (`war check`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_check(&self, Parameters(p): Parameters<CheckParams>) -> ToolResult {
        report_of(
            "check",
            crate::check::run_with(
                &crate::corpus::held(&self.repo),
                p.alias.as_deref(),
                p.generated,
            ),
        )
    }

    #[tool(
        name = "war_status",
        description = "Corpus status (`oh.war/corpus-status/v1`: ladders, Objectives, next actionable) or one Warrant's status view. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_status(&self, Parameters(p): Parameters<StatusParams>) -> ToolResult {
        match p.alias {
            Some(alias) => value_of(
                "status",
                crate::show::run(&self.repo, &alias, "status")
                    .map(|rendered| serde_json::json!({"alias": alias, "rendered": rendered})),
                "status view rendered",
            ),
            None => value_of(
                "status",
                crate::corpus::held(&self.repo).status().cloned(),
                "corpus status built",
            ),
        }
    }

    #[tool(
        name = "war_show",
        description = "Show a Warrant by its id as the document a person reads: an alias as one of its views (`war show <alias> --view <view>`), a light Warrant (`war show t-...`), or one read in place (`openspec:...`, `speckit:...`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_show(&self, Parameters(p): Parameters<ShowParams>) -> ToolResult {
        match crate::warrants::kind_of(&p.alias) {
            crate::warrants::IdKind::Light => {
                return self.ticket("show", None, |s| crate::ticket::show(s, &p.alias));
            }
            crate::warrants::IdKind::ReadInPlace => {
                let alias = p.alias.clone();
                let repo = self.repo.clone();
                return self.ticket("show", None, move |_| {
                    Ok(crate::interop::adapters::show(&repo, &alias))
                });
            }
            crate::warrants::IdKind::Directory => {}
        }
        value_of(
            "show",
            crate::show::run(&self.repo, &p.alias, &p.view)
                .map(|rendered| serde_json::json!({"alias": p.alias, "view": p.view, "rendered": rendered})),
            "view rendered",
        )
    }

    #[tool(
        name = "war_journal",
        description = "The Warrant's journal, rendered (`war journal <alias>`). Read-only.",
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
        description = "Semantic diff of a Warrant's compiled IR against the committed IR or a git ref (`war diff`). Read-only.",
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
        description = "List the registered gates and their askability without running any (`war gate`). Read-only.",
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
        description = "The accepted SAS revision and what pins to it (`war sas status`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_sas_status(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        report_of("sas.status", crate::sas::status(&self.repo))
    }

    #[tool(
        name = "war_resolve_dry_run",
        description = "§56.1's thirteen requirements assessed for a Warrant without resolving anything (`war resolve --dry-run`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_resolve_dry_run(&self, Parameters(p): Parameters<AliasParams>) -> ToolResult {
        report_of("resolve.dry_run", crate::resolve::run(&self.repo, &p.alias))
    }

    #[tool(
        name = "war_sign_list",
        description = "The acts waiting for a human signature (`war sign --list`). Read-only; signing itself happens at a terminal.",
        annotations(read_only_hint = true)
    )]
    fn war_sign_list(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        value_of(
            "sign.list",
            crate::sign::list(&self.repo).map(|pending| {
                let lines: Vec<String> = pending.iter().map(crate::sign::line).collect();
                serde_json::json!({"pending": lines, "count": lines.len(),
                    "how": "a human runs `war sign <target> --ssh-sign` at a terminal"})
            }),
            "pending acts listed",
        )
    }

    #[tool(
        name = "war_sign_show",
        description = "Show what one pending act would sign, as the human will see it (`war sign <target> --show`). Read-only.",
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
        description = "Files pinned by Warrants, with state and digest (`war pins`). With resolved_only, the files closed Warrants pin; a change to one is drafted with `war correct`. Read-only.",
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
        description = "Standing authorizations (OW-ADR-0029): each class's state, signer, expiry, count used, and what each glob matches today (`war standing show`). Read-only.",
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
        description = "What is ready and whose step each item is, agent or human, with the command for it (`war next`). A signing step is always a human's. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_next(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        value_of(
            "next",
            crate::next::run_with(&crate::corpus::held(&self.repo)),
            "next action derived",
        )
    }

    // read_only_hint stays true: the journal line a compile appends is the
    // Warrant's own history of being compiled, not a change to its records.
    #[tool(
        name = "war_dispatch",
        description = "Compile the Stage Dispatch packet (`oh.war/stage-dispatch/v1`) for a stage — the agent's context for that stage, with its token estimate and budget (`war dispatch`). Records a `dispatch.compiled` journal event; writes nothing else.",
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
        description = "Emit the authorization request a human will sign (`war authorize <alias>`). Writes nothing; authorization is a human act.",
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
        description = "Emit the resolution request with §56.1's requirements assessed (`war resolve <alias>`). Writes nothing; resolution is a human act.",
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
        description = "Emit the verification request for an INDEPENDENT verifier (`war verify <alias>`). Writes nothing; the verdicts come back through `war verify --response`, a human act.",
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
        description = "Emit the SAS acceptance request a human will sign (`war sas accept <version>`). Writes nothing.",
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
        description = "Emit the Draft Request (`oh.war/draft-request/v1`) for a vague sentence, or for an issue (`issue_file`, or `issue` through `[intake] fetch_argv`): the corpus, ADRs and answers a drafter needs (`war plan \"<sentence>\"`, `war plan --issue-file <f>`). Writes nothing.",
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
        description = "Run a Draft Proposal through §74.4's gauntlet without applying it (`war plan --proposal`). Writes nothing.",
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
        description = "Every question asked of the human across the corpus, blocking and open first, each with the command that answers it (`war questions`). Read-only.",
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
        description = "The answers a human gave for a stage, to read BEFORE performing it (`war answers`). Read-only.",
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

    // Asking is an agent's act; answering is not, and `war_answer` is in
    // REFUSED_TOOLS for the same reason `war_sign` is (OW-WAR-0069).
    #[tool(
        name = "war_ask",
        description = "Ask the human a question that blocks a stage (`war ask`): writes questions/Q-nnn.toml and a journal event. Answering is a human act and is not available here.",
        annotations(read_only_hint = false)
    )]
    fn war_ask(&self, Parameters(p): Parameters<AskParams>) -> ToolResult {
        report_of(
            "ask",
            crate::questions::ask(
                &self.repo,
                &p.alias,
                &p.stage,
                &p.question,
                &p.recommend,
                p.blocking,
            ),
        )
    }

    #[tool(
        name = "war_new",
        description = "Create a new Warrant directory with stub atoms (`war new`). Writes under docs/warrants/<alias>/ only.",
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
        description = "Read this first (`war prime`): open Warrants with their remaining items, who holds which claim, recent notes, done work compacted. Markdown in result.markdown. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_prime(&self, Parameters(p): Parameters<PrimeParams>) -> ToolResult {
        self.ticket("prime", None, |s| {
            crate::ticket::prime(s, p.ticket.as_deref())
        })
    }

    #[tool(
        name = "war_ready",
        description = "What can start now (`war ready`): open, unclaimed, unblocked items of light Warrants, most urgent and oldest first. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_ready(&self, Parameters(p): Parameters<ActorParams>) -> ToolResult {
        self.ticket("ready", p.actor.as_deref(), crate::ticket::ready)
    }

    #[tool(
        name = "war_tickets",
        description = "Every Warrant with its type, state and progress (`war warrants`, also `war tickets`): light ones in result.tickets, directory and read-in-place ones in result.warrants; optionally filtered by type, labels, state, a phrase, search words or epic: exactly the Warrants every filter admits. Read-only.",
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
            let others = crate::ticket::Others::of(&self.repo)?;
            crate::ticket::list(s, &others, &filter)
        })
    }

    #[tool(
        name = "war_create",
        description = "Create a Warrant (`war create`, a ticket): a title, optional checklist items and description. Workable at once; nothing is signed. Returns its id (t-...).",
        annotations(read_only_hint = false)
    )]
    fn war_create(&self, Parameters(p): Parameters<CreateParams>) -> ToolResult {
        let args = crate::ticket::CreateArgs {
            title: p.title,
            items: p.items,
            body: p.body,
            priority: p.priority,
            kind: p.kind,
            labels: p.labels,
            part_of: p.part_of,
            issue: None,
        };
        self.ticket("create", p.actor.as_deref(), |s| {
            crate::ticket::create(s, &args)
        })
    }

    #[tool(
        name = "war_claim",
        description = "Claim an item or a whole light Warrant (`war claim`) so no other agent works it: atomic, journalled, refused by name when someone else holds it. `steal` takes a claim past its TTL.",
        annotations(read_only_hint = false)
    )]
    fn war_claim(&self, Parameters(p): Parameters<ClaimParams>) -> ToolResult {
        self.ticket("claim", p.actor.as_deref(), |s| {
            crate::ticket::claim_cmd(s, &p.target, p.steal)
        })
    }

    #[tool(
        name = "war_heartbeat",
        description = "Renew the lease on your claims (`war heartbeat`), or on the one named, so no other agent reclaims them while you work. Every call of the Warrant loop's tools renews them too; a claim whose lease runs out is taken by a plain claim.",
        annotations(read_only_hint = false)
    )]
    fn war_heartbeat(&self, Parameters(p): Parameters<HeartbeatParams>) -> ToolResult {
        self.ticket("heartbeat", p.actor.as_deref(), |s| {
            crate::ticket::heartbeat(s, p.target.as_deref())
        })
    }

    #[tool(
        name = "war_done",
        description = "Finish a claimed item (`war done`): ticks its checkbox in the Warrant's checklist with who, when and an optional note, journals it, releases the claim.",
        annotations(read_only_hint = false)
    )]
    fn war_done(&self, Parameters(p): Parameters<DoneParams>) -> ToolResult {
        self.ticket("done", p.actor.as_deref(), |s| {
            crate::ticket::done_with(
                s,
                &p.target,
                p.note.as_deref(),
                p.if_rev.as_deref(),
                p.check,
            )
        })
    }

    #[tool(
        name = "war_add",
        description = "Append an item to a Warrant's checklist (`war add`), optionally waiting on other items or Warrants (`after`).",
        annotations(read_only_hint = false)
    )]
    fn war_add(&self, Parameters(p): Parameters<AddParams>) -> ToolResult {
        self.ticket("add", p.actor.as_deref(), |s| {
            crate::ticket::add(s, &p.ticket, &p.text, &p.after, p.if_rev.as_deref())
        })
    }

    #[tool(
        name = "war_note",
        description = "Append a dated note to a Warrant (`war note`): the durable context the next agent or person reads in `war prime`.",
        annotations(read_only_hint = false)
    )]
    fn war_note(&self, Parameters(p): Parameters<NoteParams>) -> ToolResult {
        self.ticket("note", p.actor.as_deref(), |s| {
            crate::ticket::note(s, &p.target, &p.text, p.if_rev.as_deref())
        })
    }

    #[tool(
        name = "war_evidence_record",
        description = "Run the gates the Warrant's assurance atom cites and mint §44.6 receipts into gate-runs/ (`war evidence record`).",
        annotations(read_only_hint = false)
    )]
    fn war_evidence_record(&self, Parameters(p): Parameters<EvidenceParams>) -> ToolResult {
        report_of(
            "evidence",
            crate::evidence::record(
                &self.repo,
                &p.alias,
                p.gate.as_deref(),
                p.evidence_ref.as_deref(),
            ),
        )
    }

    #[tool(
        name = "war_deliver",
        description = "Declare a Warrant's deliverables delivered (`war deliver`): record §37.2 provenance on each — the sha256 of the file now, how it was made, the build of war that recorded it — and set content_addressed. Refuses a resolved Warrant, a missing file, and a path a later authorized Warrant governs (OW-ADR-0021); any refusal writes nothing. Run it before `war_evidence_record`: deliverables.toml is bound into every tree-bound receipt.",
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
        description = "The coverage check of a standing authorization (`war standing apply`): a Warrant inside a class a human signed is authorized in that human's name; one outside is refused by the term it breaks and nothing is written. Accepting, revoking or signing a class is not a tool.",
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
        description = "Compile Warrants to their generated projections (`war compile`).",
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
        description = "Execute registered gates and report their verdicts without recording (`war gate --run`). Recording is `war evidence record`.",
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
        description = "Backfill a Warrant's journal from its records (`war journal <alias> --backfill`).",
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
        description = "Apply a REVIEWED Draft Proposal v2: creates the Warrant through `war new` and the seven §74.3 operations, recording request, proposal and pipeline under plan/ (`war plan --proposal <file> --reviewed --apply`). Refused unless `reviewed` is true, or the proposal answers an issue (`issue_file`/`issue`) under `[intake] policy_approval`; an issue-linked Warrant records plan/intake.json. Authorizes nothing.",
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

impl WarServer {
    /// Run one ticket command against this repository's ticket store and
    /// answer with its envelope; a refusal is a tool outcome, as elsewhere.
    fn ticket(
        &self,
        command: &str,
        actor: Option<&str>,
        run: impl FnOnce(&crate::ticket::Store) -> Result<crate::ticket::Outcome, RepoError>,
    ) -> ToolResult {
        // M11: every ticket tool call renews the acting agent's leases.
        let outcome = crate::ticket::Store::open(&self.repo, actor).and_then(|s| {
            s.renew_all();
            run(&s)
        });
        match outcome {
            Ok(o) => {
                let text = crate::output::envelope(command, &o.report, Some(o.result));
                let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                    McpError::internal_error(format!("envelope is not JSON: {e}"), None)
                })?;
                Ok(if o.report.is_ready() {
                    CallToolResult::structured(value)
                } else {
                    CallToolResult::structured_error(value)
                })
            }
            Err(e) => refused(command, &e),
        }
    }
}
