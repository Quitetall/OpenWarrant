// SPDX-License-Identifier: Apache-2.0
//! What the first field report on alpha.7 found (2026-11-13), each claim
//! beside the case that must not satisfy it: a single-file Warrant's checks
//! run and it joins its phase; an observation is stale only when source
//! changes; `war start` carries the records the work rests on; work waits on
//! the phases its phase depends on; a default actor name is said.
use std::path::Path;
use std::process::Command;

fn war_env(root: &Path, args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_war"));
    cmd.args(["--root", root.to_str().unwrap()])
        .args(args)
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("SSH_AUTH_SOCK");
    for name in [
        "OPENWARRANT_ACTOR",
        "OPENWARRANT_SESSION_ID",
        "KATANA_SESSION_ID",
        "CLAUDECODE",
        "CLAUDE_CODE_SESSION_ID",
        "CODEX_THREAD_ID",
        "CODEX_SESSION_ID",
        "CODEX_SANDBOX",
        "OPENCODE",
        "GEMINI_CLI",
        "CURSOR_AGENT",
    ] {
        cmd.env_remove(name);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn human(root: &Path, args: &[&str]) -> (String, i32) {
    let out = war_env(root, args, &[]);
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code().unwrap_or(-1),
    )
}

fn json(root: &Path, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = war_env(root, &all, &[]);
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{args:?}: {}", String::from_utf8_lossy(&out.stdout)))
}

fn git(root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "user.email=t@invalid",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .env_remove("SSH_AUTH_SOCK")
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

fn commit(root: &Path) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "work"]);
}

const PHASES: &str = r#"schema: "oh.war/roadmap-phases/v1"

tiers: []

phases:
  - id: "PP-PHASE-3"
    title: "Workspace"
    outcome: "the workspace builds"
    exit: "workspace tests pass"
    depends_on: []
    open: []

  - id: "PP-PHASE-4"
    title: "Web"
    outcome: "the web app"
    exit: "ci-web passes"
    depends_on: ["PP-PHASE-3"]
    open: []
"#;

const SPEC: &str = "---\nformat: openwarrant/1.0\nprofile: spec\nuuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01\nversion: 0.1.0\nsummary: PP.\n---\n# PP\n\n## Purpose\nPreamble no ticket needs: PURPOSE-MARK.\n\n## Scope\nIn: everything.\n\n## REQ-ws-1 · requirement — the workspace builds\nstatement: The workspace MUST build. REQ-MARK\n\n## CON-ws-1 · constraint — no datastore\nconstrains REQ-ws-1\nstatement: It MUST NOT add a datastore. CON-MARK\n\n## DEC-ws-1 · decision — one crate\nselected_over OPT-ws-1\nconstrains REQ-ws-1\n\nOne crate. DEC-MARK\n\n## OPT-ws-1 · option — many crates\n\nRejected. OPT-MARK\n";

/// A git repository with a roadmap (phase 4 after phase 3), a spec, and a
/// test command `sh ok.sh` that passes while `ok` holds `yes`.
fn repo() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let r = root.path();
    git(r, &["init", "-q", "."]);
    assert_eq!(json(r, &["init", "--namespace", "PP"])["exit_code"], 0);
    std::fs::create_dir_all(r.join("docs/roadmap/atoms")).unwrap();
    std::fs::write(
        r.join("docs/roadmap/roadmap.toml"),
        "schema = \"oh.war/roadmap/v1\"\nuuid = \"01a0cd3a-0a24-74cb-9a4b-5a83dfe420f9\"\nprogram = \"PP\"\nprefix = \"PP\"\n\n[[atoms]]\nordinal = 10\nrole = \"intent\"\npath = \"atoms/10-intent.md\"\n\n[[atoms]]\nordinal = 20\nrole = \"phases\"\npath = \"atoms/20-phases.yaml\"\n",
    )
    .unwrap();
    std::fs::write(
        r.join("docs/roadmap/atoms/10-intent.md"),
        "# Intent\n\nPP.\n",
    )
    .unwrap();
    std::fs::write(r.join("docs/roadmap/atoms/20-phases.yaml"), PHASES).unwrap();
    std::fs::write(r.join("SPEC.md"), SPEC).unwrap();
    std::fs::write(r.join("ok.sh"), "test \"$(cat ok)\" = yes\n").unwrap();
    std::fs::write(r.join("ok"), "yes\n").unwrap();
    std::fs::write(r.join("src.txt"), "source\n").unwrap();
    commit(r);
    root
}

