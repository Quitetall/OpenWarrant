// SPDX-License-Identifier: Apache-2.0
//! Accepting a specification, record by record (FORMAT-1.0 §4.5,
//! Appendix C.2 and C.3).
//!
//! # What is held
//!
//! Acceptance is held **per record**: every unit and item of `SPEC.md` and
//! its parts, sections included. The documents' own `doc:` records are not
//! counted. An `accept` act covers a set of `[id, revision, content_digest]`
//! triples, and its subject digest is §9's digest, domain
//! `oh.war/act-subject/v1`, over that array sorted by id. A record is
//! `accepted` while some verified act names it at the revision and content
//! digest it has now. Editing a record moves its own revision only, so only
//! that record stops being accepted. The specification is `accepted` when
//! every record is and `partially_accepted` when some are. When none is it
//! has no acceptance state.
//!
//! # The two halves
//!
//! An agent may emit the request: `war plan spec accept` writes nothing and says
//! which records are unaccepted, with the subject digest a person would
//! sign. The act itself is a person's signed response,
//! `docs/authority/responses/SPEC-<12 hex>.response.toml`, schema
//! `oh.war/accept-response/v1`, with its `.sig`. `war sign spec` drafts and
//! signs it at a terminal. This module never signs. It reads an act as
//! verified only when [`crate::authority_check::verify`] says a human holding
//! the authorizer role signed it over that subject digest, and when every
//! triple it names is the digest its own subject digest was made from.
//!
//! # The corpus's acts (debug builds only)
//!
//! The conformance corpus gives acts as `acts/*.json` taken as verified: it
//! carries no signature (`conformance/format/README.md`). A debug build of
//! `war` reads such a directory from `OPENWARRANT_CORPUS_ACTS` and says so
//! on standard error. A release build has no such path: the code is not
//! compiled, so a production `war` believes only signed acts.

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::model::{Model, Record, State};
use crate::repo::{RepoError, Repository};

pub const REQUEST_SCHEMA: &str = "oh.war/accept-request/v1";
pub const RESPONSE_SCHEMA: &str = "oh.war/accept-response/v1";
/// Appendix C.3's digest domain.
pub const SUBJECT_DOMAIN: &str = "oh.war/act-subject/v1";
/// Every specification acceptance is named `SPEC-<12 hex of its digest>`.
pub const SUBJECT_PREFIX: &str = "SPEC-";
/// The root specification's own record.
pub const ROOT: &str = "doc:SPEC.md";
/// The state family of §11.1.
pub const KIND: &str = "acceptance";
/// Debug builds only: a directory of the corpus's `acts/*.json`.
#[cfg(debug_assertions)]
pub const CORPUS_ACTS_ENV: &str = "OPENWARRANT_CORPUS_ACTS";

/// `[id, revision, content_digest]`, as Appendix C.3 binds a record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Covered(pub String, pub String, pub String);

impl Covered {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.0
    }
}

/// Appendix C.3: §9's digest, domain `oh.war/act-subject/v1`, over the
/// array of `[id, revision, content_digest]` sorted by id.
#[must_use]
pub fn subject_digest(records: &[Covered]) -> String {
    #[derive(Serialize)]
    struct Domained<'a> {
        digest_domain: &'static str,
        payload: &'a [Covered],
    }
    let mut sorted = records.to_vec();
    sorted.sort();
    sorted.dedup();
    let bytes = serde_jcs::to_vec(&Domained {
        digest_domain: SUBJECT_DOMAIN,
        payload: &sorted,
    })
    .unwrap_or_default();
    openwarrant_core::format::sha256(&bytes)
}

/// The subject an acceptance over this digest is named for.
#[must_use]
pub fn subject_name(digest: &str) -> String {
    let hex = digest.trim_start_matches("sha256:");
    format!("{SUBJECT_PREFIX}{}", &hex[..hex.len().min(12)])
}

/// The specification as it reads now.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    /// Whether `SPEC.md` is one of its documents.
    pub has_root: bool,
    /// The records acceptance is held over, sorted by id.
    pub records: Vec<Covered>,
    /// Each record's type and source, for messages.
    pub kinds: BTreeMap<String, (String, String)>,
}

