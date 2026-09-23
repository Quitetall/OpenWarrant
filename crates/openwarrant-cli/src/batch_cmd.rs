// SPDX-License-Identifier: Apache-2.0
//! `war sign --batch [targets…] --ssh-sign` — one signature over many acts
//! (OW-WAR-0072).
//!
//! The steps, and what each one refuses:
//!
//! 1. Choose the acts: every pending act, or the named targets. A target
//!    that is not pending is `batch.unknown-act`; one named twice is
//!    `batch.duplicate-act`. An act that needs a decision the batch cannot
//!    make (a resolution's outcome, a correction's kind) is left out and
//!    named; a SAS acceptance is left out too — its ingest has no dry run, so
//!    a batch holding one could not promise all-or-nothing.
//! 2. One signer, one role. Acts for another role are left out and named;
//!    they are a second batch.
//! 3. Draft every act's response exactly as `war sign` would alone, and judge
//!    every one with the act's own ingest in dry-run mode. One refusal
//!    refuses the batch: nothing is signed, every draft is removed.
//! 4. Write the batch document (`docs/authority/batches/<id>.json`, JCS),
//!    listing each response by name and exact sha256, and sign it once. That
//!    is the one dialog. A refused dialog is `batch.signature`; nothing stays.
//! 5. Judge every act again — the records may have moved while the dialog
//!    waited. One refusal now is `batch.digest-moved`: nothing is recorded,
//!    the batch is kept as `.refused.json` beside its signature.
//! 6. Record every act through its own ingest, and attest the whole batch in
//!    one DSSE envelope over the batch document, every response and every
//!    record it wrote.
//!
//! `war check` then believes each record exactly as it believes a singly
//! signed one: `authority_check` accepts a response with no `.sig` of its own
//! only when a batch whose signature verifies as that record's actor lists
//! the response's exact bytes.

use camino::Utf8PathBuf;
use openwarrant_core::batch::{BATCH_SCHEMA, Batch, BatchAct};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use crate::sign::{self, IngestMode, Pending};

pub const BATCHES: &str = "docs/authority/batches";

fn act_word(p: &Pending) -> &'static str {
    match p {
        Pending::Authorize { .. } => "authorize",
        Pending::Resolve { .. } => "resolve",
        Pending::Correct { .. } => "correct",
        Pending::Accept { .. } => "accept",
        Pending::AcceptRoadmap { .. } => "accept-roadmap",
    }
}

/// The act's own ingest, in either mode.
fn ingest(
    repo: &Repository,
    p: &Pending,
    path: &Utf8PathBuf,
    mode: IngestMode,
) -> Result<Report, RepoError> {
    match p {
        Pending::Authorize { alias, .. } => crate::authorize::ingest_with(repo, alias, path, mode),
        Pending::Resolve { alias, .. } => {
            crate::resolution_cmd::ingest_with(repo, alias, path, mode)
        }
        Pending::Correct {
            alias,
            deliverable_id,
            ..
        } => crate::correct::ingest_with(repo, alias, deliverable_id, path, mode),
        Pending::AcceptRoadmap { revision, .. } => {
            crate::roadmap_cmd::accept_ingest_with(repo, *revision, path, mode)
        }
        Pending::Accept { .. } => Err(RepoError::Message(
            "a SAS acceptance is not batched; sign it alone".to_owned(),
        )),
    }
}

struct Drafted<'a> {
    pending: &'a Pending,
    draft_path: Utf8PathBuf,
    final_path: Utf8PathBuf,
    entry: BatchAct,
}

