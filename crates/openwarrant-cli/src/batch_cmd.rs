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
//!    record it wrote. Before the first record, a durable marker
//!    (`<id>.recording`, fsynced) lists every path the batch may write with
//!    its prior sha256 or `absent`, and the prior bytes sit beside it under
//!    `<id>.recording.d/`. A refusal mid-way restores them and refuses
//!    `batch.incomplete`; a process killed mid-way leaves the marker, and
//!    until it is gone every batch refuses `batch.interrupted` and `war check`
//!    reports it. `war sign --batch recover:<id>` puts every listed path back
//!    and keeps the batch as `.refused.json` (OW-WAR-0130).
//!
//! `war check` then believes each record exactly as it believes a singly
//! signed one: `authority_check` accepts a response with no `.sig` of its own
//! only when a batch whose signature verifies as that record's actor lists
//! the response's exact bytes.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::batch::{BATCH_SCHEMA, Batch, BatchAct};
use serde::{Deserialize, Serialize};

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
    // OW-WAR-0130: `recover:<id>` undoes an interrupted batch; it signs
    // nothing, so it needs no key. It is the only thing a batch does while a
    // marker is on disk: a second batch over a half-recorded first would
    // record acts beside a hole.
    if let Some(id) = targets.iter().find_map(|t| t.strip_prefix(RECOVER_PREFIX)) {
        if targets.len() != 1 {
            report.push(Diagnostic::error(
                "batch.recover-alone",
                "war sign --batch".to_owned(),
                format!(
                    "{RECOVER_PREFIX}{id} is named with other targets; recovery is its own act"
                ),
            ));
            return Ok(report);
        }
        return recover(repo, id, report);
    }
    let interrupted = interrupted(repo);
    if !interrupted.is_empty() {
        for m in &interrupted {
            report.push(interrupted_diagnostic(repo, m));
        }
        return Ok(report);
    }
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

    // 6. Record every act through its own ingest — all of them or none.
    // Every directory an act writes is copied first, durably, beside a
    // marker listing each path and its prior digest (OW-WAR-0130). If any
    // act fails to record, each is put back byte for byte and the batch is
    // refused; if the process dies, the marker stays and names the hole. A
    // list applied with a hole in it would record acts the signer signed as
    // a list they did not sign.
    let mut dirs: Vec<Utf8PathBuf> = vec![repo.root.join("docs/authority/responses")];
    let mut drafts: Vec<Utf8PathBuf> = Vec::new();
    let mut targets_written: Vec<Utf8PathBuf> = Vec::new();
    for d in &drafted {
        let own = match d.pending {
            Pending::AcceptRoadmap { .. } => crate::roadmap_cmd::dir(repo),
            Pending::Authorize { alias, .. }
            | Pending::Resolve { alias, .. }
            | Pending::Correct { alias, .. } => repo.warrant_dir(alias)?,
            Pending::Accept { .. } => continue,
        };
        if !dirs.contains(&own) {
            dirs.push(own);
        }
        // The draft exists only because this batch wrote it: its prior state
        // is absent. The response and record the act writes are listed with
        // whatever they are now — their digest, or absent.
        drafts.push(d.draft_path.clone());
        targets_written.push(d.final_path.clone());
        if let Ok(rec) = sign::record_of(repo, d.pending) {
            targets_written.push(rec);
        }
    }
    let recording = match Recording::begin(repo, &batch.batch_id, &dirs, &drafts, &targets_written)
    {
        Ok(r) => r,
        Err(e) => {
            discard(&drafted);
            let refused = dir.join(format!("{}.refused.json", batch.batch_id));
            let _ = std::fs::rename(&batch_path, &refused);
            let _ = std::fs::rename(format!("{batch_path}.sig"), format!("{refused}.sig"));
            report.push(Diagnostic::error(
                "batch.incomplete",
                repo.relative(&refused),
                format!(
                    "the prior bytes could not be kept before the first record ({e}); nothing \
                     was recorded, and the signed batch is kept as .refused.json"
                ),
            ));
            return Ok(report);
        }
    };
    let mut records: Vec<Utf8PathBuf> = Vec::new();
    let mut failed: Option<String> = None;
    for (i, d) in drafted.iter().enumerate() {
        // The faults the battery plants, debug builds only — a release binary
        // has no way to be told to fail. FAIL_AFTER refuses after `n` acts
        // recorded; KILL_AFTER sends this process SIGKILL there, which no
        // cleanup survives: the crash the durable marker exists for.
        #[cfg(debug_assertions)]
        {
            let after = |var: &str| {
                std::env::var(var)
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    == Some(i)
            };
            #[cfg(unix)]
            if after("OPENWARRANT_TEST_BATCH_KILL_AFTER") {
                let _ = rustix::process::kill_process(
                    rustix::process::getpid(),
                    rustix::process::Signal::KILL,
                );
            }
            if after("OPENWARRANT_TEST_BATCH_FAIL_AFTER") {
                failed = Some(format!(
                    "{}: planted failure after {i} act(s)",
                    sign::line(d.pending)
                ));
                break;
            }
        }
        let _ = i;
        if let Err(why) =
            sign::retire_prior(&d.final_path, &format!("sha256:{}", d.entry.bound_digest))
        {
            failed = Some(format!("{}: {why}", sign::line(d.pending)));
            break;
        }
        if let Err(e) = std::fs::rename(&d.draft_path, &d.final_path) {
            failed = Some(format!(
                "could not move {} to {}: {e}",
                d.draft_path, d.final_path
            ));
            break;
        }
        let r = match ingest(repo, d.pending, &d.final_path, IngestMode::Record) {
            Ok(r) => r,
            Err(e) => {
                failed = Some(format!("{}: {e}", sign::line(d.pending)));
                break;
            }
        };
        let ok = r.is_ready();
        for x in r.diagnostics {
            report.push(x);
        }
        if !ok {
            failed = Some(format!(
                "{}: its ingest refused it (above)",
                sign::line(d.pending)
            ));
            break;
        }
        records.push(d.final_path.clone());
        if let Ok(rec) = sign::record_of(repo, d.pending) {
            records.push(rec);
        }
    }
    if let Some(why) = failed {
        let restored = recording.restore(repo);
        // The copy was taken with the drafts in place and lists them absent;
        // like every other refusal, a refused batch leaves no draft behind.
        discard(&drafted);
        let refused = dir.join(format!("{}.refused.json", batch.batch_id));
        let _ = std::fs::rename(&batch_path, &refused);
        let _ = std::fs::rename(format!("{batch_path}.sig"), format!("{refused}.sig"));
        let message = match &restored {
            Ok(()) => format!(
                "{why}. Nothing is recorded: every directory the batch wrote is restored \
                 byte for byte, and the signed batch is kept as .refused.json"
            ),
            Err(e) => format!(
                "{why}. The restore ALSO failed ({e}); the marker {} stays, every batch \
                 refuses batch.interrupted until `war sign --batch {RECOVER_PREFIX}{}` \
                 succeeds",
                repo.relative(&recording.marker),
                batch.batch_id
            ),
        };
        if restored.is_ok() {
            recording.finish();
        }
        report.push(Diagnostic::error(
            "batch.incomplete",
            repo.relative(&refused),
            message,
        ));
        return Ok(report);
    }
    let recorded = drafted.len();
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
    // Only now, after the last record and the attestation: a process killed
    // at any point before this line leaves the marker naming the hole.
    recording.finish();
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

