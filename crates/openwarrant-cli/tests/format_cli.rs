// SPDX-License-Identifier: Apache-2.0
//! Format 1.0 documents through the shipped binary
//! (docs/format/FORMAT-1.0.md; OW-ADR-0032). Each claim beside a refusal:
//!
//! - a single-file Warrant lowers to the same model as its directory
//!   equivalent (the fixture pair below): every record of the directory
//!   Warrant has the same `content_digest` from the single file, and the
//!   relations among them are the same; a changed meaning in the single
//!   file moves its digest away from the directory's;
//! - an unknown undotted key is refused by `format.key-unknown`; a dotted
//!   key is carried in `ext` and changes no verdict;
//! - a `requires` of an extension `war` does not understand, and a newer
//!   major, read `UNKNOWN` and add no record; the same document without
//!   them reads;
//! - `content_digest` holds under CRLF line endings and trailing blanks and
//!   moves when a field's text changes, while `revision` moves with bytes;
//! - `war new --format 1.0` writes a document `war check` accepts; any other
//!   version is refused.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-format-{tag}-{}-{}",
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
            "--program",
            "Format Test",
            "--namespace",
            "FT",
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

fn model(root: &Path) -> Value {
    let out = war(root, &["model", "--json"]);
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stderr)));
    v["result"].clone()
}

/// `war check --json`: (exit code, every (severity, rule, file)).
fn check(root: &Path) -> (i32, Vec<(String, String, String)>) {
    let out = war(root, &["--json", "check"]);
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut d: Vec<(String, String, String)> = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["severity"].as_str().unwrap_or_default().to_owned(),
                d["rule"].as_str().unwrap_or_default().to_owned(),
                d["file"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    d.sort();
    (out.status.code().unwrap_or(-1), d)
}

fn record<'a>(m: &'a Value, id: &str) -> Option<&'a Value> {
    m["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
}

/// Replace what follows an atom's frontmatter.
fn set_body(path: &Path, body: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    let end = text[3..].find("\n---\n").unwrap() + 3 + 5;
    std::fs::write(path, format!("{}{body}", &text[..end])).unwrap();
}

const TITLE: &str = "A bundle carries its output";