impl Spec {
    #[must_use]
    pub fn from_records(records: &[Record]) -> Option<Self> {
        let docs: BTreeSet<&str> = records
            .iter()
            .filter(|r| r.id.starts_with(crate::doc_index::ID_PREFIX) && r.kind == "spec")
            .map(|r| r.source.as_str())
            .collect();
        if docs.is_empty() {
            return None;
        }
        let mut out = Self {
            has_root: records.iter().any(|r| r.id == ROOT),
            ..Self::default()
        };
        for r in records.iter().filter(|r| {
            docs.contains(r.source.as_str()) && !r.id.starts_with(crate::doc_index::ID_PREFIX)
        }) {
            out.records.push(Covered(
                r.id.clone(),
                r.revision.clone(),
                r.content_digest.clone(),
            ));
            out.kinds
                .insert(r.id.clone(), (r.kind.clone(), r.source.clone()));
        }
        out.records.sort();
        out.records.dedup();
        Some(out)
    }

    /// The specification of `corpus`, read from its documents alone.
    #[must_use]
    pub fn of(corpus: &Corpus) -> Option<Self> {
        Self::from_records(&crate::model::spec_records(corpus))
    }
}

/// An act this module believes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAct {
    pub subject: String,
    pub accepted_by: String,
    pub records: Vec<Covered>,
    /// Where it was read from: a response, or a corpus file.
    pub source: String,
}

/// An act that was offered and is not believed, with the rule why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedAct {
    pub source: String,
    pub rule: &'static str,
    pub why: String,
}

#[derive(Debug, Clone, Default)]
pub struct Acts {
    pub verified: Vec<VerifiedAct>,
    pub refused: Vec<RefusedAct>,
}

/// What a person signs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptResponse {
    pub schema: String,
    /// `SPEC-<12 hex of subject_digest>`.
    pub subject: String,
    /// Appendix C.3's digest over `records`.
    pub subject_digest: String,
    /// The records this act accepts, sorted by id.
    pub records: Vec<Covered>,
    pub accepted_by: String,
    pub acting_role: String,
    pub meaning: String,
    pub effective_time: String,
}

/// What is put to the acceptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptRequest {
    pub schema: String,
    pub subject: String,
    pub subject_digest: String,
    /// The records the act would cover, sorted by id.
    pub records: Vec<Covered>,
    /// Of them, those an earlier act accepted at another revision: edited
    /// since.
    pub changed: Vec<String>,
    /// Of them, those no act has ever named.
    pub new: Vec<String>,
    /// Records of the specification in all.
    pub record_count: usize,
    pub eligible_acceptors: Vec<String>,
}

/// Whether the repository claims the authority profile (Appendix C): it
/// has `docs/authority/roles.toml` or an `[authority]` store.
#[must_use]
pub fn authority_claimed(repo: &Repository) -> bool {
    repo.config.authority.is_some()
        || crate::vfs::is_file(repo.root.join("docs/authority/roles.toml"))
}

fn responses_dir(repo: &Repository) -> Utf8PathBuf {
    repo.root.join("docs/authority/responses")
}

/// Every specification acceptance on record, each verified or refused.
#[must_use]
pub fn acts(repo: &Repository) -> Acts {
    let mut out = Acts::default();
    let dir = responses_dir(repo);
    let mut files: Vec<Utf8PathBuf> = crate::vfs::read_dir_utf8(&dir)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.starts_with(SUBJECT_PREFIX) && n.ends_with(".response.toml"))
        })
        .collect();
    files.sort();
    for path in files {
        match verify_file(repo, &path) {
            Ok(act) => out.verified.push(act),
            Err((rule, why)) => out.refused.push(RefusedAct {
                source: repo.relative(&path),
                rule,
                why,
            }),
        }
    }
    #[cfg(debug_assertions)]
    corpus_acts(&mut out);
    out
}

