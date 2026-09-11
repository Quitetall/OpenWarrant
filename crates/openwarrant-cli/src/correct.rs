// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war correct` — the correction act (OW-WAR-0064): the fifth two-half seam.
//!
//! An agent may EMIT a request naming a resolved Warrant's deliverable, the
//! digest its record pins, the digest the file has now, and who may sign. Only a
//! human's RESPONSE — ingested through the authority register, refused for every
//! agent regardless of what it claims (§27.2) — writes a correction record. The
//! record is appended beside `deliverables.toml`, never written into it: a
//! resolution binds `sha256(deliverables.toml)`, and a correction that edited the
//! manifest would stale the resolution it exists to leave standing.
//!
//! # What ingestion refuses, each before anything is written
//!
//! - a response for another Warrant or an unknown deliverable;
//! - a Warrant that is not resolved (regenerate the record instead — the
//!   ordinary remedy still applies before resolution);
//! - a file that has not drifted (there is nothing to correct);
//! - a signer the register does not know, knows as an agent, or who may not
//!   authorize (including the performer: `SelfAct`);
//! - `new_digest` that is not the file's bytes now — it corrects nothing;
//! - `superseded_digest` that is not the chain head — it supersedes nothing that
//!   happened;
//! - an empty reason, or an `effective_time` that is not RFC 3339;
//! - a record file that already exists: a second correction is a second file.
//!
//! # What `war check` then holds
//!
//! The journal carries the digest of each correction file as written. A record
//! edited afterwards, or a second correction applied as an edit to the first,
//! fails `correction.edited`; a record with no journal line is
//! `correction.unjournalled`. The superseded digest is never removed from
//! anything.

use camino::Utf8Path;
use openwarrant_core::WarUuid;
use openwarrant_core::authority::{ActorKind, ActorRole};
use openwarrant_core::correction::{
    CORRECTION_SCHEMA, ChainError, Correction, CorrectionKind, CorrectionRecord, chain_head,
};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{Loaded, RepoError, Repository};

pub const REQUEST_SCHEMA: &str = "oh.war/correction-request/v1";
pub const RESPONSE_SCHEMA: &str = "oh.war/correction-response/v1";

/// What an agent may emit. No recommendation, no suggested wording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionRequest {
    pub schema: String,
    pub warrant: String,
    pub deliverable_id: String,
    pub target_ref: String,
    pub title: String,
    /// A correction is for a RESOLVED Warrant; before resolution, regenerate.
    pub resolved: bool,
    /// The digest `deliverables.toml` records.
    pub recorded_digest: String,
    /// The digest the chain of prior corrections resolves to.
    pub chain_head: String,
    /// The digest of the file's bytes now.
    pub current_digest: String,
    /// `current_digest != chain_head`.
    pub drift: bool,
    pub prior_corrections: u32,
    pub next_sequence: u32,
    /// Humans the register lets authorize on this Warrant. Agents never appear.
    pub eligible_correctors: Vec<String>,
}

/// What the human returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResponse {
    pub schema: String,
    pub warrant: String,
    pub deliverable_id: String,
    /// Must equal the request's `chain_head` at ingest.
    pub superseded_digest: String,
    /// Must equal the file's bytes at ingest.
    pub new_digest: String,
    pub reason: String,
    pub kind: CorrectionKind,
    pub corrected_by: String,
    /// §27.4 — the role actually exercised.
    pub acting_role: String,
    pub effective_time: String,
    /// `tty` or `ssh` via `war sign`; absent when hand-written. Provenance only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_via: Option<String>,
}

fn sha256_of(path: &Utf8Path) -> Result<String, RepoError> {
    let bytes = std::fs::read(path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    Ok(format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(&bytes)
    ))
}

/// What one deliverable's records say about it, computed once so every
/// caller agrees.
struct Standing {
    target_ref: String,
    title: String,
    recorded: String,
    corrections: Vec<Correction>,
}

fn standing(repo: &Repository, one: &Loaded, alias: &str, id: &str) -> Result<Standing, RepoError> {
    let deliverables = repo.load_deliverables(&one.dir)?;
    let Some(d) = deliverables.records.iter().find(|d| d.id == id) else {
        return Err(RepoError::Message(format!(
            "{alias}: no deliverable {id:?} in deliverables.toml"
        )));
    };
    let Some(p) = d.provenance.as_ref() else {
        return Err(RepoError::Message(format!(
            "{alias}: {id} carries no provenance, so there is no recorded digest to correct"
        )));
    };
    if !d.content_addressed {
        return Err(RepoError::Message(format!(
            "{alias}: {id} is not content-addressed; there is no pinned digest to correct"
        )));
    }
    Ok(Standing {
        target_ref: d.target_ref.clone(),
        title: d.title.clone(),
        recorded: p.content_digest.clone(),
        corrections: repo.load_corrections(&one.dir)?.for_deliverable(id),
    })
}

