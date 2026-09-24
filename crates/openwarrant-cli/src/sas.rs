// SPDX-License-Identifier: Apache-2.0
//! `war sas` — the SAS as a controlled document (SAS §101; §34.1–§34.4).
//!
//! # Two halves, again
//!
//! `war sas propose <version>` writes a proposed revision: the document's
//! digest, a §106 snapshot, the diff against its predecessor. `war sas accept
//! <version>` emits a request; `--response <file>` ingests a human's
//! acceptance through the authority register, which refuses every agent
//! regardless of what the response claims. The split is the one `war
//! authorize` uses and exists for the same reason: an agent may propose a
//! revision of the document that governs it, and may not accept one.
//!
//! # What a refused acceptance writes
//!
//! Nothing. A rejected acceptance must not become a file that later reads as
//! an accepted revision.

use std::fs;

use camino::Utf8Path;
use openwarrant_compiler::sha256_hex;
use openwarrant_core::authority::ActorRole;
use openwarrant_core::sas::{
    SasAcceptance, SasRevision, SasRevisionState, Section106Diff, section_106,
};
use openwarrant_core::sas_sections::{self, Section, SectionDiff};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const ACCEPT_REQUEST_SCHEMA: &str = "oh.war/sas-acceptance-request/v1";
pub const ACCEPT_RESPONSE_SCHEMA: &str = "oh.war/sas-acceptance-response/v1";

/// Propose the document as it stands, as `version`.
pub fn propose(repo: &Repository, version: &str) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (path, bytes) = repo.sas_document()?;
    let sha256 = sha256_hex(&bytes);
    let text = String::from_utf8_lossy(&bytes);
    let requirements = section_106(&text);
    if requirements.is_empty() {
        return Err(RepoError::Message(format!(
            "{path}: §106 could not be read — no `| <PREFIX>-SAS-RQ-NNN | … |` rows"
        )));
    }

    let existing = repo.load_sas_revisions()?;
    if existing.iter().any(|r| r.version == version) {
        return Err(RepoError::Message(format!(
            "a revision named {version:?} already exists; a revision is proposed once"
        )));
    }
    let predecessor = repo.latest_sas_revision()?;
    let (pred_version, architecture_changing) = match &predecessor {
        Some(p) => {
            let d = Section106Diff::between(&p.requirements, &requirements);
            d.check_stability()
                .map_err(|e| RepoError::Message(e.to_string()))?;
            report.push(Diagnostic::pass(
                "sas.diff",
                format!("§106 against {}: {}", p.version, d.summary()),
            ));
            (Some(p.version.clone()), d.is_architecture_changing())
        }
        None => (None, false),
    };

    let record = SasRevision::proposed(
        version,
        repo.relative(&path),
        sha256.clone(),
        pred_version,
        requirements,
        architecture_changing,
    );
    record
        .validate()
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let out = repo.sas_revision_path(version);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    write_toml(&out, &record)?;
    report.push(Diagnostic::pass(
        "sas.proposed",
        format!(
            "{version} proposed at sha256:{sha256} ({} requirements in §106; architecture-changing: {}) → {}",
            record.requirements.len(),
            architecture_changing,
            repo.relative(&out)
        ),
    ));
    report.note(
        "Proposed, not accepted. §101.2's accepted revision needs a human: `war sas accept <version>` \
         emits the request, `--response <file>` ingests the signature."
            .to_owned(),
    );
    Ok(report)
}

/// What is put to the acceptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptRequest {
    pub schema: String,
    pub version: String,
    pub source: String,
    pub sha256: String,
    pub predecessor: Option<String>,
    pub architecture_changing: bool,
    /// Whether §101.3 requires an `adr_ref` in the response.
    pub adr_required: bool,
    pub diff: Section106Diff,
    pub requirement_count: usize,
    pub eligible_acceptors: Vec<String>,
}

/// What the acceptor returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptResponse {
    pub schema: String,
    pub version: String,
    /// Must equal the record's AND the document's current digest.
    pub sha256: String,
    pub accepted_by: String,
    pub acting_role: String,
    pub meaning: String,
    pub effective_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adr_ref: Option<String>,
}