/// Debug builds only: the corpus's acts, taken as verified because the
/// corpus says they are (`conformance/format/README.md`).
#[cfg(debug_assertions)]
fn corpus_acts(out: &mut Acts) {
    #[derive(Deserialize)]
    struct CorpusAct {
        act: String,
        records: Vec<Covered>,
    }
    let Some(dir) = std::env::var_os(CORPUS_ACTS_ENV) else {
        return;
    };
    let Some(dir) = Utf8PathBuf::from_path_buf(dir.into()).ok() else {
        return;
    };
    let mut files: Vec<Utf8PathBuf> = crate::vfs::read_dir_utf8(&dir)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.extension() == Some("json"))
        .collect();
    files.sort();
    static SAID: std::sync::Once = std::sync::Once::new();
    SAID.call_once(|| {
        eprintln!(
            "war: {CORPUS_ACTS_ENV}: acceptance acts in {dir} are taken as verified, unsigned \
             (a debug build, for the conformance corpus only)"
        );
    });
    for f in files {
        let parsed = crate::vfs::read(&f)
            .ok()
            .and_then(|b| serde_json::from_slice::<CorpusAct>(&b).ok());
        match parsed {
            Some(a) if a.act == "accept" => out.verified.push(VerifiedAct {
                subject: subject_name(&subject_digest(&a.records)),
                accepted_by: "corpus".to_owned(),
                records: a.records,
                source: f.to_string(),
            }),
            Some(_) => {}
            None => out.refused.push(RefusedAct {
                source: f.to_string(),
                rule: "spec.response-malformed",
                why: "not an `{act, records}` corpus act".to_owned(),
            }),
        }
    }
}

/// One response believed, or the rule it fails.
fn verify_file(repo: &Repository, path: &Utf8Path) -> Result<VerifiedAct, (&'static str, String)> {
    let text = crate::vfs::read_to_string(path)
        .map_err(|e| ("spec.response-malformed", format!("could not be read: {e}")))?;
    let response: AcceptResponse =
        toml::from_str(&text).map_err(|e| ("spec.response-malformed", e.to_string()))?;
    consistent(&response)?;
    let name = path.file_name().unwrap_or_default();
    if !name.starts_with(&response.subject) {
        return Err((
            "spec.response-subject",
            format!(
                "{name} carries subject {}; a response is named for its subject",
                response.subject
            ),
        ));
    }
    permitted(repo, &response.accepted_by)?;
    let verdict = crate::authority_check::verify(
        repo,
        crate::authority_check::Act::AcceptSpec,
        &response.subject,
        &response.accepted_by,
        Some(&response.subject_digest),
    );
    if !verdict.is_signed() {
        return Err((verdict.rule(), verdict.why()));
    }
    Ok(VerifiedAct {
        subject: response.subject,
        accepted_by: response.accepted_by,
        records: response.records,
        source: repo.relative(path),
    })
}

/// A response's own fields agree: its schema, a set of records, the digest
/// over exactly them, and the name that digest gives.
fn consistent(r: &AcceptResponse) -> Result<(), (&'static str, String)> {
    if r.schema != RESPONSE_SCHEMA {
        return Err((
            "spec.response-schema",
            format!("schema {:?}, want {RESPONSE_SCHEMA}", r.schema),
        ));
    }
    if r.records.is_empty() {
        return Err((
            "spec.response-empty",
            "an accept names at least one record".to_owned(),
        ));
    }
    let want = subject_digest(&r.records);
    if r.subject_digest != want {
        return Err((
            "spec.subject-digest",
            format!(
                "subject_digest {} is not the digest of the records it names ({want}); a \
                 signature over one set cannot accept another",
                r.subject_digest
            ),
        ));
    }
    if r.subject != subject_name(&want) {
        return Err((
            "spec.response-subject",
            format!(
                "subject {} is not {}, the name its digest gives",
                r.subject,
                subject_name(&want)
            ),
        ));
    }
    Ok(())
}

/// The actor is in the register, human, and may accept (an authorizer who
/// is not this repository's performer).
fn permitted(repo: &Repository, actor: &str) -> Result<(), (&'static str, String)> {
    let register = repo
        .load_authority_register()
        .map_err(|e| ("spec.not-permitted", e.to_string()))?;
    let Some(assignment) = register.actor(actor) else {
        return Err((
            "spec.unknown-actor",
            format!("{actor:?} holds no role assignment in docs/authority/roles.toml"),
        ));
    };
    assignment
        .may_authorize(&repo.performer())
        .map_err(|e| ("spec.not-permitted", e.to_string()))
}

/// The acceptance of each record now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Acceptance {
    /// Records some verified act names at their revision and digest now.
    pub accepted: BTreeSet<String>,
    /// Records no act names as they are now, sorted.
    pub unaccepted: Vec<String>,
    /// Of the unaccepted, those an act named at another revision.
    pub lapsed: BTreeSet<String>,
    /// `accepted`, `partially_accepted`, or none.
    pub document: Option<&'static str>,
}

