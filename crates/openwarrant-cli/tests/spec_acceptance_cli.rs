// SPDX-License-Identifier: Apache-2.0
//! The specification type through the shipped binary (FORMAT-1.0 §4.5,
//! §14, Appendix C.2 and C.3): acceptance held per record; an edited record
//! lapsing only itself; `war impact`, `war spec status` and `war next`
//! naming what is waiting; a signed accept believed, and an unsigned, a
//! stale, a tampered or an agent's refused; `war spec migrate`; parts
//! outside `spec/`.
//!
//! Signed acts here are made by a throwaway key generated in a scratch
//! directory, with `ssh-keygen -Y sign -f <private key>` and no agent in
//! the environment, the way `ssh_verdicts.rs` makes them. The corpus's
//! unsigned acts (`OPENWARRANT_CORPUS_ACTS`) are a debug build's only, and
//! these tests run one.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SPEC: &str = "---
format: openwarrant/1.0
profile: spec
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01
version: 0.3.0
summary: Password reset for the account service.
---
# Account service

## Purpose
Users who forget a password regain access without support staff.

## Scope
In: reset by email link. Out: SMS, account recovery without email.

## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once and only within 30
  minutes of issue.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.

## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
";

const TICKET: &str = "---
format: openwarrant/1.0
profile: ticket
alias: t-0d61d
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4f
summary: Send the password reset email.
implements: REQ-reset-1
---
# Reset email

## Checklist
- [ ] Email template (i-4d11)
";

const WARRANT: &str = "---
format: openwarrant/1.0
profile: delivery
alias: SA-WAR-0146
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e
summary: A reset link is refused once used.
---
# A reset link is refused once used

## Intent
A reused link is refused.

## Basis
The reset requirement.

## Work order
Deliverables: the link check.

## Assurance
## OBL-001 · obligation — a consumed link is refused
scope: the reset endpoint, one consumed link.
evaluates REQ-reset-1
evidence: the second use answers 410.
refusal: a fresh link is accepted.
";

const ROLES: &str = "[[assignment]]
actor = \"alice\"
actor_kind = \"human\"
roles = [\"authorizer\", \"resolver\"]
assigned_by = \"alice\"
effective_time = \"2026-01-01T00:00:00Z\"
ssh_principal = \"alice@example.test\"

[[assignment]]
actor = \"claude\"
actor_kind = \"agent\"
roles = [\"performer\"]
assigned_by = \"alice\"
effective_time = \"2026-01-01T00:00:00Z\"
";

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-spec-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "."])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let out = war(&root, &["init", "--namespace", "SA"], &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    root
}

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// The repository: the specification, a ticket implementing REQ-reset-1,
/// a Warrant whose obligation evaluates it, and a register.
fn repo(tag: &str) -> PathBuf {
    let root = scratch(tag);
    write(&root, "SPEC.md", SPEC);
    write(&root, "docs/tickets/t-0d61d.md", TICKET);
    write(&root, "docs/warrants/SA-WAR-0146.md", WARRANT);
    write(&root, "docs/authority/roles.toml", ROLES);
    root
}

