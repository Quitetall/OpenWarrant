// SPDX-License-Identifier: Apache-2.0
//! `war admin mcp` — the Model Context Protocol server over stdio (OW-ADR-0014).
//!
//! The only place in this workspace where an async runtime exists. Every tool
//! body is a synchronous call into the module that already implements the
//! command; the runtime drives the transport and nothing else. Nothing here
//! can authorize, resolve, verify, accept, sign or ingest a response: those
//! are human acts (§27.2), and the tool table is asserted against a refusal
//! list in this module's tests.
//!
//! Stdout belongs to the transport. No function reached from a tool may print;
//! the two commands whose printers live in pinned files (`war admin compile`,
//! `war sign --show`) are run as a child process with stdout captured.

mod resources;
mod tools;

use std::sync::Arc;

use rmcp::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ErrorData as McpError,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ServerCapabilities, ServerConfig,
};
use rmcp::service::{RequestContext, RoleServer};

use crate::repo::{RepoError, Repository};

/// What the server tells a client at `initialize`. It leads with what an
/// agent most needs to hear (M9): ordinary work needs none of this. The
/// non-authority facts follow, as what the server does and does not do.
pub const INSTRUCTIONS: &str = "OpenWarrant (`war`) MCP server. Ordinary work needs none of \
this: edit, build and test as in any repository. To track work: `war_start` takes the next \
ready Warrant (or `target`) and returns its brief; `war_done` closes it with your last test \
run; `war_status` says where work stands. No ticket step needs a signature. This server \
cannot authorize, resolve, verify, accept a SAS, correct a delivered file, or sign anything: \
those are human acts done at a terminal with `war sign`. A reply is the CLI's text: a finding \
reads `refused (rule): ...`, and a nonzero exit ends `exit N, not_ready`. `json: true` returns \
the `oh.war/report/v1` envelope instead.";

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
    // Answering is `war_note`'s, where the ladder decides (OW-ADR-0034).
    "war_answer",
    "war_init",
    "war_gate_record",
    "war_plan_draft",
    // OW-ADR-0029: a class is proposed at the CLI and signed, accepted,
    // revoked or ingested only by a human.
    "war_standing_propose_ingest",
    "war_standing_accept",
    "war_standing_revoke",
    "war_standing_ingest",
];

/// How the server answers: which tools it serves and in what form.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// Serve every tool, not only [`tools::DEFAULT_TOOLS`].
    pub all: bool,
    /// Answer every call with the envelope, as `json: true` asks for one.
    pub structured: bool,
}

impl Options {
    /// `[mcp]` in `openwarrant.toml` (`tools = "default" | "all"`,
    /// `structured = true`), with `--all` on top. A table that does not read
    /// is refused by name, `mcp.config`, before anything is served.
    pub fn read(root: &camino::Utf8Path, all: bool) -> Result<Self, RepoError> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Table {
            #[serde(default)]
            tools: Option<String>,
            #[serde(default)]
            structured: bool,
        }
        #[derive(serde::Deserialize)]
        struct File {
            #[serde(default)]
            mcp: Option<Table>,
        }
        let path = root.join(crate::init::CONFIG_FILE);
        let text = crate::vfs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;
        let file: File = toml::from_str(&text)
            .map_err(|e| RepoError::Message(format!("mcp.config: {path}: the [mcp] table: {e}")))?;
        let table = file.mcp.unwrap_or(Table {
            tools: None,
            structured: false,
        });
        let all_tools = match table.tools.as_deref() {
            None | Some("default") => false,
            Some("all") => true,
            Some(other) => {
                return Err(RepoError::Message(format!(
                    "mcp.config: {path}: [mcp] tools is \"default\" or \"all\", not {other:?}"
                )));
            }
        };
        Ok(Self {
            all: all || all_tools,
            structured: table.structured,
        })
    }
}

/// The server: one repository, one tool table.
#[derive(Clone)]
pub struct WarServer {
    repo: Arc<Repository>,
    tool_router: ToolRouter<Self>,
    opts: Options,
}