#[must_use]
pub fn compute(spec: &Spec, acts: &[VerifiedAct]) -> Acceptance {
    let named: BTreeSet<&Covered> = acts.iter().flat_map(|a| &a.records).collect();
    let ever: BTreeSet<&str> = named.iter().map(|c| c.id()).collect();
    let mut out = Acceptance::default();
    for c in &spec.records {
        if named.contains(c) {
            out.accepted.insert(c.0.clone());
        } else {
            out.unaccepted.push(c.0.clone());
            if ever.contains(c.id()) {
                out.lapsed.insert(c.0.clone());
            }
        }
    }
    if !spec.records.is_empty() && !out.accepted.is_empty() {
        out.document = Some(if out.unaccepted.is_empty() {
            "accepted"
        } else {
            "partially_accepted"
        });
    }
    out
}

/// §11.1's `acceptance` states for `oh.war/model/v1`: `accepted`, recorded,
/// for each accepted record; for `doc:SPEC.md`, `accepted` or
/// `partially_accepted`, computed.
#[must_use]
pub fn model_states(corpus: &Corpus, model: &Model) -> Vec<State> {
    let Some(spec) = Spec::from_records(&model.records) else {
        return Vec::new();
    };
    let acts = acts(corpus.repo());
    let a = compute(&spec, &acts.verified);
    let state = |record: &str, value: &str, provenance: &str| State {
        record: record.to_owned(),
        kind: KIND.to_owned(),
        value: value.to_owned(),
        provenance: provenance.to_owned(),
        facet: None,
        refines: None,
        lapsed: false,
    };
    let mut out: Vec<State> = a
        .accepted
        .iter()
        .map(|id| state(id, "accepted", "recorded"))
        .collect();
    if let (Some(v), true) = (a.document, spec.has_root) {
        out.push(state(ROOT, v, "computed"));
    }
    out
}

/// The acceptance request over `records`.
fn request_over(
    repo: &Repository,
    spec: &Spec,
    acts: &Acts,
    records: Vec<Covered>,
) -> AcceptRequest {
    let a = compute(spec, &acts.verified);
    let ever: BTreeSet<String> = acts
        .verified
        .iter()
        .flat_map(|x| x.records.iter().map(|c| c.0.clone()))
        .collect();
    let digest = subject_digest(&records);
    let eligible = repo
        .load_authority_register()
        .map(|register| {
            register
                .holders(openwarrant_core::authority::ActorRole::Authorizer)
                .filter(|x| x.may_authorize(&repo.performer()).is_ok())
                .map(|x| x.actor.clone())
                .collect()
        })
        .unwrap_or_default();
    AcceptRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        subject: subject_name(&digest),
        subject_digest: digest,
        changed: records
            .iter()
            .filter(|c| a.lapsed.contains(&c.0))
            .map(|c| c.0.clone())
            .collect(),
        new: records
            .iter()
            .filter(|c| !ever.contains(&c.0))
            .map(|c| c.0.clone())
            .collect(),
        records,
        record_count: spec.records.len(),
        eligible_acceptors: eligible,
    }
}

/// The request `war plan spec accept` emits: the unaccepted records, or with
/// `all` every record. `None` when there is no specification, or nothing
/// to accept.
pub fn request(corpus: &Corpus, all: bool) -> Result<Option<AcceptRequest>, RepoError> {
    let repo = corpus.repo();
    let Some(spec) = Spec::of(corpus) else {
        return Ok(None);
    };
    let acts = acts(repo);
    let a = compute(&spec, &acts.verified);
    let records: Vec<Covered> = spec
        .records
        .iter()
        .filter(|c| all || !a.accepted.contains(&c.0))
        .cloned()
        .collect();
    if records.is_empty() {
        return Ok(None);
    }
    Ok(Some(request_over(repo, &spec, &acts, records)))
}

/// The acceptance waiting on a person, for `war sign` and `war view next`: under
/// the authority profile, the specification's unaccepted records.
pub fn pending_request(corpus: &Corpus) -> Result<Option<AcceptRequest>, RepoError> {
    if !authority_claimed(corpus.repo()) {
        return Ok(None);
    }
    request(corpus, false)
}