/// The target that asks for recovery: `war sign --batch recover:<id>`.
pub const RECOVER_PREFIX: &str = "recover:";

/// The marker's own schema. A marker is a transient file of this tool, not a
/// record: it exists only while a batch records, or after one was killed.
const RECORDING_SCHEMA: &str = "oh.war/batch-recording/v1";

/// `docs/authority/batches/<id>.recording`: what a batch in step 6 may write,
/// and what each path was before it began.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    pub schema: String,
    pub batch_id: String,
    /// Each directory the batch writes into, restored whole: every file the
    /// batch added under it is removed, and every one it changed returns.
    pub roots: Vec<Root>,
    /// Every path the batch may write, with its prior digest or `absent`.
    pub paths: Vec<Listed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root {
    /// Repository-relative.
    pub dir: String,
    /// Where its prior bytes are, relative to `<id>.recording.d/`; `None`
    /// when the directory did not exist.
    pub copy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listed {
    /// Repository-relative.
    pub path: String,
    /// `sha256:<hex>` of the prior bytes, or `absent`.
    pub prior: String,
}

const ABSENT: &str = "absent";

/// Every marker on disk: a batch that began recording and never finished.
#[must_use]
pub fn interrupted(repo: &Repository) -> Vec<(Utf8PathBuf, Marker)> {
    let Ok(rd) = repo.root.join(BATCHES).read_dir_utf8() else {
        return vec![];
    };
    let mut out: Vec<(Utf8PathBuf, Marker)> = rd
        .flatten()
        .map(|e| e.path().to_owned())
        .filter(|p| p.extension() == Some("recording") && p.is_file())
        .map(|p| {
            // A marker that does not parse is still a marker: the batch it
            // belonged to is still unfinished, and it is named by its file.
            let m = std::fs::read_to_string(&p)
                .ok()
                .and_then(|t| serde_json::from_str::<Marker>(&t).ok())
                .unwrap_or_else(|| Marker {
                    schema: RECORDING_SCHEMA.to_owned(),
                    batch_id: p.file_stem().unwrap_or_default().to_owned(),
                    roots: vec![],
                    paths: vec![],
                });
            (p, m)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// `batch.interrupted`, naming the batch, every path it lists, and the undo.
#[must_use]
pub fn interrupted_diagnostic(repo: &Repository, (path, m): &(Utf8PathBuf, Marker)) -> Diagnostic {
    Diagnostic::error(
        "batch.interrupted",
        repo.relative(path),
        format!(
            "batch {} began recording and never finished: the process stopped between its first \
             record and its last, so what it recorded is a list with a hole in it. Paths it may \
             have written: {}. `war sign --batch {RECOVER_PREFIX}{}` restores each to its prior \
             bytes and keeps the batch as .refused.json; no batch records until then",
            m.batch_id,
            if m.paths.is_empty() {
                "(the marker does not parse; its roots cannot be read)".to_owned()
            } else {
                m.paths
                    .iter()
                    .map(|l| l.path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            },
            m.batch_id
        ),
    )
}

/// Whether a marker lists a path under `dir` (for `war check`, which reports
/// a marker under each Warrant it touches).
#[must_use]
pub fn touches(repo: &Repository, m: &Marker, dir: &Utf8Path) -> bool {
    let rel = repo.relative(dir);
    m.roots
        .iter()
        .any(|r| r.dir == rel || r.dir.starts_with(&format!("{rel}/")))
}

/// `war sign --batch recover:<id>`: put every path the marker lists back,
/// then keep the batch as `.refused.json`. The restore is this tool undoing
/// its own partial write (§27.2); it decides no act.
fn recover(repo: &Repository, id: &str, mut report: Report) -> Result<Report, RepoError> {
    let dir = repo.root.join(BATCHES);
    let marker = dir.join(format!("{id}.recording"));
    let Some((_, m)) = interrupted(repo).into_iter().find(|(p, _)| *p == marker) else {
        report.push(Diagnostic::error(
            "batch.nothing-to-recover",
            repo.relative(&marker),
            format!("no batch {id} is interrupted: there is no {id}.recording to recover from"),
        ));
        return Ok(report);
    };
    if m.schema != RECORDING_SCHEMA || m.batch_id != id {
        report.push(Diagnostic::error(
            "batch.recover-unreadable",
            repo.relative(&marker),
            format!(
                "the marker does not read as a {RECORDING_SCHEMA} for {id}; nothing restored. \
                 `git status` shows what the batch left behind"
            ),
        ));
        return Ok(report);
    }
    let recording = Recording {
        marker: marker.clone(),
        copies: dir.join(format!("{id}.recording.d")),
        m,
    };
    if let Err(e) = recording.restore(repo) {
        report.push(Diagnostic::error(
            "batch.recover-failed",
            repo.relative(&marker),
            format!(
                "{e}. The marker stays and names every path; run the recovery again, or restore \
                 them with `git checkout`"
            ),
        ));
        return Ok(report);
    }
    let batch_path = dir.join(format!("{id}.json"));
    let refused = dir.join(format!("{id}.refused.json"));
    if batch_path.is_file() {
        let _ = std::fs::rename(&batch_path, &refused);
    }
    let sig = Utf8PathBuf::from(format!("{batch_path}.sig"));
    if sig.is_file() {
        let _ = std::fs::rename(&sig, format!("{refused}.sig"));
    }
    let n = recording.m.paths.len();
    recording.finish();
    report.push(Diagnostic::pass(
        "batch.recovered",
        format!(
            "batch {id}: {n} listed path(s) restored to their prior bytes or removed as before, \
             each checked against the marker's digest; the signed batch is kept as {}. \
             Nothing it listed is recorded",
            repo.relative(&refused)
        ),
    ));
    Ok(report)
}

/// The durable copy step 6 keeps while it records.
struct Recording {
    marker: Utf8PathBuf,
    copies: Utf8PathBuf,
    m: Marker,
}

impl Recording {
    /// Copy each root, fsynced, then write the marker through the atomic
    /// path. The marker exists only once every prior byte it points at does.
    fn begin(
        repo: &Repository,
        batch_id: &str,
        roots: &[Utf8PathBuf],
        drafts: &[Utf8PathBuf],
        targets: &[Utf8PathBuf],
    ) -> Result<Self, RepoError> {
        let io = |context: String| move |source| RepoError::Io { context, source };
        let dir = repo.root.join(BATCHES);
        let copies = dir.join(format!("{batch_id}.recording.d"));
        std::fs::create_dir_all(&copies).map_err(io(format!("could not create {copies}")))?;
        let mut m = Marker {
            schema: RECORDING_SCHEMA.to_owned(),
            batch_id: batch_id.to_owned(),
            roots: vec![],
            paths: vec![],
        };
        for (n, d) in roots.iter().enumerate() {
            if d.is_dir() {
                let copy = copies.join(n.to_string());
                copy_tree(d, &copy).map_err(io(format!("could not copy {d}")))?;
                let mut files = Vec::new();
                list_files(d, &mut files).map_err(io(format!("could not list {d}")))?;
                for f in files {
                    if drafts.contains(&f) {
                        continue;
                    }
                    let bytes = std::fs::read(&f).map_err(io(format!("could not read {f}")))?;
                    m.paths.push(Listed {
                        path: repo.relative(&f),
                        prior: format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes)),
                    });
                }
                m.roots.push(Root {
                    dir: repo.relative(d),
                    copy: Some(n.to_string()),
                });
            } else {
                m.roots.push(Root {
                    dir: repo.relative(d),
                    copy: None,
                });
            }
        }
        for c in drafts.iter().chain(targets.iter().filter(|t| !t.exists())) {
            m.paths.push(Listed {
                path: repo.relative(c),
                prior: ABSENT.to_owned(),
            });
        }
        m.paths.sort_by(|a, b| a.path.cmp(&b.path));
        m.paths.dedup_by(|a, b| a.path == b.path);
        sync_dir(&copies).map_err(io(format!("could not fsync {copies}")))?;
        let text = serde_json::to_string_pretty(&m)
            .map_err(|e| RepoError::Message(format!("could not render the marker: {e}")))?;
        let marker = dir.join(format!("{batch_id}.recording"));
        crate::compile::atomic::write(&marker, format!("{text}\n"))?;
        Ok(Self { marker, copies, m })
    }

    /// Put every root back whole, remove every path listed absent, and check
    /// every listed path against its prior digest. An error leaves the
    /// marker in place.
    fn restore(&self, repo: &Repository) -> std::io::Result<()> {
        for r in &self.m.roots {
            let d = repo.root.join(&r.dir);
            if d.exists() {
                std::fs::remove_dir_all(&d)?;
            }
            if let Some(c) = &r.copy {
                copy_tree(&self.copies.join(c), &d)?;
            }
        }
        let mut moved = Vec::new();
        for l in &self.m.paths {
            let p = repo.root.join(&l.path);
            if l.prior == ABSENT {
                if p.is_file() {
                    std::fs::remove_file(&p)?;
                }
                if p.exists() {
                    moved.push(l.path.clone());
                }
                continue;
            }
            let now = std::fs::read(&p)
                .map(|b| format!("sha256:{}", openwarrant_compiler::sha256_hex(&b)))
                .unwrap_or_else(|_| ABSENT.to_owned());
            if now != l.prior {
                moved.push(l.path.clone());
            }
        }
        if moved.is_empty() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "after the restore these paths still differ from their prior bytes: {}",
                moved.join(", ")
            )))
        }
    }

    /// The batch is whole (recorded, or undone): the marker goes first, then
    /// the copies. A crash between the two leaves copies no marker names.
    fn finish(&self) {
        let _ = std::fs::remove_file(&self.marker);
        if let Some(parent) = self.marker.parent() {
            let _ = sync_dir(parent);
        }
        let _ = std::fs::remove_dir_all(&self.copies);
    }
}

fn list_files(dir: &Utf8Path, out: &mut Vec<Utf8PathBuf>) -> std::io::Result<()> {
    for entry in dir.read_dir_utf8()? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            list_files(entry.path(), out)?;
        } else {
            out.push(entry.path().to_owned());
        }
    }
    Ok(())
}

fn sync_dir(dir: &Utf8Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

/// Copy a tree, fsyncing every file and directory it creates: the copy is
/// what a recovery after a crash restores from.
fn copy_tree(from: &Utf8Path, to: &Utf8Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in from.read_dir_utf8()? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
            std::fs::File::open(&dest)?.sync_all()?;
        }
    }
    sync_dir(to)
}
