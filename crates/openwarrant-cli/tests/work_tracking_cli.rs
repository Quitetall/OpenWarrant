// SPDX-License-Identifier: Apache-2.0
//! Work tracking under FORMAT-1.0 §11 (draft 3), through the shipped binary:
//! a Format 1.0 ticket worked by `war claim`, `war done` and `war admin
//! reopen` with its journal in `v: 2`; a legacy ticket's journal kept
//! `v: 1`; one journal holding both; `war next`'s order (due, then the
//! critical path, then priority, age, alias and position) the same for
//! every caller at one `--at`; and the `work.*` refusals, each beside the
//! write it lets through.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-work-{tag}-{}-{}",
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
        None,
        &[
            "init",
            "--no-preset",
            "--namespace",
            "WK",
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

fn war(root: &Path, actor: Option<&str>, args: &[&str]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_war"));
    c.current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .args(args);
    if let Some(a) = actor {
        c.env("OPENWARRANT_ACTOR", a);
    }
    c.output().unwrap()
}

fn json(root: &Path, actor: Option<&str>, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = war(root, actor, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{args:?}: not an envelope ({e}): {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn rules(v: &serde_json::Value) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["rule"].as_str().map(str::to_owned))
        .collect()
}

/// A Format 1.0 ticket, `docs/tickets/<alias>.md` (§4.1).
fn ticket(root: &Path, alias: &str, uuid: &str, extra: &str, items: &str) {
    let dir = root.join("docs/tickets");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{alias}.md")),
        format!(
            "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: {alias}\nuuid: {uuid}\n\
             summary: Work {alias}.\n{extra}---\n# Work {alias}\n\n## Checklist\n\n{items}"
        ),
    )
    .unwrap();
}

const U1: &str = "01a0d342-ee5e-7b71-9623-f2d8cd5acd4e";
const U2: &str = "01a0d342-ee5e-7b71-9623-f2d8cd5acd4f";
const U3: &str = "01a0d342-ee5e-7b71-9623-f2d8cd5acd50";

fn journal(root: &Path, alias: &str) -> String {
    std::fs::read_to_string(root.join(format!("docs/tickets/{alias}/journal.jsonl")))
        .unwrap_or_default()
}

fn ready_targets(root: &Path, at: &str) -> Vec<String> {
    let v = json(root, None, &["next", "--at", at]);
    assert!(v["result"].is_object(), "{v}");
    // An empty ready list is absent from the envelope.
    v["result"]["ready"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|r| {
            let t = r["ticket"].as_str().unwrap();
            match r["item"].as_str() {
                Some(i) => format!("{t}/{i}"),
                None => t.to_owned(),
            }
        })
        .collect()
}

const AT: &str = "2026-10-15T12:00:00Z";

/// §11.2: a Format 1.0 ticket's journal is appended in v2 under `work.*`
/// names; claim, done and reopen each journal once.
#[test]
fn a_format_ticket_is_worked_with_a_v2_journal() {
    let root = scratch("v2");
    ticket(&root, "t-3f2a", U1, "", "- [ ] Reset endpoint (i-9c01)\n");
    let claimed = json(&root, Some("alice"), &["claim", "t-3f2a/i-9c01"]);
    assert_eq!(claimed["exit_code"], 0, "{claimed}");
    let done = json(&root, Some("alice"), &["done", "i-9c01", "--note", "ok"]);
    assert_eq!(done["exit_code"], 0, "{done}");
    let doc = std::fs::read_to_string(root.join("docs/tickets/t-3f2a.md")).unwrap();
    assert!(
        doc.contains("- [x] Reset endpoint (i-9c01) — done by alice, "),
        "{doc}"
    );

    // Reopened: the box opens, the suffix goes, the journal says why.
    let reopened = json(
        &root,
        Some("alice"),
        &["admin", "reopen", "i-9c01", "--reason", "regressed"],
    );
    assert_eq!(reopened["exit_code"], 0, "{reopened}");
    let doc = std::fs::read_to_string(root.join("docs/tickets/t-3f2a.md")).unwrap();
    assert!(doc.contains("- [ ] Reset endpoint (i-9c01)\n"), "{doc}");
    // Refused: reopening an open item writes nothing.
    let again = json(
        &root,
        Some("alice"),
        &["admin", "reopen", "i-9c01", "--reason", "again"],
    );
    assert_eq!(again["exit_code"], 2, "{again}");
    assert!(
        rules(&again).contains(&"ticket.not-done".to_owned()),
        "{again}"
    );

    let j = journal(&root, "t-3f2a");
    let events: Vec<serde_json::Value> = j
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let types: Vec<&str> = events.iter().map(|e| e["type"].as_str().unwrap()).collect();
    assert_eq!(
        types,
        ["work.claimed", "work.item_done", "work.item_reopened"]
    );
    for e in &events {
        assert_eq!(e["v"], 2, "{e}");
        assert_eq!(e["document_uuid"], U1, "{e}");
        assert!(e["payload"].is_object(), "{e}");
        assert!(e.get("warrant_uuid").is_none(), "{e}");
    }
    assert_eq!(events[2]["payload"]["reason"], "regressed");
    assert!(
        events[0]["payload"]["lease_until"].is_string(),
        "{}",
        events[0]
    );
    // `war check` reads the v2 journal as it reads any.
    let check = json(&root, None, &["check"]);
    assert!(
        !rules(&check).iter().any(|r| r.starts_with("work.journal")),
        "{check}"
    );
}

/// §11.2: a legacy directory ticket keeps writing v1 `ticket.*` lines, and
/// one journal holding v1 and v2 lines reads, with neither rewritten.
#[test]
fn a_legacy_journal_stays_v1_and_a_mixed_journal_reads() {
    let root = scratch("v1");
    let created = json(
        &root,
        None,
        &["create", "--legacy", "Legacy", "--item", "one"],
    );
    let id = created["result"]["id"].as_str().unwrap().to_owned();
    let target = format!(
        "{id}/{}",
        json(&root, None, &["ready"])["result"]["ready"][0]["item"]
            .as_str()
            .unwrap()
    );
    assert_eq!(json(&root, None, &["claim", &target])["exit_code"], 0);
    let j = journal(&root, &id);
    for line in j.lines() {
        let e: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(e["v"], 1, "{e}");
        assert!(e["type"].as_str().unwrap().starts_with("ticket."), "{e}");
        assert!(e["payload"].is_string(), "{e}");
    }
    // A v2 line appended beside them (as a Format 1.0 tracker would after
    // a migration): the journal reads, and the claim's done still lands.
    let v1_before = j.clone();
    let uuid: String = serde_json::from_str::<serde_json::Value>(j.lines().next().unwrap())
        .unwrap()["warrant_uuid"]
        .as_str()
        .unwrap()
        .to_owned();
    let key = {
        use sha2::Digest;
        let payload = format!("{{\"note\":\"x\",\"target\":\"{id}\"}}");
        let d = sha2::Sha256::digest(format!("{uuid}\nwork.note_added\n{payload}").as_bytes());
        d.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let v2 = format!(
        "{{\"actor_ref\":\"claude\",\"class\":\"draft_history\",\"document_uuid\":\"{uuid}\",\
         \"id\":\"01a0d342-ee5e-7b71-9623-f2d8cd5acd99\",\"idempotency_key\":\"{key}\",\
         \"occurred_at\":\"2026-10-15T00:00:00Z\",\"payload\":{{\"note\":\"x\",\"target\":\"{id}\"}},\
         \"type\":\"work.note_added\",\"v\":2}}\n"
    );
    std::fs::write(
        root.join(format!("docs/tickets/{id}/journal.jsonl")),
        format!("{j}{v2}"),
    )
    .unwrap();
    let done = json(&root, None, &["done", &target]);
    assert_eq!(done["exit_code"], 0, "{done}");
    let after = journal(&root, &id);
    assert!(after.starts_with(&v1_before), "a v1 line was rewritten");
    assert!(after.contains(&v2));
    let last: serde_json::Value = serde_json::from_str(after.lines().last().unwrap()).unwrap();
    assert_eq!(
        (last["v"].as_u64(), last["type"].as_str()),
        (Some(1), Some("ticket.item_done"))
    );
    let check = json(&root, None, &["check"]);
    assert!(
        !rules(&check).iter().any(|r| r.starts_with("work.journal")),
        "{check}"
    );
    // Refused: a line that does not parse is `work.journal-malformed`.
    std::fs::write(
        root.join(format!("docs/tickets/{id}/journal.jsonl")),
        format!("{after}{{\"v\":2,\"payload\":\"text\"}}\n"),
    )
    .unwrap();
    let bad = json(&root, None, &["check"]);
    assert!(
        rules(&bad).contains(&"work.journal-malformed".to_owned()),
        "{bad}"
    );
}

/// §11.5: `due` decides before priority, an item inherits the earliest
/// due of what waits on it, and without dates the critical path (the
/// longest remaining path) goes first, then priority, then position.
#[test]
fn next_orders_by_due_then_critical_path_then_priority() {
    let root = scratch("order");
    // t-a: priority 0, no due. t-b: priority 3, due. Due wins.
    ticket(
        &root,
        "t-aaaa",
        U1,
        "priority: 0\n",
        "- [ ] urgent by priority (i-a001)\n",
    );
    ticket(
        &root,
        "t-bbbb",
        U2,
        "priority: 3\ndue: 2026-11-01\n",
        "- [ ] due soon (i-b001)\n",
    );
    assert_eq!(ready_targets(&root, AT), ["t-bbbb/i-b001", "t-aaaa/i-a001"]);
    // Refusal half: without the due date, priority decides.
    ticket(
        &root,
        "t-bbbb",
        U2,
        "priority: 3\n",
        "- [ ] due soon (i-b001)\n",
    );
    assert_eq!(ready_targets(&root, AT), ["t-aaaa/i-a001", "t-bbbb/i-b001"]);

    // Critical path: in t-c, i-c002 waits on i-c001, so i-c001's remaining
    // path is two estimates long and goes before t-a's lower priority item.
    std::fs::remove_file(root.join("docs/tickets/t-bbbb.md")).unwrap();
    ticket(
        &root,
        "t-cccc",
        U3,
        "priority: 2\n",
        "- [ ] last (i-c009)\n- [ ] first (i-c001)\n- [ ] then (i-c002, after i-c001)\n",
    );
    assert_eq!(
        ready_targets(&root, AT),
        ["t-cccc/i-c001", "t-aaaa/i-a001", "t-cccc/i-c009"]
    );
    // An item inherits the due of what waits on it: t-a waits on i-c009.
    ticket(
        &root,
        "t-aaaa",
        U1,
        "priority: 0\ndue: 2026-10-20\n",
        "- [ ] urgent by priority (i-a001, after t-cccc/i-c009)\n",
    );
    assert_eq!(
        ready_targets(&root, AT),
        ["t-cccc/i-c009", "t-cccc/i-c001"],
        "i-c009 inherits t-aaaa's due; i-a001 waits"
    );
}

/// §11.5: a pure function of the documents, journals, claims and a time:
/// ten callers at one `--at` get byte-identical answers.
#[test]
fn ten_callers_at_one_time_get_one_answer() {
    let root = scratch("same");
    ticket(
        &root,
        "t-aaaa",
        U1,
        "",
        "- [ ] one (i-a001)\n- [ ] two (i-a002)\n",
    );
    ticket(
        &root,
        "t-bbbb",
        U2,
        "due: 2026-12-01\n",
        "- [ ] three (i-b001)\n",
    );
    let root = std::sync::Arc::new(root);
    let outs: Vec<Vec<u8>> = (0..10)
        .map(|_| {
            let root = root.clone();
            std::thread::spawn(move || war(&root, None, &["--json", "next", "--at", AT]).stdout)
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    assert!(!outs[0].is_empty());
    assert!(outs.iter().all(|o| *o == outs[0]), "callers disagreed");
    // Refused: a time that is not RFC 3339.
    let bad = war(&root, None, &["next", "--at", "tomorrow"]);
    assert!(!bad.status.success());
}

/// §11.3: of ten actors racing for one item of a Format 1.0 ticket exactly
/// one wins, and every other is refused `work.claimed-by-other`.
#[test]
fn ten_actors_racing_one_format_item_one_wins() {
    let root = scratch("race");
    ticket(&root, "t-3f2a", U1, "", "- [ ] contested (i-9c01)\n");
    let root = std::sync::Arc::new(root);
    let outs: Vec<Output> = (0..10)
        .map(|n| {
            let root = root.clone();
            std::thread::spawn(move || {
                war(
                    &root,
                    Some(&format!("racer-{n}")),
                    &["claim", "t-3f2a/i-9c01"],
                )
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    let won = outs.iter().filter(|o| o.status.success()).count();
    assert_eq!(won, 1, "exactly one claim wins");
    for o in outs.iter().filter(|o| !o.status.success()) {
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains("refused (work.claimed-by-other)"), "{err}");
    }
    assert_eq!(journal(&root, "t-3f2a").lines().count(), 1);
}

/// §11.3: a write whose base revision is stale is refused
/// `work.stale-revision` and writes nothing; the current one lands.
#[test]
fn a_stale_revision_writes_nothing() {
    let root = scratch("cas");
    ticket(&root, "t-3f2a", U1, "", "- [ ] one (i-9c01)\n");
    assert_eq!(json(&root, None, &["claim", "i-9c01"])["exit_code"], 0);
    let before = std::fs::read(root.join("docs/tickets/t-3f2a.md")).unwrap();
    let journal_before = journal(&root, "t-3f2a");
    let stale = json(
        &root,
        None,
        &[
            "done",
            "i-9c01",
            "--if-rev",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ],
    );
    assert_eq!(stale["exit_code"], 2, "{stale}");
    assert!(
        rules(&stale).contains(&"work.stale-revision".to_owned()),
        "{stale}"
    );
    assert_eq!(
        std::fs::read(root.join("docs/tickets/t-3f2a.md")).unwrap(),
        before
    );
    assert_eq!(journal(&root, "t-3f2a"), journal_before);
    let shown = json(&root, None, &["show", "t-3f2a"]);
    let rev = shown["result"]["items"][0]["revision"]
        .as_str()
        .unwrap_or_else(|| panic!("{shown}"))
        .to_owned();
    let ok = json(&root, None, &["done", "i-9c01", "--if-rev", &rev]);
    assert_eq!(ok["exit_code"], 0, "{ok}");
}

/// §11.4: a level the journal does not back reads down, with a
/// `work.tick-unbacked` warning; a tick with no marker raises none.
#[test]
fn an_unbacked_marker_is_a_warning() {
    let root = scratch("unbacked");
    ticket(
        &root,
        "t-3f2a",
        U1,
        "",
        "- [x] forged (i-9c01) — done by claude, 2026-10-12 [observed]: trust me\n\
         - [x] plain (i-9c02) — done by claude, 2026-10-12: fine\n",
    );
    let shown = json(&root, None, &["show", "t-3f2a"]);
    let warned: Vec<&serde_json::Value> = shown["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"] == "work.tick-unbacked")
        .collect();
    assert_eq!(warned.len(), 1, "{shown}");
    assert_eq!(warned[0]["severity"], "warn", "{shown}");
    assert!(warned[0]["message"].as_str().unwrap().contains("i-9c01"));
    assert_eq!(shown["exit_code"], 0, "a warning does not block");
}

/// FORMAT-1.0 §11.1 (draft 3 revision 1): a Format 1.0 ticket with no
/// items is `open` and a `next` candidate; claim and done act on the
/// document, whose done is a `work.item_done` naming it, and no line is
/// added; reopened (a later `work.item_reopened`), it is open again.
#[test]
fn a_title_only_document_is_worked_as_itself() {
    let root = scratch("whole");
    ticket(&root, "t-7a7a", U1, "", "");
    assert_eq!(ready_targets(&root, AT), ["t-7a7a"]);
    let before = std::fs::read(root.join("docs/tickets/t-7a7a.md")).unwrap();
    assert_eq!(json(&root, None, &["claim", "t-7a7a"])["exit_code"], 0);
    let done = json(&root, None, &["done", "t-7a7a"]);
    assert_eq!(done["exit_code"], 0, "{done}");
    assert_eq!(
        std::fs::read(root.join("docs/tickets/t-7a7a.md")).unwrap(),
        before,
        "no line is added"
    );
    let j = journal(&root, "t-7a7a");
    let last: serde_json::Value = serde_json::from_str(j.lines().last().unwrap()).unwrap();
    assert_eq!(last["type"], "work.item_done");
    assert_eq!(last["payload"]["target"], "t-7a7a");
    assert!(ready_targets(&root, AT).is_empty(), "done: no candidate");
    // Refused: claiming a done document.
    let again = json(&root, None, &["claim", "t-7a7a"]);
    assert!(
        rules(&again).contains(&"ticket.already-done".to_owned()),
        "{again}"
    );
    let reopened = json(
        &root,
        None,
        &["admin", "reopen", "t-7a7a", "--reason", "not yet"],
    );
    assert_eq!(reopened["exit_code"], 0, "{reopened}");
    assert_eq!(ready_targets(&root, AT), ["t-7a7a"]);
    // Done again: a new fact, recorded, not a replay of the first.
    assert_eq!(json(&root, None, &["claim", "t-7a7a"])["exit_code"], 0);
    assert_eq!(json(&root, None, &["done", "t-7a7a"])["exit_code"], 0);
    assert!(ready_targets(&root, AT).is_empty());
    assert_eq!(
        journal(&root, "t-7a7a").matches("work.item_done").count(),
        2
    );
}

/// FORMAT-1.0 §11.1: an item is `blocked` while its document's
/// `depends_on` is not done, and is no candidate; once it is, the item is.
#[test]
fn a_documents_depends_on_blocks_its_items() {
    let root = scratch("docdeps");
    ticket(&root, "t-1111", U1, "", "- [ ] first (i-1101)\n");
    ticket(
        &root,
        "t-2222",
        U2,
        "depends_on: t-1111\n",
        "- [ ] after the other (i-2201)\n",
    );
    assert_eq!(ready_targets(&root, AT), ["t-1111/i-1101"]);
    let refused = json(&root, None, &["claim", "t-2222/i-2201"]);
    assert!(
        rules(&refused).contains(&"ticket.blocked".to_owned()),
        "{refused}"
    );
    assert_eq!(json(&root, None, &["claim", "i-1101"])["exit_code"], 0);
    assert_eq!(json(&root, None, &["done", "i-1101"])["exit_code"], 0);
    assert_eq!(ready_targets(&root, AT), ["t-2222/i-2201"]);
}