/// Judge a response as the act would be judged, and record nothing: the
/// signed response is itself the act (Appendix C.2). Under `Record`, the
/// signature must verify; under `DryRun` (before signing) it is not asked.
pub fn ingest_with(
    repo: &Repository,
    response_path: &Utf8Path,
    mode: crate::sign::IngestMode,
) -> Result<Report, RepoError> {
    let corpus = Corpus::new(repo);
    let corpus = &corpus;
    let mut report = Report::default();
    let refuse = |report: &mut Report, rule: &str, why: String| {
        report.push(Diagnostic::error(rule, response_path.to_string(), why));
    };
    let text = crate::vfs::read_to_string(response_path).map_err(|source| RepoError::Io {
        context: format!("could not read {response_path}"),
        source,
    })?;
    let response: AcceptResponse = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            refuse(&mut report, "spec.response-malformed", e.to_string());
            return Ok(report);
        }
    };
    if let Err((rule, why)) = consistent(&response) {
        refuse(&mut report, rule, why);
        return Ok(report);
    }
    if let Err(e) = openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time) {
        refuse(
            &mut report,
            "spec.effective-time",
            format!("effective_time {:?}: {e}", response.effective_time),
        );
        return Ok(report);
    }
    let Some(spec) = Spec::of(corpus) else {
        refuse(
            &mut report,
            "spec.no-specification",
            "this repository has no Format 1.0 specification (SPEC.md)".to_owned(),
        );
        return Ok(report);
    };
    // Appendix C.2: the subject digest equals the subject's digest now.
    // Every record the act names must be a record of the specification with
    // that revision and content digest now; an act over a record since
    // edited would accept bytes nobody can read any more.
    let now: BTreeMap<&str, &Covered> = spec.records.iter().map(|c| (c.id(), c)).collect();
    let stale: Vec<String> = response
        .records
        .iter()
        .filter(|c| now.get(c.id()).is_none_or(|n| *n != *c))
        .map(|c| match now.get(c.id()) {
            Some(n) => format!("{} is at {} now, the act names {}", c.0, n.1, c.1),
            None => format!("{} is not a record of the specification", c.0),
        })
        .collect();
    if !stale.is_empty() {
        refuse(
            &mut report,
            "spec.stale-digest",
            format!(
                "the act's subject is not the specification's digest now: {}. Emit the request \
                 again (`war plan spec accept`) and sign that",
                stale.join("; ")
            ),
        );
        return Ok(report);
    }
    if let Err((rule, why)) = permitted(repo, &response.accepted_by) {
        refuse(&mut report, rule, why);
        return Ok(report);
    }
    if mode == crate::sign::IngestMode::DryRun {
        report.push(Diagnostic::pass(
            "spec.would-accept",
            format!(
                "{} would accept {} record(s) by {}; nothing written",
                response.subject,
                response.records.len(),
                response.accepted_by
            ),
        ));
        return Ok(report);
    }
    let verdict = crate::authority_check::verify(
        repo,
        crate::authority_check::Act::AcceptSpec,
        &response.subject,
        &response.accepted_by,
        Some(&response.subject_digest),
    );
    if !verdict.is_signed() {
        refuse(&mut report, verdict.rule(), verdict.why());
        return Ok(report);
    }
    report.push(Diagnostic::pass(
        "spec.accepted",
        format!(
            "{} accepts {} record(s), by {} acting as {}: {}",
            response.subject,
            response.records.len(),
            response.accepted_by,
            response.acting_role,
            verdict.why()
        ),
    ));
    Ok(report)
}

/// One unaccepted record and what cites it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unaccepted {
    pub record: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
    /// An act accepted it at another revision: it was edited since.
    pub lapsed: bool,
    /// The work documents and obligations that cite it, directly or
    /// through records that do (`war impact`).
    pub cited_by: Vec<String>,
}

/// `war plan spec status`: each record's acceptance, and the work citing each
/// unaccepted one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    pub schema: &'static str,
    /// `accepted`, `partially_accepted` or `none`.
    pub document: String,
    pub records: usize,
    pub accepted: usize,
    pub unaccepted: Vec<Unaccepted>,
    pub acts: Vec<String>,
}

