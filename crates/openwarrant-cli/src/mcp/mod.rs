// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war mcp` — the Model Context Protocol server over stdio (OW-ADR-0014).
//!
//! The only place in this workspace where an async runtime exists. Every tool
//! body is a synchronous call into the module that already implements the
//! command; the runtime drives the transport and nothing else. Nothing here
//! can authorize, resolve, verify, accept, sign or ingest a response: those
//! are human acts (§27.2), and the tool table is asserted against a refusal
//! list in this module's tests.
//!
//! Stdout belongs to the transport. No function reached from a tool may print;
//! the two commands whose printers live in pinned files (`war compile`,
//! `war sign --show`) are run as a child process with stdout captured.

mod resources;
mod tools;

use std::sync::Arc;

use rmcp::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{
    ErrorData as McpError, ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};

use crate::repo::{RepoError, Repository};

/// What the server tells a client at `initialize`. Verbatim non-authority:
/// an agent reading this knows whose act is whose before it calls anything.
pub const INSTRUCTIONS: &str = "OpenWarrant (`war`) MCP server. It reads and drafts Work \
Authorization Records for a repository; it holds NO authority. It cannot authorize, resolve, \
verify, accept a SAS, correct a delivered artifact, or sign anything, and no tool ingests a \
signed response: those are human acts done at a terminal with `war sign`. Tools ending in \
`_request` emit the document a human will sign; they write nothing. `war_next` says whose \
action comes next and whether it is the agent's or a human's. `war_pins` lists the files an \
agent may not edit (pinned by resolved Warrants). Every tool returns an `oh.war/report/v1` \
envelope as structured content: `diagnostics[]` name a rule per finding, `verdict` is \
ready|not_ready, `exit_code` is what the CLI would have exited with.";

/// Tool names the server must never register. Asserted by a test below
/// against the live router, and by a source grep against `tools.rs`.
pub const REFUSED_TOOLS: &[&str] = &[
    "war_sign",
    "war_sign_ssh",
    "war_authorize",
    "war_authorize_ingest",
    "war_resolve",
    "war_resolve_ingest",
    "war_verify",
    "war_verify_ingest",
    "war_correct",
    "war_correct_ingest",
    "war_sas_accept",
    "war_sas_accept_ingest",
    "war_sas_propose",
    "war_kf",
    "war_telemetry",
    "war_migrate",
    "war_export",
    "war_bonsai",
    "war_eval",
    // An agent asks (war_ask); only a human answers.
    "war_answer",
    "war_init",
    "war_gate_record",
    "war_plan_draft",
];

/// The server: one repository, one tool table.
#[derive(Clone)]
pub struct WarServer {
    repo: Arc<Repository>,
    tool_router: ToolRouter<Self>,
}

impl WarServer {
    #[must_use]
    pub fn new(repo: Repository) -> Self {
        Self {
            repo: Arc::new(repo),
            tool_router: Self::tool_router(),
        }
    }

    /// Every registered tool name, sorted — for the refusal test and `--list`.
    #[must_use]
    pub fn tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .tool_router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        names.sort();
        names
    }
}

#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for WarServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions(INSTRUCTIONS);
        info.server_info = rmcp::model::Implementation::new("war", env!("CARGO_PKG_VERSION"));
        info
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        std::future::ready(resources::list(&self.repo))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResponse, McpError>> + Send + '_ {
        std::future::ready(
            resources::read(&self.repo, &request.uri).map(ReadResourceResponse::Complete),
        )
    }
}

/// Serve on stdio until the client closes the pipe. The runtime is built here
/// and dropped here; `Repository::discover` ran before it existed.
pub fn run(repo: Repository) -> Result<(), RepoError> {
    let server = WarServer::new(repo);
    // stdin/stdout go through tokio's blocking pool, not the reactor, so no
    // driver is needed; `enable_all` enables whatever the enabled features
    // provide and nothing more.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| RepoError::Message(format!("mcp: could not start the runtime: {e}")))?;
    runtime.block_on(async move {
        let service = rmcp::serve_server(server, rmcp::transport::io::stdio())
            .await
            .map_err(|e| RepoError::Message(format!("mcp: initialize failed: {e}")))?;
        service
            .waiting()
            .await
            .map_err(|e| RepoError::Message(format!("mcp: transport task failed: {e}")))?;
        Ok(())
    })
}

/// Print the tool table and the refusal list without serving — what an
/// operator reads to know what an agent can and cannot do here.
pub fn describe(repo: Repository) -> String {
    let server = WarServer::new(repo);
    let mut out = format!("tools ({}):\n", server.tool_names().len());
    let mut tools = server.tool_router.list_all();
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    for t in tools {
        out.push_str(&format!(
            "  {:<24} {}\n",
            t.name,
            t.description.as_deref().unwrap_or("")
        ));
    }
    out.push_str("never registered (human acts, or out of scope for an agent):\n");
    for r in REFUSED_TOOLS {
        out.push_str(&format!("  {r}\n"));
    }
    out.push_str("resources: warrant://<alias>[/status|/journal], status://corpus, sas://current, pins://all, next://\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> Repository {
        Repository::discover(Some(
            camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        ))
        .expect("this repository is an OpenWarrant repository")
    }

    #[test]
    fn no_refused_tool_is_registered_and_no_signing_or_ingesting_name_exists() {
        let names = WarServer::new(repo()).tool_names();
        assert!(names.len() >= 20, "{names:?}");
        for r in REFUSED_TOOLS {
            assert!(!names.iter().any(|n| n == r), "{r} is registered");
        }
        // A signing act under any spelling.
        for n in &names {
            assert!(
                !n.contains("sign") || n == "war_sign_list" || n == "war_sign_show",
                "{n}"
            );
            assert!(!n.contains("ingest"), "{n}");
        }
    }

    /// The source of the tool table names no ingest, sign or propose function.
    /// Defence in depth behind the router-level test above: it skips `//`
    /// lines and not block comments, which is fine for a second line of
    /// defence — the first one reads the live router.
    #[test]
    fn tools_source_never_calls_an_authority_seam() {
        let src = include_str!("tools.rs");
        for needle in [
            "::ingest(",
            "accept_ingest",
            "sign::run(",
            "sign::draft(",
            "sas::propose",
            "kf::",
            "telemetry::",
            "migrate::",
            "export::",
            "bonsai::",
            "init::",
            "record: true",
            "run_drafter",
            "println!",
            "print!(",
        ] {
            assert!(
                !src.lines()
                    .any(|l| !l.trim_start().starts_with("//") && l.contains(needle)),
                "tools.rs reaches {needle}"
            );
        }
    }

    #[test]
    fn instructions_state_non_authority_verbatim() {
        for phrase in ["cannot authorize", "sign anything", "human acts"] {
            assert!(INSTRUCTIONS.contains(phrase), "{phrase}");
        }
    }
}
