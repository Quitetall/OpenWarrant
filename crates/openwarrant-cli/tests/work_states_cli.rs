// SPDX-License-Identifier: Apache-2.0
//! FORMAT-1.0 §11 (draft 3, revision 1) as the conformance corpus reads it,
//! through the shipped binary: `war plan states` (the states of §11.1 by
//! family), the journal's refusals at their lines (§11.2, §13.1), a note as
//! a journal event written exactly once, `episode` in the payloads that
//! carry it, a done based on a revision, the whole-document claim, the done
//! suffix's tick level (§3.8) and a profile file without `format` (§7.5).
//! Each claim is paired with the refusal that shows the control rejects.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-states-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q", "."]);
    let out = war(
        &root,
        None,
        &[
            "init",
            "--no-preset",
            "--namespace",
            "WS",
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

fn git(root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .env_remove("SSH_AUTH_SOCK")
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
}

fn commit(root: &Path) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "fixture"]);
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

/// `<rule> <file>` of each diagnostic whose rule starts with `prefix`.
fn found(v: &serde_json::Value, prefix: &str) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["rule"].as_str().is_some_and(|r| r.starts_with(prefix)))
        .map(|d| {
            format!(
                "{} {}",
                d["rule"].as_str().unwrap(),
                d["file"].as_str().unwrap_or("")
            )
        })
        .collect()
}

/// `<rule> <record>` of each finding of `war plan model` whose rule starts
/// with `prefix` (the model reports where it read each fault).
fn model_found(root: &Path, prefix: &str) -> (serde_json::Value, Vec<String>) {
    let v = json(root, None, &["plan", "model"]);
    let got = v["result"]["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("{v}"))
        .iter()
        .filter(|d| d["rule"].as_str().is_some_and(|r| r.starts_with(prefix)))
        .map(|d| {
            format!(
                "{} {}",
                d["rule"].as_str().unwrap(),
                d["record"].as_str().unwrap()
            )
        })
        .collect();
    (v, got)
}

const U1: &str = "01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10";
const U2: &str = "01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e11";
const AT: &str = "2026-10-14T10:00:00Z";

fn ticket(root: &Path, alias: &str, uuid: &str, items: &str) {
    let dir = root.join("docs/tickets");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{alias}.md")),
        format!(
            "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: {alias}\nuuid: {uuid}\n\
             summary: Password reset by email.\n---\n# Password reset by email\n\n\
             ## Checklist\n{items}"
        ),
    )
    .unwrap();
}

const THREE: &str = "- [ ] Reset endpoint (i-9c01)\n- [ ] Email template (i-4d11)\n\
                     - [ ] Docs (i-77be, after i-4d11)\n";

fn journal_path(root: &Path, alias: &str) -> PathBuf {
    root.join(format!("docs/tickets/{alias}/journal.jsonl"))
}