/// The work documents and obligations among the records that cite `id`,
/// transitively, read from the model as `war impact` reads it.
#[must_use]
pub fn citing_work(corpus: &Corpus, model: &Model, id: &str) -> Vec<String> {
    let Ok((_, Some(impact))) = crate::impact::build_on(corpus, model, id) else {
        return Vec::new();
    };
    let mut out: BTreeSet<String> = BTreeSet::new();
    for a in &impact.affected {
        if matches!(
            a.kind.as_str(),
            "warrant" | "ticket" | "obligation" | "item"
        ) {
            out.insert(a.id.clone());
        }
    }
    for d in &impact.documents {
        if matches!(d.kind.as_str(), "warrant" | "ticket") {
            out.insert(d.id.clone());
        }
    }
    out.into_iter().collect()
}

pub fn status(corpus: &Corpus) -> Result<(Report, Option<Status>), RepoError> {
    let mut report = Report::default();
    let model = crate::model::build(corpus)?;
    let Some(spec) = Spec::from_records(&model.records) else {
        report.push(Diagnostic::warn(
            "spec.no-specification",
            "SPEC.md".to_owned(),
            "no Format 1.0 specification: SPEC.md with profile `spec`".to_owned(),
        ));
        return Ok((report, None));
    };
    let acts = acts(corpus.repo());
    for r in &acts.refused {
        report.push(Diagnostic::warn(
            r.rule,
            r.source.clone(),
            format!("not counted: {}", r.why),
        ));
    }
    let a = compute(&spec, &acts.verified);
    let unaccepted: Vec<Unaccepted> = a
        .unaccepted
        .iter()
        .map(|id| {
            let (kind, source) = spec.kinds.get(id).cloned().unwrap_or_default();
            Unaccepted {
                record: id.clone(),
                kind,
                source,
                lapsed: a.lapsed.contains(id),
                cited_by: citing_work(corpus, &model, id),
            }
        })
        .collect();
    for u in unaccepted.iter().filter(|u| !u.cited_by.is_empty()) {
        report.push(Diagnostic::warn(
            "spec.unaccepted-cited",
            u.record.clone(),
            format!(
                "{} is not accepted{} and is cited by {}",
                u.record,
                if u.lapsed {
                    " (edited since its acceptance)"
                } else {
                    ""
                },
                u.cited_by.join(", ")
            ),
        ));
    }
    let st = Status {
        schema: "oh.war/spec-status/v1",
        document: a.document.unwrap_or("none").to_owned(),
        records: spec.records.len(),
        accepted: a.accepted.len(),
        unaccepted,
        acts: acts.verified.iter().map(|x| x.source.clone()).collect(),
    };
    report.push(Diagnostic::pass(
        "spec.status",
        format!(
            "specification {}: {} of {} record(s) accepted",
            st.document, st.accepted, st.records
        ),
    ));
    Ok((report, Some(st)))
}

