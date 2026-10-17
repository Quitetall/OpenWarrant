// SPDX-License-Identifier: Apache-2.0
//! `war context --preset` (Format 1.0 section 10.4) and `war apply` (10.5)
//! through the shipped binary, over a Format 1.0 ticket and specification:
//! each preset's header and selection, the state a reading preset shows at
//! a time, and a package edited and given back. Each claim beside the
//! input it refuses.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

const TICKET: &str = "docs/tickets/t-0d61d.md";

const TICKET_TEXT: &str = "---
format: openwarrant/1.0
profile: ticket
alias: t-0d61d
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10
summary: Password reset by email.
implements: REQ-reset-1
---
# Password reset by email

## Intent
Users reset a password with a link sent by email.

## Checklist
- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12
- [ ] Email template (i-4d11)
- [ ] Docs (i-77be, after i-4d11)
- [ ] Rate limit (i-5e02)

## CHK-reset · check — reset tests pass
run: cargo test -p accounts reset::
evaluates i-9c01, i-4d11
";

const SPEC_TEXT: &str = "---
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
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.

## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1

## DEC-1 · decision — links are single-use tokens
selected_over OPT-1
implements REQ-reset-1

## OPT-1 · option — signed expiring links without state
";

/// A program holding the ticket and the specification, committed.
fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-ctxapply-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q", "."]);
    ok(&war(
        &root,
        &["init", "--program", "Apply Test", "--namespace", "AT"],
    ));
    std::fs::create_dir_all(root.join("docs/tickets")).unwrap();
    std::fs::write(root.join(TICKET), TICKET_TEXT).unwrap();
    std::fs::write(root.join("SPEC.md"), SPEC_TEXT).unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixtures"]);
    root
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_default()
}

fn header(md: &str) -> &str {
    md.lines().nth(1).unwrap_or_default()
}

fn index_line<'a>(md: &'a str, id: &str) -> Option<&'a str> {
    md.lines().find(|l| {
        l.starts_with('[')
            && l.get(4..)
                .is_some_and(|r| r.starts_with(&format!("{id} · ")))
    })
}

