// SPDX-License-Identifier: Apache-2.0
//! `war context` through the shipped binary (Format 1.0 section 10): the
//! same bytes twice and in a moved clone, the budget's refusal and its [S]
//! lines, `--strict`, path targets, an unknown target, `--json`, the index
//! `war admin compile` writes and `war check --generated` compares, and the
//! summary tier. Each claim beside the input it refuses.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn war(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn ok(out: &Output) -> String {
    assert!(
        out.status.success(),
        "exit {:?}\n{}{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "ow-context-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn git(root: &Path, args: &[&str]) {
    let s = Command::new("git")
        .args([
            "-c",
            "user.email=t@invalid",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgSign=false",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
}

const AREA: &str = "docs/records/password-reset";

/// A program with one directory Warrant, one ticket with a summary and two
/// items, and the password-reset record area; compiled and committed.
fn scratch(tag: &str) -> PathBuf {
    let root = tmp(tag);
    git(&root, &["init", "-q", "."]);
    ok(&war(
        &root,
        &["init", "--program", "Context Test", "--namespace", "CT"],
    ));
    std::fs::create_dir_all(root.join("profiles")).unwrap();
    for p in [
        "delivery",
        "decision",
        "ticket",
        "prd",
        "architecture",
        "test-plan",
        "agent-packet",
    ] {
        std::fs::copy(
            repo().join("profiles").join(format!("{p}.toml")),
            root.join("profiles").join(format!("{p}.toml")),
        )
        .unwrap();
    }
    std::fs::create_dir_all(root.join(AREA)).unwrap();
    for f in ["10-records.md", "20-product.md", "30-architecture.md"] {
        std::fs::copy(repo().join(AREA).join(f), root.join(AREA).join(f)).unwrap();
    }
    ok(&war(&root, &["new", "Parse the manifest"]));
    ok(&war(
        &root,
        &[
            "create",
            "Ship the parser",
            "--summary",
            "The parser reads manifests and refuses duplicates.",
            "--item",
            "Read the file",
            "--item",
            "Refuse a duplicate",
        ],
    ));
    ok(&war(&root, &["compile"]));
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixtures"]);
    root
}

fn ticket_id(root: &Path) -> String {
    let mut ids: Vec<String> = std::fs::read_dir(root.join("docs/tickets"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with("t-"))
        .collect();
    ids.sort();
    ids.remove(0)
}

fn index_lines(md: &str) -> Vec<&str> {
    md.split("\n## Index\n")
        .nth(1)
        .unwrap()
        .split("\n\n## Records\n")
        .next()
        .unwrap()
        .lines()
        .collect()
}

#[test]
fn the_same_tree_gives_the_same_bytes_here_and_in_a_moved_clone_and_an_edit_moves_the_digest() {
    let root = scratch("det");
    let a = ok(&war(&root, &["context", "CT-WAR-0001"]));
    let b = ok(&war(&root, &["view", "context", "CT-WAR-0001"]));
    assert_eq!(
        a, b,
        "the group spelling and the top-level one print the same bytes"
    );
    assert!(a.starts_with("# Context: CT-WAR-0001\n<!-- oh.war/context/v1 · budget 16000 · depth 3 · walk core · selection sha256:"));
    assert!(
        !a.contains(root.to_str().unwrap()),
        "no absolute path in the packet"
    );
    let moved = tmp("moved").join("deeper");
    let s = Command::new("git")
        .args(["clone", "-q"])
        .arg(&root)
        .arg(&moved)
        .output()
        .unwrap();
    assert!(s.status.success());
    let c = ok(&war(&moved, &["context", "CT-WAR-0001"]));
    assert_eq!(a, c, "a clone at another path gives the same bytes");
    // An absolute path target inside the repository is read as relative.
    let ticket = ticket_id(&root);
    let rel = format!("docs/tickets/{ticket}/atoms/15-checklist.md");
    let by_rel = ok(&war(&root, &["context", &rel]));
    let by_abs = ok(&war(&root, &["context", root.join(&rel).to_str().unwrap()]));
    assert_eq!(by_rel, by_abs);
    // Refusal: a changed record is a different input.
    let intent = moved.join("docs/warrants/CT-WAR-0001/atoms/10-intent.md");
    let mut text = std::fs::read_to_string(&intent).unwrap();
    text.push_str("\nOne more line.\n");
    std::fs::write(&intent, text).unwrap();
    let d = ok(&war(&moved, &["context", "CT-WAR-0001"]));
    assert_ne!(
        a.lines().nth(1),
        d.lines().nth(1),
        "the selection digest moved"
    );
}

#[test]
fn a_warrant_comes_first_then_its_sections_and_every_full_line_has_its_whole_block() {
    let root = scratch("order");
    let md = ok(&war(&root, &["context", "CT-WAR-0001"]));
    let lines = index_lines(&md);
    let types: Vec<&str> = lines
        .iter()
        .map(|l| l.split(" · ").nth(1).unwrap())
        .collect();
    assert_eq!(types[0], "warrant");
    assert_eq!(&types[1..4], ["section", "section", "section"], "{types:?}");
    let rank = |t: &str| match t {
        "warrant" => 0,
        "section" => 1,
        "obligation" => 2,
        "milestone" => 3,
        "stage" => 4,
        _ => 5,
    };
    let ranks: Vec<usize> = types.iter().map(|t| rank(t)).collect();
    let mut sorted = ranks.clone();
    sorted.sort_unstable();
    assert_eq!(ranks, sorted, "distance 0 in type rank: {types:?}");
    for l in &lines {
        let id = l[4..].split(" · ").next().unwrap();
        let has_block = md.contains(&format!("\n> {id} · "));
        assert_eq!(l.starts_with("[F] "), has_block, "{l}");
    }
    // The sections are the atoms' exact bytes.
    let intent =
        std::fs::read_to_string(root.join("docs/warrants/CT-WAR-0001/atoms/10-intent.md")).unwrap();
    assert!(md.contains(&intent));
}

#[test]
fn roots_over_budget_are_refused_and_a_tighter_budget_marks_the_rest_summary() {
    let root = scratch("budget");
    let full = ok(&war(&root, &["context", "CT-WAR-0001"]));
    // Refused: the roots do not fit; nothing on stdout, the rule on stderr.
    let tiny = war(&root, &["context", "CT-WAR-0001", "--budget", "300"]);
    assert_eq!(tiny.status.code(), Some(2));
    assert!(tiny.stdout.is_empty());
    assert!(String::from_utf8_lossy(&tiny.stderr).contains("context.over-budget"));
    let json = war(
        &root,
        &["--json", "context", "CT-WAR-0001", "--tokens", "75"],
    );
    assert_eq!(json.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(v["diagnostics"][0]["rule"], "context.over-budget");
    assert!(v["result"].is_null());
    // Between the roots and the whole: whole records, then [S].
    let budget = full.len() - 40;
    let part = ok(&war(
        &root,
        &["context", "CT-WAR-0001", "--budget", &budget.to_string()],
    ));
    assert!(part.len() <= budget);
    let lines = index_lines(&part);
    let first_s = lines
        .iter()
        .position(|l| l.starts_with("[S] "))
        .expect("a summary line");
    assert!(
        lines[first_s..].iter().all(|l| l.starts_with("[S] ")),
        "{lines:?}"
    );
    for l in &lines[..first_s] {
        let id = l[4..].split(" · ").next().unwrap();
        // The block in the partial packet is the block in the full one.
        let head = format!("\n> {id} · ");
        let take = |md: &str| -> String {
            let at = md.find(&head).unwrap();
            let rest = &md[at + 1..];
            rest[..rest.find("\n\n").map_or(rest.len(), |e| e + 2)].to_owned()
        };
        assert_eq!(take(&part), take(&full), "{id} was cut");
    }
    // --strict: the same budget is refused; the full one is not.
    let strict = war(
        &root,
        &[
            "context",
            "CT-WAR-0001",
            "--budget",
            &budget.to_string(),
            "--strict",
        ],
    );
    assert_eq!(strict.status.code(), Some(2));
    assert!(strict.stdout.is_empty());
    assert!(String::from_utf8_lossy(&strict.stderr).contains("context.over-budget"));
    assert_eq!(
        ok(&war(&root, &["context", "CT-WAR-0001", "--strict"])),
        full
    );
    // --tokens n is --budget 4n.
    let t = ok(&war(&root, &["context", "CT-WAR-0001", "--tokens", "4000"]));
    assert!(t.lines().nth(1).unwrap().contains("budget 16000"));
}

#[test]
fn paths_items_and_records_resolve_and_an_unknown_target_is_refused() {
    let root = scratch("targets");
    let ticket = ticket_id(&root);
    let md = ok(&war(
        &root,
        &[
            "context",
            &format!("docs/tickets/{ticket}/atoms/15-checklist.md"),
        ],
    ));
    assert!(index_lines(&md)[0].starts_with(&format!(
        "[F] {ticket} · ticket · open · Ship the parser · "
    )));
    let recs = ok(&war(
        &root,
        &["context", &format!("{AREA}/10-records.md"), "--depth", "0"],
    ));
    assert!(
        index_lines(&recs)
            .iter()
            .all(|l| l.contains(&format!("{AREA}/10-records.md:")))
    );
    assert!(recs.contains("[F] REQ-pr1 · requirement"));
    // An item: the item, then its ticket's intent at distance 1.
    let json = war(&root, &["--json", "context", &format!("{ticket}/i-0")]);
    assert_eq!(json.status.code(), Some(2), "no such item");
    let model: serde_json::Value =
        serde_json::from_str(&ok(&war(&root, &["--json", "plan", "model"]))).unwrap();
    let item = model["result"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["type"] == "item" && r["title"] == "Read the file")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let v: serde_json::Value =
        serde_json::from_str(&ok(&war(&root, &["--json", "context", &item]))).unwrap();
    let r = &v["result"];
    assert_eq!(r["schema"], "oh.war/context/v1");
    assert_eq!(r["records"][0]["id"], item.as_str());
    assert_eq!(r["records"][0]["distance"], 0);
    let intent = r["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == format!("{ticket}/S-intent").as_str())
        .expect("the ticket's intent");
    assert_eq!(intent["distance"], 1);
    assert_eq!(r["records"][0]["tier"], "full");
    assert!(
        r["records"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Read the file")
    );
    // Refused: nothing has the id, and no Warrant's paths match the path.
    for t in ["NOPE-404", "src/nowhere.rs"] {
        let out = war(&root, &["context", t]);
        assert_eq!(out.status.code(), Some(2), "{t}");
        assert!(out.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("context.target-unknown"),
            "{t}"
        );
    }
}

#[test]
fn compile_writes_the_index_check_passes_it_and_refuses_a_hand_edit() {
    let root = scratch("index");
    let idx = root.join("docs/generated/llms.txt");
    let text = std::fs::read_to_string(&idx).unwrap();
    assert!(text.starts_with("# Context Test\n\n> OpenWarrant index (openwarrant/1.0). Generated; do not edit. Read any line's record with `war context <id>`.\n"));
    assert!(text.contains("\n> The parser reads manifests and refuses duplicates.\n"));
    assert!(text.contains("\n- CT-WAR-0001 · warrant · "));
    assert!(text.contains("\n- REQ-pr1 · requirement · "));
    let check = |root: &Path| -> serde_json::Value {
        let out = war(root, &["--json", "check", "--generated"]);
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let rules = |v: &serde_json::Value| -> Vec<String> {
        v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["rule"].as_str().unwrap().starts_with("context-index"))
            .map(|d| {
                format!(
                    "{}:{}",
                    d["severity"].as_str().unwrap(),
                    d["rule"].as_str().unwrap()
                )
            })
            .collect()
    };
    assert_eq!(rules(&check(&root)), ["pass:context-index.drift"]);
    std::fs::write(&idx, format!("{text}- HAND-1 · edited\n")).unwrap();
    assert_eq!(rules(&check(&root)), ["error:context-index.drift"]);
}

#[test]
fn a_summary_is_written_bounded_and_required_of_a_format_1_work_document() {
    let root = scratch("summary");
    let ticket = ticket_id(&root);
    let manifest =
        std::fs::read_to_string(root.join(format!("docs/tickets/{ticket}/manifest.toml"))).unwrap();
    assert!(manifest.contains("summary = \"The parser reads manifests and refuses duplicates.\""));
    let long = "x".repeat(201);
    let out = war(&root, &["--json", "create", "Too long", "--summary", &long]);
    assert_ne!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("\"ticket.summary\""));
    // A ticket written without one reads as it did: no `summary` key.
    ok(&war(&root, &["create", "No summary"]));
    let missing = |root: &Path| -> Vec<String> {
        let out = war(root, &["--json", "check"]);
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["rule"] == "format.field-missing")
            .map(|d| d["file"].as_str().unwrap_or_default().to_owned())
            .collect()
    };
    assert!(missing(&root).is_empty());
    let doc = root.join("docs/warrants/CT-WAR-0900.md");
    std::fs::write(
        &doc,
        "---\nformat: openwarrant/1.0\nprofile: delivery\nalias: CT-WAR-0900\nuuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e\n---\n# A Format 1.0 Warrant\n",
    )
    .unwrap();
    assert_eq!(missing(&root), ["docs/warrants/CT-WAR-0900.md:1"]);
    std::fs::write(
        &doc,
        "---\nformat: openwarrant/1.0\nprofile: delivery\nalias: CT-WAR-0900\nuuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e\nsummary: What it is for.\n---\n# A Format 1.0 Warrant\n",
    )
    .unwrap();
    assert!(missing(&root).is_empty());
}
