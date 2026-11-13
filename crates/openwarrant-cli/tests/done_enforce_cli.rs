// SPDX-License-Identifier: Apache-2.0
//! t-3c16c W3, step 6: `war done` holds a change and a widening to the item's
//! level. Conformance plant 189 takes the paths that need a human's signature.
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

fn err(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap()
}

/// One repository on `main` with `docs/plan.md`, as it is before any item.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("r");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        assert_eq!(
            json(&root, "owner", &["init", "--namespace", "DN"])["exit_code"],
            0
        );
        std::fs::write(root.join("docs/plan.md"), "# Plan\n\nOLD\n").unwrap();
        Self { _dir: dir, root }
    }

    fn create(&self, args: &[&str]) -> String {
        let mut all = vec!["create"];
        all.extend(args);
        let v = json(&self.root, "owner", &all);
        assert_eq!(v["exit_code"], 0, "{v}");
        v["result"]["id"].as_str().unwrap().to_owned()
    }

    fn commit(&self, message: &str) {
        git(&self.root, &["add", "-A"]);
        git(&self.root, &["commit", "-qm", message]);
    }

    fn start_worktree(&self, actor: &str, id: &str) -> PathBuf {
        let v = json(&self.root, actor, &["start", "--worktree", "--print", id]);
        assert_eq!(v["exit_code"], 0, "{v}");
        PathBuf::from(v["result"]["worktree"].as_str().unwrap())
    }

    fn amend_plan(&self) {
        std::fs::write(self.root.join("docs/plan.md"), "# Plan\n\nNEW\n").unwrap();
        self.commit("amend");
    }

    fn doc(&self, id: &str) -> PathBuf {
        crate::work_paths::at(self.root.join(format!("docs/tickets/{id}.md")))
    }

    fn edit(&self, id: &str, from: &str, to: &str) {
        let text = std::fs::read_to_string(self.doc(id)).unwrap();
        assert!(text.contains(from), "{text}");
        std::fs::write(self.doc(id), text.replace(from, to)).unwrap();
    }

    fn order(&self, title: &str) -> (String, String) {
        let v = json(
            &self.root,
            "owner",
            &[
                "create",
                title,
                "--tool",
                "Read",
                "-i",
                "the release",
                "-i",
                "the notes",
            ],
        );
        let id = v["result"]["id"].as_str().unwrap().to_owned();
        let item = |text: &str| {
            v["result"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["text"] == text)
                .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let (signed, free) = (item("the release"), item("the notes"));
        let target = format!("{id}/{signed}");
        assert_eq!(
            json(
                &self.root,
                "owner",
                &["plan", "add", &target, "--min", "signed"]
            )["exit_code"],
            0
        );
        (id, free)
    }
}

const FOLLOW: &str = "Follow docs/plan.md.";