pub fn accept_request(repo: &Repository, version: &str) -> Result<AcceptRequest, RepoError> {
    let record = find(repo, version)?;
    let predecessor = record
        .predecessor
        .as_deref()
        .map(|v| find(repo, v))
        .transpose()?;
    // Record against record, not against the document on disk: the request is
    // about the PROPOSAL as recorded, and ingestion separately refuses to accept
    // it if the bytes have since moved (the stale-digest check). Diffing against
    // disk here would describe a document nobody has proposed.
    let diff = predecessor
        .as_ref()
        .map(|p| Section106Diff::between(&p.requirements, &record.requirements))
        .unwrap_or_default();
    let register = repo.load_authority_register()?;
    Ok(AcceptRequest {
        schema: ACCEPT_REQUEST_SCHEMA.to_owned(),
        version: record.version.clone(),
        source: record.source.clone(),
        sha256: record.sha256.clone(),
        predecessor: record.predecessor.clone(),
        architecture_changing: record.architecture_changing,
        adr_required: record.architecture_changing,
        diff,
        requirement_count: record.requirements.len(),
        eligible_acceptors: register
            .holders(ActorRole::Authorizer)
            .filter(|a| a.may_authorize(&repo.performer()).is_ok())
            .map(|a| a.actor.clone())
            .collect(),
    })
}

pub fn accept_ingest(
    repo: &Repository,
    version: &str,
    response_path: &Utf8Path,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let record = find(repo, version)?;
    let text = fs::read_to_string(response_path).map_err(|source| RepoError::Io {
        context: format!("could not read {response_path}"),
        source,
    })?;
    let response: AcceptResponse = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            report.push(Diagnostic::error(
                "sas.response-malformed",
                response_path.to_string(),
                e.to_string(),
            ));
            return Ok(report);
        }
    };
    let refuse = |report: &mut Report, rule: &str, why: String| {
        report.push(Diagnostic::error(rule, response_path.to_string(), why));
    };
    if response.schema != ACCEPT_RESPONSE_SCHEMA {
        refuse(
            &mut report,
            "sas.response-schema",
            format!("unknown schema {:?}", response.schema),
        );
        return Ok(report);
    }
    if response.version != version {
        refuse(
            &mut report,
            "sas.response-version",
            format!("response names {:?}, ingesting {version}", response.version),
        );
        return Ok(report);
    }
    if let Err(e) = openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time) {
        refuse(
            &mut report,
            "sas.effective-time",
            format!(
                "effective_time {:?} is not an RFC 3339 UTC timestamp ({e}); a record dated \
                 \"soon\" cannot be ordered against any other",
                response.effective_time
            ),
        );
        return Ok(report);
    }
    // The signature is over a digest; it must match the record AND the bytes on
    // disk right now. A document edited between proposal and acceptance is a
    // different document.
    let (_, bytes) = repo.sas_document()?;
    let current = sha256_hex(&bytes);
    if response.sha256 != record.sha256 || current != record.sha256 {
        refuse(
            &mut report,
            "sas.stale-digest",
            format!(
                "response signs {}, record holds {}, document is now {current}. §101.6 is about the \
                 EXACT revision; re-propose and sign again",
                response.sha256, record.sha256
            ),
        );
        return Ok(report);
    }
    let register = repo.load_authority_register()?;
    let Some(assignment) = register.actor(&response.accepted_by) else {
        refuse(
            &mut report,
            "sas.unknown-actor",
            format!(
                "{:?} holds no role assignment in docs/authority/roles.toml",
                response.accepted_by
            ),
        );
        return Ok(report);
    };
    if let Err(e) = assignment.may_authorize(&repo.performer()) {
        refuse(&mut report, "sas.not-permitted", e.to_string());
        return Ok(report);
    }
    // §101.3's ADR, named either way a person would name it: the path to the
    // atom, or the local alias the atom declares. An alias is what `war sign
    // --adr` is given and what the ADR calls itself, so resolving it here is
    // the difference between a refusal and a signature for the same intent.
    if let Some(adr) = &response.adr_ref
        && !repo.root.join(adr).is_file()
    {
        let known = repo.load_adrs().map(|a| a.records).unwrap_or_default();
        if !known.iter().any(|r| &r.local_alias == adr) {
            let aliases: Vec<&str> = known.iter().map(|r| r.local_alias.as_str()).collect();
            refuse(
                &mut report,
                "sas.adr-missing",
                format!(
                    "adr_ref {adr:?} is neither a file in this repository nor the local alias of a recorded ADR. Recorded: {}",
                    if aliases.is_empty() {
                        "none".to_owned()
                    } else {
                        aliases.join(", ")
                    }
                ),
            );
            return Ok(report);
        }
    }
    // The same exception the other three seams carry: the revision is already
    // accepted, this response accepts the same digest, and no verified
    // signature exists for it. Then the record is left alone and the signature
    // is what changes — `record.accept()` below would refuse it outright,
    // leaving an accepted-but-unsigned revision with no way to ever be signed.
    if let Some(existing) = &record.acceptance
        && record.state == openwarrant_core::sas::SasRevisionState::Accepted
        && !crate::authority_check::verify_excluding(
            repo,
            crate::authority_check::Act::Accept,
            &format!("SAS-{version}"),
            &existing.accepted_by,
            Some(&record.sha256),
            Some(response_path),
        )
        .is_signed()
    {
        report.push(Diagnostic::warn(
            "sas.signature-supplied",
            response_path.to_string(),
            format!(
                "{version} was accepted without a verified signature; this signature supplies                  it. The revision record is unchanged"
            ),
        ));
        return Ok(report);
    }
    let accepted = record
        .accept(SasAcceptance {
            accepted_by: response.accepted_by.clone(),
            actor_kind: assignment.actor_kind,
            acting_role: response.acting_role.clone(),
            meaning: response.meaning.clone(),
            effective_time: response.effective_time.clone(),
            adr_ref: response.adr_ref.clone(),
        })
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let out = repo.sas_revision_path(version);
    write_toml(&out, &accepted)?;
    report.push(Diagnostic::pass(
        "sas.accepted",
        format!(
            "{version} accepted by {} acting as {} → {}",
            response.accepted_by,
            response.acting_role,
            repo.relative(&out)
        ),
    ));
    Ok(report)
}