/// `war correct <alias> <deliverable-id>`: the request. Writes nothing.
pub fn request(repo: &Repository, alias: &str, id: &str) -> Result<CorrectionRequest, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let s = standing(repo, &one, alias, id)?;
    let head = chain_head(&s.recorded, &s.corrections)
        .map_err(|e| RepoError::Message(format!("{alias}/{id}: {e}")))?;
    let current = sha256_of(&repo.root.join(&s.target_ref))?;
    let resolved = repo.load_resolution(&dir)?.is_some();
    let register = repo.load_authority_register()?;
    let performer = repo.performer();
    let eligible = register
        .holders(ActorRole::Authorizer)
        .filter(|a| a.actor_kind == ActorKind::Human)
        .filter(|a| a.may_authorize(&performer).is_ok())
        .map(|a| a.actor.clone())
        .collect();
    let prior = u32::try_from(s.corrections.len()).unwrap_or(u32::MAX);
    Ok(CorrectionRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        deliverable_id: id.to_owned(),
        target_ref: s.target_ref,
        title: s.title,
        resolved,
        recorded_digest: s.recorded,
        drift: current != head,
        chain_head: head,
        current_digest: current,
        prior_corrections: prior,
        next_sequence: prior + 1,
        eligible_correctors: eligible,
    })
}