fn rules(v: &Value) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_ticket_warns_of_an_unseen_change_and_closes() {
    let r = Repo::new();
    let id = r.create(&["A ticket", "--body", FOLLOW]);
    r.commit("start");
    let at = r.start_worktree("alice", &id);
    r.amend_plan();
    let v = json(&at, "alice", &["done", &id]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert_eq!(rules(&v), ["done.unseen"], "{v}");
    assert_eq!(v["result"]["ticket_state"], "done", "{v}");
}

#[test]
fn an_unaffected_item_hears_nothing_and_closes_normally() {
    let r = Repo::new();
    let ticket = r.create(&["A ticket", "--body", "Nothing cited."]);
    let warrant = r.create(&["A Warrant", "--test", "true"]);
    r.commit("start");
    let at_t = r.start_worktree("alice", &ticket);
    let at_w = r.start_worktree("bob", &warrant);
    r.amend_plan();
    for (at, actor, id) in [(&at_t, "alice", &ticket), (&at_w, "bob", &warrant)] {
        let out = run(at, actor, &["done", id]);
        assert_eq!(code(&out), 0, "{}", err(&out));
        assert_eq!(err(&out), "");
    }
}

#[test]
fn a_warrant_is_refused_until_seen_and_held_then_its_trigger_runs() {
    let r = Repo::new();
    let id = r.create(&[
        "A Warrant",
        "--body",
        FOLLOW,
        "--test",
        "grep -q NEW docs/plan.md",
    ]);
    r.commit("start");
    let at = r.start_worktree("alice", &id);
    r.amend_plan();
    let doc = crate::work_paths::at(at.join(format!("docs/tickets/{id}.md")));
    let before = std::fs::read(&doc).unwrap();

    let refused = run(&at, "alice", &["done", &id]);
    assert_eq!(code(&refused), 2);
    assert!(
        err(&refused).contains("refused (done.unseen)"),
        "{}",
        err(&refused)
    );
    assert!(err(&refused).contains(&format!("war start {id}")));
    assert_eq!(
        std::fs::read(&doc).unwrap(),
        before,
        "a refusal writes nothing"
    );

    let seen = json(&at, "alice", &["start", &id]);
    assert_eq!(seen["result"]["changed"][0]["source"], "docs/plan.md");

    // Seen, and the workspace does not hold it: a trigger here proves nothing.
    let behind = run(&at, "alice", &["done", &id]);
    assert_eq!(code(&behind), 2);
    assert!(
        err(&behind).contains("refused (done.behind)"),
        "{}",
        err(&behind)
    );

    git(&at, &["merge", "-q", "--no-edit", "main"]);
    let v = json(&at, "alice", &["done", &id]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert_eq!(v["result"]["level"], "observed", "{v}");
    assert_eq!(v["result"]["ticket_state"], "done", "{v}");
}

#[test]
fn a_trigger_that_fails_on_the_new_state_does_not_close() {
    let r = Repo::new();
    let id = r.create(&[
        "A Warrant",
        "--body",
        FOLLOW,
        "--test",
        "grep -q OLD docs/plan.md",
    ]);
    r.commit("start");
    let at = r.start_worktree("alice", &id);
    r.amend_plan();
    json(&at, "alice", &["start", &id]);
    git(&at, &["merge", "-q", "--no-edit", "main"]);
    let v = json(&at, "alice", &["done", &id]);
    assert_eq!(v["exit_code"], 2, "{v}");
    let shown = json(&at, "alice", &["view", "show", &id]);
    assert_ne!(shown["result"]["ticket_state"], "done", "{shown}");
}

#[test]
fn a_seen_change_runs_a_trigger_plain_done_would_not() {
    let r = Repo::new();
    let id = r.create(&["--legacy", "A light Warrant", "--body", FOLLOW]);
    assert_eq!(
        json(
            &r.root,
            "owner",
            &["plan", "add", &id, "--test", "grep -q OLD docs/plan.md"]
        )["exit_code"],
        0
    );
    r.commit("start");
    let at = r.start_worktree("alice", &id);
    r.amend_plan();
    json(&at, "alice", &["start", &id]);
    git(&at, &["merge", "-q", "--no-edit", "main"]);
    let v = json(&at, "alice", &["done", &id]);
    assert_eq!(rules(&v), ["ticket.check-failed"], "{v}");
    assert_eq!(v["exit_code"], 2);
}

#[test]
fn a_ticket_widening_changes_nothing() {
    let r = Repo::new();
    let id = r.create(&["A ticket", "--tool", "Read"]);
    r.commit("start");
    json(&r.root, "alice", &["start", &id]);
    r.edit(&id, "  - \"Read\"\n", "  - \"Read\"\n  - \"Bash\"\n");
    let out = run(&r.root, "alice", &["done", &id]);
    assert_eq!(code(&out), 0, "{}", err(&out));
    assert_eq!(err(&out), "");
}

#[test]
fn a_warrant_widening_is_flagged_by_name_journalled_and_not_started_away() {
    let r = Repo::new();
    let id = r.create(&["A Warrant", "--tool", "Read", "--test", "true"]);
    r.commit("start");
    json(&r.root, "alice", &["start", &id]);
    r.edit(&id, "  - \"Read\"\n", "  - \"Read\"\n  - \"Bash\"\n");
    json(&r.root, "alice", &["start", &id]);
    let v = json(&r.root, "alice", &["done", &id]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert_eq!(rules(&v), ["done.widened"], "{v}");
    assert!(
        v["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("x.tools + Bash"),
        "{v}"
    );
    assert_eq!(v["result"]["widened"][0], "x.tools + Bash", "{v}");
    let out = Command::new("grep")
        .args(["-rlF", "work.scope_widened", "docs/work"])
        .current_dir(&r.root)
        .output()
        .unwrap();
    assert!(!out.stdout.is_empty(), "no journal holds the widening");
}

#[test]
fn an_unchanged_order_is_not_refused_and_a_widened_one_is_by_name() {
    let r = Repo::new();
    let (same, same_free) = r.order("Left alone");
    let (wide, wide_free) = r.order("Widened");
    r.commit("start");
    json(&r.root, "alice", &["start", &same]);
    let v = json(&r.root, "alice", &["done", &format!("{same}/{same_free}")]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(rules(&v).is_empty(), "{v}");

    json(&r.root, "alice", &["start", &wide]);
    r.edit(&wide, "  - \"Read\"\n", "  - \"Read\"\n  - \"Bash\"\n");
    let before = std::fs::read(r.doc(&wide)).unwrap();
    let v = json(&r.root, "alice", &["done", &format!("{wide}/{wide_free}")]);
    assert_eq!(v["exit_code"], 2, "{v}");
    assert_eq!(rules(&v), ["done.order-widened"], "{v}");
    assert!(
        v["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("x.tools + Bash"),
        "{v}"
    );
    assert_eq!(
        v["result"]["command"],
        format!("war sign approve {wide} --ssh-sign")
    );
    assert!(
        v["result"]["statement"]
            .as_str()
            .unwrap()
            .contains("tools Bash")
    );
    assert_eq!(std::fs::read(r.doc(&wide)).unwrap(), before);
}

#[test]
fn an_unsigned_order_may_be_amended_and_a_change_to_it_must_be_seen() {
    let r = Repo::new();
    let (id, free) = r.order("Unsigned");
    r.edit(&id, "Unsigned", "Unsigned, from docs/plan.md");
    r.commit("start");
    json(&r.root, "alice", &["start", &id]);
    r.edit(&id, "the notes", "the notes, amended");
    let v = json(&r.root, "alice", &["done", &format!("{id}/{free}")]);
    assert_eq!(v["exit_code"], 0, "{v}");
}

#[test]
fn an_order_is_refused_until_a_change_is_seen() {
    let r = Repo::new();
    let (id, free) = r.order("An Order");
    r.edit(&id, "\n## ", "\nFollow docs/plan.md.\n\n## ");
    r.commit("start");
    let at = r.start_worktree("alice", &id);
    r.amend_plan();
    let v = json(&at, "alice", &["done", &format!("{id}/{free}")]);
    assert_eq!(v["exit_code"], 2, "{v}");
    assert_eq!(rules(&v), ["done.unseen"], "{v}");
}

#[test]
fn what_cannot_be_read_is_unknown_on_a_warrant_and_a_warning_on_a_ticket() {
    let r = Repo::new();
    let warrant = r.create(&["A Warrant", "--test", "true"]);
    let ticket = r.create(&["A ticket"]);
    r.commit("start");
    json(&r.root, "alice", &["start", &warrant]);
    json(&r.root, "alice", &["start", &ticket]);
    let notices = r.root.join(".git/openwarrant/notices");
    std::fs::create_dir_all(&notices).unwrap();
    std::fs::write(notices.join("garbage.json"), "{ not json").unwrap();

    let out = run(&r.root, "alice", &["done", &warrant]);
    assert_eq!(code(&out), 2);
    assert!(
        err(&out).contains("UNKNOWN (done.seen-unknown)"),
        "{}",
        err(&out)
    );

    let v = json(&r.root, "alice", &["done", &ticket]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert_eq!(rules(&v), ["done.seen-unknown"], "{v}");

    std::fs::remove_file(notices.join("garbage.json")).unwrap();
    let v = json(&r.root, "alice", &["done", &warrant]);
    assert_eq!(v["exit_code"], 0, "{v}");
}

#[test]
fn an_unread_level_is_unknown_and_does_not_close() {
    let r = Repo::new();
    let id = r.create(&["--legacy", "A light Warrant", "-i", "one"]);
    r.commit("start");
    json(&r.root, "alice", &["start", &id]);
    std::fs::create_dir_all(crate::work_paths::at(
        r.root.join(format!("docs/tickets/{id}/atoms/30-checks.md")),
    ))
    .unwrap();
    let out = run(&r.root, "alice", &["done", &id]);
    assert_eq!(code(&out), 2);
    assert!(
        err(&out).contains("UNKNOWN (done.level-unknown)"),
        "{}",
        err(&out)
    );
}

#[test]
fn work_claimed_without_start_is_said_and_not_refused() {
    let r = Repo::new();
    let id = r.create(&["A Warrant", "--test", "true"]);
    r.commit("start");
    json(&r.root, "alice", &["admin", "claim", &id]);
    let v = json(&r.root, "alice", &["done", &id]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert_eq!(rules(&v), ["done.unrecorded"], "{v}");
}

/// `[tickets] done_test` decides the agent's tick on an Order and never its
/// signature: a failing default hands nothing over, and a passing one leaves
/// the Order waiting on its signer.
#[test]
fn an_order_still_waits_for_its_signer_after_the_default_passes() {
    let r = Repo::new();
    let (id, free) = r.order("Ship it");
    let config = r.root.join("openwarrant.toml");
    let mut toml = std::fs::read_to_string(&config).unwrap();
    toml.push_str("\n[tickets]\ndone_test = \"test -f ok\"\n");
    std::fs::write(&config, toml).unwrap();
    r.commit("an order");
    assert_eq!(json(&r.root, "agent", &["start", &id])["exit_code"], 0);
    let target = format!("{id}/{free}");
    let refused = json(&r.root, "agent", &["done", &target]);
    assert!(
        rules(&refused).contains(&"ticket.check-failed".to_owned()),
        "{refused}"
    );
    let journal = || {
        std::fs::read_to_string(crate::work_paths::at(
            r.root.join(format!("docs/tickets/{id}/journal.jsonl")),
        ))
        .unwrap_or_default()
    };
    assert!(
        !journal().contains("work.awaits_signature"),
        "{}",
        journal()
    );
    std::fs::write(r.root.join("ok"), "").unwrap();
    let done = json(&r.root, "agent", &["done", &target]);
    assert_eq!(done["exit_code"], 0, "{done}");
    assert_eq!(done["result"]["level"], "observed", "{done}");
    assert_eq!(
        done["result"]["ticket_state"], "awaiting_signature",
        "{done}"
    );
    assert!(journal().contains("work.awaits_signature"), "{}", journal());
    let text = std::fs::read_to_string(r.doc(&id)).unwrap();
    assert!(text.contains("- [ ] the release"), "{text}");
}
