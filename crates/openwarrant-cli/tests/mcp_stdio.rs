// SPDX-License-Identifier: Apache-2.0
//! `war mcp` over stdio, driven by hand-written JSON-RPC (OW-ADR-0014).
//!
//! No client SDK: a foreign harness speaks exactly this, so the test does too.
//! Runs the shipped binary against the repository's own corpus, read-only.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Session {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
}

impl Session {
    fn start() -> Self {
        Self::start_at(&repo_root())
    }

    fn start_at(root: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
            .arg("mcp")
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("war mcp starts");
        let out = BufReader::new(child.stdout.take().expect("stdout"));
        let mut s = Self { child, out };
        let init = s.call(
            1,
            "initialize",
            serde_json::json!({"protocolVersion":"2025-06-18","capabilities":{},
                "clientInfo":{"name":"test","version":"0"}}),
        );
        assert_eq!(init["result"]["serverInfo"]["name"], "war");
        assert!(
            init["result"]["instructions"]
                .as_str()
                .unwrap_or("")
                .contains("cannot authorize"),
            "instructions state non-authority: {init}"
        );
        s.notify("notifications/initialized");
        s
    }

    fn send(&mut self, v: serde_json::Value) {
        let stdin = self.child.stdin.as_mut().expect("stdin");
        stdin.write_all(format!("{v}\n").as_bytes()).expect("write");
        stdin.flush().expect("flush");
    }

    fn notify(&mut self, method: &str) {
        self.send(serde_json::json!({"jsonrpc":"2.0","method":method}));
    }

    fn call(&mut self, id: u64, method: &str, params: serde_json::Value) -> serde_json::Value {
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let mut line = String::new();
        self.out.read_line(&mut line).expect("read");
        let v: serde_json::Value =
            serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON-RPC: {e}\n{line}"));
        assert_eq!(v["id"], id, "{v}");
        v
    }

    fn tool(&mut self, id: u64, name: &str, args: serde_json::Value) -> serde_json::Value {
        self.call(
            id,
            "tools/call",
            serde_json::json!({"name":name,"arguments":args}),
        )
    }

    fn finish(mut self) -> i32 {
        drop(self.child.stdin.take());
        self.child.wait().expect("wait").code().unwrap_or(-1)
    }
}

#[test]
fn tools_list_has_no_signing_or_ingesting_tool() {
    let mut s = Session::start();
    let list = s.call(2, "tools/list", serde_json::json!({}));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert!(names.len() >= 20, "{names:?}");
    for n in &names {
        assert!(
            !n.contains("ingest")
                && (!n.contains("sign") || n.ends_with("_list") || n.ends_with("_show")),
            "{n} in the tool table"
        );
    }
    for must in [
        "war_check",
        "war_next",
        "war_pins",
        "war_plan_request",
        "war_authorize_request",
    ] {
        assert!(names.contains(&must), "{must} missing from {names:?}");
    }
    assert_eq!(s.finish(), 0);
}

#[test]
fn calling_a_signing_tool_is_a_protocol_error_not_a_message() {
    let mut s = Session::start();
    for (id, name) in [(2, "war_sign"), (3, "war_resolve"), (4, "war_authorize")] {
        let r = s.tool(id, name, serde_json::json!({"alias":"OW-WAR-0062"}));
        assert!(r.get("error").is_some(), "{name} answered: {r}");
    }
    assert_eq!(s.finish(), 0);
}

