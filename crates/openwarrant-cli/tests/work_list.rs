// SPDX-License-Identifier: Apache-2.0
//! The one list of work (t-3c16c W1.1), through its interface: every encoding
//! comes back as one kind of item, and a level is what the item's fields make
//! it. Each claim is paired with the case that must not satisfy it.
use std::path::Path;
use std::process::Command;

use camino::Utf8PathBuf;
use openwarrant_cli::repo::Repository;
use openwarrant_cli::work::{Encoding, Item, Level, LevelCounts, Work};

fn war_raw(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap()])
        .args(args)
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .output()
        .unwrap()
}

fn war(root: &Path, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = war_raw(root, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "{args:?}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn human(root: &Path, args: &[&str]) -> String {
    String::from_utf8(war_raw(root, args).stdout).unwrap()
}

fn repo() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "NX"])["exit_code"],
        0
    );
    root
}

fn create(root: &Path, title: &str, extra: &[&str]) -> String {
    let mut args = vec!["create", title];
    args.extend_from_slice(extra);
    let created = war(root, &args);
    assert_eq!(created["exit_code"], 0, "{created}");
    created["result"]["id"].as_str().unwrap().to_owned()
}

fn list(root: &Path) -> Work {
    let path = Utf8PathBuf::from_path_buf(root.canonicalize().unwrap()).unwrap();
    Work::read(&Repository::open(path).unwrap())
}

fn item(work: &Work, id: &str) -> Item {
    work.items()
        .iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("{id} is not listed: {:?}", work.items()))
        .clone()
}

fn level(root: &Path, id: &str) -> Level {
    item(&list(root), id).level
}

/// A single-file Warrant of `profile`, carrying `tail`.
fn warrant_document(root: &Path, alias: &str, profile: &str, tail: &str) {
    let uuid = "01a1aea7-a3fc-76f2-862d-7f8f8ffed201";
    let text = format!(
        "---\nformat: openwarrant/1.0\nprofile: {profile}\nalias: {alias}\nuuid: {uuid}\n\
         summary: A document Warrant.\n---\n\
         # A document Warrant\n\n## Intent\nWhy.\n\n## Basis\nWhat.\n\n## Work order\nThe work.\n\n\
         ## Assurance\n## OBL-001 · obligation — it works\nscope: this.\n\
         evidence: a run.\nrefusal: a planted failure.\n{tail}"
    );
    std::fs::write(root.join(format!("docs/warrants/{alias}.md")), text).unwrap();
}

fn adapter(root: &Path) {
    let change = root.join("openspec/changes/add-2fa");
    std::fs::create_dir_all(&change).unwrap();
    std::fs::write(change.join("proposal.md"), "# Add 2FA\n\nWhy it matters.\n").unwrap();
    std::fs::write(change.join("tasks.md"), "- [x] 1.1 One\n- [ ] 1.2 Two\n").unwrap();
    let config = root.join("openwarrant.toml");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text.push_str("\n[[adapters]]\nkind = \"openspec\"\npath = \"openspec\"\n");
    std::fs::write(config, text).unwrap();
}

#[test]
fn a_ticket_is_a_ticket_and_adding_a_word_about_checks_does_not_change_that() {
    let root = repo();
    let plain = create(root.path(), "A plain piece of work", &["--item", "Do it"]);
    let wordy = create(
        root.path(),
        "Add a test, a gate and a signature",
        &["--item", "signed off by a person, tests pass"],
    );
    assert_eq!(level(root.path(), &plain), Level::Ticket);
    assert_eq!(level(root.path(), &wordy), Level::Ticket);
}

#[test]
fn adding_a_trigger_makes_a_warrant_in_place_and_stores_no_level() {
    let root = repo();
    let id = create(root.path(), "Ship it", &[]);
    assert_eq!(level(root.path(), &id), Level::Ticket);
    let added = war(root.path(), &["plan", "add", &id, "--test", "true"]);
    assert_eq!(added["exit_code"], 0, "{added}");
    assert_eq!(level(root.path(), &id), Level::Warrant);
    let document = std::fs::read_to_string(root.path().join(format!("docs/tickets/{id}.md")));
    assert!(!document.unwrap().contains("level"), "a level was stored");
}