fn load_response(path: &Utf8Path) -> Result<CorrectionResponse, RepoError> {
    let text = std::fs::read_to_string(path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    toml::from_str(&text).map_err(|e| RepoError::Message(format!("{path}: {e}")))
}

/// `war correct <alias> <deliverable-id> --response <file>`: ingest a human's
/// correction. Every refusal happens before anything is written.
pub fn ingest(
    repo: &Repository,
    alias: &str,
    id: &str,
    path: &Utf8Path,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let refuse = |report: &mut Report, rule: &'static str, why: String| {
        report.push(Diagnostic::error(rule, path.to_string(), why));
    };
    let response = load_response(path)?;
    if response.schema != RESPONSE_SCHEMA {
        refuse(
            &mut report,
            "correction.schema",
            format!("expected {RESPONSE_SCHEMA}, found {:?}", response.schema),
        );
        return Ok(report);
    }
    if response.warrant != alias || response.deliverable_id != id {
        refuse(
            &mut report,
            "correction.wrong-warrant",
            format!(
                "the response names {}/{}; ingesting for {alias}/{id}",
                response.warrant, response.deliverable_id
            ),
        );
        return Ok(report);
    }
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let s = match standing(repo, &one, alias, id) {
        Ok(s) => s,
        Err(e) => {
            refuse(&mut report, "correction.unknown-deliverable", e.to_string());
            return Ok(report);
        }
    };
    if repo.load_resolution(&dir)?.is_none() {
        refuse(
            &mut report,
            "correction.not-resolved",
            format!(
                "{alias} is not resolved. A correction supersedes a pin a resolution holds; \
                 before resolution, regenerate deliverables.toml"
            ),
        );
        return Ok(report);
    }
    let head = match chain_head(&s.recorded, &s.corrections) {
        Ok(h) => h,
        Err(e) => {
            refuse(
                &mut report,
                "correction.chain",
                format!("{alias}/{id}: {e}"),
            );
            return Ok(report);
        }
    };
    let current = sha256_of(&repo.root.join(&s.target_ref))?;
    if current == head {
        refuse(
            &mut report,
            "correction.no-drift",
            format!("{alias}/{id}: the file is at {head}; there is nothing to correct"),
        );
        return Ok(report);
    }
    if response.new_digest != current {
        refuse(
            &mut report,
            "correction.stale",
            format!(
                "{alias}/{id}: the response records new digest {} but the file is {current} — \
                 it corrects nothing",
                response.new_digest
            ),
        );
        return Ok(report);
    }
    if response.superseded_digest != head {
        refuse(
            &mut report,
            "correction.superseded-mismatch",
            format!(
                "{alias}/{id}: the response supersedes {} but the digest on record is {head} — \
                 it supersedes nothing that happened",
                response.superseded_digest
            ),
        );
        return Ok(report);
    }
    if response.reason.trim().is_empty() {
        refuse(
            &mut report,
            "correction.reason-empty",
            format!("{alias}/{id}: a correction with no reason is a change nobody can answer for"),
        );
        return Ok(report);
    }
    if let Err(e) = openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time) {
        refuse(
            &mut report,
            "correction.effective-time",
            format!("{alias}/{id}: effective_time {}", e),
        );
        return Ok(report);
    }

    // The signer, through the register. Never through the response's claims.
    let register = repo.load_authority_register()?;
    let Some(assignment) = register.actor(&response.corrected_by) else {
        refuse(
            &mut report,
            "correction.unknown-actor",
            format!(
                "{:?} holds no role assignment in docs/authority/roles.toml",
                response.corrected_by
            ),
        );
        return Ok(report);
    };
    if assignment.actor_kind == ActorKind::Agent {
        refuse(
            &mut report,
            "correction.agent",
            format!(
                "{:?} is an agent. §27.2: an agent SHALL NOT correct a delivered artifact — a \
                 performer that could move its own pins would be judging its own work twice",
                response.corrected_by
            ),
        );
        return Ok(report);
    }
    if let Err(e) = assignment.may_authorize(&repo.performer()) {
        refuse(
            &mut report,
            "correction.not-permitted",
            format!("{}: {e}", response.corrected_by),
        );
        return Ok(report);
    }

    let sequence = u32::try_from(s.corrections.len()).unwrap_or(u32::MAX - 1) + 1;
    let now = crate::gate_cmd::receipt::now_rfc3339_public();
    let correction = Correction {
        id: WarUuid::mint().to_string(),
        deliverable_id: id.to_owned(),
        target_ref: s.target_ref.clone(),
        sequence,
        superseded_digest: response.superseded_digest.clone(),
        new_digest: response.new_digest.clone(),
        reason: response.reason.clone(),
        kind: response.kind,
        authorized_by_ref: format!("person://{}", response.corrected_by),
        acting_role_ref: format!("role://{}", response.acting_role),
        effective_at: response.effective_time.clone(),
        recorded_at: now,
    };
    if let Err(e) = correction.validate() {
        refuse(
            &mut report,
            "correction.malformed",
            format!("{alias}/{id}: {e}"),
        );
        return Ok(report);
    }
    let record = CorrectionRecord {
        schema: CORRECTION_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        correction,
    };
    let cdir = dir.join("corrections");
    std::fs::create_dir_all(&cdir).map_err(|source| RepoError::Io {
        context: format!("could not create {cdir}"),
        source,
    })?;
    let out = cdir.join(format!("{id}-{sequence}.toml"));
    let body = toml::to_string_pretty(&record)
        .map_err(|e| RepoError::Message(format!("could not render the correction: {e}")))?;
    // `create_new`: a second correction is a second file, never an overwrite.
    let mut f = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out)
    {
        Ok(f) => f,
        Err(e) => {
            refuse(
                &mut report,
                "correction.exists",
                format!(
                    "{}: already exists ({e}); a correction is never overwritten",
                    repo.relative(&out)
                ),
            );
            return Ok(report);
        }
    };
    use std::io::Write as _;
    f.write_all(body.as_bytes())
        .map_err(|source| RepoError::Io {
            context: format!("could not write {out}"),
            source,
        })?;
    drop(f);
    let record_digest = sha256_of(&out)?;
    if let Some(v) = &one.validated {
        crate::journal_cmd::record(
            &dir,
            &v.uuid.to_string(),
            crate::journal_cmd::CORRECTION_RECORDED,
            &format!("person://{}", response.corrected_by),
            &format!(
                "{{\"deliverable\":\"{id}\",\"sequence\":{sequence},\"superseded\":\"{}\",\"new\":\"{}\",\"record_digest\":\"{record_digest}\",\"channel\":\"{}\"}}",
                response.superseded_digest,
                response.new_digest,
                response.signed_via.as_deref().unwrap_or("file")
            ),
        )?;
    }
    report.push(Diagnostic::pass(
        "correction.recorded",
        format!(
            "{alias}/{id}: correction {sequence} by {} — {} superseded by {} → {}",
            response.corrected_by,
            response.superseded_digest,
            response.new_digest,
            repo.relative(&out)
        ),
    ));
    Ok(report)
}

