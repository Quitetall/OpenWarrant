// SPDX-License-Identifier: Apache-2.0
//! t-3c16c W3: a change on the default branch reaches the agent it affects,
//! through the commands an agent runs, once; nobody else hears.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(root: &Path, actor: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap()])
        .args(args)
        .current_dir(root)
        .env("OPENWARRANT_ACTOR", actor)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("OPENWARRANT_NO_NOTICES")
        .env_remove("OPENWARRANT_CLAIM_ID")
        .output()
        .unwrap()
}

fn json(root: &Path, actor: &str, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend(args);
    let out = run(root, actor, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "{args:?}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.email=t@invalid", "-c", "user.name=t"])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn heard(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with("notice: "))
        .map(str::to_owned)
        .collect()
}

struct Program {
    _dir: tempfile::TempDir,
    root: PathBuf,
    alice: (String, PathBuf),
    bob: (String, PathBuf),
}

fn program() -> Program {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("r");
    std::fs::create_dir_all(root.join("docs")).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    assert_eq!(
        json(&root, "owner", &["init", "--namespace", "LV"])["exit_code"],
        0
    );
    std::fs::write(root.join("docs/plan.md"), "# Plan\n\nfirst\n").unwrap();
    let mut made = Vec::new();
    for (title, body) in [
        ("Alice works from the plan", "Follow docs/plan.md."),
        ("Bob works elsewhere", "Nothing cited."),
    ] {
        let v = json(&root, "owner", &["create", title, "--body", body]);
        made.push(v["result"]["id"].as_str().unwrap().to_owned());
    }
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "start"]);
    let start = |actor: &str, id: &str| -> PathBuf {
        let v = json(&root, actor, &["start", "--worktree", "--print", id]);
        assert_eq!(v["exit_code"], 0, "{v}");
        PathBuf::from(v["result"]["worktree"].as_str().unwrap())
    };
    let wa = start("alice", &made[0]);
    let wb = start("bob", &made[1]);
    Program {
        root,
        alice: (made[0].clone(), wa),
        bob: (made[1].clone(), wb),
        _dir: dir,
    }
}

fn amend(p: &Program, text: &str) {
    std::fs::write(p.root.join("docs/plan.md"), text).unwrap();
    git(&p.root, &["add", "-A"]);
    git(&p.root, &["commit", "-qm", "amend"]);
}

#[test]
fn only_the_affected_agent_hears_and_only_once() {
    let p = program();
    amend(&p, "# Plan\n\nSECOND\n");
    // Bob's write finds the change. It is not his.
    let bob_note = run(&p.bob.1, "bob", &["note", &p.bob.0, "going"]);
    assert!(bob_note.status.success());
    assert!(heard(&bob_note).is_empty());

    let first = run(&p.alice.1, "alice", &["view", "ready"]);
    let lines = heard(&first);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines[0],
        format!(
            "notice: docs/plan.md changed on main; run `war start {}`.",
            p.alice.0
        )
    );
    assert!(heard(&run(&p.alice.1, "alice", &["view", "ready"])).is_empty());
    assert!(heard(&run(&p.bob.1, "bob", &["view", "ready"])).is_empty());
}

#[test]
fn start_prints_the_change_and_records_it_as_seen() {
    let p = program();
    amend(&p, "# Plan\n\nSECOND\n");
    run(&p.bob.1, "bob", &["note", &p.bob.0, "going"]);

    let v = json(&p.alice.1, "alice", &["start", &p.alice.0]);
    assert_eq!(v["result"]["changed"][0]["source"], "docs/plan.md");
    assert!(
        v["result"]["changed"][0]["diff"]
            .as_str()
            .unwrap()
            .contains("+SECOND")
    );
    let again = json(&p.alice.1, "alice", &["start", &p.alice.0]);
    assert!(again["result"].get("changed").is_none(), "{again}");

    // A later change is a new notice.
    amend(&p, "# Plan\n\nTHIRD\n");
    run(&p.bob.1, "bob", &["note", &p.bob.0, "again"]);
    assert_eq!(
        heard(&run(&p.alice.1, "alice", &["view", "ready"])).len(),
        1
    );
}

#[test]
fn a_json_caller_reads_the_line_in_notes_and_stdout_stays_one_envelope() {
    let p = program();
    amend(&p, "# Plan\n\nSECOND\n");
    run(&p.bob.1, "bob", &["note", &p.bob.0, "going"]);
    let out = run(&p.alice.1, "alice", &["--json", "view", "ready"]);
    assert!(heard(&out).is_empty());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let notes = v["notes"].as_array().unwrap();
    assert!(
        notes
            .iter()
            .any(|n| n.as_str().unwrap().starts_with("notice: ")),
        "{notes:?}"
    );
}

#[test]
fn a_broken_notices_directory_never_fails_a_read() {
    let p = program();
    let notices = p.root.join(".git/openwarrant/notices");
    std::fs::remove_dir_all(&notices).unwrap();
    std::fs::write(&notices, "not a directory").unwrap();
    amend(&p, "# Plan\n\nSECOND\n");
    let read = run(&p.alice.1, "alice", &["view", "ready"]);
    assert!(read.status.success());
    assert!(read.stderr.is_empty(), "{:?}", read.stderr);
    let write = run(&p.bob.1, "bob", &["note", &p.bob.0, "going"]);
    assert!(write.status.success());
    assert!(String::from_utf8_lossy(&write.stderr).contains("notices.unavailable"));
}

#[test]
fn impact_names_the_claims_a_change_reaches_and_no_others() {
    let p = program();
    let v = json(&p.root, "owner", &["plan", "impact", &p.alice.0]);
    let claims = v["result"]["claims"].as_array().unwrap();
    assert_eq!(claims.len(), 1, "{claims:?}");
    assert_eq!(claims[0]["ticket"], p.alice.0);
    assert_eq!(claims[0]["actor"], "alice");
    assert_eq!(claims[0]["via"], "manifest");
}

#[test]
fn off_by_the_switch() {
    let p = program();
    amend(&p, "# Plan\n\nSECOND\n");
    run(&p.bob.1, "bob", &["note", &p.bob.0, "going"]);
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", p.alice.1.to_str().unwrap(), "view", "ready"])
        .env("OPENWARRANT_ACTOR", "alice")
        .env("OPENWARRANT_NO_NOTICES", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap();
    assert!(heard(&out).is_empty());
    assert_eq!(
        heard(&run(&p.alice.1, "alice", &["view", "ready"])).len(),
        1,
        "the switch leaves it waiting"
    );
}

#[test]
fn an_agent_on_mcp_reads_the_line_once() {
    use std::io::{BufRead, BufReader, Write};
    let p = program();
    amend(&p, "# Plan\n\nSECOND\n");
    run(&p.bob.1, "bob", &["note", &p.bob.0, "going"]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["mcp"])
        .current_dir(&p.alice.1)
        .env("OPENWARRANT_ACTOR", "alice")
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut call = |id: u64, method: &str, params: Value| -> String {
        let v = serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        stdin.write_all(format!("{v}\n").as_bytes()).unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        out.read_line(&mut line).unwrap();
        line
    };
    call(
        1,
        "initialize",
        serde_json::json!({"protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"test","version":"0"}}),
    );
    let tool = serde_json::json!({"name": "war_ready", "arguments": {}});
    let first = call(2, "tools/call", tool.clone());
    let second = call(3, "tools/call", tool);
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        first.contains("notice: docs/plan.md changed on main"),
        "{first}"
    );
    assert!(!second.contains("notice: "), "{second}");
}