#[test]
fn a_test_given_at_creation_makes_a_warrant_on_either_ticket_encoding() {
    let root = repo();
    let single = create(root.path(), "One file", &["--test", "true"]);
    let legacy = create(root.path(), "A directory", &["--legacy"]);
    assert_eq!(level(root.path(), &legacy), Level::Ticket);
    assert_eq!(
        war(root.path(), &["plan", "add", &legacy, "--test", "true"])["exit_code"],
        0
    );
    let work = list(root.path());
    assert_eq!(item(&work, &single).level, Level::Warrant);
    assert_eq!(item(&work, &single).encoding, Encoding::SingleFile);
    assert_eq!(item(&work, &legacy).level, Level::Warrant);
    assert_eq!(item(&work, &legacy).encoding, Encoding::Light);
}

#[test]
fn a_measurement_that_decides_nothing_is_no_trigger() {
    let root = repo();
    let id = create(root.path(), "Watch it", &[]);
    let args = [
        "plan",
        "add",
        &id,
        "--kpi",
        "speed",
        "--cmd",
        "echo 1",
        "--direction",
        "max",
        "--mode",
        "optimise",
    ];
    let added = war(root.path(), &args);
    assert_eq!(added["exit_code"], 0, "{added}");
    assert_eq!(level(root.path(), &id), Level::Ticket);
}

#[test]
fn a_required_signature_makes_an_order() {
    let root = repo();
    let id = create(root.path(), "Sign it", &[]);
    let args = [
        "plan",
        "add",
        &id,
        "--milestone",
        "Accepted",
        "--min",
        "signed",
    ];
    let added = war(root.path(), &args);
    assert_eq!(added["exit_code"], 0, "{added}");
    assert_eq!(level(root.path(), &id), Level::Order);
}

#[test]
fn a_warrant_without_a_signature_cannot_read_as_an_order() {
    let root = repo();
    let tested = create(
        root.path(),
        "Signature required, says the title",
        &["--test", "true"],
    );
    let observed = create(root.path(), "Observed only", &[]);
    let args = [
        "plan",
        "add",
        &observed,
        "--milestone",
        "Seen",
        "--min",
        "observed",
    ];
    assert_eq!(war(root.path(), &args)["exit_code"], 0);
    let args = ["plan", "add", &observed, "--test", "true"];
    assert_eq!(war(root.path(), &args)["exit_code"], 0);
    assert_eq!(level(root.path(), &tested), Level::Warrant);
    assert_eq!(level(root.path(), &observed), Level::Warrant);
}

#[test]
fn a_directory_warrant_stays_readable_and_is_an_order_by_its_profile() {
    let root = repo();
    let new = war(
        root.path(),
        &["plan", "new", "--format", "legacy", "A delivery Warrant"],
    );
    assert_eq!(new["exit_code"], 0, "{new}");
    let work = list(root.path());
    let order = work
        .items()
        .iter()
        .find(|i| i.encoding == Encoding::Directory)
        .unwrap();
    assert_eq!(order.level, Level::Order);
    assert_eq!(order.title, "A delivery Warrant");
    assert_eq!(order.state, "draft");
    assert_eq!(order.place, format!("docs/warrants/{}", order.id));
}

#[test]
fn a_single_file_warrant_is_listed_and_a_profile_decides_its_level() {
    let root = repo();
    warrant_document(root.path(), "NX-WAR-0001", "delivery", "");
    let work = list(root.path());
    let order = item(&work, "NX-WAR-0001");
    assert_eq!(order.level, Level::Order);
    assert_eq!(order.encoding, Encoding::SingleFile);
    assert_eq!(order.state, "draft");
    assert_eq!(order.place, "docs/warrants/NX-WAR-0001.md");
    assert!(human(root.path(), &["view", "tickets"]).contains("NX-WAR-0001"));
}