/// A ticket with `items`, `part_of` `phase` and implementing `implements`.
fn ticket(r: &Path, title: &str, items: &[&str], phase: &str, implements: &[&str]) -> Vec<String> {
    let mut args = vec!["create", title];
    for i in items {
        args.extend(["--item", i]);
    }
    let made = json(r, &args);
    assert_eq!(made["exit_code"], 0, "{made}");
    let id = made["result"]["id"].as_str().unwrap().to_owned();
    let path = crate::work_paths::at(r.join(format!("docs/tickets/{id}.md")));
    let mut links = String::new();
    if !phase.is_empty() {
        links.push_str(&format!("part_of: \"{phase}\"\n"));
    }
    if !implements.is_empty() {
        links.push_str("implements:\n");
        for i in implements {
            links.push_str(&format!("  - {i}\n"));
        }
    }
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replacen("priority:", &format!("{links}priority:"), 1),
    )
    .unwrap();
    let mut ids = vec![id];
    ids.extend(
        made["result"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap().to_owned()),
    );
    ids
}

fn single_file_warrant(r: &Path) {
    let (out, code) = human(r, &["plan", "new", "Workspace builds", "--format", "1.0"]);
    assert_eq!(code, 0, "{out}");
    let path = r.join("docs/warrants/PP-WAR-0001.md");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!(
            "{}\n## CHK-workspace · check — workspace tests pass\nrun: sh ok.sh\non: manual\nevaluates OBL-001\n",
            text.replacen("summary:", "part_of: \"PP-PHASE-3\"\nsummary:", 1)
        ),
    )
    .unwrap();
}

#[test]
fn a_single_file_warrants_check_runs_by_alias_and_by_id() {
    let root = repo();
    let r = root.path();
    single_file_warrant(r);
    commit(r);
    for target in ["PP-WAR-0001", "CHK-workspace", "PP-WAR-0001/CHK-workspace"] {
        let (out, code) = human(r, &["check", "run", target]);
        assert_eq!(code, 0, "{target}: {out}");
        assert!(
            out.contains("PP-WAR-0001/CHK-workspace PASS"),
            "{target}: {out}"
        );
    }
    // Its runs are kept in its own state, beside the document.
    assert!(r.join("docs/warrants/PP-WAR-0001/journal.jsonl").is_file());
    assert!(!r.join("docs/tickets/PP-WAR-0001").exists());
    // The refusal: a failing check fails there too, and a name nothing has is refused.
    std::fs::write(r.join("ok"), "no\n").unwrap();
    let (out, code) = human(r, &["check", "run", "PP-WAR-0001"]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("check.failed"), "{out}");
    let (out, code) = human(r, &["check", "run", "CHK-absent"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("check.nothing"), "{out}");
}

#[test]
fn a_single_file_warrant_joins_the_phase_it_is_part_of() {
    let root = repo();
    let r = root.path();
    single_file_warrant(r);
    let (out, _) = human(r, &["check"]);
    assert!(!out.contains("roadmap.unassigned"), "{out}");
    assert!(!out.contains("corpus.empty"), "{out}");
    let view = json(r, &["plan", "roadmap"]);
    let phases = view["result"]["phases"].as_array().unwrap();
    let members = |id: &str| phases.iter().find(|p| p["id"] == id).unwrap()["members"].clone();
    assert_eq!(members("PP-PHASE-3"), serde_json::json!(["PP-WAR-0001"]));
    assert_eq!(members("PP-PHASE-4"), serde_json::json!([]));
    // The refusal: without the relation it names no phase, and check says so
    // at the document, never at a manifest it does not have.
    let path = r.join("docs/warrants/PP-WAR-0001.md");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("part_of: \"PP-PHASE-3\"\n", "")).unwrap();
    let (out, _) = human(r, &["check"]);
    assert!(out.contains("roadmap.unassigned"), "{out}");
    assert!(out.contains("docs/warrants/PP-WAR-0001.md"), "{out}");
    assert!(!out.contains("PP-WAR-0001/manifest.toml"), "{out}");
}

#[test]
fn an_observation_is_stale_only_when_source_changes() {
    let root = repo();
    let r = root.path();
    let a = ticket(r, "First", &["one"], "", &[]);
    let b = ticket(r, "Second", &["one"], "", &[]);
    for t in [&a, &b] {
        let (out, code) = human(
            r,
            &["plan", "add", &t[0], "--test", "sh ok.sh", "--name", "t"],
        );
        assert_eq!(code, 0, "{out}");
    }
    commit(r);
    // Twice with no edit: each run's own receipts, and the other ticket's,
    // stale nothing.
    for round in 0..2 {
        let (out, code) = human(r, &["check", "run"]);
        assert_eq!(code, 0, "{out}");
        assert!(!out.contains("check.source-stale"), "round {round}: {out}");
        assert!(!out.contains(" stale by "), "round {round}: {out}");
    }
    // Other work being tracked meanwhile is not a source change either.
    ticket(r, "Third", &["one"], "", &[]);
    let (out, _) = human(r, &["check", "evaluate"]);
    assert!(!out.contains("check.source-stale"), "{out}");
    // The refusal: an edit to a source file does stale what was observed.
    std::fs::write(r.join("src.txt"), "changed\n").unwrap();
    let (out, _) = human(r, &["check", "evaluate"]);
    assert!(out.contains("check.source-stale"), "{out}");
}