fn war(root: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_war"));
    cmd.current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("OPENWARRANT_CORPUS_ACTS")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_AGENT_PID")
        .args(args)
        .arg("--root")
        .arg(root);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn json(root: &Path, args: &[&str], env: &[(&str, &str)]) -> serde_json::Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = war(root, &all, env);
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

/// `(record, value, provenance)` of each `acceptance` state, sorted.
fn acceptance(root: &Path, env: &[(&str, &str)]) -> Vec<(String, String, String)> {
    let m = json(root, &["model"], env);
    let mut out: Vec<_> = m["result"]["states"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["kind"] == "acceptance")
        .map(|s| {
            (
                s["record"].as_str().unwrap().to_owned(),
                s["value"].as_str().unwrap().to_owned(),
                s["provenance"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    out.sort();
    out
}

fn st(record: &str, value: &str, provenance: &str) -> (String, String, String) {
    (record.into(), value.into(), provenance.into())
}

/// The request `war spec accept` emits.
fn request(root: &Path, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["spec", "accept"];
    all.extend_from_slice(args);
    let v = json(root, &all, &[]);
    v["result"].clone()
}

/// A corpus act over the request's records, as `acts/*.json` holds one.
fn corpus_act(dir: &Path, name: &str, req: &serde_json::Value) {
    fs::create_dir_all(dir).unwrap();
    let act = serde_json::json!({"act": "accept", "records": req["records"]});
    fs::write(dir.join(name), serde_json::to_vec(&act).unwrap()).unwrap();
}

fn edit_requirement(root: &Path) {
    let text = fs::read_to_string(root.join("SPEC.md")).unwrap();
    fs::write(
        root.join("SPEC.md"),
        text.replace("30\n  minutes of issue.", "15\n  minutes of issue."),
    )
    .unwrap();
}

#[test]
fn an_edited_record_lapses_only_itself_and_names_its_citers() {
    let root = repo("lapse");
    let acts = root.join("corpus-acts");
    let env_acts = acts.to_str().unwrap().to_owned();
    let env = [("OPENWARRANT_CORPUS_ACTS", env_acts.as_str())];
    // No act: no acceptance state at all (§4.5), and the acceptance waits
    // on a person in `war next`.
    assert!(acceptance(&root, &env).is_empty());
    let next = json(&root, &["next"], &[]);
    let waiting: Vec<&serde_json::Value> = next["result"]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["command"] == "war sign spec")
        .collect();
    assert_eq!(waiting.len(), 1, "{next}");
    assert_eq!(waiting[0]["actor"], "human");

    let first = request(&root, &[]);
    assert_eq!(first["records"].as_array().unwrap().len(), 4, "{first}");
    corpus_act(&acts, "accept-1.json", &first);
    assert_eq!(
        acceptance(&root, &env),
        [
            st("CON-1", "accepted", "recorded"),
            st("REQ-reset-1", "accepted", "recorded"),
            st("S-purpose", "accepted", "recorded"),
            st("S-scope", "accepted", "recorded"),
            st("doc:SPEC.md", "accepted", "computed"),
        ]
    );

    edit_requirement(&root);
    assert_eq!(
        acceptance(&root, &env),
        [
            st("CON-1", "accepted", "recorded"),
            st("S-purpose", "accepted", "recorded"),
            st("S-scope", "accepted", "recorded"),
            st("doc:SPEC.md", "partially_accepted", "computed"),
        ],
        "the edit lapses REQ-reset-1 alone"
    );

    // `war impact` names the work and the obligation citing it.
    let impact = json(&root, &["impact", "REQ-reset-1"], &env);
    let warn = impact["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["rule"] == "impact.unaccepted")
        .unwrap_or_else(|| panic!("{impact}"));
    let msg = warn["message"].as_str().unwrap();
    assert!(msg.contains("edited after the act"), "{msg}");
    assert!(msg.contains("t-0d61d"), "{msg}");
    assert!(msg.contains("SA-WAR-0146/OBL-001"), "{msg}");
    // An accepted record carries no such warning.
    let impact = json(&root, &["impact", "CON-1"], &env);
    assert!(!rules(&impact).contains(&"impact.unaccepted".to_owned()));

    // `war spec status` says the same, record by record.
    let status = json(&root, &["spec", "status"], &env);
    let r = &status["result"];
    assert_eq!(r["document"], "partially_accepted");
    assert_eq!(r["accepted"], 3);
    let u = &r["unaccepted"][0];
    assert_eq!(u["record"], "REQ-reset-1");
    assert_eq!(u["lapsed"], true);
    assert!(
        u["cited_by"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "SA-WAR-0146/OBL-001")
    );

    // The request names the changed record only, and an accept over it
    // restores the specification.
    let out = war(&root, &["--json", "spec", "accept"], &env);
    let again: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let again = &again["result"];
    let ids: Vec<&str> = again["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c[0].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["REQ-reset-1"]);
    assert_eq!(again["changed"], serde_json::json!(["REQ-reset-1"]));
    corpus_act(&acts, "accept-2.json", again);
    assert!(
        acceptance(&root, &env).contains(&st("doc:SPEC.md", "accepted", "computed")),
        "a later accept naming the changed record restores it"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_act_no_acceptance_and_none_waits_without_the_authority_profile() {
    let root = scratch("no-authority");
    write(&root, "SPEC.md", SPEC);
    assert!(acceptance(&root, &[]).is_empty());
    let next = json(&root, &["next"], &[]);
    assert!(
        !next.to_string().contains("war sign spec"),
        "without docs/authority/roles.toml nothing asks for an accept: {next}"
    );
    let impact = json(&root, &["impact", "REQ-reset-1"], &[]);
    assert!(!rules(&impact).contains(&"impact.unaccepted".to_owned()));
    let _ = fs::remove_dir_all(&root);
}

// ---- signed acts ----------------------------------------------------------

#[cfg(unix)]
fn ssh_keygen(dir: &Path, args: &[&str]) -> bool {
    Command::new("ssh-keygen")
        .args(args)
        .current_dir(dir)
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_AGENT_PID")
        .stdin(std::process::Stdio::null())
        .output()
        .is_ok_and(|o| o.status.success())
}

/// A throwaway key under `<root>/.keys`, bound to alice in
/// allowed_signers. `None` when ssh-keygen is not installed.
#[cfg(unix)]
fn throwaway_key(root: &Path) -> Option<PathBuf> {
    let keys = root.join(".keys");
    fs::create_dir_all(&keys).unwrap();
    if !ssh_keygen(
        &keys,
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "throwaway",
            "-f",
            "k",
        ],
    ) {
        return None;
    }
    let public = fs::read_to_string(keys.join("k.pub")).unwrap();
    write(
        root,
        "docs/authority/allowed_signers",
        &format!(
            "alice@example.test namespaces=\"oh.war/response\" {}",
            public.trim()
        ),
    );
    Some(keys.join("k"))
}

/// The response a person would sign for `req`, written where `war sign`
/// writes it; with `key`, signed beside it.
#[cfg(unix)]
fn response(root: &Path, req: &serde_json::Value, actor: &str, key: Option<&Path>) -> PathBuf {
    // The records as the request lists them, in TOML's array syntax (the
    // same strings, JSON-quoted, are valid TOML basic strings).
    let records = req["records"].to_string();
    let body = format!(
        "schema = \"oh.war/accept-response/v1\"\nsubject = {}\nsubject_digest = {}\n\
         records = {records}\naccepted_by = \"{actor}\"\nacting_role = \"authorizer\"\n\
         meaning = \"accept these records\"\neffective_time = \"2026-10-14T10:00:00Z\"\n",
        req["subject"], req["subject_digest"]
    );
    let path = root.join(format!(
        "docs/authority/responses/{}.response.toml",
        req["subject"].as_str().unwrap()
    ));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    if let Some(k) = key {
        let dir = path.parent().unwrap();
        assert!(ssh_keygen(
            dir,
            &[
                "-Y",
                "sign",
                "-f",
                k.to_str().unwrap(),
                "-n",
                "oh.war/response",
                path.file_name().unwrap().to_str().unwrap(),
            ]
        ));
    }
    path
}

#[cfg(unix)]
#[test]
fn a_signed_accept_is_believed_and_an_act_over_a_stale_digest_is_refused() {
    let root = repo("signed");
    let Some(key) = throwaway_key(&root) else {
        eprintln!("ssh-keygen is not installed; skipped");
        return;
    };
    let req = request(&root, &[]);
    // Unsigned: a draft, whatever its fields claim.
    let path = response(&root, &req, "alice", None);
    let p = path.to_str().unwrap();
    let v = json(&root, &["spec", "accept", "--response", p], &[]);
    assert!(rules(&v).contains(&"authority.unsigned".to_owned()), "{v}");
    assert!(acceptance(&root, &[]).is_empty());
    let status = json(&root, &["spec", "status"], &[]);
    assert!(rules(&status).contains(&"authority.unsigned".to_owned()));

    // Signed by alice's key: believed.
    let path = response(&root, &req, "alice", Some(&key));
    let p = path.to_str().unwrap();
    let v = json(&root, &["spec", "accept", "--response", p], &[]);
    assert!(rules(&v).contains(&"spec.accepted".to_owned()), "{v}");
    assert!(acceptance(&root, &[]).contains(&st("doc:SPEC.md", "accepted", "computed")));
    let next = json(&root, &["next"], &[]);
    assert!(!next.to_string().contains("war sign spec"), "{next}");

    // The record moves: the same act is now over a stale digest.
    edit_requirement(&root);
    let v = json(&root, &["spec", "accept", "--response", p], &[]);
    assert_eq!(rules(&v), ["spec.stale-digest"], "{v}");
    assert!(acceptance(&root, &[]).contains(&st("doc:SPEC.md", "partially_accepted", "computed")));
    let next = json(&root, &["next"], &[]);
    assert!(next.to_string().contains("war sign spec"), "{next}");
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn a_tampered_or_an_agents_accept_is_refused() {
    let root = repo("refused");
    let Some(key) = throwaway_key(&root) else {
        eprintln!("ssh-keygen is not installed; skipped");
        return;
    };
    let req = request(&root, &[]);
    // An agent's: refused by kind, whatever it signs.
    let path = response(&root, &req, "claude", Some(&key));
    let v = json(
        &root,
        &["spec", "accept", "--response", path.to_str().unwrap()],
        &[],
    );
    assert_eq!(rules(&v), ["spec.not-permitted"], "{v}");
    assert!(acceptance(&root, &[]).is_empty());
    fs::remove_file(&path).unwrap();
    let _ = fs::remove_file(format!("{}.sig", path.display()));

    // Signed, then a record added to the list: the digest no longer covers
    // what it names.
    let path = response(&root, &req, "alice", Some(&key));
    let text = fs::read_to_string(&path).unwrap();
    let tampered = text.replacen(
        "records = [",
        "records = [[\"X-1\", \"sha256:00\", \"sha256:00\"], ",
        1,
    );
    assert_ne!(text, tampered);
    fs::write(&path, tampered).unwrap();
    let v = json(
        &root,
        &["spec", "accept", "--response", path.to_str().unwrap()],
        &[],
    );
    assert_eq!(rules(&v), ["spec.subject-digest"], "{v}");
    assert!(acceptance(&root, &[]).is_empty());
    let _ = fs::remove_dir_all(&root);
}

// ---- war spec migrate -------------------------------------------------------

const SAS: &str = "# Demo Software Architecture Specification\n\n## A subtitle\n\n\
# Part I — Foundations\n\nPart prose.\n\n## 1. Purpose\n\nWhy it exists.\n\n\
## 2. Scope\n\nWhat it covers.\n\n# Part II — Index\n\n## 3. Requirement index\n\n\
| ID | Requirement |\n|---|---|\n| SA-SAS-RQ-001 | Every record has an id |\n";

#[test]
fn migrate_is_a_dry_run_until_written_and_writes_over_nothing() {
    let root = scratch("migrate");
    write(&root, "docs/sas/Demo_SAS.md", SAS);
    let v = json(&root, &["spec", "migrate", "--version", "0.1.0"], &[]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(!root.join("SPEC.md").exists(), "a dry run writes nothing");
    assert!(!root.join("spec").exists());

    let v = json(
        &root,
        &["spec", "migrate", "--version", "0.1.0", "--write"],
        &[],
    );
    assert!(rules(&v).contains(&"spec.migrated".to_owned()), "{v}");
    let part = fs::read_to_string(root.join("spec/part-II-index.md")).unwrap();
    assert!(part.contains("part_of: doc:SPEC.md"));
    assert!(part.contains("## REQ-001 · requirement — Every record has an id"));
    assert!(part.contains("legacy.id: SA-SAS-RQ-001"));
    assert_eq!(
        fs::read_to_string(root.join("docs/sas/Demo_SAS.md")).unwrap(),
        SAS,
        "the SAS is not touched"
    );
    // The model reads it, its requirement keeps the old id, and nobody has
    // accepted it.
    let m = json(&root, &["model"], &[]);
    let req = m["result"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "REQ-001")
        .unwrap_or_else(|| panic!("{m}"))
        .clone();
    assert_eq!(req["ext"]["legacy.id"], "SA-SAS-RQ-001");
    assert!(acceptance(&root, &[]).is_empty());
    let format_findings: Vec<&serde_json::Value> = m["result"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"].as_str().unwrap_or("").starts_with("format."))
        .collect();
    assert!(format_findings.is_empty(), "{format_findings:?}");

    // A second write overwrites nothing.
    let v = json(
        &root,
        &["spec", "migrate", "--version", "0.1.0", "--write"],
        &[],
    );
    assert_eq!(rules(&v).last().unwrap(), "spec.migrate-exists", "{v}");
    let _ = fs::remove_dir_all(&root);
}

// ---- parts outside spec/ -----------------------------------------------------

#[test]
fn a_part_may_live_outside_spec_and_a_second_root_is_refused_anywhere() {
    let root = scratch("parts");
    write(&root, "SPEC.md", SPEC);
    write(
        &root,
        "docs/design/reset-lifetime.md",
        "---\nformat: openwarrant/1.0\nprofile: spec\npart_of: doc:SPEC.md\n\
summary: The reset link's lifetime.\n---\n# Reset link\n\n\
## REQ-reset-2 · requirement — a reset link expires\n\
statement: A reset link MUST expire 30 minutes after it is issued.\n",
    );
    let m = json(&root, &["model"], &[]);
    let records = m["result"]["records"].as_array().unwrap();
    let of = |id: &str| records.iter().find(|r| r["id"] == id).cloned();
    assert_eq!(
        of("doc:docs/design/reset-lifetime.md").unwrap()["type"],
        "spec"
    );
    assert_eq!(
        of("REQ-reset-2").unwrap()["source"],
        "docs/design/reset-lifetime.md"
    );
    // ... and is accepted with the rest: the request covers it.
    write(&root, "docs/authority/roles.toml", ROLES);
    let req = request(&root, &[]);
    assert!(req["records"].to_string().contains("REQ-reset-2"), "{req}");

    write(
        &root,
        "docs/notes/stray.md",
        "---\nformat: openwarrant/1.0\nprofile: spec\nversion: 1.0.0\n\
summary: A second root.\n---\n# Stray\n\n## Purpose\nx\n\n## Scope\ny\n",
    );
    let m = json(&root, &["model"], &[]);
    let dup: Vec<&serde_json::Value> = m["result"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"] == "format.document-duplicate")
        .collect();
    assert_eq!(dup.len(), 1, "{m}");
    assert!(dup[0].to_string().contains("docs/notes/stray.md"));
    let _ = fs::remove_dir_all(&root);
}