#[test]
fn every_preset_names_itself_and_its_walk_and_an_unknown_preset_is_refused() {
    let root = scratch("presets");
    let agent = ok(&war(&root, &["context", "t-0d61d/i-4d11"]));
    assert!(
        header(&agent).contains("· budget 16000 · depth 3 · preset agent (walk core) ·"),
        "{agent}"
    );
    let min = ok(&war(
        &root,
        &["context", "SPEC.md", "--preset", "agent.min"],
    ));
    assert!(
        header(&min).contains("· budget 4000 · depth 1 · preset agent.min (walk agent.min) ·"),
        "{min}"
    );
    // agent.min leaves out options and cuts rationale; agent keeps both.
    assert!(
        !min.contains("rationale:") && !min.contains("OPT-1 · option"),
        "{min}"
    );
    assert!(min.contains("statement: A reset link MUST be accepted at most once."));
    let full = ok(&war(&root, &["context", "SPEC.md"]));
    assert!(
        full.contains("rationale: a reused link is a takeover path."),
        "{full}"
    );
    assert!(full.contains("[F] OPT-1 · option"), "{full}");
    // `--budget` and `--depth` override the preset's own.
    let narrow = ok(&war(
        &root,
        &[
            "context",
            "SPEC.md",
            "--preset",
            "agent.min",
            "--budget",
            "9000",
            "--depth",
            "2",
        ],
    ));
    assert!(header(&narrow).contains("· budget 9000 · depth 2 · preset agent.min"));
    let bad = war(&root, &["context", "t-0d61d", "--preset", "everything"]);
    assert!(!bad.status.success());
    assert!(bad.stdout.is_empty(), "a refused preset prints no packet");
    assert!(
        String::from_utf8_lossy(&bad.stderr).contains("context.preset-unknown"),
        "{}",
        String::from_utf8_lossy(&bad.stderr)
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn human_gives_everything_editable_its_own_units_first_and_the_document_record_unwrapped() {
    let root = scratch("human");
    let md = ok(&war(&root, &["context", "t-0d61d", "--preset", "human"]));
    assert!(header(&md).contains("· budget none · depth 3 · preset human (walk core) ·"));
    assert!(
        !md.contains("[S] "),
        "no budget: every record is given in full\n{md}"
    );
    let intent = md.find("> t-0d61d/S-intent").unwrap();
    let check = md.find("> t-0d61d/CHK-reset").unwrap();
    let req = md.find("> REQ-reset-1").unwrap();
    assert!(
        intent < check && check < req,
        "own units first, in document order\n{md}"
    );
    assert!(md.contains(&format!("<!-- rec t-0d61d/S-intent {TICKET}:11 sha256:")));
    // The ticket's own record (frontmatter and preamble) is never wrapped.
    assert!(!md.contains("<!-- rec t-0d61d "), "{md}");
    // agent marks nothing editable.
    let agent = ok(&war(&root, &["context", "t-0d61d"]));
    assert!(!agent.contains("<!-- rec "), "{agent}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn resume_shows_holder_and_lease_while_a_claim_lives_and_offers_the_item_once_it_ran_out() {
    let root = scratch("resume");
    ok(&war(&root, &["claim", "t-0d61d/i-5e02", "--as", "bob"]));
    let at = "2099-01-01T00:00:00Z";
    // Without `--at`, now: the header records the time it answered at.
    let live = ok(&war(&root, &["context", "t-0d61d", "--preset", "resume"]));
    assert!(
        header(&live).contains("· preset resume (walk core) ·"),
        "{live}"
    );
    assert!(header(&live).contains(" · journal sha256:"), "{live}");
    assert!(
        header(&live).contains(" · at 20") && header(&live).ends_with("Z -->"),
        "{live}"
    );
    assert!(
        live.contains("state claimed · holder bob · lease "),
        "{live}"
    );
    assert!(
        live.contains("state done · by claude · 2026-10-12 · claimed"),
        "{live}"
    );
    assert!(live.contains("state blocked\n"), "{live}");
    let next = live.split("## Next\n").nth(1).unwrap();
    assert!(!next.contains("i-5e02"), "a held item is not next\n{next}");
    assert!(next.contains("- t-0d61d/i-4d11 · Email template"), "{next}");
    // At a time past its lease, the claim holds nothing: open, and next.
    let later = ok(&war(
        &root,
        &["context", "t-0d61d", "--preset", "resume", "--at", at],
    ));
    assert!(!later.contains("holder bob"), "{later}");
    assert!(
        later
            .split("## Next\n")
            .nth(1)
            .unwrap()
            .contains("t-0d61d/i-5e02"),
        "{later}"
    );
    // A preset that does not read state never shows the claim.
    let agent = ok(&war(&root, &["context", "t-0d61d/i-5e02"]));
    assert!(
        index_line(&agent, "t-0d61d/i-5e02").is_some_and(|l| l.contains(" · item · open · ")),
        "{agent}"
    );
    // resume picks up work: a requirement is not a target for it.
    let refused = war(
        &root,
        &["context", "REQ-reset-1", "--preset", "resume", "--at", at],
    );
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("context.target-unknown"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_index_flag_prints_what_compile_writes() {
    let root = scratch("index");
    ok(&war(&root, &["compile"]));
    let printed = ok(&war(&root, &["context", "--index"]));
    assert_eq!(printed, read(&root, "docs/generated/llms.txt"));
    assert!(printed.contains("\n## SPEC.md\n> Password reset for the account service.\n"));
    assert!(printed.contains("- t-0d61d · ticket · in_progress · Password reset by email"));
    let _ = std::fs::remove_dir_all(&root);
}

/// The human package of the ticket, written to `package.md`.
fn package(root: &Path) -> PathBuf {
    let md = ok(&war(root, &["context", "t-0d61d", "--preset", "human"]));
    let p = root.join("package.md");
    std::fs::write(&p, md).unwrap();
    p
}

fn edit(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{from:?} is not in {}", path.display());
    std::fs::write(path, text.replacen(from, to, 1)).unwrap();
}

fn apply(root: &Path, pkg: &Path) -> (Output, serde_json::Value) {
    let out = war(
        root,
        &[
            "--json",
            "apply",
            pkg.to_str().unwrap(),
            "--as",
            "alice",
            "--at",
            "2026-10-14T10:00:00Z",
        ],
    );
    let v = serde_json::from_slice(&out.stdout).unwrap_or_default();
    (out, v)
}

fn refusals(v: &serde_json::Value) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["severity"] == "error")
        .map(|d| {
            format!(
                "{} {}",
                d["rule"].as_str().unwrap_or(""),
                d["file"].as_str().unwrap_or("")
            )
        })
        .collect()
}

#[test]
fn an_edited_block_is_written_back_and_a_package_older_than_the_document_is_refused() {
    let root = scratch("apply-edit");
    let pkg = package(&root);
    edit(
        &pkg,
        "with a link sent by email.",
        "with a single-use link sent by email.",
    );
    // An index line edited outside the markers is never written back.
    edit(&pkg, "· Password reset by email ·", "· Something else ·");
    let (out, v) = apply(&root, &pkg);
    assert!(out.status.success(), "{v}");
    assert_eq!(
        read(&root, TICKET),
        TICKET_TEXT.replace(
            "with a link sent by email.",
            "with a single-use link sent by email."
        )
    );
    // The same package again: the block's record moved on since it was made.
    let before = read(&root, TICKET);
    std::fs::write(
        &pkg,
        std::fs::read_to_string(&pkg)
            .unwrap()
            .replace("single-use", "one-time"),
    )
    .unwrap();
    let (out, v) = apply(&root, &pkg);
    assert!(!out.status.success());
    assert_eq!(refusals(&v), [format!("apply.stale {TICKET}:11")]);
    assert_eq!(
        read(&root, TICKET),
        before,
        "a refused document is not written"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_ticked_box_is_done_by_the_applier_an_unticked_one_reopens_and_unchanged_blocks_write_nothing()
{
    let root = scratch("apply-box");
    let journal = root.join("docs/tickets/t-0d61d/journal.jsonl");
    // Unchanged: nothing written, nothing journalled.
    let pkg = package(&root);
    let (out, v) = apply(&root, &pkg);
    assert!(out.status.success(), "{v}");
    assert_eq!(read(&root, TICKET), TICKET_TEXT);
    assert!(!journal.exists());
    // Tick a box, untick another.
    edit(
        &pkg,
        "- [ ] Email template (i-4d11)",
        "- [x] Email template (i-4d11)",
    );
    edit(
        &pkg,
        "- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12",
        "- [ ] Reset endpoint (i-9c01) — done by claude, 2026-10-12",
    );
    let (out, v) = apply(&root, &pkg);
    assert!(out.status.success(), "{v}");
    let after = read(&root, TICKET);
    assert!(
        after.contains("- [x] Email template (i-4d11) — done by alice, 2026-10-14\n"),
        "{after}"
    );
    assert!(after.contains("- [ ] Reset endpoint (i-9c01)\n"), "{after}");
    let events = std::fs::read_to_string(&journal).unwrap();
    let types: Vec<String> = events
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .map(|e| {
            format!(
                "{} {} {}",
                e["type"].as_str().unwrap(),
                e["actor_ref"].as_str().unwrap(),
                e["payload"]["target"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(
        types,
        [
            "work.item_reopened alice t-0d61d/i-9c01",
            "work.item_done alice t-0d61d/i-4d11"
        ],
        "in the document's order"
    );
    // A hand-ticked box reads done at `claimed`, never higher.
    let at = "2026-10-14T10:00:00Z";
    let resume = ok(&war(
        &root,
        &["context", "t-0d61d", "--preset", "resume", "--at", at],
    ));
    assert!(
        resume.contains("state done · by alice · 2026-10-14 · claimed"),
        "{resume}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_block_that_changes_its_identity_or_does_not_parse_is_refused_and_nothing_is_written() {
    let root = scratch("apply-refuse");
    let pkg = package(&root);
    let clean = std::fs::read_to_string(&pkg).unwrap();
    // Identity: the Intent block renamed.
    edit(&pkg, "## Intent\nUsers reset", "## Purpose\nUsers reset");
    let (out, v) = apply(&root, &pkg);
    assert!(!out.status.success());
    assert_eq!(refusals(&v), [format!("apply.identity {TICKET}:11")]);
    // Does not parse: a line in a record's attribute block that is no field.
    std::fs::write(
        &pkg,
        clean.replace(
            "evaluates i-9c01, i-4d11\n<!-- /rec -->",
            "evaluates i-9c01, i-4d11\nNot a field.\n<!-- /rec -->",
        ),
    )
    .unwrap();
    let (out, v) = apply(&root, &pkg);
    assert!(!out.status.success());
    assert_eq!(refusals(&v), [format!("format.attribute {TICKET}:23")]);
    // One refused block holds back the document's other blocks.
    std::fs::write(
        &pkg,
        clean
            .replace("with a link sent by email.", "with an emailed link.")
            .replace(
                "evaluates i-9c01, i-4d11\n<!-- /rec -->",
                "evaluates i-9c01, i-4d11\nNot a field.\n<!-- /rec -->",
            ),
    )
    .unwrap();
    let (out, _) = apply(&root, &pkg);
    assert!(!out.status.success());
    assert_eq!(read(&root, TICKET), TICKET_TEXT);
    let _ = std::fs::remove_dir_all(&root);
}
