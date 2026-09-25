// SPDX-License-Identifier: Apache-2.0
//! The ticket loop end to end (OW-WAR-0147), through the shipped binary and
//! through `war mcp`: create → ready → claim → done with no signature and no
//! human act, claims that exactly one of several processes wins, and a
//! ticket that `war check` validates for structure and nothing else.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-tickets-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "."])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let out = war(
        &root,
        &[
            "init",
            "--namespace",
            "TK",
            "--root",
            root.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    root
}

fn war(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .args(args)
        .output()
        .unwrap()
}

fn json(root: &Path, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = war(root, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{args:?}: not an envelope ({e}): {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn create_ready_claim_done_needs_no_signature_and_no_human() {
    let root = scratch("loop");
    let created = json(
        &root,
        &["create", "Ship it", "--item", "one", "--item", "two"],
    );
    assert_eq!(created["exit_code"], 0, "{created}");
    let id = created["result"]["id"].as_str().unwrap().to_owned();
    assert!(id.starts_with("t-"));

    let ready = json(&root, &["ready"]);
    let rows = ready["result"]["ready"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let first = format!("{id}/{}", rows[0]["item"].as_str().unwrap());

    for step in [
        vec!["claim", first.as_str()],
        vec!["done", first.as_str(), "--note", "did one"],
    ] {
        let v = json(&root, &step);
        assert_eq!(v["exit_code"], 0, "{step:?}: {v}");
    }
    let second = json(&root, &["ready"])["result"]["ready"][0]["item"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(json(&root, &["claim", &second])["exit_code"], 0);
    let done = json(&root, &["done", &second]);
    assert_eq!(done["result"]["ticket_state"], "done", "{done}");

    // The file is the state: both boxes ticked, by whom and when.
    let checklist =
        std::fs::read_to_string(root.join(format!("docs/tickets/{id}/atoms/15-checklist.md")))
            .unwrap();
    assert_eq!(checklist.matches("- [x] ").count(), 2, "{checklist}");
    assert!(checklist.contains("— done by claude, "), "{checklist}");
    assert!(checklist.contains(": did one"), "{checklist}");

    // Nothing asked for a signature, and nothing was signed.
    let tree: Vec<String> = walk(&root.join("docs/tickets"));
    assert!(
        !tree
            .iter()
            .any(|p| p.contains("authorization") || p.contains("resolution") || p.contains(".sig")),
        "{tree:?}"
    );
    // `war check` reads the ticket for structure only: no finding about a
    // missing authorization, evidence or verification.
    let check = json(&root, &["check", &id]);
    assert_eq!(check["exit_code"], 0, "{check}");
    assert!(
        check["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["rule"] == "ticket.well-formed"),
        "{check}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

fn walk(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p.display().to_string());
        }
    }
    out
}

/// Twelve `war claim` processes started together on one item: exactly one
/// exits 0, and every other one is refused naming the winner.
#[test]
fn of_concurrent_claim_processes_exactly_one_wins() {
    let root = scratch("race");
    let created = json(&root, &["create", "Race", "--item", "the only item"]);
    let id = created["result"]["id"].as_str().unwrap().to_owned();
    let item = created["result"]["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let target = format!("{id}/{item}");
    let children: Vec<(String, std::process::Child)> = (0..12)
        .map(|n| {
            let actor = format!("agent-{n}");
            let child = Command::new(env!("CARGO_BIN_EXE_war"))
                .current_dir(&root)
                .env("OPENWARRANT_NO_PROJECTS", "1")
                .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
                .args(["claim", &target, "--as", &actor])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            (actor, child)
        })
        .collect();
    let results: Vec<(String, Output)> = children
        .into_iter()
        .map(|(a, c)| (a, c.wait_with_output().unwrap()))
        .collect();
    let winners: Vec<&String> = results
        .iter()
        .filter(|(_, o)| o.status.success())
        .map(|(a, _)| a)
        .collect();
    assert_eq!(winners.len(), 1, "{winners:?}");
    for (actor, out) in &results {
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(
                err.contains("ticket.claimed-by-other") && err.contains(winners[0].as_str()),
                "{actor}: {err}"
            );
        }
    }
    // The winner's claim is journalled once.
    let journal =
        std::fs::read_to_string(root.join(format!("docs/tickets/{id}/journal.jsonl"))).unwrap();
    assert_eq!(
        journal.matches("\"ticket.claimed\"").count(),
        1,
        "{journal}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An unsigned ticket is a normal working state: `war check` on a program
/// holding one reports no error for it, and a malformed checklist is named.
#[test]
fn check_validates_structure_and_never_asks_for_authority() {
    let root = scratch("check");
    let id = json(&root, &["create", "Checked", "--item", "a"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let whole = json(&root, &["check"]);
    let errors: Vec<&serde_json::Value> = whole["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["severity"] == "error")
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    let path = root.join(format!("docs/tickets/{id}/atoms/15-checklist.md"));
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("- [?] what is this\n");
    std::fs::write(&path, text).unwrap();
    let bad = json(&root, &["check", &id]);
    assert_eq!(bad["exit_code"], 2, "{bad}");
    assert!(
        bad["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "ticket.checklist-malformed"),
        "{bad}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

struct Mcp {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
    next: u64,
}

impl Mcp {
    fn start(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
            .arg("mcp")
            .current_dir(root)
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut s = Self {
            child,
            out,
            next: 1,
        };
        s.call(
            "initialize",
            serde_json::json!({"protocolVersion":"2025-06-18","capabilities":{},
                "clientInfo":{"name":"test","version":"0"}}),
        );
        s.send(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        s
    }

    fn send(&mut self, v: serde_json::Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(format!("{v}\n").as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    fn call(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let id = self.next;
        self.next += 1;
        self.send(serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let mut line = String::new();
        self.out.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    fn tool(&mut self, name: &str, args: serde_json::Value) -> serde_json::Value {
        let v = self.call(
            "tools/call",
            serde_json::json!({"name": name, "arguments": args}),
        );
        v["result"]["structuredContent"].clone()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn an_agent_works_a_ticket_over_mcp() {
    let root = scratch("mcp");
    let mut mcp = Mcp::start(&root);
    let names: Vec<String> = mcp.call("tools/list", serde_json::json!({}))["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect();
    for want in [
        "war_create",
        "war_ready",
        "war_claim",
        "war_done",
        "war_add",
        "war_note",
        "war_prime",
        "war_show",
        "war_tickets",
    ] {
        assert!(names.iter().any(|n| n == want), "{want} in {names:?}");
    }
    let created = mcp.tool(
        "war_create",
        serde_json::json!({"title": "Over MCP", "items": ["first"]}),
    );
    let id = created["result"]["id"].as_str().unwrap().to_owned();
    let item = created["result"]["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let target = format!("{id}/{item}");
    assert_eq!(
        mcp.tool("war_claim", serde_json::json!({"target": target}))["exit_code"],
        0
    );
    // A second agent is refused by name.
    let other = mcp.tool(
        "war_claim",
        serde_json::json!({"target": target, "actor": "other"}),
    );
    assert_eq!(other["exit_code"], 2, "{other}");
    assert!(
        other["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("claude")
    );
    let added = mcp.tool(
        "war_add",
        serde_json::json!({"ticket": id, "text": "second", "after": [item]}),
    );
    assert_eq!(added["exit_code"], 0, "{added}");
    let noted = mcp.tool("war_note", serde_json::json!({"target": id, "text": "why"}));
    assert_eq!(noted["exit_code"], 0, "{noted}");
    let done = mcp.tool(
        "war_done",
        serde_json::json!({"target": target, "note": "ok"}),
    );
    assert_eq!(done["result"]["progress"]["done"], 1, "{done}");
    let prime = mcp.tool("war_prime", serde_json::json!({}));
    let md = prime["result"]["markdown"].as_str().unwrap();
    assert!(md.contains("second") && !md.contains("- [ ] first"), "{md}");
    let shown = mcp.tool("war_show", serde_json::json!({"alias": id}));
    assert!(
        shown["result"]["markdown"]
            .as_str()
            .unwrap()
            .contains("## Checklist")
    );
    let _ = std::fs::remove_dir_all(&root);
}