impl WarServer {
    #[must_use]
    pub fn new(repo: Repository, opts: Options) -> Self {
        let mut tool_router = Self::tool_router();
        if !opts.all {
            for t in tool_router.list_all() {
                if !tools::DEFAULT_TOOLS.contains(&t.name.as_ref()) {
                    tool_router.remove_route(&t.name);
                }
            }
        }
        Self {
            repo: Arc::new(repo),
            tool_router,
            opts,
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

    /// The tools as `tools/list` serves them: each input schema flattened
    /// ([`flat`]) and given the `json` switch every call accepts.
    #[must_use]
    pub fn listed(&self) -> Vec<rmcp::model::Tool> {
        self.tool_router
            .list_all()
            .into_iter()
            .map(|mut t| {
                let mut schema = serde_json::Value::Object((*t.input_schema).clone());
                flat(&mut schema);
                if let Some(props) = schema
                    .as_object_mut()
                    .map(|o| {
                        o.entry("properties")
                            .or_insert_with(|| serde_json::json!({}))
                    })
                    .and_then(serde_json::Value::as_object_mut)
                {
                    props.insert("json".to_owned(), serde_json::json!({"type": "boolean"}));
                }
                if let serde_json::Value::Object(o) = schema {
                    t.input_schema = Arc::new(o);
                }
                t
            })
            .collect()
    }
}

/// A JSON Schema as an agent needs it: types, descriptions, required fields
/// and real defaults. What schemars adds for a validator (`$schema`, `title`,
/// integer formats and bounds, `additionalProperties: false`, the `null` in
/// an optional field's type, a default of nothing) is dropped. The server
/// still refuses an unknown field: `deny_unknown_fields` is on every shape.
fn flat(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            for k in ["$schema", "title", "format", "minimum", "maximum"] {
                o.remove(k);
            }
            if o.get("additionalProperties") == Some(&serde_json::Value::Bool(false)) {
                o.remove("additionalProperties");
            }
            if o.get("default").is_some_and(|d| {
                d.is_null()
                    || *d == serde_json::Value::Bool(false)
                    || d.as_array().is_some_and(Vec::is_empty)
                    || d.as_object().is_some_and(serde_json::Map::is_empty)
            }) {
                o.remove("default");
            }
            if let Some(serde_json::Value::Array(types)) = o.get("type") {
                let kept: Vec<serde_json::Value> =
                    types.iter().filter(|t| *t != "null").cloned().collect();
                if let [one] = kept.as_slice() {
                    o.insert("type".to_owned(), one.clone());
                }
            }
            if o.get("required")
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                o.remove("required");
            }
            // Recurse into subschemas only: a key of `properties` is a field
            // name (`title`, `format`), not a schema keyword.
            for (k, child) in o.iter_mut() {
                match (k.as_str(), child) {
                    ("properties" | "$defs" | "definitions", serde_json::Value::Object(m)) => {
                        m.values_mut().for_each(flat);
                    }
                    ("items" | "additionalProperties", c @ serde_json::Value::Object(_)) => flat(c),
                    ("anyOf" | "oneOf" | "allOf", serde_json::Value::Array(a)) => {
                        a.iter_mut().for_each(flat);
                    }
                    _ => {}
                }
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(flat),
        _ => {}
    }
}

/// What a caller reads when it did not ask for the envelope: the CLI's text
/// where the tool built one, else the envelope's findings and answer as
/// lines. A finding keeps its rule id (`refused (rule): ...`); a nonzero exit
/// ends the text as `exit N, verdict`.
#[must_use]
pub fn terse(mut r: CallToolResult) -> CallToolResult {
    let Some(env) = r.structured_content.take() else {
        return r;
    };
    let own = r
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.clone());
    // A tool that built no text of its own carries the envelope as its text.
    let as_built = env.to_string();
    let mut text = match own {
        Some(t) if t != as_built => t.trim_end().to_owned(),
        _ => lines_of(&env),
    };
    if let Some(code) = env["exit_code"].as_u64().filter(|c| *c != 0) {
        let verdict = env["verdict"].as_str().unwrap_or("not_ready");
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!("exit {code}, {verdict}"));
    }
    r.content = vec![rmcp::model::ContentBlock::text(text)];
    r
}

/// What a caller reads when it asked for the envelope: the envelope as
/// structured content, and the same envelope as its text block, for a client
/// that reads no structured content.
#[must_use]
pub fn enveloped(mut r: CallToolResult) -> CallToolResult {
    if let Some(env) = &r.structured_content {
        r.content = vec![rmcp::model::ContentBlock::text(env.to_string())];
    }
    r
}

