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

#[test]
fn claiming_a_whole_ticket_names_a_command_that_works() {
    let root = scratch("hint");
    let created = json(
        &root,
        &["create", "Two steps", "--item", "one", "--item", "two"],
    );
    let id = created["result"]["id"].as_str().unwrap().to_owned();
    let out = war(&root, &["claim", &id]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    // A whole ticket closes in one call: the hint names it, and it works.
    let hinted = text
        .split('`')
        .find(|s| s.starts_with("war done "))
        .unwrap_or_else(|| panic!("no done command in: {text}"))
        .to_owned();
    let target = hinted.split_whitespace().nth(2).unwrap().to_owned();
    assert_eq!(target, id, "{hinted}");
    // The refusal beside it: another actor cannot close what this one holds.
    let other = json(&root, &["done", &id, "--as", "mallory"]);
    assert_ne!(other["exit_code"], 0, "{other}");
    let progress = json(&root, &["show", &id]);
    assert_eq!(
        progress["result"]["ticket"]["done"], 0,
        "nothing ticked: {progress}"
    );
    let done = json(&root, &["done", &target, "--note", "first"]);
    assert_eq!(done["exit_code"], 0, "the hinted command: {done}");
    assert_eq!(done["result"]["ticket_state"], "done", "{done}");
    assert_eq!(
        done["result"]["progress"]["done"], 2,
        "every item at once: {done}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `war start` takes the next ready Warrant whole and prints only it: its
/// description, open items and notes, and the command that closes it. Run
/// again by the same actor it prints the same one (a session resumes); another
/// actor gets the next; with nothing left, it says so and claims nothing.
#[test]
fn start_takes_the_next_warrant_and_prints_only_it() {
    let root = scratch("start");
    let first = json(
        &root,
        &[
            "create",
            "First job",
            // Next by priority, not by a tie-break on random ids (§11.5).
            "--priority",
            "1",
            "--body",
            "Why the first job matters.",
            "--item",
            "alpha",
            "--item",
            "beta",
        ],
    )["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = json(&root, &["create", "Second job", "--item", "gamma"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        json(&root, &["note", &first, "use the old parser"])["exit_code"],
        0
    );
    let out = war(&root, &["start", "--as", "ann"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let brief = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        brief.starts_with(&format!("# {first} — First job")),
        "{brief}"
    );
    for want in [
        "Why the first job matters.",
        "- [ ] alpha",
        "- [ ] beta",
        "use the old parser",
    ] {
        assert!(brief.contains(want), "missing {want:?}: {brief}");
    }
    assert!(
        brief.contains(&format!("&& war done {first} --note")),
        "{brief}"
    );
    // Only this Warrant: nothing of the other.
    assert!(
        !brief.contains("Second job") && !brief.contains("gamma"),
        "{brief}"
    );
    // The same actor again: the one it holds, no second claim.
    let again = json(&root, &["start", "--as", "ann"]);
    assert_eq!(again["result"]["ticket"], first.as_str(), "{again}");
    // Another actor: the next one.
    let other = json(&root, &["start", "--as", "bob"]);
    assert_eq!(other["result"]["ticket"], second.as_str(), "{other}");
    // A third: nothing ready, and nothing claimed.
    let none = war(&root, &["start", "--as", "cy"]);
    assert!(none.status.success());
    assert!(
        String::from_utf8_lossy(&none.stdout).starts_with("nothing tracked is ready"),
        "{}",
        String::from_utf8_lossy(&none.stdout)
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `war done` with no id closes the one Warrant the actor holds, every item
/// in one write; with nothing held it is refused by name and writes nothing.
#[test]
fn done_with_no_id_closes_what_you_hold() {
    let root = scratch("done-held");
    let id = json(
        &root,
        &[
            "create", "Job", "--item", "one", "--item", "two", "--item", "three",
        ],
    )["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let refused = json(&root, &["done", "--as", "ann", "--note", "x"]);
    assert_eq!(
        refused["diagnostics"][0]["rule"], "ticket.nothing-held",
        "{refused}"
    );
    assert_eq!(json(&root, &["start", "--as", "ann"])["exit_code"], 0);
    let done = json(&root, &["done", "--as", "ann", "--note", "all three"]);
    assert_eq!(done["exit_code"], 0, "{done}");
    assert_eq!(done["result"]["ticket"], id.as_str());
    assert_eq!(done["result"]["progress"]["done"], 3, "{done}");
    assert_eq!(
        done["result"]["items"].as_array().map(Vec::len),
        Some(3),
        "{done}"
    );
    // One note on the Warrant, not a copy on every line.
    let one = String::from_utf8_lossy(&war(&root, &["view", "prime", &id]).stdout).into_owned();
    assert_eq!(one.matches("all three").count(), 1, "{one}");
    let _ = std::fs::remove_dir_all(&root);
}

/// `war done <ticket>` is all or nothing: an item still waiting on other
/// work refuses the whole, by name, and no item is ticked.
#[test]
fn done_on_a_ticket_ticks_every_item_or_none() {
    let root = scratch("done-all");
    let blocker = json(&root, &["create", "Blocker", "--item", "first"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let id = json(&root, &["create", "Job", "--item", "free"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        json(&root, &["add", &id, "waits", "--after", &blocker])["exit_code"],
        0
    );
    assert_eq!(json(&root, &["claim", &id])["exit_code"], 0);
    let refused = json(&root, &["done", &id]);
    assert_eq!(
        refused["diagnostics"][0]["rule"], "ticket.blocked",
        "{refused}"
    );
    let shown = json(&root, &["show", &id]);
    assert_eq!(
        shown["result"]["ticket"]["done"], 0,
        "nothing ticked: {shown}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `war view prime --session` is opt-in: without `[tickets] session_start`
/// it prints nothing, so the plugin's SessionStart hook adds nothing. Enabled,
/// it shows the open Warrants and how to take one; holding one, its brief.
#[test]
fn the_session_start_view_is_opt_in() {
    let root = scratch("session");
    let id = json(
        &root,
        &[
            "create",
            "Job",
            "--body",
            "What the job is.",
            "--item",
            "one",
        ],
    )["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let text = |args: &[&str]| String::from_utf8_lossy(&war(&root, args).stdout).into_owned();
    // Off (the default): nothing at all.
    assert_eq!(
        text(&["view", "prime", "--session", "--as", "ann"]).trim(),
        ""
    );
    let config = root.join("openwarrant.toml");
    let mut toml = std::fs::read_to_string(&config).unwrap();
    toml.push_str("\n[tickets]\nsession_start = true\n");
    std::fs::write(&config, toml).unwrap();
    // On, nothing held: the overview and how to take the next.
    let open = text(&["view", "prime", "--session", "--as", "ann"]);
    assert!(
        open.contains(&format!("- {id} — Job")) && open.contains("war start"),
        "{open}"
    );
    assert!(!open.contains("What the job is."), "{open}");
    // On, holding it: its brief, and no call needed to find it.
    assert_eq!(json(&root, &["start", "--as", "ann"])["exit_code"], 0);
    let held = text(&["view", "prime", "--session", "--as", "ann"]);
    assert!(
        held.contains(&format!("you hold {id}")) && held.contains("What the job is."),
        "{held}"
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
                err.contains("work.claimed-by-other") && err.contains(winners[0].as_str()),
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

/// t-9d3e: once `war done` on a blocker has exited 0, `war ready` lists the
/// item it held and `war claim` takes it — never one without the other. Four
/// workers on one corpus, ten rounds each, every command's exit checked by
/// name so a step that failed cannot pass for the two disagreeing.
#[test]
fn after_done_ready_and_claim_agree_under_concurrency() {
    let root = scratch("agree");
    let workers: Vec<_> = (0..4)
        .map(|w| {
            let root = root.clone();
            std::thread::spawn(move || {
                let actor = format!("worker-{w}");
                let step = |args: &[&str]| -> serde_json::Value {
                    let mut all = vec!["--json"];
                    all.extend_from_slice(args);
                    all.extend_from_slice(&["--as", &actor]);
                    let out = war(&root, &all);
                    assert!(
                        out.status.success(),
                        "{actor}: {args:?} exited {:?}: {}{}",
                        out.status.code(),
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    );
                    serde_json::from_slice(&out.stdout).unwrap()
                };
                for n in 0..10 {
                    let title = format!("{actor} round {n}");
                    let created = step(&["create", &title, "--item", "Foundation"]);
                    let t = created["result"]["id"].as_str().unwrap().to_owned();
                    let f = created["result"]["items"][0]["id"]
                        .as_str()
                        .unwrap()
                        .to_owned();
                    let added = step(&["add", &t, "Walls", "--after", &f]);
                    let walls = added["result"]["item"].as_str().unwrap().to_owned();
                    step(&["claim", &format!("{t}/{f}")]);
                    step(&["done", &format!("{t}/{f}")]);
                    let ready = step(&["ready"]);
                    let listed = ready["result"]["ready"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|r| r["ticket"] == t.as_str() && r["item"] == walls.as_str());
                    assert!(
                        listed,
                        "{actor}: {t}/{walls} not ready once its blocker was done"
                    );
                    step(&["claim", &format!("{t}/{walls}")]);
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
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
    // The overview is one line per Warrant; one Warrant in full lists only
    // what remains.
    let prime = mcp.tool("war_prime", serde_json::json!({}));
    let md = prime["result"]["markdown"].as_str().unwrap();
    assert!(
        md.contains(&format!("- {id} — ")) && !md.contains("- [ ]"),
        "{md}"
    );
    let one = mcp.tool("war_prime", serde_json::json!({"ticket": id}));
    let md = one["result"]["markdown"].as_str().unwrap();
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