#[test]
fn a_document_of_a_profile_that_cannot_be_found_has_no_level_to_guess() {
    let root = repo();
    warrant_document(root.path(), "NX-WAR-0002", "no-such-profile", "");
    let work = list(root.path());
    assert_eq!(
        work.items()
            .iter()
            .find(|i| i.id == "NX-WAR-0002")
            .map(|i| i.level),
        Some(Level::Unknown)
    );
    assert_eq!(LevelCounts::open(work.items()).unknown, 1);
}

#[test]
fn a_refused_document_is_said_and_never_listed_under_a_guess() {
    let root = repo();
    warrant_document(root.path(), "NX-WAR-0003", "delivery", "");
    let path = root.path().join("docs/warrants/NX-WAR-0003.md");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("summary: A document Warrant.\n", "")).unwrap();
    let work = list(root.path());
    assert!(work.items().iter().all(|i| i.id != "NX-WAR-0003"));
    let fault = work
        .other_faults()
        .iter()
        .find(|f| f.rule == "warrants.document-unreadable")
        .expect("a refused document is said");
    assert!(fault.message.contains("summary"), "{}", fault.message);
}

#[test]
fn an_item_read_in_place_declares_neither_so_it_is_a_ticket() {
    let root = repo();
    adapter(root.path());
    let work = list(root.path());
    let read = item(&work, "openspec:add-2fa");
    assert_eq!(read.level, Level::Ticket);
    assert_eq!(read.encoding, Encoding::ReadInPlace);
    assert_eq!(read.state, "in_progress");
    assert!(
        read.place.starts_with("openspec/changes/add-2fa"),
        "{}",
        read.place
    );
}

#[test]
fn every_encoding_comes_back_from_one_read() {
    let root = repo();
    create(root.path(), "One file", &[]);
    create(root.path(), "A directory ticket", &["--legacy"]);
    war(
        root.path(),
        &["plan", "new", "--format", "legacy", "A delivery Warrant"],
    );
    warrant_document(root.path(), "NX-WAR-0009", "delivery", "");
    adapter(root.path());
    let work = list(root.path());
    for encoding in [
        Encoding::SingleFile,
        Encoding::Light,
        Encoding::Directory,
        Encoding::ReadInPlace,
    ] {
        assert!(
            work.items().iter().any(|i| i.encoding == encoding),
            "{encoding:?} is missing: {:?}",
            work.items()
        );
    }
    assert_eq!(work.items().len(), 5);
    assert!(work.complete());
}

#[test]
fn a_checks_atom_that_cannot_be_read_leaves_the_level_unknown_not_ticket() {
    let root = repo();
    let id = create(root.path(), "Unreadable parts", &["--legacy"]);
    let atom = root
        .path()
        .join(format!("docs/tickets/{id}/atoms/30-checks.md"));
    std::fs::create_dir_all(atom.parent().unwrap()).unwrap();
    std::fs::create_dir(&atom).unwrap();
    assert_eq!(level(root.path(), &id), Level::Unknown);
}

#[test]
fn prime_and_the_terse_status_show_a_level_only_where_it_is_not_the_default() {
    let root = repo();
    let plain = create(root.path(), "Plain", &[]);
    let tested = create(root.path(), "Tested", &["--test", "true"]);
    let prime = human(root.path(), &["view", "prime"]);
    let line = |id: &str| prime.lines().find(|l| l.contains(id)).unwrap().to_owned();
    assert!(line(&tested).contains(" · Warrant"), "{prime}");
    assert!(
        line(&plain).ends_with("Plain · 0/0 done · p2"),
        "{}",
        line(&plain)
    );
    assert!(!prime.contains("Also open elsewhere"), "{prime}");
    assert!(human(root.path(), &["status"]).contains("Open by level: 1 Ticket, 1 Warrant\n"));

    war(
        root.path(),
        &["plan", "new", "--format", "legacy", "A delivery Warrant"],
    );
    let prime = human(root.path(), &["view", "prime"]);
    assert!(
        prime.contains("Also open elsewhere: 1 Order (`war view tickets`)"),
        "{prime}"
    );
    assert!(
        human(root.path(), &["status"]).contains("Open by level: 1 Ticket, 1 Warrant, 1 Order\n")
    );

    let one = human(root.path(), &["view", "prime", &tested]);
    assert!(one.contains("open · Warrant · 0/0 done"), "{one}");
}