/// The directory Warrant FT-WAR-0002, in a fresh program: `war new`'s
/// manifest and atom headers, the bodies below. Returns its uuid.
fn directory_warrant(root: &Path) -> String {
    let out = war(root, &["new", TITLE]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dir = root.join("docs/warrants/FT-WAR-0002");
    let atoms = dir.join("atoms");
    set_body(
        &atoms.join("10-intent.md"),
        "\n# Intent\n\nThe blind verifier decides from the bundle alone.\n",
    );
    set_body(
        &atoms.join("20-basis.md"),
        "\n# Basis\n\nThe bundle format.\n",
    );
    set_body(
        &atoms.join("40-work-order.md"),
        "\n# Work order\n\nWrite the bundle writer.\n",
    );
    set_body(
        &atoms.join("60-assurance.md"),
        "\n# Assurance\n\n### OBL-001 — a bundled run carries its output\n\
         - **scope:** one bundled run.\n- **evidence:** the digest of the file matches.\n",
    );
    std::fs::write(
        atoms.join("45-milestones.yaml"),
        "schema: \"oh.war/milestones/v1\"\nmilestones:\n  - id: \"M-1\"\n    title: \"The bundle\"\n    \
         stage_refs: [\"STAGE-001\"]\n    obligation_refs: [\"OBL-001\"]\nstages:\n  - id: \"STAGE-001\"\n    \
         title: \"Write the writer\"\n    executor_kind: \"agent\"\n    responsibility_tier: \"T2\"\n",
    )
    .unwrap();
    let manifest = std::fs::read_to_string(dir.join("manifest.toml")).unwrap();
    let line = manifest.lines().find(|l| l.starts_with("uuid = ")).unwrap();
    line.trim_start_matches("uuid = ")
        .trim_matches('"')
        .to_owned()
}

/// The single-file equivalent of [`directory_warrant`].
fn single_file(uuid: &str, extra_frontmatter: &str, scope: &str) -> String {
    format!(
        "---\nformat: openwarrant/1.0\nprofile: delivery\nalias: FT-WAR-0002\nuuid: {uuid}\n\
summary: The bundle carries what each gate printed.\nassurance_level: basic\n{extra_frontmatter}---\n\
# {TITLE}\n\n## Intent\nThe blind verifier decides from the bundle alone.\n\n## Basis\n\
The bundle format.\n\n## Work order\nWrite the bundle writer.\n\n## Assurance\n\n\
## OBL-001 · obligation — a bundled run carries its output\nscope: {scope}\n\
evidence: the digest of the file matches.\npart_of M-1\n\n## M-1 · milestone — The bundle\n\n\
## STAGE-001 · stage — Write the writer\nexecutor: agent\ntier: T2\npart_of M-1\n"
    )
}

fn write_single(root: &Path, text: &str) -> PathBuf {
    let path = root.join("docs/warrants/FT-WAR-0002.md");
    std::fs::write(&path, text).unwrap();
    path
}

/// The fixture pair: the single file lowers to the directory Warrant's
/// model. Refused: a single file whose obligation scope says something else
/// is not the same record.
#[test]
fn a_single_file_warrant_lowers_to_its_directory_equivalent() {
    let a = scratch("dir");
    let uuid = directory_warrant(&a);
    let b = scratch("file");
    write_single(&b, &single_file(&uuid, "", "one bundled run."));
    let (ma, mb) = (model(&a), model(&b));

    let ids = [
        "FT-WAR-0002/S-intent",
        "FT-WAR-0002/S-basis",
        "FT-WAR-0002/S-work-order",
        "FT-WAR-0002/OBL-001",
        "FT-WAR-0002/M-1",
        "FT-WAR-0002/STAGE-001",
    ];
    for id in ids {
        let (ra, rb) = (
            record(&ma, id).unwrap_or_else(|| panic!("{id} not in the directory model")),
            record(&mb, id).unwrap_or_else(|| panic!("{id} not in the single-file model")),
        );
        for member in ["type", "title", "fields", "document", "content_digest"] {
            assert_eq!(ra[member], rb[member], "{id}: {member} differs");
        }
        assert!(
            ra["content_digest"]
                .as_str()
                .unwrap()
                .starts_with("sha256:")
        );
        // Bytes differ between the encodings; meaning does not.
        assert_ne!(ra["revision"], rb["revision"], "{id}");
        assert_ne!(ra["source"], rb["source"], "{id}");
    }
    // Every record the directory Warrant lowers to is in the single file's
    // model, and the relations among them are the same.
    let scoped = |m: &Value| -> Vec<String> {
        let mut v: Vec<String> = m["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["document"] == "FT-WAR-0002")
            .filter(|r| r["id"] != "FT-WAR-0002/S-assurance")
            .map(|r| r["id"].as_str().unwrap().to_owned())
            .collect();
        v.sort();
        v
    };
    assert_eq!(scoped(&ma), scoped(&mb));
    let edges = |m: &Value| -> Vec<String> {
        let mut v: Vec<String> = m["relations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["from"].as_str().unwrap().starts_with("FT-WAR-0002/")
                    && r["from"] != "FT-WAR-0002/S-assurance"
            })
            .map(|r| format!("{} {} {}", r["from"], r["kind"], r["to"]))
            .collect();
        v.sort();
        v
    };
    assert_eq!(edges(&ma), edges(&mb));
    assert!(
        edges(&mb).contains(&"\"FT-WAR-0002/OBL-001\" \"part_of\" \"FT-WAR-0002/M-1\"".to_owned())
    );
    // The document record: the same title and the same fields, but for the
    // `summary` a directory Warrant has no place for.
    let (wa, wb) = (
        record(&ma, "FT-WAR-0002").unwrap(),
        record(&mb, "FT-WAR-0002").unwrap(),
    );
    assert_eq!(wa["title"], wb["title"]);
    let mut fb = wb["fields"].clone();
    fb.as_object_mut().unwrap().remove("summary");
    assert_eq!(wa["fields"], fb);
    assert_eq!(mb["format"], "openwarrant/1.0");
    assert_eq!(ma["format"], "openwarrant/1.0");

    // Refused: another scope is another record.
    write_single(&b, &single_file(&uuid, "", "every bundled run."));
    let mc = model(&b);
    assert_ne!(
        record(&ma, "FT-WAR-0002/OBL-001").unwrap()["content_digest"],
        record(&mc, "FT-WAR-0002/OBL-001").unwrap()["content_digest"]
    );
    assert_eq!(
        record(&ma, "FT-WAR-0002/S-intent").unwrap()["content_digest"],
        record(&mc, "FT-WAR-0002/S-intent").unwrap()["content_digest"]
    );
    for r in [&a, &b] {
        std::fs::remove_dir_all(r).ok();
    }
}

/// An unknown undotted key is refused by rule and line; a dotted one is
/// carried in `ext` and the verdict is the same as without it.
#[test]
fn unknown_keys_are_refused_and_dotted_keys_carried() {
    let root = scratch("keys");
    let uuid = "01a0d342-ee5e-7b71-9623-f2d8cd5acd4e";
    write_single(&root, &single_file(uuid, "", "one run."));
    let (plain_code, plain) = check(&root);
    assert!(
        plain.iter().any(|d| d.1 == "format.well-formed"),
        "{plain:?}"
    );

    write_single(&root, &single_file(uuid, "sumary: a typo\n", "one run."));
    let (code, diags) = check(&root);
    assert_ne!(code, 0);
    assert!(
        diags.contains(&(
            "error".to_owned(),
            "format.key-unknown".to_owned(),
            "docs/warrants/FT-WAR-0002.md:8".to_owned()
        )),
        "{diags:?}"
    );
    assert!(record(&model(&root), "FT-WAR-0002").is_none());

    write_single(
        &root,
        &single_file(uuid, "x.note: carried\nkf.enterprise_id: E-7\n", "one run."),
    );
    let (dotted_code, dotted) = check(&root);
    assert_eq!(dotted_code, plain_code);
    // The same findings, by severity and rule; the lines below the two
    // added frontmatter lines move by two.
    let verdicts = |d: &[(String, String, String)]| -> Vec<(String, String)> {
        d.iter().map(|x| (x.0.clone(), x.1.clone())).collect()
    };
    assert_eq!(
        verdicts(&dotted),
        verdicts(&plain),
        "a dotted key changed the verdict"
    );
    let m = model(&root);
    let w = record(&m, "FT-WAR-0002").unwrap();
    assert_eq!(w["ext"]["x.note"], "carried");
    assert_eq!(w["ext"]["kf.enterprise_id"], "E-7");
    assert!(w["fields"].get("x.note").is_none());
    std::fs::remove_dir_all(&root).ok();
}

/// `requires` naming an extension `war` does not understand, and a newer
/// major, read UNKNOWN: never PASS, never a record.
#[test]
fn not_understood_reads_unknown() {
    let root = scratch("unknown");
    let uuid = "01a0d342-ee5e-7b71-9623-f2d8cd5acd4e";
    for (extra, rule) in [
        (
            "requires:\n  - com.example.risk\n",
            "format.requires-unknown",
        ),
        ("", "format.newer-major"),
    ] {
        let mut text = single_file(uuid, extra, "one run.");
        if rule == "format.newer-major" {
            text = text.replace("openwarrant/1.0", "openwarrant/2.0");
        }
        write_single(&root, &text);
        let (code, diags) = check(&root);
        assert_ne!(code, 0, "{rule}: UNKNOWN is not ready");
        assert!(
            diags.iter().any(|d| d.0 == "unknown" && d.1 == rule),
            "{rule}: {diags:?}"
        );
        assert!(
            !diags.iter().any(|d| d.1 == "format.well-formed"),
            "{rule}: a PASS was reported over an UNKNOWN document"
        );
        assert!(record(&model(&root), "FT-WAR-0002").is_none(), "{rule}");
    }
    // The same document without them is read.
    write_single(&root, &single_file(uuid, "", "one run."));
    assert!(record(&model(&root), "FT-WAR-0002/OBL-001").is_some());
    std::fs::remove_dir_all(&root).ok();
}

/// content_digest is over meaning: CRLF endings and trailing blanks move
/// every revision and no content_digest; a field's text moves its record's.
#[test]
fn content_digest_holds_over_bytes_and_moves_with_meaning() {
    let root = scratch("digest");
    let uuid = "01a0d342-ee5e-7b71-9623-f2d8cd5acd4e";
    let text = single_file(uuid, "", "one run.");
    write_single(&root, &text);
    let base = model(&root);
    let digests = |m: &Value, member: &str| -> Vec<(String, String)> {
        m["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["source"] == "docs/warrants/FT-WAR-0002.md")
            .map(|r| {
                (
                    r["id"].as_str().unwrap().to_owned(),
                    r[member].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let crlf = text
        .replace('\n', "\r\n")
        .replace("alone.\r\n", "alone.  \r\n\r\n\r\n");
    write_single(&root, &crlf);
    let moved = model(&root);
    assert_eq!(
        digests(&base, "content_digest"),
        digests(&moved, "content_digest")
    );
    let (rb, rm) = (digests(&base, "revision"), digests(&moved, "revision"));
    assert!(
        rb.iter().zip(&rm).all(|(x, y)| x.1 != y.1),
        "a revision held over new bytes"
    );

    write_single(&root, &single_file(uuid, "", "two runs."));
    let changed = model(&root);
    let (cb, cc) = (
        digests(&base, "content_digest"),
        digests(&changed, "content_digest"),
    );
    for (x, y) in cb.iter().zip(&cc) {
        if x.0 == "FT-WAR-0002/OBL-001" {
            assert_ne!(x.1, y.1, "a changed scope kept its content_digest");
        } else {
            assert_eq!(x.1, y.1, "{} moved with another record's meaning", x.0);
        }
    }
    std::fs::remove_dir_all(&root).ok();
}

/// `war new --format 1.0` writes one file that `war check` reads with no
/// refusal; `--format 2.0` is refused and writes nothing.
#[test]
fn new_writes_a_single_file_warrant() {
    let root = scratch("new");
    let out = war(
        &root,
        &[
            "new",
            "A single-file Warrant: the writer",
            "--format",
            "1.0",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let path = root.join("docs/warrants/FT-WAR-0002.md");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.starts_with("---\nformat: openwarrant/1.0\nprofile: delivery\nalias: FT-WAR-0002\n")
    );
    let (_, diags) = check(&root);
    assert!(
        diags.iter().any(|d| d.1 == "format.well-formed"),
        "{diags:?}"
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.0 == "error" && d.1.starts_with("format."))
    );
    let m = model(&root);
    assert_eq!(
        record(&m, "FT-WAR-0002").unwrap()["title"],
        "A single-file Warrant: the writer"
    );
    assert!(record(&m, "FT-WAR-0002/OBL-001").is_some());

    let out = war(&root, &["new", "Another", "--format", "2.0"]);
    assert!(!out.status.success());
    assert!(!root.join("docs/warrants/FT-WAR-0003.md").exists());
    std::fs::remove_dir_all(&root).ok();
}