/// §106 of the document as it stands versus a candidate document.
pub fn diff(repo: &Repository, candidate: &Utf8Path) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (_, bytes) = repo.sas_document()?;
    let current = section_106(&String::from_utf8_lossy(&bytes));
    let next_text = fs::read_to_string(candidate).map_err(|source| RepoError::Io {
        context: format!("could not read {candidate}"),
        source,
    })?;
    sections_diff(&bytes, next_text.as_bytes(), candidate, &mut report);
    let next = section_106(&next_text);
    if next.is_empty() {
        report.push(Diagnostic::error(
            "sas.no-index",
            candidate.to_string(),
            "no §106 rows found; a candidate with no requirement index cannot be compared",
        ));
        return Ok(report);
    }
    let d = Section106Diff::between(&current, &next);
    for id in &d.added {
        report.push(Diagnostic::pass(
            "sas.diff.added",
            format!("{id}: {}", next[id]),
        ));
    }
    for (id, before, after) in &d.retitled {
        report.push(Diagnostic::warn(
            "sas.diff.retitled",
            candidate.to_string(),
            format!("{id}: {before:?} → {after:?} (architecture-changing; needs an ADR to accept)"),
        ));
    }
    for id in &d.removed {
        report.push(Diagnostic::error(
            "sas.diff.removed",
            candidate.to_string(),
            format!(
                "{id} — {}",
                openwarrant_core::sas::SasError::IdRemoved { id: id.clone() }
            ),
        ));
    }
    if !d.is_architecture_changing() {
        report.push(Diagnostic::pass("sas.diff", "§106 unchanged".to_owned()));
    }
    report.note(format!(
        "{}; architecture-changing: {}",
        d.summary(),
        d.is_architecture_changing()
    ));
    Ok(report)
}