#[cfg(unix)]
#[test]
fn work_that_could_not_be_read_is_a_question_mark_never_an_empty_list() {
    use std::os::unix::fs::PermissionsExt;
    let root = repo();
    let id = create(root.path(), "Plain", &[]);
    war(
        root.path(),
        &["plan", "new", "--format", "legacy", "A delivery Warrant"],
    );
    let warrants = root.path().join("docs/warrants");
    let mode = |bits| std::fs::Permissions::from_mode(bits);
    std::fs::set_permissions(&warrants, mode(0o000)).unwrap();
    let prime = war_raw(root.path(), &["view", "prime"]);
    let status = war_raw(root.path(), &["status"]);
    let tickets = war_raw(root.path(), &["view", "tickets"]);
    std::fs::set_permissions(&warrants, mode(0o755)).unwrap();

    let said = |out: &std::process::Output| String::from_utf8_lossy(&out.stdout).into_owned();
    let warned = |out: &std::process::Output| String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(said(&prime).contains(&id), "{prime:?}");
    assert!(said(&prime).contains("Also open elsewhere: ?"), "{prime:?}");
    assert!(
        warned(&prime).contains("UNKNOWN (work.unreadable)"),
        "{prime:?}"
    );
    assert!(said(&status).contains("Open by level: ?\n"), "{status:?}");
    assert_ne!(status.status.code(), Some(0), "{status:?}");
    assert!(said(&tickets).contains(&id), "{tickets:?}");
    assert!(
        warned(&tickets).contains("UNKNOWN (work.unreadable)"),
        "{tickets:?}"
    );
}

#[test]
fn the_json_list_carries_each_rows_level() {
    let root = repo();
    create(root.path(), "Tested", &["--test", "true"]);
    war(
        root.path(),
        &["plan", "new", "--format", "legacy", "A delivery Warrant"],
    );
    let listed = war(root.path(), &["view", "tickets"]);
    assert_eq!(listed["result"]["tickets"][0]["level"], "warrant");
    assert_eq!(listed["result"]["warrants"][0]["level"], "order");
}

#[test]
fn promotion_adds_a_field_in_place_and_writes_no_new_document() {
    let tmp = repo();
    let root = tmp.path();
    let id = create(root, "Plain work", &["--item", "one"]);
    assert_eq!(level(root, &id), Level::Ticket);
    let warrants = || {
        std::fs::read_dir(root.join("docs/warrants"))
            .unwrap()
            .count()
    };
    let before = warrants();

    let raised = war(root, &["plan", "promote", &id, "--test", "true"]);
    assert_eq!(raised["result"]["warrant"], id.as_str());
    assert_eq!(raised["result"]["level"], "warrant");
    assert_eq!(level(root, &id), Level::Warrant);

    let raised = war(root, &["plan", "promote", &id]);
    assert_eq!(raised["result"]["level"], "order");
    assert_eq!(level(root, &id), Level::Order);
    assert_eq!(warrants(), before, "a promotion wrote a new document");

    // The refusal: it is an Order already, and nothing moves.
    let bytes = std::fs::read(root.join(format!("docs/tickets/{id}.md"))).unwrap();
    let again = war_raw(root, &["plan", "promote", &id]);
    assert_eq!(again.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&again.stderr).contains("ticket.already-promoted"));
    assert_eq!(
        std::fs::read(root.join(format!("docs/tickets/{id}.md"))).unwrap(),
        bytes
    );
}

