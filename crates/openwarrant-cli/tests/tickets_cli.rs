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
fn legacy_completion_names_the_real_workflow_without_ticking_or_resolving() {
    let root = scratch("legacy-completion");
    let created = war(
        &root,
        &["plan", "new", "Legacy completion", "--format", "legacy"],
    );
    assert!(created.status.success());
    let dir = std::fs::read_dir(root.join("docs/warrants"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|dir| {
            std::fs::read_to_string(dir.join("manifest.toml"))
                .is_ok_and(|text| text.contains("title = \"Legacy completion\""))
        })
        .expect("the directory Warrant exists");
    let alias = dir.file_name().unwrap().to_str().unwrap();
    let journal = std::fs::read(dir.join("journal.jsonl")).unwrap();
    let manifest = std::fs::read(dir.join("manifest.toml")).unwrap();
    let stage = format!("{alias}/STAGE-001");
    for (target, rule) in [
        (alias, "done.legacy-warrant"),
        (stage.as_str(), "done.legacy-stage"),
    ] {
        let result = json(&root, &["done", target, "--check", "--note", "complete"]);
        assert_eq!(result["exit_code"], 2, "{result}");
        let diagnostics = result["diagnostics"].as_array().unwrap();
        assert!(diagnostics.iter().any(|d| d["rule"] == rule), "{result}");
        assert!(!diagnostics.iter().any(|d| d["rule"] == "ticket.unknown"));
        assert!(diagnostics.iter().any(|d| {
            d["message"]
                .as_str()
                .unwrap()
                .contains("war sign resolve --dry-run")
        }));
    }
    let bad_stage = json(&root, &["done", &format!("{alias}/STAGE-missing")]);
    assert!(
        bad_stage["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "done.legacy-stage-unknown")
    );
    assert_eq!(std::fs::read(dir.join("journal.jsonl")).unwrap(), journal);
    assert_eq!(std::fs::read(dir.join("manifest.toml")).unwrap(), manifest);
    assert!(!dir.join("resolution.toml").exists());
    let unknown = json(&root, &["done", "NOT-A-WARRANT/STAGE-001"]);
    assert!(
        unknown["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "ticket.unknown")
    );
    std::fs::remove_dir_all(root).unwrap();
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
        std::fs::read_to_string(root.join(created["result"]["source"].as_str().unwrap())).unwrap();
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
/// A single-file ticket keeps its notes in the journal, not its body, and
/// the comment left on its issue still carries them.
#[cfg(unix)]
#[test]
fn the_issue_comment_carries_the_notes_of_a_single_file_ticket() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("writeback-notes");
    let bin = root.join("fake-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let gh = bin.join("gh");
    std::fs::write(
        &gh,
        "#!/bin/sh\n\
         case \"$1 $2\" in\n\
         'issue view') printf '{\"number\":%s,\"title\":\"T\",\"body\":\"B\",\"url\":\"https://github.com/o/r/issues/%s\"}' \"$3\" \"$3\";;\n\
         'issue comment') printf '%s' \"$5\" > \"$FAKE_GH_COMMENT\";;\n\
         esac\n",
    )
    .unwrap();
    std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
    let config = root.join("openwarrant.toml");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text.push_str(
        "\n[intake]\nfetch_argv = [\"gh\", \"issue\", \"view\", \"{id}\", \"--json\", \"number,title,body,url\"]\n\
         [intake.writeback]\ncomment_argv = [\"gh\", \"issue\", \"comment\", \"{id}\", \"--body\", \"{body}\"]\n\
         close_argv = [\"gh\", \"issue\", \"close\", \"{id}\"]\n",
    );
    std::fs::write(&config, text).unwrap();
    let comment = root.join("comment.txt");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&root)
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .env("PATH", &path)
            .env("FAKE_GH_COMMENT", &comment)
            .env_remove("OPENWARRANT_ACTOR")
            .env_remove("SSH_AUTH_SOCK")
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let made: serde_json::Value =
        serde_json::from_str(&run(&["--json", "create", "--issue", "13", "-i", "Fix it"])).unwrap();
    let id = made["result"]["id"].as_str().unwrap();
    let item = made["result"]["items"][0]["id"].as_str().unwrap();
    assert!(
        root.join(format!("docs/tickets/{id}.md")).is_file(),
        "not a single-file ticket"
    );
    run(&["note", id, "root cause: an unchecked empty string"]);
    let target = format!("{id}/{item}");
    run(&["claim", &target]);
    run(&["done", &target, "--note", "guarded"]);
    let body = std::fs::read_to_string(&comment).unwrap();
    assert!(
        body.contains("Fix it") && body.contains(": guarded"),
        "{body}"
    );
    assert!(body.contains("\nNotes:\n\n"), "{body}");
    assert!(
        body.contains("root cause: an unchecked empty string"),
        "{body}"
    );
}

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
    declare_scope(&root, &first, "work/first/**");
    declare_scope(&root, &second, "work/second/**");
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

/// `war view prime --prompt`: a prompt that names one open Warrant gets its
/// brief, unclaimed and within budget; one naming nothing, several, only done
/// work, a word that merely resembles a name, or a Warrant already held, gets
/// no brief; and `[tickets] session_start = false` is silent.
#[test]
fn a_prompt_gets_the_brief_of_the_item_it_names_and_never_a_guess() {
    let root = scratch("prompt");
    let make = |title: &str, body: &str, extra: &[&str]| {
        let mut args = vec!["create", title, "--body", body, "--item", "the one step"];
        args.extend_from_slice(extra);
        json(&root, &args)["result"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let t8 = make("T8: Monthly budget tables", "MARKER-T8 body.", &[]);
    let t9 = make("T9: Net worth over time", "MARKER-T9 body.", &[]);
    let long = make("Long plan", &"A line of a long plan.\n".repeat(400), &[]);
    let sha = make("Pin SHA256 digests", "MARKER-SHA body.", &[]);
    let text = |prompt: &str| {
        String::from_utf8_lossy(
            &war(&root, &["view", "prime", "--prompt", prompt, "--as", "ann"]).stdout,
        )
        .trim()
        .to_owned()
    };

    // By id, by capital code and by whole title: its brief, and no claim.
    for prompt in [
        format!("please do {t8} next"),
        "Do backlog item T8.".to_owned(),
        "Work on t8 monthly budget tables now".to_owned(),
        "the \"monthly budget\" one".to_owned(),
    ] {
        let said = text(&prompt);
        assert!(said.contains("MARKER-T8"), "{prompt}: {said}");
        assert!(said.contains("not claimed"), "{said}");
        assert!(said.contains(&format!("war start {t8}")), "{said}");
        assert!(!said.contains("MARKER-T9"), "{said}");
    }
    let held = json(&root, &["view", "prime", "--as", "ann"]);
    assert!(
        !serde_json::to_string(&held).unwrap().contains("claimed by"),
        "a prompt claimed something: {held}"
    );

    // Naming nothing: nothing, whatever it looks like.
    for prompt in [
        "fix the flaky parser test",
        "budget tables are slow",
        "use SHA256 here",
        "wear a t-shirt in T1000 weather",
        "t-1 i-5",
        "",
    ] {
        assert_eq!(text(prompt), "", "{prompt}");
    }

    // Several: one line, no brief, no guess.
    let said = text("T8 and T9 both");
    assert_eq!(said.lines().count(), 1, "{said}");
    assert!(said.contains(&t8) && said.contains(&t9), "{said}");
    assert!(!said.contains("MARKER"), "{said}");
    let said = text(&format!("{t8} and {t9}"));
    assert!(
        said.contains("none loaded") && !said.contains("MARKER"),
        "{said}"
    );
    assert!(said.len() <= 320, "{} bytes", said.len());

    // An id outranks a word; an unknown id is no match.
    assert!(text(&format!("{t9}, not T8")).contains("MARKER-T9"));
    assert_eq!(text("t-zzzzzz"), "");

    // Over budget: cut at a line, the cut declared, and how to get the rest.
    let said = text(&long);
    assert!(said.len() <= 6144, "{} bytes", said.len());
    assert!(said.contains("cut to 6144 bytes") && said.contains(&format!("war start {long}")));

    // From stdin, as the hook hands it over.
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(&root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .args(["view", "prime", "--prompt", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("take {sha}").as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("MARKER-SHA"));

    // Held: it has this brief already. Someone else's: said, with no offer.
    assert!(war(&root, &["start", &t9, "--as", "ann"]).status.success());
    assert_eq!(text("T9"), "");
    let other = String::from_utf8_lossy(
        &war(&root, &["view", "prime", "--prompt", "T9", "--as", "bob"]).stdout,
    )
    .into_owned();
    assert!(
        other.contains("claimed by ann") && !other.contains("Take it"),
        "{other}"
    );

    // Done work: nothing.
    assert!(
        war(&root, &["done", &t9, "--note", "ok", "--as", "ann"])
            .status
            .success()
    );
    assert_eq!(text("T9"), "");

    // `[tickets] session_start = false`: silent, on a real match too.
    let config = root.join("openwarrant.toml");
    let before = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        format!("{before}\n[tickets]\nsession_start = false\n"),
    )
    .unwrap();
    assert_eq!(text("T8"), "");
    std::fs::write(&config, before).unwrap();
    assert!(text("T8").contains("MARKER-T8"));
    let _ = std::fs::remove_dir_all(&root);
}

/// Every hook the plugin registers names a script that ships, and
/// UserPromptSubmit runs the first-brief hook: a script nobody registered
/// never fires, and a registered one that is gone fails every prompt.
#[test]
fn the_plugin_registers_the_prompt_hook_and_every_script_exists() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(repo.join(".claude/hooks/hooks.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut prompt_hook = false;
    for (event, groups) in config["hooks"].as_object().unwrap() {
        for group in groups.as_array().unwrap() {
            for hook in group["hooks"].as_array().unwrap() {
                let command = hook["command"].as_str().unwrap();
                let script = command
                    .split("${CLAUDE_PLUGIN_ROOT}/")
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .unwrap_or_else(|| panic!("{event}: {command}"));
                assert!(repo.join(script).is_file(), "{event}: no {script}");
                prompt_hook |=
                    event == "UserPromptSubmit" && script.ends_with("user-prompt-submit.sh");
            }
        }
    }
    assert!(prompt_hook, "UserPromptSubmit does not run the brief hook");
}

/// `war view prime --session` is on by default: the open Warrants and how
/// to take one, or, holding one, its brief, so a session needs no call to
/// find its work. `[tickets] session_start = false` silences it, so the
/// plugin's SessionStart hook adds nothing.
#[test]
fn the_session_start_view_is_on_unless_turned_off() {
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
    // Turned off: nothing at all.
    let config = root.join("openwarrant.toml");
    let before = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        format!("{before}\n[tickets]\nsession_start = false\n"),
    )
    .unwrap();
    assert_eq!(
        text(&["view", "prime", "--session", "--as", "ann"]).trim(),
        ""
    );
    std::fs::write(&config, before).unwrap();
    // On (the default), nothing held: the overview and how to take the next.
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

/// A request names the work ("Do backlog item T8"), not its id: `war start`
/// and the other ticket commands take words of a Warrant's title or labels,
/// so an agent needs no lookup first (the adoption study, 2026-10-29: two to
/// three turns a session went on finding the id). Words that fit several, or
/// none, are refused by name, never guessed.
#[test]
fn start_takes_a_warrant_named_by_words_of_its_title() {
    let root = scratch("start-words");
    let id_of = |v: serde_json::Value| v["result"]["id"].as_str().unwrap().to_owned();
    let t8 = id_of(json(
        &root,
        &["create", "T8: Monthly budget report", "--item", "a"],
    ));
    let t9 = id_of(json(
        &root,
        &["create", "T9: Yearly budget report", "--item", "b"],
    ));
    let out = war(&root, &["start", "T8", "--as", "ann"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let brief = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        brief.starts_with(&format!("# {t8} — T8: Monthly budget report")),
        "{brief}"
    );
    // Several words, in any order, each a whole word of the title.
    let shown = String::from_utf8_lossy(&war(&root, &["view", "prime", "report yearly"]).stdout)
        .into_owned();
    assert!(shown.starts_with(&format!("# {t9}")), "{shown}");
    // Fits two: refused, naming both. Fits none (`T1` is not `T10`): refused.
    let many = json(&root, &["start", "budget", "--as", "bob"]);
    assert_eq!(many["diagnostics"][0]["rule"], "ticket.ambiguous", "{many}");
    let message = many["diagnostics"][0]["message"].as_str().unwrap();
    assert!(message.contains(&t8) && message.contains(&t9), "{message}");
    let none = json(&root, &["start", "T1", "--as", "bob"]);
    assert_eq!(none["diagnostics"][0]["rule"], "ticket.unknown", "{none}");
    assert_ne!(none["exit_code"], 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// A value can be set back: each edit is its own fact, whoever makes it.
#[test]
fn an_edit_may_repeat_an_earlier_value() {
    let root = scratch("edit-repeat");
    let id = json(&root, &["create", "Job", "--item", "one"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    for (priority, actor) in [("1", "ann"), ("3", "ann"), ("1", "bob"), ("3", "ann")] {
        let edited = json(&root, &["edit", &id, "--priority", priority, "--as", actor]);
        assert_eq!(edited["exit_code"], 0, "{edited}");
    }
    let journal =
        std::fs::read_to_string(root.join("docs/tickets").join(&id).join("journal.jsonl")).unwrap();
    assert_eq!(journal.matches("\"work.edited\"").count(), 4, "{journal}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Agents side by side in linked worktrees share the repository's claims. Named
/// by nobody, each acts as the performer named for its worktree, so the second
/// takes the next Warrant instead of resuming the first one's as its own.
#[test]
fn agents_in_linked_worktrees_claim_under_their_own_names() {
    let root = scratch("tree-actor");
    for title in ["First job", "Second job"] {
        assert_eq!(
            json(&root, &["create", title, "--item", "x"])["exit_code"],
            0
        );
    }
    let git = |dir: &Path, args: &[&str]| {
        let ok = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@invalid"])
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "tickets"]);
    let trees: Vec<PathBuf> = ["a", "b"]
        .iter()
        .map(|n| {
            root.with_file_name(format!(
                "{}-{n}",
                root.file_name().unwrap().to_string_lossy()
            ))
        })
        .collect();
    for (n, tree) in trees.iter().enumerate() {
        git(
            &root,
            &[
                "worktree",
                "add",
                "-q",
                tree.to_str().unwrap(),
                "-b",
                &format!("agent{n}"),
            ],
        );
    }
    let take = |dir: &Path| {
        let v = json(dir, &["start"]);
        assert_eq!(v["exit_code"], 0, "{v}");
        v["result"]["ticket"].as_str().unwrap().to_owned()
    };
    let (first, second) = (take(&trees[0]), take(&trees[1]));
    assert_ne!(
        first, second,
        "the second agent resumed the first one's claim"
    );
    let claimed = String::from_utf8_lossy(&war(&trees[0], &["view", "prime"]).stdout).into_owned();
    let name = format!("claude@{}", trees[0].file_name().unwrap().to_string_lossy());
    assert!(claimed.contains(&name), "{claimed}");
    for tree in &trees {
        let _ = std::fs::remove_dir_all(tree);
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Starting work carries the complete context envelope and its Markdown
/// brief. Other replies stay terse. A
/// misnamed parameter is refused, never ignored: an ignored `ticket` once
/// made `war_start` take the next Warrant instead of the one named.
#[test]
fn mcp_ticket_tools_read_like_the_cli_and_refuse_a_misnamed_parameter() {
    let root = scratch("mcp-text");
    let id = json(&root, &["create", "T8: Monthly budget", "--item", "a"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut mcp = Mcp::start(&root);
    let call = |mcp: &mut Mcp, name: &str, args: serde_json::Value| {
        mcp.call(
            "tools/call",
            serde_json::json!({"name": name, "arguments": args}),
        )
    };
    let bad = call(&mut mcp, "war_start", serde_json::json!({"warrant": id}));
    assert!(
        bad.to_string().contains("unknown field") && bad.to_string().contains("target"),
        "{bad}"
    );
    assert!(
        json(&root, &["view", "prime"])["result"]["claims"]
            .as_array()
            .is_none_or(Vec::is_empty),
        "a refused call claimed something"
    );
    let started = call(&mut mcp, "war_start", serde_json::json!({"ticket": "T8"}));
    let text = started["result"]["content"][0]["text"].as_str().unwrap();
    let envelope = &started["result"]["structuredContent"];
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(text).unwrap(),
        *envelope
    );
    assert!(
        envelope["result"]["markdown"]
            .as_str()
            .unwrap()
            .starts_with(&format!("# {id} — T8: Monthly budget")),
        "{started}"
    );
    assert_eq!(
        envelope["result"]["context"]["schema"],
        "oh.war/task-context/v1"
    );
    let done = call(
        &mut mcp,
        "war_done",
        serde_json::json!({"id": id, "note": "did it"}),
    );
    let text = done["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.starts_with(&format!("done {id}")) && text.len() < 200,
        "{text}"
    );
    drop(mcp);
    let _ = std::fs::remove_dir_all(&root);
}

/// The most `war done` prints when it finishes a ticket and names the next
/// (titles of 60 characters): about 80 tokens.
const DONE_BUDGET: usize = 320;
/// The most bare `war status` prints with ready work named: about 160 tokens.
const STATUS_BUDGET: usize = 640;

/// What is next is said without asking, and inside a stated byte budget:
/// `war done` names the next ready ticket and nothing when none is, `war
/// status` names two ready tickets however many there are, `war view prime
/// --ready` is the ready rows alone, and `.openwarrant/next.md` follows
/// create, start and done without ever reading as a change to the checkout.
#[test]
fn done_and_status_name_what_is_next_inside_their_budgets() {
    let root = scratch("next");
    let said = |o: &Output| String::from_utf8_lossy(&o.stdout).into_owned();
    let next_md = root.join(".openwarrant/next.md");
    let dirt = || {
        let o = Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=all"])
            .current_dir(&root)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    assert!(!next_md.exists(), "nothing is tracked yet");
    let title = |n: usize| format!("Ticket {n} {}", "x".repeat(51));
    let ids: Vec<String> = (1..=4)
        .map(|n| {
            json(&root, &["create", &title(n)])["result"]["id"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    // Created: the ready rows, and not a change to the checkout.
    let ready = std::fs::read_to_string(&next_md).unwrap();
    assert!(
        ready.starts_with("OpenWarrant: nothing is held here.") && ready.contains(&ids[3]),
        "{ready}"
    );
    assert!(!dirt().contains("next.md"), "{}", dirt());

    let status = said(&war(&root, &["status"]));
    assert_eq!(status.matches("`war start t-").count(), 2, "{status}");
    assert!(
        status.len() <= STATUS_BUDGET,
        "{} bytes\n{status}",
        status.len()
    );
    let rows = said(&war(&root, &["view", "prime", "--ready"]));
    assert_eq!(rows.lines().count(), 4, "{rows}");
    assert!(
        rows.lines().all(|l| l.contains("`war start t-")) && !rows.contains("How to work"),
        "{rows}"
    );

    // Started: the brief of what this actor holds.
    let started = json(&root, &["start", "--as", "ann"]);
    let held = started["result"]["ticket"]
        .as_str()
        .or_else(|| started["result"]["id"].as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            ids.iter()
                .find(|id| started.to_string().contains(id.as_str()))
                .unwrap()
                .clone()
        });
    let brief = std::fs::read_to_string(&next_md).unwrap();
    assert!(
        brief.starts_with(&format!("OpenWarrant: ann holds {held};")) && brief.len() <= 6144,
        "{brief}"
    );
    assert!(!dirt().contains("next.md"), "{}", dirt());

    // Done: the next ready one, named with the command that takes it.
    let done = said(&war(&root, &["done", "--as", "ann", "--note", "did it"]));
    let next = done
        .lines()
        .find_map(|l| l.strip_prefix("next: "))
        .unwrap_or_else(|| panic!("no next line: {done}"));
    let next_id = next.split(' ').next().unwrap().to_owned();
    assert!(
        ids.contains(&next_id)
            && next_id != held
            && next.ends_with(&format!("(`war start {next_id}`)")),
        "{done}"
    );
    assert!(done.len() <= DONE_BUDGET, "{} bytes\n{done}", done.len());
    let after = std::fs::read_to_string(&next_md).unwrap();
    assert!(
        after.starts_with("OpenWarrant: nothing is held here."),
        "{after}"
    );

    // The rest, then the last: nothing is next, and the file is gone.
    let mut last = String::new();
    for _ in 0..3 {
        assert!(war(&root, &["start", "--as", "ann"]).status.success());
        last = said(&war(&root, &["done", "--as", "ann"]));
    }
    assert!(
        last.starts_with("done t-") && !last.contains("next:"),
        "{last}"
    );
    assert!(!next_md.exists(), "nothing is held or ready");
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
    assert_eq!(journal.matches("\"work.claimed\"").count(), 1, "{journal}");
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
                    declare_scope(&root, &t, &format!("work/{actor}/round-{n}/**"));
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
    let id = json(&root, &["create", "--legacy", "Checked", "--item", "a"])["result"]["id"]
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
        Self::start_with(root, &[])
    }

    /// The whole tool table (`war mcp --all`).
    fn start_all(root: &Path) -> Self {
        Self::start_with(root, &["--all"])
    }

    fn start_with(root: &Path, args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
            .arg("mcp")
            .args(args)
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

    /// One call, answered with its envelope (`json: true`).
    fn tool(&mut self, name: &str, mut args: serde_json::Value) -> serde_json::Value {
        args["json"] = serde_json::json!(true);
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
    let mut mcp = Mcp::start_all(&root);
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

// The concurrent workers declare disjoint payload scopes. Their tracker writes
// are serialized separately; a missing scope must never imply disjoint files.
fn declare_scope(root: &Path, id: &str, paths: &str) {
    let file = root.join(format!("docs/tickets/{id}.md"));
    let text = std::fs::read_to_string(&file).unwrap();
    let staged = file.with_extension("scope-tmp");
    std::fs::write(
        &staged,
        text.replacen("\n---\n", &format!("\npaths: {paths}\n---\n"), 1),
    )
    .unwrap();
    std::fs::rename(staged, file).unwrap();
}

#[test]
fn legacy_completion_migrated_master_keeps_identity_and_enforces_checks() {
    let root = scratch("migrated-master");
    let alias = "TK-WAR-0999";
    let state = root.join("docs/work").join(alias);
    std::fs::create_dir_all(&state).unwrap();
    let source = state.join(format!("{alias}-retained-identity.md"));
    std::fs::write(&source, format!("---\nformat: openwarrant/1.0\nprofile: ticket\nalias: {alias}\nuuid: 01a1b75a-8404-70a5-ac4f-a730d8f7110b\nsummary: Retained identity\n---\n# Retained identity\n\n## Checklist\n\n## CHK-result · check — required result\nrun: test -f passed\non: done\nevaluates {alias}\n")).unwrap();
    for _ in 0..2 {
        let started = json(&root, &["start", alias, "--as", "agent://test"]);
        assert_eq!(started["exit_code"], 0, "{started}");
    }
    let failed = json(&root, &["done", alias, "--check", "--as", "agent://test"]);
    assert!(
        failed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "check.failed"),
        "{failed}"
    );
    assert!(!state.join("resolution.toml").exists());
    std::fs::write(root.join("passed"), "ok").unwrap();
    let passed = json(&root, &["done", alias, "--check", "--as", "agent://test"]);
    assert_eq!(passed["exit_code"], 0, "{passed}");
    let history = std::fs::read_to_string(state.join("journal.jsonl")).unwrap();
    assert!(history.contains("work.item_done") && history.contains(alias));
    assert!(!root.join("docs/tickets").join(alias).exists());
    assert!(!state.join("resolution.toml").exists());
    assert!(
        std::fs::read_to_string(source)
            .unwrap()
            .contains("uuid: 01a1b75a-8404-70a5-ac4f-a730d8f7110b")
    );
    std::fs::remove_dir_all(root).unwrap();
}