fn write_journal(root: &Path, alias: &str, text: &str) {
    let p = journal_path(root, alias);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn events(root: &Path, alias: &str) -> Vec<serde_json::Value> {
    std::fs::read_to_string(journal_path(root, alias))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// A v2 line as a tracker writes it (§11.2), its key over its content.
fn line(uuid: &str, id: &str, actor: &str, at: &str, ty: &str, payload: &str) -> String {
    use sha2::Digest as _;
    let p: serde_json::Value = serde_json::from_str(payload).unwrap();
    let canonical = serde_jcs::to_string(&p).unwrap();
    let key = hex(&sha2::Sha256::digest(
        format!("{uuid}\n{ty}\n{canonical}").as_bytes(),
    ));
    let ev = serde_json::json!({"v": 2, "id": id, "document_uuid": uuid, "type": ty,
        "class": "action_receipt", "actor_ref": actor, "occurred_at": at,
        "payload": p, "idempotency_key": key});
    serde_jcs::to_string(&ev).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `(record, kind, value, provenance)` of `war plan states --at AT`.
fn states(root: &Path) -> (serde_json::Value, Vec<(String, String, String, String)>) {
    let v = json(root, None, &["plan", "states", "--at", AT]);
    let rows = v["result"]["states"]
        .as_array()
        .unwrap_or_else(|| panic!("{v}"))
        .iter()
        .map(|s| {
            let f = |k: &str| s[k].as_str().unwrap().to_owned();
            (f("record"), f("kind"), f("value"), f("provenance"))
        })
        .collect();
    (v, rows)
}

fn row(r: &str, k: &str, v: &str, p: &str) -> (String, String, String, String) {
    (r.into(), k.into(), v.into(), p.into())
}

/// §11.1: a work document's state is its `phase`, an item's its one
/// `checklist` state (a live claim beats blocked), a done title-only
/// document's phase is `recorded`; and a journal line newer than v2 makes
/// every state of its document UNKNOWN, none reported in its place.
#[test]
fn states_are_reported_by_family_and_unknown_when_the_journal_is_newer() {
    let root = scratch("family");
    ticket(&root, "t-0d61d", U1, THREE);
    ticket(&root, "t-0e72a", U2, "");
    commit(&root);
    let claim = json(
        &root,
        Some("alice"),
        &["claim", "t-0d61d/i-4d11", "--at", AT],
    );
    assert_eq!(claim["exit_code"], 0, "{claim}");
    let done = json(&root, Some("alice"), &["claim", "t-0e72a", "--at", AT]);
    assert_eq!(done["exit_code"], 0, "{done}");
    let done = json(&root, Some("alice"), &["done", "t-0e72a", "--at", AT]);
    assert_eq!(done["exit_code"], 0, "{done}");
    let (v, rows) = states(&root);
    assert_eq!(
        rows,
        vec![
            row("t-0d61d", "phase", "in_progress", "computed"),
            row("t-0d61d/i-4d11", "checklist", "claimed", "computed"),
            row("t-0d61d/i-77be", "checklist", "blocked", "computed"),
            row("t-0d61d/i-9c01", "checklist", "open", "computed"),
            row("t-0e72a", "phase", "done", "recorded"),
        ],
        "{v}"
    );
    assert!(found(&v, "work.").is_empty(), "{v}");

    // A v3 line: UNKNOWN, and t-0d61d's states are not reported at all.
    let mut text = std::fs::read_to_string(journal_path(&root, "t-0d61d")).unwrap();
    text.push_str(
        &line(
            U1,
            "01a0f015-0000-7000-8000-000000000015",
            "bob",
            "2026-10-13T08:00:00Z",
            "work.note_added",
            r#"{"note":"x","target":"t-0d61d"}"#,
        )
        .replace(r#""v":2"#, r#""v":3"#),
    );
    text.push('\n');
    write_journal(&root, "t-0d61d", &text);
    let (v, rows) = states(&root);
    assert_eq!(
        rows,
        vec![row("t-0e72a", "phase", "done", "recorded")],
        "{v}"
    );
    assert_eq!(
        v["result"]["unknown"],
        serde_json::json!(["t-0d61d"]),
        "{v}"
    );
    assert_eq!(found(&v, "work.journal-newer").len(), 1, "{v}");
    assert!(found(&v, "work.journal-malformed").is_empty(), "{v}");
}

/// §11.2 and §13.1: a journal is refused at its faulty line: an
/// unterminated last line, parseable or not (torn); a line whose key does
/// not match (malformed); a committed line that is gone (rewritten). The
/// same journal with every line whole and kept is read.
#[test]
fn journal_refusals_name_the_faulty_line() {
    let root = scratch("lines");
    ticket(
        &root,
        "t-0d61d",
        U1,
        "- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12\n",
    );
    let one = line(
        U1,
        "01a0f014-0000-7000-8000-000000000014",
        "claude",
        "2026-10-12T11:00:00Z",
        "work.item_done",
        r#"{"item":"i-9c01","on":"2026-10-12","target":"t-0d61d/i-9c01"}"#,
    );
    let two = line(
        U1,
        "01a0f015-0000-7000-8000-000000000015",
        "bob",
        "2026-10-13T08:00:00Z",
        "work.note_added",
        r#"{"note":"the template needs a plain-text part","target":"t-0d61d"}"#,
    );
    let rel = "docs/tickets/t-0d61d/journal.jsonl";

    // Accepted: both lines whole.
    write_journal(&root, "t-0d61d", &format!("{one}\n{two}\n"));
    commit(&root);
    let (v, _) = states(&root);
    assert!(found(&v, "work.journal").is_empty(), "{v}");

    // Torn: the same second line without its newline.
    write_journal(&root, "t-0d61d", &format!("{one}\n{two}"));
    let (v, _) = states(&root);
    assert_eq!(
        found(&v, "work.journal"),
        vec![format!("work.journal-torn {rel}:2")],
        "{v}"
    );

    // Malformed: the second line's key does not match its content.
    let forged = two.replace(
        &two[two.find("\"idempotency_key\":\"").unwrap() + 19..][..64],
        &"f".repeat(64),
    );
    write_journal(&root, "t-0d61d", &format!("{one}\n{forged}\n"));
    let (v, _) = states(&root);
    assert_eq!(
        found(&v, "work.journal"),
        vec![format!("work.journal-malformed {rel}:2")],
        "{v}"
    );

    // Rewritten: the committed second line is gone.
    write_journal(&root, "t-0d61d", &format!("{one}\n"));
    let (v, _) = states(&root);
    assert_eq!(
        found(&v, "work.journal"),
        vec![format!("work.journal-rewritten {rel}:2")],
        "{v}"
    );
}

/// §11.2: a Format 1.0 document's note is one `work.note_added` event,
/// `{target, note}`, and the document is not touched. The same note from
/// the same actor replays and writes nothing; from another actor it is
/// refused `work.idempotency-conflict` at the line that records it.
#[test]
fn a_note_is_one_journal_event_written_once() {
    let root = scratch("note");
    ticket(&root, "t-0d61d", U1, THREE);
    commit(&root);
    let doc = std::fs::read(root.join("docs/tickets/t-0d61d.md")).unwrap();
    let noted = json(
        &root,
        Some("bob"),
        &["note", "t-0d61d", "plain-text part", "--at", AT],
    );
    assert_eq!(noted["exit_code"], 0, "{noted}");
    assert_eq!(
        std::fs::read(root.join("docs/tickets/t-0d61d.md")).unwrap(),
        doc
    );
    let ev = events(&root, "t-0d61d");
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0]["type"], "work.note_added");
    assert_eq!(ev[0]["occurred_at"], AT);
    assert_eq!(
        ev[0]["payload"],
        serde_json::json!({"target": "t-0d61d", "note": "plain-text part"})
    );
    // `war show` still shows it.
    let shown = json(&root, None, &["show", "t-0d61d"]);
    assert!(
        shown["result"]["markdown"]
            .as_str()
            .unwrap()
            .contains("bob:** plain-text part"),
        "{shown}"
    );

    let replay = json(&root, Some("bob"), &["note", "t-0d61d", "plain-text part"]);
    assert_eq!(replay["exit_code"], 0, "{replay}");
    assert_eq!(replay["result"]["replayed"], true, "{replay}");
    assert_eq!(events(&root, "t-0d61d").len(), 1);

    let other = json(
        &root,
        Some("alice"),
        &["note", "t-0d61d", "plain-text part"],
    );
    assert_eq!(other["exit_code"], 2, "{other}");
    assert_eq!(
        found(&other, "work."),
        vec!["work.idempotency-conflict docs/tickets/t-0d61d/journal.jsonl:1".to_owned()],
        "{other}"
    );
    assert_eq!(events(&root, "t-0d61d").len(), 1);
}

/// §11.2: `work.claimed`, `work.item_done`, `work.item_reopened` and
/// `work.released` carry `episode`, the count of the same type for the
/// same target already in the journal; so done, reopened and done again
/// are three events, the second done not a replay of the first.
#[test]
fn episode_counts_the_same_fact_happening_again() {
    let root = scratch("episode");
    ticket(&root, "t-0d61d", U1, "- [ ] Reset endpoint (i-9c01)\n");
    commit(&root);
    let alice = Some("alice");
    for args in [
        vec!["claim", "i-9c01"],
        vec!["done", "i-9c01"],
        vec!["admin", "reopen", "i-9c01", "--reason", "regressed"],
        vec!["claim", "i-9c01"],
        vec!["done", "i-9c01"],
    ] {
        let v = json(&root, alice, &args);
        assert_eq!(v["exit_code"], 0, "{args:?}: {v}");
    }
    let got: Vec<(String, u64)> = events(&root, "t-0d61d")
        .iter()
        .map(|e| {
            (
                e["type"].as_str().unwrap().to_owned(),
                e["payload"]["episode"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{e}")),
            )
        })
        .collect();
    let want: Vec<(String, u64)> = [
        ("work.claimed", 0),
        ("work.item_done", 0),
        ("work.item_reopened", 0),
        ("work.claimed", 1),
        ("work.item_done", 1),
    ]
    .iter()
    .map(|(t, n)| ((*t).to_owned(), *n))
    .collect();
    assert_eq!(got, want);
}

/// §11.3: a done that names the revision it was based on needs no prior
/// claim (compare-and-set keeps two finishers from both writing); without
/// a base, an unclaimed done is still refused; a stale base is refused at
/// the item's line and writes nothing.
#[test]
fn a_done_based_on_a_revision_needs_no_prior_claim() {
    let root = scratch("based");
    ticket(&root, "t-0d61d", U1, THREE);
    commit(&root);
    let path = root.join("docs/tickets/t-0d61d.md");
    let before = std::fs::read(&path).unwrap();

    let bare = json(&root, Some("alice"), &["done", "t-0d61d/i-4d11"]);
    assert_eq!(bare["exit_code"], 2, "{bare}");
    assert!(found(&bare, "ticket.not-claimed").len() == 1, "{bare}");

    let stale = json(
        &root,
        Some("alice"),
        &[
            "done",
            "t-0d61d/i-4d11",
            "--if-rev",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ],
    );
    assert_eq!(
        found(&stale, "work."),
        vec!["work.stale-revision docs/tickets/t-0d61d.md:12".to_owned()],
        "{stale}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);

    let shown = json(&root, None, &["show", "t-0d61d"]);
    let rev = shown["result"]["items"][1]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let ok = json(
        &root,
        Some("alice"),
        &["done", "t-0d61d/i-4d11", "--if-rev", &rev, "--at", AT],
    );
    assert_eq!(ok["exit_code"], 0, "{ok}");
    let doc = std::fs::read_to_string(&path).unwrap();
    assert!(
        doc.contains("- [x] Email template (i-4d11) — done by alice, 2026-10-14\n"),
        "{doc}"
    );
    // The same base again is stale now: the second finisher writes nothing.
    let again = json(
        &root,
        Some("bob"),
        &["done", "t-0d61d/i-4d11", "--if-rev", &rev],
    );
    assert_eq!(again["exit_code"], 2, "{again}");
}

/// §11.3: a claim on a work document covers its items, so it is refused
/// while another actor holds one of them, naming that claim's file; an
/// managed item claim keeps its reservation after expiry until released.
#[test]
fn a_document_claim_is_refused_while_another_holds_an_item() {
    let root = scratch("whole");
    ticket(&root, "t-0d61d", U1, THREE);
    commit(&root);
    let bob = json(
        &root,
        Some("bob"),
        &["claim", "t-0d61d/i-4d11", "--at", "2026-10-14T09:50:00Z"],
    );
    assert_eq!(bob["exit_code"], 0, "{bob}");
    // The claim is dated when it acted (`--at`).
    assert_eq!(
        events(&root, "t-0d61d")[0]["payload"]["since"],
        "2026-10-14T09:50:00Z"
    );
    let alice = json(&root, Some("alice"), &["claim", "t-0d61d", "--at", AT]);
    assert_eq!(alice["exit_code"], 2, "{alice}");
    let refused = found(&alice, "work.claimed-by-other");
    assert_eq!(refused.len(), 1, "{alice}");
    assert!(
        refused[0].ends_with("openwarrant/claims/t-0d61d--i-4d11.lock"),
        "{alice}"
    );
    // An hour later bob's lease (30 minutes) has run out: the claim stands.
    let later = json(
        &root,
        Some("alice"),
        &["claim", "t-0d61d", "--at", "2026-10-14T11:00:00Z"],
    );
    assert_eq!(later["exit_code"], 2, "{later}");
    assert!(
        later["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "work.writer-unfenced"),
        "{later}"
    );
    let released = json(&root, Some("bob"), &["release", "t-0d61d/i-4d11"]);
    assert_eq!(released["exit_code"], 0, "{released}");
    let after_release = json(
        &root,
        Some("alice"),
        &["claim", "t-0d61d", "--at", "2026-10-14T11:00:00Z"],
    );
    assert_eq!(after_release["exit_code"], 0, "{after_release}");
}

/// §3.8, Appendix A: a done suffix's tick level is observed, independent
/// or signed; `[claimed]` (which writes none) is refused `format.item` at
/// its line, where `[observed]` reads (and warns, unbacked, §11.4).
#[test]
fn an_unknown_tick_level_is_refused() {
    let root = scratch("level");
    ticket(
        &root,
        "t-0d61d",
        U1,
        "- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12 [observed]\n",
    );
    let (ok, items) = model_found(&root, "format.item");
    assert!(items.is_empty(), "{ok}");
    let (v, _) = states(&root);
    assert_eq!(found(&v, "work.tick-unbacked").len(), 1, "{v}");
    ticket(
        &root,
        "t-0d61d",
        U1,
        "- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12 [claimed]\n",
    );
    let (bad, items) = model_found(&root, "format.item");
    assert_eq!(
        items,
        vec!["format.item docs/tickets/t-0d61d.md:11".to_owned()],
        "{bad}"
    );
}

/// §11.4: `observed` is backed by a done naming passing `check.run`s by
/// their `run` id; a run that failed backs nothing (`work.tick-unbacked`).
#[test]
fn an_observed_tick_is_backed_by_a_passing_check_run() {
    for (verdict, warned) in [("PASS", 0), ("FAIL", 1)] {
        let root = scratch("observed");
        ticket(
            &root,
            "t-0d61d",
            U1,
            "- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12 [observed]\n",
        );
        let run = line(
            U1,
            "01a0f007-0000-7000-8000-000000000007",
            "ci",
            "2026-10-12T10:59:00Z",
            "check.run",
            &format!(
                r#"{{"check":"CHK-reset","revision":"sha256:{}","run":"r-1","subjects":[],"verdict":"{verdict}"}}"#,
                "1".repeat(64)
            ),
        );
        let done = line(
            U1,
            "01a0f008-0000-7000-8000-000000000008",
            "claude",
            "2026-10-12T11:00:00Z",
            "work.item_done",
            r#"{"item":"i-9c01","level":"observed","on":"2026-10-12","runs":["r-1"],"target":"t-0d61d/i-9c01"}"#,
        );
        write_journal(&root, "t-0d61d", &format!("{run}\n{done}\n"));
        commit(&root);
        let (v, _) = states(&root);
        assert_eq!(
            found(&v, "work.tick-unbacked").len(),
            warned,
            "{verdict}: {v}"
        );
    }
}

/// §7.5: a profile file written in 7.5's shape without `format` is a
/// legacy profile and governs nothing: its document reads
/// `format.profile-unknown` (with a warning naming the missing key) and
/// nothing is refused; with `format` it governs the document.
#[test]
fn a_profile_file_without_format_governs_nothing() {
    let root = scratch("legacy-profile");
    let doc = root.join("docs/records/ops/restart.md");
    std::fs::create_dir_all(doc.parent().unwrap()).unwrap();
    std::fs::write(
        &doc,
        "---\nformat: openwarrant/1.0\nprofile: runbook\nowner: ops\nsummary: Restart.\n---\n\
         # Restart the account service\n\n## Steps\n## STEP-1 · step — restart the unit\n\
         command: systemctl restart accounts\n\nWait.\n",
    )
    .unwrap();
    let body = "schema = \"oh.war/profile/v1\"\nname = \"runbook\"\nform = \"document\"\n\
                note = \"A runbook.\"\ncapabilities = [\"structure\", \"links\"]\n\n\
                [frontmatter]\nrequired = []\noptional = [\"owner\"]\n\n\
                [sections]\nrequired = [\"Steps\"]\noptional = []\n\n[records]\ntypes = [\"step\"]\n\n\
                [fields.step]\ncommand = { required = true, shape = \"line\" }\n";
    std::fs::create_dir_all(root.join("profiles")).unwrap();
    std::fs::write(root.join("profiles/runbook.toml"), body).unwrap();
    let (legacy, unknown) = model_found(&root, "format.profile-unknown");
    assert_ne!(legacy["command"], "error", "{legacy}");
    assert_eq!(unknown.len(), 1, "{legacy}");
    let (_, profile) = model_found(&root, "profile.");
    assert_eq!(
        profile,
        vec!["profile.format-missing profiles/runbook.toml:1".to_owned()],
        "{legacy}"
    );

    std::fs::write(
        root.join("profiles/runbook.toml"),
        body.replace("name = ", "format = \"openwarrant/1.0\"\nname = "),
    )
    .unwrap();
    let (governed, unknown) = model_found(&root, "format.profile-unknown");
    assert!(unknown.is_empty(), "{governed}");
    let (_, profile) = model_found(&root, "profile.");
    assert!(profile.is_empty(), "{governed}");
}