#[test]
fn start_prints_the_records_the_work_rests_on() {
    let root = repo();
    let r = root.path();
    let t = ticket(r, "Build it", &["one"], "PP-PHASE-3", &["REQ-ws-1"]);
    let (out, code) = human(r, &["start", &t[0]]);
    assert_eq!(code, 0, "{out}");
    for mark in ["REQ-MARK", "CON-MARK", "DEC-MARK"] {
        assert!(out.contains(mark), "{mark}: {out}");
    }
    assert!(out.contains("> REQ-ws-1 · requirement · SPEC.md:"), "{out}");
    // Only what the task packet selects: no rejected option, no spec
    // preamble, no instruction file, and the closing line stays last but one.
    for absent in [
        "OPT-MARK",
        "PURPOSE-MARK",
        "Ordinary coding needs no Warrant",
    ] {
        assert!(!out.contains(absent), "{absent}: {out}");
    }
    assert!(out.find("## Records").unwrap() < out.find("Close it in the same command").unwrap());
    // A ticket that cites nothing has no Records section.
    let plain = ticket(r, "Plain", &["one"], "PP-PHASE-3", &[]);
    let (out, _) = human(r, &["start", &plain[0]]);
    assert!(!out.contains("## Records"), "{out}");
}

#[test]
fn a_record_too_long_for_the_brief_is_listed_and_the_cut_is_said() {
    let root = repo();
    let r = root.path();
    let long = "Long prose. ".repeat(700);
    std::fs::write(
        r.join("SPEC.md"),
        SPEC.replace(
            "It MUST NOT add a datastore. CON-MARK",
            &format!("{long}CON-MARK"),
        ),
    )
    .unwrap();
    let t = ticket(r, "Build it", &["one"], "", &["REQ-ws-1"]);
    let (out, code) = human(r, &["start", &t[0]]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("REQ-MARK"), "{out}");
    assert!(!out.contains("CON-MARK"), "{out}");
    assert!(
        out.contains("Not printed (this brief holds 6144 bytes)"),
        "{out}"
    );
    assert!(out.contains("- CON-ws-1 · constraint · SPEC.md:"), "{out}");
    assert!(out.len() < 6144 + 512, "{} bytes", out.len());
}

#[test]
fn work_waits_on_the_phases_its_phase_depends_on() {
    let root = repo();
    let r = root.path();
    let three = ticket(r, "Phase three work", &["one"], "PP-PHASE-3", &[]);
    let four = ticket(r, "Phase four work", &["one"], "PP-PHASE-4", &[]);
    let ready = |r: &Path| human(r, &["view", "next"]).0;
    let listed = ready(r);
    assert!(listed.contains(&three[0]), "{listed}");
    assert!(!listed.contains(&four[0]), "{listed}");
    let (prime, _) = human(r, &["view", "prime"]);
    assert!(prime.contains("waits on PP-PHASE-3"), "{prime}");
    let (show, _) = human(r, &["view", "show", &four[0]]);
    assert!(show.contains("part of PP-PHASE-4 (Web)"), "{show}");
    assert!(
        show.contains(&format!(
            "`{}`",
            crate::work_paths::at(r.join(format!("docs/tickets/{}.md", four[0])))
                .strip_prefix(r)
                .unwrap()
                .display()
        )),
        "{show}"
    );
    // Taking it by name is refused the same way, naming the phase.
    let (out, code) = human(r, &["start", &four[0]]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("PP-PHASE-3"), "{out}");
    // Once phase 3 holds no open work, phase 4's is ready.
    let (out, code) = human(r, &["done", &three[0]]);
    assert_eq!(code, 0, "{out}");
    let listed = ready(r);
    assert!(listed.contains(&four[0]), "{listed}");
    // A phase the roadmap does not declare resolves to nothing, and is said.
    let lost = ticket(r, "Lost", &["one"], "PP-PHASE-9", &[]);
    let (show, _) = human(r, &["view", "show", &lost[0]]);
    assert!(show.contains("part of PP-PHASE-9 (unknown)"), "{show}");
}