/// An envelope as lines: each finding but a pass, its notes, then the answer (its
/// Markdown or rendering when it has one, else compact JSON).
fn lines_of(env: &serde_json::Value) -> String {
    let mut out: Vec<String> = Vec::new();
    for d in env["diagnostics"].as_array().into_iter().flatten() {
        let tag = match d["severity"].as_str() {
            Some("error") => "refused",
            Some("warn") => "warning",
            Some("unknown") => "UNKNOWN",
            _ => continue,
        };
        let rule = d["rule"].as_str().unwrap_or("");
        let file = d["file"].as_str().filter(|f| !f.is_empty());
        let msg = d["message"].as_str().unwrap_or("");
        out.push(match file {
            Some(f) => format!("{tag} ({rule}) {f}: {msg}"),
            None => format!("{tag} ({rule}): {msg}"),
        });
    }
    if let Some(folded) = env["folded"]["warn"].as_object() {
        let n: u64 = folded.values().filter_map(serde_json::Value::as_u64).sum();
        out.push(format!(
            "{n} more warning(s) folded; `verbose: true` lists them"
        ));
    }
    // A note is what the CLI prints beside its answer (which pin was read, a
    // notice of a change under you).
    out.extend(
        env["notes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned),
    );
    let result = &env["result"];
    if let Some(md) = result["markdown"]
        .as_str()
        .or_else(|| result["rendered"].as_str())
    {
        out.push(md.trim_end().to_owned());
    } else if !result.is_null() {
        out.push(result.to_string());
    }
    out.join("\n")
}

#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for WarServer {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions(INSTRUCTIONS);
        info.server_info = rmcp::model::Implementation::new("war", env!("CARGO_PKG_VERSION"));
        info
    }

    /// The router's call, with `json` taken out of the arguments first: it
    /// picks the reply's form and is no tool's parameter.
    async fn call_tool(
        &self,
        mut request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let asked = request.arguments.as_mut().and_then(|a| a.remove("json"));
        let json = match asked {
            None | Some(serde_json::Value::Null) => false,
            Some(serde_json::Value::Bool(b)) => b,
            Some(_) => return Err(McpError::invalid_params("`json` is a boolean", None)),
        };
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        Ok(match self.tool_router.call(tcc).await? {
            CallToolResponse::Complete(r) if json || self.opts.structured => {
                CallToolResponse::Complete(enveloped(r))
            }
            CallToolResponse::Complete(r) => CallToolResponse::Complete(terse(r)),
            other => other,
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let supports_cache_hints = context
            .protocol_version()
            .is_some_and(|version| version >= rmcp::model::ProtocolVersion::V_2026_07_28);
        Ok(ListToolsResult {
            result_type: Some(rmcp::model::ResultType::COMPLETE),
            tools: self.listed(),
            meta: None,
            next_cursor: None,
            ttl_ms: supports_cache_hints.then_some(0),
            cache_scope: supports_cache_hints.then_some(rmcp::model::CacheScope::Public),
        })
    }

    fn get_tool(&self, name: &str) -> Option<rmcp::model::Tool> {
        self.listed().into_iter().find(|t| t.name == name)
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
pub fn run(repo: Repository, opts: Options) -> Result<(), RepoError> {
    crate::ticket::live::serve(&repo.root);
    let server = WarServer::new(repo, opts);
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
pub fn describe(repo: Repository, opts: Options) -> String {
    let server = WarServer::new(repo, opts);
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
    if !opts.all {
        out.push_str("more tools: `war admin mcp --all` (or `[mcp] tools = \"all\"`) serves the whole table\n");
    }
    out.push_str("replies: the CLI's text; `json: true` (or `[mcp] structured = true`) returns the oh.war/report/v1 envelope\n");
    out.push_str("never registered (human acts, or out of scope for an agent):\n");
    for r in REFUSED_TOOLS {
        out.push_str(&format!("  {r}\n"));
    }
    out.push_str("resources: warrant://<alias>[/status|/journal], status://corpus, sas://current, pins://all, next://, context://index, context://<target>\n");
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
        let all = Options {
            all: true,
            structured: false,
        };
        let names = WarServer::new(repo(), all).tool_names();
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
            "standing_cmd::propose",
            "accept_ingest_with",
            "revoke_ingest_with",
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
        // M9: the first sentence after the server's name is that ordinary
        // work needs none of it.
        let lead = INSTRUCTIONS
            .split_once(". ")
            .map(|(_, rest)| rest)
            .unwrap_or_default();
        assert!(
            lead.starts_with("Ordinary work needs none of this"),
            "{lead}"
        );
    }

    /// The default table is exactly the daily set; `--all` keeps the rest,
    /// and neither holds a human act.
    #[test]
    fn the_default_table_is_the_daily_set_and_all_is_the_whole_table() {
        let default = WarServer::new(repo(), Options::default()).tool_names();
        assert_eq!(default, tools::DEFAULT_TOOLS);
        let all = WarServer::new(
            repo(),
            Options {
                all: true,
                structured: false,
            },
        )
        .tool_names();
        for t in tools::DEFAULT_TOOLS {
            assert!(all.iter().any(|n| n == t), "{t}");
        }
        assert!(all.len() > default.len() + 30, "{all:?}");
    }

    /// Token cost is a product measure: the default `tools/list` stays under
    /// 6,400 bytes (about 1,600 tokens). Raise it only on purpose.
    #[test]
    fn the_default_tools_list_stays_inside_its_budget() {
        let server = WarServer::new(repo(), Options::default());
        let listed = serde_json::to_string(&serde_json::json!({"tools": server.listed()})).unwrap();
        assert!(
            listed.len() <= 6_400,
            "default tools/list is {} bytes",
            listed.len()
        );
        // Flattened: nothing for a validator only, and every tool takes `json`.
        for needle in [
            "\"$schema\"",
            "\"null\"",
            "\"format\"",
            "additionalProperties\":false",
        ] {
            assert!(!listed.contains(needle), "{needle} in {listed}");
        }
        for t in server.listed() {
            // A field named like a keyword is still a field.
            assert!(t.input_schema.get("title").is_none(), "{}", t.name);
            let schema = serde_json::Value::Object((*t.input_schema).clone());
            for r in schema["required"].as_array().into_iter().flatten() {
                let r = r.as_str().unwrap_or_default();
                assert!(schema["properties"][r].is_object(), "{}: {r}", t.name);
            }
            assert!(schema["properties"]["json"].is_object(), "{}", t.name);
            let d = t.description.as_deref().unwrap_or("");
            assert!(
                !d.is_empty() && !d.contains('\n') && d.len() <= 120,
                "{}: {d}",
                t.name
            );
        }
    }

    /// A reply is text that keeps the rule and the exit; the envelope comes
    /// only when asked. A refused envelope reads `refused (rule): ...`.
    #[test]
    fn terse_keeps_the_rule_and_the_exit() {
        let env = serde_json::json!({
            "schema": "oh.war/report/v1",
            "diagnostics": [
                {"severity": "pass", "rule": "x.pass", "file": null, "message": "fine"},
                {"severity": "error", "rule": "work.claimed", "file": null, "message": "held by b"}
            ],
            "verdict": "not_ready", "exit_code": 1, "result": null
        });
        let text = terse(CallToolResult::structured_error(env.clone()));
        assert!(text.structured_content.is_none());
        let said = text.content[0].as_text().unwrap().text.clone();
        assert_eq!(said, "refused (work.claimed): held by b\nexit 1, not_ready");
        // A tool's own text is kept, and the exit still ends it.
        let mut own = CallToolResult::structured_error(env);
        own.content = vec![rmcp::model::ContentBlock::text("what the CLI says\n")];
        let said = terse(own).content[0].as_text().unwrap().text.clone();
        assert_eq!(said, "what the CLI says\nexit 1, not_ready");
    }

    #[test]
    fn an_mcp_table_is_read_and_a_bad_one_refused_by_name() {
        let dir = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-mcp-opts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(crate::init::CONFIG_FILE);
        std::fs::write(&file, "[mcp]\ntools = \"all\"\nstructured = true\n").unwrap();
        assert_eq!(
            Options::read(&dir, false).unwrap(),
            Options {
                all: true,
                structured: true
            }
        );
        std::fs::write(&file, "[project]\nname = \"x\"\n").unwrap();
        assert_eq!(Options::read(&dir, false).unwrap(), Options::default());
        assert!(Options::read(&dir, true).unwrap().all);
        std::fs::write(&file, "[mcp]\ntools = \"some\"\n").unwrap();
        let e = Options::read(&dir, false).unwrap_err().to_string();
        assert!(e.contains("mcp.config"), "{e}");
        std::fs::write(&file, "[mcp]\nstructure = true\n").unwrap();
        let e = Options::read(&dir, false).unwrap_err().to_string();
        assert!(e.contains("mcp.config"), "{e}");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