/// The section half of `war sas diff` (OW-WAR-0125): the candidate's split is
/// shown to be lossless, then every section added, removed or changed against
/// the document in force is named by id. §106's comparison, beside it, is
/// untouched: a section that changed says where the prose moved, not whether
/// a requirement did.
fn sections_diff(current: &[u8], candidate: &[u8], path: &Utf8Path, report: &mut Report) {
    let next = sas_sections::split(candidate);
    let (joined, digest) = (
        sha256_hex(&sas_sections::join(&next)),
        sha256_hex(candidate),
    );
    if joined == digest {
        report.push(Diagnostic::pass(
            "sas.sections",
            format!(
                "candidate split into {} section(s); joined sha256:{joined} is the candidate's \
                 own sha256",
                next.len()
            ),
        ));
    } else {
        report.push(Diagnostic::error(
            "sas.section-split",
            path.to_string(),
            format!(
                "the candidate's sections join to sha256:{joined}, not its own sha256:{digest}; \
                 a lossy split names nothing reliably"
            ),
        ));
        return;
    }
    let before = sas_sections::split(current);
    let d = SectionDiff::between(&before, &next);
    for id in &d.added {
        report.push(Diagnostic::pass(
            "sas.diff.section-added",
            format!("section {id} added"),
        ));
    }
    for id in &d.removed {
        // A citation of it at a later revision will not resolve (R-001).
        report.push(Diagnostic::warn(
            "sas.diff.section-removed",
            path.to_string(),
            format!("section {id} removed; a reference to it no longer resolves at this revision"),
        ));
    }
    for id in &d.changed {
        let old = before.iter().find(|s| &s.id == id);
        let new = next.iter().find(|s| &s.id == id);
        let subs: Vec<&str> = match (old, new) {
            (Some(old), Some(new)) => new
                .subsections
                .iter()
                .filter(|s| {
                    old.subsections
                        .iter()
                        .find(|o| o.id == s.id)
                        .is_none_or(|o| o.sha256 != s.sha256)
                })
                .map(|s| s.id.as_str())
                .collect(),
            _ => Vec::new(),
        };
        report.push(Diagnostic::pass(
            "sas.diff.section-changed",
            if subs.is_empty() {
                format!("section {id} changed")
            } else {
                format!("section {id} changed (in {})", subs.join(", "))
            },
        ));
    }
    if d.is_empty() {
        report.push(Diagnostic::pass(
            "sas.diff.sections",
            "no section added, removed or changed".to_owned(),
        ));
    }
}

/// Run one read-only Git command in the repository root, returning stdout.
fn git(repo: &Repository, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("git")
        .current_dir(&repo.root)
        .args(["--no-pager", "--no-replace-objects"])
        .args(args)
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("Git is unavailable ({e})"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(format!("`git {}` failed", args.join(" ")))
    }
}

/// A recorded revision's bytes, found by the digest its record pins
/// (OW-WAR-0125, A-002): the document on disk when it is that revision, else
/// the first commit in this repository's history whose blob at the record's
/// `source` has that sha256. Local and deterministic; nothing is fetched.
///
/// `Err` says why the bytes cannot be had — no Git, a shallow clone, or a
/// history that holds no such blob — so a caller can report UNKNOWN with the
/// reason (Law 15) rather than guess.
pub fn revision_bytes(repo: &Repository, revision: &SasRevision) -> Result<Vec<u8>, String> {
    if let Ok((_, bytes)) = repo.sas_document()
        && sha256_hex(&bytes) == revision.sha256
    {
        return Ok(bytes);
    }
    let source = revision.source.as_str();
    let log = git(
        repo,
        &["log", "--full-history", "--format=%H", "HEAD", "--", source],
    )?;
    let log = String::from_utf8_lossy(&log);
    for commit in log.lines().map(str::trim).filter(|c| !c.is_empty()) {
        let Ok(bytes) = git(repo, &["cat-file", "blob", &format!("{commit}:./{source}")]) else {
            continue;
        };
        if sha256_hex(&bytes) == revision.sha256 {
            return Ok(bytes);
        }
    }
    let shallow = git(repo, &["rev-parse", "--is-shallow-repository"])
        .map(|o| String::from_utf8_lossy(&o).trim() == "true")
        .unwrap_or(false);
    Err(if shallow {
        format!(
            "this clone is shallow, and no commit it retains holds {source} at sha256:{} \
             (revision {}); fetch the history to answer",
            revision.sha256, revision.version
        )
    } else {
        format!(
            "no commit in this repository's history holds {source} at sha256:{} (revision {})",
            revision.sha256, revision.version
        )
    })
}