#[test]
fn every_tool_answers_with_a_report_envelope() {
    let mut s = Session::start();
    let r = s.tool(2, "war_next", serde_json::json!({}));
    let sc = &r["result"]["structuredContent"];
    assert_eq!(sc["schema"], "oh.war/report/v1", "{r}");
    assert_eq!(sc["command"], "next");
    for key in ["diagnostics", "counts", "verdict", "exit_code", "result"] {
        assert!(sc.get(key).is_some(), "{key} missing: {sc}");
    }
    // The text block is the same envelope, so a client without structured
    // content support reads the same thing.
    let text = r["result"]["content"][0]["text"].as_str().expect("text");
    let parsed: serde_json::Value = serde_json::from_str(text).expect("text is JSON");
    assert_eq!(&parsed, sc);
    // A refusal is an is_error result carrying the envelope, not a crash.
    let r = s.tool(3, "war_check", serde_json::json!({"alias":"OW-WAR-9999"}));
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(
        r["result"]["structuredContent"]["schema"],
        "oh.war/report/v1"
    );
    assert_eq!(s.finish(), 0);
}

#[test]
fn a_request_half_writes_nothing() {
    let before = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root())
        .output()
        .expect("git")
        .stdout;
    let mut s = Session::start();
    let r = s.tool(
        2,
        "war_authorize_request",
        serde_json::json!({"alias":"OW-WAR-0063"}),
    );
    assert_eq!(
        r["result"]["structuredContent"]["command"], "authorize.request",
        "{r}"
    );
    assert_eq!(s.finish(), 0);
    let after = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root())
        .output()
        .expect("git")
        .stdout;
    assert_eq!(before, after, "a request half changed the tree");
}

#[test]
fn resources_are_listed_and_readable() {
    let mut s = Session::start();
    let list = s.call(2, "resources/list", serde_json::json!({}));
    let uris: Vec<&str> = list["result"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .map(|r| r["uri"].as_str().expect("uri"))
        .collect();
    for must in [
        "status://corpus",
        "pins://all",
        "next://",
        "sas://current",
        "warrant://OW-WAR-0062",
    ] {
        assert!(uris.contains(&must), "{must} missing");
    }
    let r = s.call(3, "resources/read", serde_json::json!({"uri":"next://"}));
    let text = r["result"]["contents"][0]["text"].as_str().expect("text");
    assert!(text.contains("oh.war/next/v1"), "{text}");
    let r = s.call(
        5,
        "resources/read",
        serde_json::json!({"uri":"warrant://OW-WAR-0062"}),
    );
    let text = r["result"]["contents"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"));
    assert!(text.contains("OW-WAR-0062"), "{text}");
    let r = s.call(
        4,
        "resources/read",
        serde_json::json!({"uri":"warrant://NOPE"}),
    );
    assert!(r.get("error").is_some(), "{r}");
    assert_eq!(s.finish(), 0);
}

#[test]
fn outside_a_repository_the_server_refuses_before_any_runtime() {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .arg("mcp")
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::null())
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty(), "nothing on stdout outside a repo");
}

#[test]
fn questions_tool_preserves_partial_store_diagnostics() {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let root =
        std::env::temp_dir().join(format!("ow-mcp-question-integrity-{}", std::process::id()));
    copy(
        &repo_root().join("conformance/fixtures/inbox/repository"),
        &root,
    );
    let mut s = Session::start_at(&root);
    let args = serde_json::json!({"alias":"IX-WAR-0002","open":true});
    let good = s.tool(2, "war_questions", args.clone());
    assert_eq!(good["result"]["structuredContent"]["exit_code"], 0);
    std::fs::write(
        root.join("docs/warrants/IX-WAR-0002/questions/Q-002.toml"),
        "broken TOML",
    )
    .unwrap();
    let bad = s.tool(3, "war_questions", args);
    let envelope = &bad["result"]["structuredContent"];
    assert_eq!(envelope["exit_code"], 2, "{bad}");
    assert_eq!(bad["result"]["isError"], true);
    assert_eq!(envelope["result"]["blocking_open"], 1);
    assert!(envelope.to_string().contains("question.malformed"));
    let answers = s.tool(4, "war_answers", serde_json::json!({"alias":"IX-WAR-0002"}));
    assert_eq!(answers["result"]["structuredContent"]["exit_code"], 2);
    assert_eq!(s.finish(), 0);
    std::fs::remove_dir_all(root).unwrap();
}