#[test]
fn a_ticket_test_is_one_line_and_raises_what_was_ticked_before_it() {
    let root = repo();
    let r = root.path();
    let t = ticket(r, "Late test", &["first", "second", "third"], "", &[]);
    assert_eq!(human(r, &["start", &t[0]]).1, 0);
    let (out, code) = human(r, &["done", &t[1]]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("ticked as claimed"), "{out}");
    let (out, code) = human(
        r,
        &["plan", "add", &t[0], "--test", "sh ok.sh", "--name", "late"],
    );
    assert_eq!(code, 0, "{out}");
    let doc = std::fs::read_to_string(crate::work_paths::at(
        r.join(format!("docs/tickets/{}.md", t[0])),
    ))
    .unwrap();
    assert_eq!(doc.matches("## CHK-late").count(), 1, "{doc}");
    // The refusal: a failing test ticks nothing and raises nothing.
    std::fs::write(r.join("ok"), "no\n").unwrap();
    let (out, code) = human(r, &["done", &t[0], "--check"]);
    assert_ne!(code, 0, "{out}");
    let (show, _) = human(r, &["view", "show", &t[0]]);
    assert!(
        show.contains(&format!("- [x] (claimed) first ({})", t[1])),
        "{show}"
    );
    assert!(show.contains(&format!("- [ ] second ({})", t[2])), "{show}");
    std::fs::write(r.join("ok"), "yes\n").unwrap();
    let (out, code) = human(r, &["done", &t[0], "--check"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(&format!("raised from claimed to observed: {}", t[1])),
        "{out}"
    );
    let (show, _) = human(r, &["view", "show", &t[0]]);
    assert!(!show.contains("(claimed)"), "{show}");
    assert_eq!(show.matches("- test late:").count(), 1, "{show}");
    assert!(
        show.contains(&format!("(for {}, {}, {})", t[1], t[2], t[3])),
        "{show}"
    );
    assert!(!show.contains("atoms/30-checks.md"), "{show}");
}

#[test]
fn a_default_actor_name_is_said_and_a_chosen_one_is_not() {
    let root = repo();
    let r = root.path();
    let t = ticket(r, "One", &["one"], "", &[]);
    let start = |env: &[(&str, &str)]| {
        let _ = war_env(r, &["done", &t[0], "--release"], env);
        let out = war_env(r, &["start", &t[0]], env);
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let out = start(&[]);
    assert!(
        out.contains("acting as claude, the default name: `--as <name>`"),
        "{out}"
    );
    let _ = war_env(r, &["done", &t[0], "--release"], &[]);
    let out = start(&[("CODEX_THREAD_ID", "abc")]);
    assert!(out.contains("in a codex session: `--as codex`"), "{out}");
    let _ = war_env(
        r,
        &["done", &t[0], "--release"],
        &[("CODEX_THREAD_ID", "abc")],
    );
    // Named, or confirmed by the harness the default names: nothing is added.
    for env in [[("OPENWARRANT_ACTOR", "ada")], [("CLAUDECODE", "1")]] {
        let out = start(&env);
        assert!(!out.contains("the default name"), "{env:?}: {out}");
        let _ = war_env(r, &["done", &t[0], "--release"], &env);
    }
}

/// PR #424's fast CI: a read verb rewrote `.git/index`. Git refreshes it on
/// `status` and on a working-tree `diff` when a tracked file's stat data moved
/// and its bytes did not; here one always has.
#[test]
fn read_verbs_leave_the_index_bytes_alone() {
    let root = repo();
    let r = root.path();
    single_file_warrant(r);
    let t = ticket(r, "Checked", &["one"], "PP-PHASE-3", &["REQ-ws-1"]);
    let (out, code) = human(
        r,
        &["plan", "add", &t[0], "--test", "sh ok.sh", "--name", "t"],
    );
    assert_eq!(code, 0, "{out}");
    commit(r);
    assert_eq!(human(r, &["check", "run"]).1, 0);
    commit(r);
    let touch = |name: &str| {
        let file = std::fs::File::options()
            .write(true)
            .open(r.join(name))
            .unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        file.set_modified(later).unwrap();
    };
    let index = || std::fs::read(r.join(".git/index")).unwrap();
    for name in ["src.txt", "SPEC.md", "ok.sh"] {
        touch(name);
    }
    let before = index();
    for verb in [
        &["status"][..],
        &["status", "--verbose"],
        &["check"],
        &["check", "evaluate"],
        &["view", "next"],
        &["view", "prime"],
        &["view", "show", &t[0]],
        &["view", "context", "--index"],
        &["context", &t[0]],
        &["plan", "frontier", "--all"],
        &["plan", "estimate"],
        &["plan", "roadmap"],
    ] {
        let _ = human(r, verb);
        assert!(
            index() == before,
            "`war {}` rewrote .git/index",
            verb.join(" ")
        );
    }
    // The refusal: git asked the same way without the guard does rewrite it.
    git(r, &["diff", "--quiet", "HEAD"]);
    assert!(
        index() != before,
        "the touched files did not make git refresh the index"
    );
}