pub fn run(
    repo: &Repository,
    targets: &[String],
    opts: &sign::Options,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if !opts.ssh_sign && !opts.dry_run {
        report.push(Diagnostic::error(
            "batch.needs-ssh",
            "war sign --batch".to_owned(),
            "a batch is signed once with your key: add --ssh-sign (or --dry-run to see it judged)",
        ));
        return Ok(report);
    }
    let all = sign::pending(repo)?;

    // 1. Choose.
    let mut chosen: Vec<&Pending> = Vec::new();
    if targets.is_empty() {
        chosen.extend(all.iter());
    } else {
        let mut seen = std::collections::BTreeSet::new();
        for t in targets {
            if !seen.insert(t.as_str()) {
                report.push(Diagnostic::error(
                    "batch.duplicate-act",
                    t.clone(),
                    format!("{t} is named twice; a batch lists each act once"),
                ));
                return Ok(report);
            }
            match sign::select(&all, t) {
                Some(p) => chosen.push(p),
                None => {
                    report.push(Diagnostic::error(
                        "batch.unknown-act",
                        t.clone(),
                        format!("{t} awaits no signature; `war sign --list` shows what does"),
                    ));
                    return Ok(report);
                }
            }
        }
    }

    // 2–3. One signer, one role; draft and judge each.
    let now = crate::gate_cmd::receipt::now_rfc3339_public();
    let mut signer: Option<(String, &'static str)> = None;
    let mut drafted: Vec<Drafted> = Vec::new();
    let mut left_out: Vec<String> = Vec::new();
    for p in chosen {
        if matches!(p, Pending::Accept { .. }) {
            left_out.push(format!(
                "{} — a SAS acceptance signs alone: `war sign {} --ssh-sign`",
                sign::line(p),
                sign::target_of(p)
            ));
            continue;
        }
        let actor = match sign::choose_actor(sign::eligible(p), opts) {
            Ok(a) => a,
            Err(why) => {
                report.push(Diagnostic::error("sign.who", sign::line(p), why));
                discard(&drafted);
                return Ok(report);
            }
        };
        let role = sign::role(p);
        match &signer {
            None => signer = Some((actor.clone(), role)),
            Some((a, r)) if *a != actor || *r != role => {
                left_out.push(format!(
                    "{} — {} as {}; this batch is {} as {}: a second batch",
                    sign::line(p),
                    actor,
                    role,
                    a,
                    r
                ));
                continue;
            }
            Some(_) => {}
        }
        let mut per_act = opts.clone();
        if matches!(p, Pending::Correct { .. }) {
            per_act.meaning = sign::reason_for(repo, p, opts);
        }
        let d = match sign::draft(p, &actor, &per_act, &now) {
            Ok(d) => d,
            Err(why) => {
                left_out.push(format!("{} — {why}", sign::line(p)));
                continue;
            }
        };
        let draft_path = sign::write_response(repo, &d)?;
        let final_path = sign::signed_path(&draft_path);
        let bytes = std::fs::read(&draft_path).map_err(|source| RepoError::Io {
            context: format!("could not read {draft_path}"),
            source,
        })?;
        drafted.push(Drafted {
            entry: BatchAct {
                act: act_word(p).to_owned(),
                target: sign::target_of(p),
                response: final_path.file_name().unwrap_or_default().to_owned(),
                response_sha256: openwarrant_compiler::sha256_hex(&bytes),
                bound_digest: d.digest().trim_start_matches("sha256:").to_owned(),
            },
            pending: p,
            draft_path,
            final_path,
        });
    }
    for why in &left_out {
        report.push(Diagnostic::warn(
            "batch.left-out",
            "war sign --batch".to_owned(),
            why.clone(),
        ));
    }
    let Some((actor, role)) = signer else {
        report.push(Diagnostic::error(
            "batch.empty",
            "war sign --batch".to_owned(),
            "no act can be batched; the ones left out are named above",
        ));
        return Ok(report);
    };
    if drafted.is_empty() {
        report.push(Diagnostic::error(
            "batch.empty",
            "war sign --batch".to_owned(),
            "no act can be batched; the ones left out are named above",
        ));
        return Ok(report);
    }
    if !judge(repo, &drafted, &mut report)? {
        discard(&drafted);
        report.push(Diagnostic::error(
            "batch.refused",
            "war sign --batch".to_owned(),
            "an act in the batch would be refused (above); nothing was signed and every draft is removed",
        ));
        return Ok(report);
    }
    if opts.dry_run {
        // Everything the real batch would do before asking the key, and
        // nothing after: the drafts go, and the verdict is the list.
        discard(&drafted);
        report.push(Diagnostic::pass(
            "batch.would-record",
            format!(
                "{} act(s) as {actor} ({role}), one signature: {}",
                drafted.len(),
                drafted
                    .iter()
                    .map(|d| format!("{} {}", d.entry.act, d.entry.target))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
        return Ok(report);
    }

    // 4. The batch document, signed once.
    let batch = Batch {
        schema: BATCH_SCHEMA.to_owned(),
        batch_id: String::new(),
        drafted_at: now.clone(),
        signer: actor.clone(),
        role: role.to_owned(),
        acts: drafted.iter().map(|d| d.entry.clone()).collect(),
    };
    let id_seed = serde_jcs::to_string(&batch).map_err(|e| RepoError::Message(e.to_string()))?;
    let batch = Batch {
        batch_id: format!(
            "B-{}-{}",
            now.replace([':', '-'], ""),
            &openwarrant_compiler::sha256_hex(id_seed.as_bytes())[..8]
        ),
        ..batch
    };
    batch
        .validate()
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let dir = repo.root.join(BATCHES);
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let batch_path = dir.join(format!("{}.json", batch.batch_id));
    let text = serde_jcs::to_string(&batch).map_err(|e| RepoError::Message(e.to_string()))? + "\n";
    std::fs::write(&batch_path, &text).map_err(|source| RepoError::Io {
        context: format!("could not write {batch_path}"),
        source,
    })?;
    println!(
        "┌ batch {} · {} act(s) · {} as {}",
        batch.batch_id,
        batch.acts.len(),
        actor,
        role
    );
    for a in &batch.acts {
        println!("│ {:<15} {}", a.act, a.target);
    }
    println!(
        "└ One dialog signs all {}. Confirm in your agent's dialog.",
        batch.acts.len()
    );
    let principal = match sign::principal_of(repo, &actor) {
        Ok(p) => p,
        Err(why) => {
            let _ = std::fs::remove_file(&batch_path);
            discard(&drafted);
            report.push(Diagnostic::error(
                "batch.signature",
                repo.relative(&batch_path),
                why,
            ));
            return Ok(report);
        }
    };
    if let Err(why) =
        sign::ssh_sign_file(&sign::allowed_signers_path(repo), &principal, &batch_path)
    {
        let _ = std::fs::remove_file(&batch_path);
        discard(&drafted);
        report.push(Diagnostic::error(
            "batch.signature",
            repo.relative(&batch_path),
            format!("not signed; nothing written: {why}"),
        ));
        return Ok(report);
    }

    // 5. Judge again: the dialog may have waited while records moved.
    let mut again = Report::default();
    if !judge(repo, &drafted, &mut again)? {
        for d in again.diagnostics {
            report.push(d);
        }
        let refused = dir.join(format!("{}.refused.json", batch.batch_id));
        let _ = std::fs::rename(&batch_path, &refused);
        let _ = std::fs::rename(format!("{batch_path}.sig"), format!("{refused}.sig"));
        discard(&drafted);
        report.push(Diagnostic::error(
            "batch.digest-moved",
            repo.relative(&refused),
            "a record moved between drafting and the signature; the whole batch is refused, nothing recorded, and the signed batch kept as .refused.json",
        ));
        return Ok(report);
    }

    // 6. Record each act through its own ingest.
    let mut records: Vec<Utf8PathBuf> = Vec::new();
    let mut recorded = 0usize;
    for d in &drafted {
        if let Err(why) =
            sign::retire_prior(&d.final_path, &format!("sha256:{}", d.entry.bound_digest))
        {
            report.push(Diagnostic::error(
                "sign.response-exists",
                sign::line(d.pending),
                why,
            ));
            continue;
        }
        std::fs::rename(&d.draft_path, &d.final_path).map_err(|source| RepoError::Io {
            context: format!("could not move {} to {}", d.draft_path, d.final_path),
            source,
        })?;
        let r = ingest(repo, d.pending, &d.final_path, IngestMode::Record)?;
        let ok = r.is_ready();
        for x in r.diagnostics {
            report.push(x);
        }
        if ok {
            recorded += 1;
            records.push(d.final_path.clone());
            if let Ok(rec) = sign::record_of(repo, d.pending) {
                records.push(rec);
            }
        }
    }
    report.push(Diagnostic::pass(
        "batch.recorded",
        format!(
            "batch {}: {recorded} of {} act(s) recorded under one signature → {}",
            batch.batch_id,
            drafted.len(),
            repo.relative(&batch_path)
        ),
    ));

    // One attestation over the batch, every response and every record.
    let mut files = vec![Utf8PathBuf::from(repo.relative(&batch_path))];
    files.extend(records.iter().map(|p| Utf8PathBuf::from(repo.relative(p))));
    let a = crate::attest::Attestable {
        act: "batch",
        target: &batch.batch_id,
        files,
        extra_subjects: vec![(
            format!("batch:{}", batch.batch_id),
            openwarrant_compiler::sha256_hex(text.as_bytes()),
        )],
        // The batch itself, plus `actor`: `attest --verify` finds the key
        // through the predicate's actor, as it does for every single act.
        predicate: {
            let mut v = serde_json::to_value(&batch).unwrap_or_default();
            if let serde_json::Value::Object(map) = &mut v {
                map.insert("actor".to_owned(), batch.signer.clone().into());
            }
            v
        },
        actor: &actor,
    };
    match crate::attest::emit(repo, &a) {
        Ok(p) => report.push(Diagnostic::pass(
            "attest.emitted",
            format!(
                "batch {}: attestation written to {}",
                batch.batch_id,
                repo.relative(&p)
            ),
        )),
        Err(e) => report.push(Diagnostic::warn(
            "attest.not-emitted",
            repo.relative(&batch_path),
            format!("the acts are recorded; the batch's attestation was not: {e}"),
        )),
    }
    Ok(report)
}

/// Judge every drafted act with its own ingest in dry-run mode.
fn judge(
    repo: &Repository,
    drafted: &[Drafted<'_>],
    report: &mut Report,
) -> Result<bool, RepoError> {
    let mut all = true;
    for d in drafted {
        let r = ingest(repo, d.pending, &d.draft_path, IngestMode::DryRun)?;
        if !r.is_ready() {
            all = false;
            for x in r.diagnostics {
                if x.severity >= crate::diagnostic::Severity::Unknown {
                    report.push(x);
                }
            }
        }
    }
    Ok(all)
}

fn discard(drafted: &[Drafted<'_>]) {
    for d in drafted {
        let _ = std::fs::remove_file(&d.draft_path);
    }
}

/// Every batch on disk, parsed and validated, with its path. A file that does
/// not parse is skipped: it covers nothing.
#[must_use]
pub fn load_all(repo: &Repository) -> Vec<(Utf8PathBuf, Batch)> {
    let dir = repo.root.join(BATCHES);
    let Ok(rd) = dir.read_dir_utf8() else {
        return vec![];
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let p = e.path().to_owned();
        let name = p.file_name().unwrap_or_default();
        if !name.ends_with(".json") || name.ends_with(".refused.json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        let Ok(b) = serde_json::from_str::<Batch>(&text) else {
            continue;
        };
        if b.validate().is_ok() {
            out.push((p, b));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
