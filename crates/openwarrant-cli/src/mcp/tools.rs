// SPDX-License-Identifier: AGPL-3.0-or-later
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
    /// Local alias.
    pub alias: String,
    /// View name: `war`, `status`, `plan`, or another `war show` view.
    #[serde(default = "default_view")]
    pub view: String,
}

fn default_view() -> String {
    "war".to_owned()
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
    /// Only files pinned by RESOLVED Warrants (the ones an agent may not edit).
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
    /// The vague request, verbatim — one sentence is enough.
    pub sentence: String,
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
    /// The sentence the proposal answers (recorded under plan/).
    #[serde(default)]
    pub sentence: String,
    /// `delivery` or `decision`.
    #[serde(default = "default_profile")]
    pub profile: String,
    /// `basic`, `standard` or `high`.
    #[serde(default = "default_assurance")]
    pub assurance: String,
    /// Interview answers by question id.
    #[serde(default)]
    pub answers: BTreeMap<String, String>,
    /// §74.4 step 6: a human has reviewed this proposal. Required true to apply.
    #[serde(default)]
    pub reviewed: bool,
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
            crate::check::run(&self.repo, p.alias.as_deref(), p.generated),
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
                crate::status::build(&self.repo),
                "corpus status built",
            ),
        }
    }

    #[tool(
        name = "war_show",
        description = "Render a Warrant view (`war show <alias> <view>`). Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_show(&self, Parameters(p): Parameters<ShowParams>) -> ToolResult {
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
        description = "Files pinned by Warrants, with state and digest (`war pins`). With resolved_only, the files an agent may not edit. Read-only.",
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
        name = "war_next",
        description = "Whose act comes next — agent or human — and the command for it (`war next`). Never assigns a signing act to an agent. Read-only.",
        annotations(read_only_hint = true)
    )]
    fn war_next(&self, Parameters(_p): Parameters<NoParams>) -> ToolResult {
        value_of("next", crate::next::run(&self.repo), "next action derived")
    }

    #[tool(
        name = "war_dispatch",
        description = "Compile the Stage Dispatch packet (`oh.war/stage-dispatch/v1`) for a stage — the agent's context for that stage (`war dispatch`). Writes nothing to the repository.",
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
            kind,
            &p.prior_failure,
            Some(&scratch),
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
        description = "Emit the Draft Request (`oh.war/draft-request/v1`) for a vague sentence: the corpus, ADRs and answers a drafter needs (`war plan \"<sentence>\"`). Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_plan_request(&self, Parameters(p): Parameters<PlanRequestParams>) -> ToolResult {
        value_of(
            "plan.request",
            crate::plan::request(
                &self.repo,
                &p.sentence,
                &p.profile,
                &p.assurance,
                &p.answers,
            ),
            "draft request emitted; answer it with an `oh.war/draft-proposal/v2` document",
        )
    }

    #[tool(
        name = "war_plan_validate",
        description = "Run a Draft Proposal through §74.4's gauntlet without applying it (`war plan --proposal`). Writes nothing.",
        annotations(read_only_hint = true)
    )]
    fn war_plan_validate(&self, Parameters(p): Parameters<PlanProposalParams>) -> ToolResult {
        let answered: BTreeSet<String> = p.answers.keys().cloned().collect();
        let known = match crate::plan::known_refs(&self.repo) {
            Ok(k) => k,
            Err(e) => return refused("plan.validate", &e),
        };
        match crate::plan::validate_v2(&p.proposal_json, p.reviewed, &answered, &known) {
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

    // -- writes an agent may make --

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

    #[tool(
        name = "war_evidence_record",
        description = "Run the gates the Warrant's assurance atom cites and mint §44.6 receipts into gate-runs/ (`war evidence record`).",
        annotations(read_only_hint = false)
    )]
    fn war_evidence_record(&self, Parameters(p): Parameters<EvidenceParams>) -> ToolResult {
        report_of(
            "evidence",
            crate::evidence::record(&self.repo, &p.alias, p.gate.as_deref()),
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
        description = "Apply a REVIEWED Draft Proposal v2: creates the Warrant through `war new` and the seven §74.3 operations, recording request, proposal and pipeline under plan/ (`war plan --proposal <file> --reviewed --apply`). Refused unless `reviewed` is true.",
        annotations(read_only_hint = false)
    )]
    fn war_plan_apply(&self, Parameters(p): Parameters<PlanProposalParams>) -> ToolResult {
        if !p.reviewed {
            return refused(
                "plan.apply",
                &RepoError::Message(
                    "§74.4 step 6: a proposal is applied only after a human reviewed it; \
                     pass reviewed=true once that happened"
                        .to_owned(),
                ),
            );
        }
        let answered: BTreeSet<String> = p.answers.keys().cloned().collect();
        let known = match crate::plan::known_refs(&self.repo) {
            Ok(k) => k,
            Err(e) => return refused("plan.apply", &e),
        };
        let (proposal, mut pipeline) =
            match crate::plan::validate_v2(&p.proposal_json, true, &answered, &known) {
                Ok(v) => v,
                Err(e) => return refused("plan.apply", &e),
            };
        let request = match crate::plan::request(
            &self.repo,
            &p.sentence,
            &p.profile,
            &p.assurance,
            &p.answers,
        ) {
            Ok(r) => r,
            Err(e) => return refused("plan.apply", &e),
        };
        match crate::plan::apply(
            &self.repo,
            &proposal,
            &mut pipeline,
            &request,
            None,
            &p.proposal_json,
        ) {
            Ok((applied, report)) => {
                envelope("plan.apply", &report, Some(crate::output::value(&applied)))
            }
            Err(e) => refused("plan.apply", &e),
        }
    }
}