#[test]
fn create_makes_every_level_as_one_document() {
    let tmp = repo();
    let root = tmp.path();
    let order = create(root, "Needs a person", &["--sign"]);
    let both = create(root, "Both", &["--test", "true", "--sign"]);
    let warrant = create(root, "Tested", &["--test", "true"]);
    assert_eq!(level(root, &order), Level::Order);
    assert_eq!(level(root, &both), Level::Order);
    assert_eq!(level(root, &warrant), Level::Warrant);
    for id in [&order, &both, &warrant] {
        assert!(root.join(format!("docs/tickets/{id}.md")).is_file());
        assert!(
            !root
                .join(format!("docs/tickets/{id}/manifest.toml"))
                .exists()
        );
    }
    // An agent's `war done` hands an Order over and does not finish it.
    let done = war(root, &["done", &order, "--note", "no"]);
    assert_eq!(done["exit_code"], 0, "{done}");
    assert_eq!(done["result"]["ticket_state"], "awaiting_signature");
    let shown = war(root, &["view", "show", &order]);
    assert_eq!(shown["result"]["ticket"]["state"], "open", "{shown}");
}

#[test]
fn a_directory_warrant_can_still_be_drafted_from_a_ticket() {
    let tmp = repo();
    let root = tmp.path();
    let id = create(root, "Contract", &["--item", "one"]);
    let made = war(root, &["plan", "new", "--from", &id]);
    let alias = made["result"]["warrant"].as_str().unwrap().to_owned();
    assert!(
        root.join(format!("docs/warrants/{alias}/manifest.toml"))
            .is_file()
    );
    let again = war_raw(root, &["plan", "promote", &id]);
    assert_eq!(
        again.status.code(),
        Some(2),
        "promoted into a Warrant already"
    );
}

#[test]
fn check_status_next_and_context_all_find_a_single_file_warrant() {
    let tmp = repo();
    let root = tmp.path();
    assert!(
        war_raw(root, &["plan", "new", "--format", "1.0", "One document"])
            .status
            .success()
    );
    let alias = war(root, &["view", "warrants"])["result"]["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["encoding"] == "single_file")
        .expect("listed")["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let checked = war_raw(root, &["check", &alias]);
    assert!(checked.status.success(), "{checked:?}");

    // docs/design/one-format.md: a single-file Warrant is in the corpus, so
    // its status is any Warrant's, read through the one loader.
    let status = war(root, &["status", &alias]);
    assert_eq!(status["exit_code"], 0, "{status}");
    assert_eq!(status["result"]["alias"], alias.as_str());
    assert!(human(root, &["view", "show", &alias]).contains("## Intent"));

    // docs/design/one-format.md: a single-file Warrant is in the corpus, so
    // `next` lists its acts with every Warrant's, never again under `elsewhere`.
    let next = war(root, &["view", "next"]);
    assert!(next["result"].get("elsewhere").is_none(), "{next}");
    assert!(
        next.to_string().contains(&format!("war sign {alias}")),
        "{next}"
    );
    assert!(human(root, &["view", "next"]).contains(&alias));

    let context = war(root, &["view", "context", &alias]);
    assert_eq!(context["exit_code"], 0, "{context}");

    // The refusal: a document that does not read is said, by its rule.
    let path = root.join(format!("docs/warrants/{alias}.md"));
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("## Intent", "## Not the intent")).unwrap();
    let refused = war_raw(root, &["check", &alias]);
    assert_ne!(refused.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&refused.stdout).contains("format.section-missing"));
}

#[test]
fn nothing_else_open_keeps_the_next_envelope_as_it_was() {
    let tmp = repo();
    let next = war(tmp.path(), &["view", "next"]);
    assert!(next["result"].get("elsewhere").is_none());
}