/// The head digest and the corrections behind it, for `war check`'s digest
/// comparison. `None` when the deliverable has no recorded digest to chain from.
pub fn head_for(
    corrections: &crate::repo::CorrectionSet,
    deliverable_id: &str,
    recorded: &str,
) -> (Vec<Correction>, Result<String, ChainError>) {
    let list = corrections.for_deliverable(deliverable_id);
    let head = chain_head(recorded, &list);
    (list, head)
}

/// Structural checks on the records themselves: parse, warrant, actor kind,
/// and the journal witness. The digest chain is checked where the digests are
/// compared (`check_deliverable_digests`), so the two cannot disagree.
pub fn check(repo: &Repository, one: &Loaded, alias: &str, report: &mut Report) {
    let set = match repo.load_corrections(&one.dir) {
        Ok(s) => s,
        Err(e) => {
            report.push(Diagnostic::error(
                "correction.unreadable",
                repo.relative(&one.dir.join("corrections")),
                format!("{alias}: {e}"),
            ));
            return;
        }
    };
    for (path, why) in &set.failures {
        report.push(Diagnostic::error(
            "correction.malformed",
            path.clone(),
            format!("{alias}: {why} — an unreadable correction is not an absent one"),
        ));
    }
    if set.records.is_empty() {
        return;
    }
    let register = repo.load_authority_register().ok();
    let journal = crate::journal_cmd::load(&one.dir).ok();
    for (path, rec) in &set.records {
        let c = &rec.correction;
        if rec.schema != CORRECTION_SCHEMA {
            report.push(Diagnostic::error(
                "correction.malformed",
                path.clone(),
                format!(
                    "{alias}: schema {:?}, expected {CORRECTION_SCHEMA}",
                    rec.schema
                ),
            ));
            continue;
        }
        if let Err(e) = c.validate() {
            report.push(Diagnostic::error(
                "correction.malformed",
                path.clone(),
                format!("{alias}: {e}"),
            ));
            continue;
        }
        if rec.warrant != alias {
            report.push(Diagnostic::error(
                "correction.wrong-warrant",
                path.clone(),
                format!("{alias}: the record names {:?}", rec.warrant),
            ));
            continue;
        }
        let actor = c.authorized_by_ref.trim_start_matches("person://");
        match register.as_ref().and_then(|r| r.actor(actor)) {
            None => report.push(Diagnostic::error(
                "correction.unknown-actor",
                path.clone(),
                format!("{alias}: {actor:?} holds no role assignment in docs/authority/roles.toml"),
            )),
            Some(a) if a.actor_kind == ActorKind::Agent => report.push(Diagnostic::error(
                "correction.agent",
                path.clone(),
                format!("{alias}: {actor:?} is an agent; §27.2"),
            )),
            Some(_) => {}
        }
        // The journal witness: the file as written, or not this file.
        let witnessed = journal.as_ref().and_then(|j| {
            j.events.iter().find_map(|e| {
                if e.event_type != crate::journal_cmd::CORRECTION_RECORDED {
                    return None;
                }
                let v: serde_json::Value = serde_json::from_str(&e.payload).ok()?;
                (v.get("deliverable")?.as_str()? == c.deliverable_id
                    && v.get("sequence")?.as_u64()? == u64::from(c.sequence))
                .then(|| v.get("record_digest")?.as_str().map(str::to_owned))
                .flatten()
            })
        });
        match witnessed {
            None => report.push(Diagnostic::error(
                "correction.unjournalled",
                path.clone(),
                format!(
                    "{alias}: no `correction.recorded` journal event names {} sequence {}; a \
                     correction nobody ingested is a file, not an act",
                    c.deliverable_id, c.sequence
                ),
            )),
            Some(expected) => match sha256_of(&repo.root.join(path)) {
                Ok(actual) if actual == expected => {}
                Ok(actual) => report.push(Diagnostic::error(
                    "correction.edited",
                    path.clone(),
                    format!(
                        "{alias}: the journal witnessed this record as {expected} and it is now \
                         {actual} — edited after authorization. A second correction is a second \
                         file ({}-{}.toml), never an edit",
                        c.deliverable_id,
                        c.sequence + 1
                    ),
                )),
                Err(e) => report.push(Diagnostic::error(
                    "correction.unreadable",
                    path.clone(),
                    format!("{alias}: {e}"),
                )),
            },
        }
    }
}