/// For `war impact`: when `id` is a specification record that is not
/// accepted under the authority profile, the warning that names the work
/// and obligations citing it.
#[must_use]
pub fn impact_warning(corpus: &Corpus, model: &Model, id: &str) -> Option<Diagnostic> {
    let spec = Spec::from_records(&model.records)?;
    spec.kinds.get(id)?;
    let acts = acts(corpus.repo());
    if !authority_claimed(corpus.repo()) && acts.verified.is_empty() {
        return None;
    }
    let a = compute(&spec, &acts.verified);
    if a.accepted.contains(id) {
        return None;
    }
    let cited = citing_work(corpus, model, id);
    Some(Diagnostic::warn(
        "impact.unaccepted",
        id.to_owned(),
        format!(
            "{id} is not accepted{}; {}. `war sign spec` accepts it (a person's act)",
            if a.lapsed.contains(id) {
                ": it was edited after the act that accepted it"
            } else {
                ""
            },
            if cited.is_empty() {
                "no work or obligation cites it".to_owned()
            } else {
                format!("work and obligations citing it: {}", cited.join(", "))
            }
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: &str, rev: &str) -> Covered {
        Covered(id.into(), format!("sha256:{rev}"), format!("sha256:c{rev}"))
    }

    fn spec(records: &[Covered]) -> Spec {
        Spec {
            has_root: true,
            records: records.to_vec(),
            kinds: BTreeMap::new(),
        }
    }

    fn act(records: &[Covered]) -> VerifiedAct {
        VerifiedAct {
            subject: subject_name(&subject_digest(records)),
            accepted_by: "alice".into(),
            records: records.to_vec(),
            source: "t".into(),
        }
    }

    #[test]
    fn the_subject_digest_is_order_free_and_moves_with_any_triple() {
        let a = [c("B", "1"), c("A", "2")];
        let b = [c("A", "2"), c("B", "1")];
        assert_eq!(subject_digest(&a), subject_digest(&b));
        assert_ne!(
            subject_digest(&a),
            subject_digest(&[c("A", "2"), c("B", "9")])
        );
        assert!(subject_name(&subject_digest(&a)).starts_with("SPEC-"));
        assert_eq!(subject_name(&subject_digest(&a)).len(), 5 + 12);
    }

    /// The Appendix C.3 digest is §9's: JCS of `{digest_domain, payload}`.
    #[test]
    fn the_subject_digest_is_the_domained_jcs() {
        let recs = [c("A", "1")];
        let want = openwarrant_core::format::sha256(
            br#"{"digest_domain":"oh.war/act-subject/v1","payload":[["A","sha256:1","sha256:c1"]]}"#,
        );
        assert_eq!(subject_digest(&recs), want);
    }

    #[test]
    fn an_edited_record_lapses_only_itself() {
        let before = [c("A", "1"), c("B", "1"), c("C", "1")];
        let after = [c("A", "1"), c("B", "2"), c("C", "1")];
        let a = compute(&spec(&after), &[act(&before)]);
        assert_eq!(a.document, Some("partially_accepted"));
        assert_eq!(
            a.accepted.iter().cloned().collect::<Vec<_>>(),
            ["A".to_owned(), "C".to_owned()]
        );
        assert_eq!(a.unaccepted, ["B"]);
        assert!(a.lapsed.contains("B"));
        // A later accept naming the changed record alone restores it.
        let a = compute(&spec(&after), &[act(&before), act(&[c("B", "2")])]);
        assert_eq!(a.document, Some("accepted"));
        assert!(a.unaccepted.is_empty());
    }

    #[test]
    fn no_act_no_acceptance_state() {
        let a = compute(&spec(&[c("A", "1")]), &[]);
        assert_eq!(a.document, None);
        assert!(a.accepted.is_empty());
        assert!(a.lapsed.is_empty());
    }

    /// A content digest that moved under the same revision is not the
    /// record the act named either.
    #[test]
    fn both_digests_must_match() {
        let named = Covered("A".into(), "sha256:1".into(), "sha256:old".into());
        let a = compute(&spec(&[c("A", "1")]), &[act(&[named])]);
        assert_eq!(a.document, None);
        assert!(a.lapsed.contains("A"));
    }

    #[test]
    fn a_response_whose_digest_is_not_its_records_is_refused() {
        let records = vec![c("A", "1")];
        let digest = subject_digest(&records);
        let mut r = AcceptResponse {
            schema: RESPONSE_SCHEMA.into(),
            subject: subject_name(&digest),
            subject_digest: digest,
            records,
            accepted_by: "alice".into(),
            acting_role: "authorizer".into(),
            meaning: "m".into(),
            effective_time: "2026-10-14T10:00:00Z".into(),
        };
        assert!(consistent(&r).is_ok());
        r.records.push(c("B", "1"));
        assert_eq!(consistent(&r).unwrap_err().0, "spec.subject-digest");
        r.records.pop();
        r.subject = "SPEC-000000000000".into();
        assert_eq!(consistent(&r).unwrap_err().0, "spec.response-subject");
        r.records.clear();
        assert_eq!(consistent(&r).unwrap_err().0, "spec.response-empty");
    }

    #[test]
    fn a_response_round_trips_through_toml() {
        let records = vec![c("A", "1"), c("B", "2")];
        let digest = subject_digest(&records);
        let r = AcceptResponse {
            schema: RESPONSE_SCHEMA.into(),
            subject: subject_name(&digest),
            subject_digest: digest,
            records,
            accepted_by: "alice".into(),
            acting_role: "authorizer".into(),
            meaning: "m".into(),
            effective_time: "2026-10-14T10:00:00Z".into(),
        };
        let text = toml::to_string_pretty(&r).unwrap();
        assert_eq!(toml::from_str::<AcceptResponse>(&text).unwrap(), r);
    }
}