/// Each recorded revision split into sections, read at most once per run.
/// `war check` asks it for the revision every citing Warrant is pinned to and
/// for the latest accepted one.
pub struct RevisionSections<'a> {
    repo: &'a Repository,
    revisions: Vec<SasRevision>,
    read: std::cell::RefCell<std::collections::BTreeMap<String, Result<Split, String>>>,
}

/// One revision's sections, and the namespace its §106 carries.
#[derive(Clone)]
pub struct Split {
    pub sections: std::rc::Rc<Vec<Section>>,
    pub namespace: Option<String>,
}

impl<'a> RevisionSections<'a> {
    #[must_use]
    pub fn new(repo: &'a Repository) -> Self {
        Self {
            repo,
            revisions: repo.load_sas_revisions().unwrap_or_default(),
            read: std::cell::RefCell::default(),
        }
    }

    #[must_use]
    pub fn revisions(&self) -> &[SasRevision] {
        &self.revisions
    }

    /// The sections of revision `version`, or why they cannot be read.
    pub fn get(&self, version: &str) -> Result<Split, String> {
        if let Some(hit) = self.read.borrow().get(version) {
            return hit.clone();
        }
        let result = match self.revisions.iter().find(|r| r.version == version) {
            None => Err(format!("no SAS revision {version} is recorded")),
            Some(rev) => revision_bytes(self.repo, rev).map(|bytes| Split {
                namespace: sas_sections::namespace(&String::from_utf8_lossy(&bytes)),
                sections: std::rc::Rc::new(sas_sections::split(&bytes)),
            }),
        };
        self.read
            .borrow_mut()
            .insert(version.to_owned(), result.clone());
        result
    }
}

/// Every revision on record, and which one the document currently matches.
pub fn status(repo: &Repository) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (path, bytes) = repo.sas_document()?;
    let current = sha256_hex(&bytes);
    let revisions = repo.load_sas_revisions()?;
    if revisions.is_empty() {
        report.push(Diagnostic::warn(
            "sas.unrecorded",
            repo.relative(&path),
            format!(
                "no revision is recorded; the document is at sha256:{current} and nothing pins it"
            ),
        ));
        return Ok(report);
    }
    for r in &revisions {
        let line = format!("{} · {} · sha256:{}", r.version, r.state, &r.sha256[..12]);
        if r.sha256 == current {
            report.push(Diagnostic::pass(
                "sas.revision",
                format!("{line} · matches the document"),
            ));
        } else {
            // A recorded revision the document no longer matches is worth a
            // reader's attention even when a newer one does match it.
            report.push(Diagnostic::warn(
                "sas.revision",
                repo.relative(&path),
                format!("{line} · does not match the document"),
            ));
        }
    }
    Ok(report)
}

fn find(repo: &Repository, version: &str) -> Result<SasRevision, RepoError> {
    repo.load_sas_revisions()?
        .into_iter()
        .find(|r| r.version == version)
        .ok_or_else(|| RepoError::Message(format!("no SAS revision named {version:?}")))
}

fn write_toml<T: Serialize>(path: &Utf8Path, value: &T) -> Result<(), RepoError> {
    let rendered = toml::to_string_pretty(value).map_err(|e| RepoError::Message(e.to_string()))?;
    fs::write(path, rendered).map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })
}

/// Which revision the document is currently held to: the newest accepted, else
/// the newest proposed. `None` when nothing is recorded.
#[must_use]
pub fn pin_of(revisions: &[SasRevision]) -> Option<&SasRevision> {
    revisions
        .iter()
        .filter(|r| r.state == SasRevisionState::Accepted)
        .max_by(|a, b| a.version.cmp(&b.version))
        .or_else(|| revisions.iter().max_by(|a, b| a.version.cmp(&b.version)))
}
