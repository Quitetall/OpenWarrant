// SPDX-License-Identifier: Apache-2.0
//! `war sign standing` — a standing authorization (OW-ADR-0029; SAS §28.8 as
//! proposed in 1.2.0).
//!
//! # The shape, in one paragraph
//!
//! The owner signs one CLASS of routine work. The class is a file under
//! `docs/authority/standing/<id>@<revision>.toml`, proposed by anyone with
//! `war sign standing propose`, and it covers nothing until a human signs it with
//! `war sign standing:<id>@<revision> --ssh-sign` — the acceptance is that
//! signed response, bound to the class file's exact sha256. Each routine
//! Warrant is then checked against the class by `war sign standing apply`: inside,
//! it gets the same `oh.war/authorization/v1` record a signature writes, with
//! the class's signer as `authorizer` and `policy_basis =
//! "standing://<id>@<revision>"`; outside, it is refused by the term it
//! breaks and nothing is written. Revoking is one more human act
//! (`war sign standing:<id>@<revision> --revoke`).
//!
//! # Why an agent cannot use this to authorize itself
//!
//! - The class is signed by a human: `war sign` drafts the acceptance and the
//!   ingest refuses an agent by kind and the performer as `SelfAct`, exactly
//!   as for an authorization. `war admin mcp` registers no tool that accepts,
//!   revokes or ingests a class.
//! - The class cannot be widened in place: its signature binds the file's
//!   bytes, so an edited glob leaves the class unsigned and every Warrant it
//!   covered reads `standing.unsigned`.
//! - The class cannot reach authority: [`openwarrant_core::standing::NEVER_COVERABLE`]
//!   is a constant, and a class whose globs could match it is refused when
//!   proposed, when signed and when applied.
//! - A covered record is never trusted. [`covered_verdict`] and [`check`]
//!   re-derive it from the class, its signature and the Warrant as it stands,
//!   on every read.
//! - Resolution is untouched: a covered Warrant is resolved by a human, and
//!   `resolution_cmd` refuses a policy service by name
//!   (`resolve.standing-needs-human`).

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::contract::{
    ActorKind, Authorization, ContractRevision, Independence, RevisionState,
};
use openwarrant_core::role::ProfileRegistry;
use openwarrant_core::standing::{self, Refusal, StandingAuthorization};
use serde::{Deserialize, Serialize};

use crate::authority_check::{Act, Verdict};
use crate::authorize::{AUTHORIZATION_SCHEMA, AuthorizationRecord};
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// Where class records live.
pub const DIR: &str = "docs/authority/standing";

pub const ACCEPT_REQUEST_SCHEMA: &str = "oh.war/standing-acceptance-request/v1";
pub const ACCEPT_RESPONSE_SCHEMA: &str = "oh.war/standing-acceptance-response/v1";
pub const REVOKE_REQUEST_SCHEMA: &str = "oh.war/standing-revocation-request/v1";
pub const REVOKE_RESPONSE_SCHEMA: &str = "oh.war/standing-revocation-response/v1";

/// The channel a covered authorization is journalled under.
pub const CHANNEL: &str = "standing";

/// The subject a class revision's signed responses are named for.
#[must_use]
pub fn subject(id: &str, revision: u32) -> String {
    format!("STANDING-{id}@{revision}")
}

/// The target `war sign` takes for a class revision.
#[must_use]
pub fn target(id: &str, revision: u32) -> String {
    format!("standing:{id}@{revision}")
}

fn class_path(repo: &Repository, id: &str, revision: u32) -> Utf8PathBuf {
    repo.root.join(DIR).join(format!("{id}@{revision}.toml"))
}

/// One class file on disk.
#[derive(Debug, Clone)]
pub struct ClassFile {
    pub id: String,
    pub revision: u32,
    pub path: Utf8PathBuf,
    /// sha256 of the exact bytes: what an acceptance signs.
    pub sha256: String,
    pub parsed: Result<StandingAuthorization, Vec<Refusal>>,
}

impl ClassFile {
    #[must_use]
    pub fn reference(&self) -> String {
        standing::reference(&self.id, self.revision)
    }

    #[must_use]
    pub fn subject(&self) -> String {
        subject(&self.id, self.revision)
    }
}

/// Whether a class may cover `profile`: its definition's `standing_coverage`
/// (OW-ADR-0031). A name the registry does not admit may not be covered.
fn may_cover(profiles: &ProfileRegistry, profile: &str) -> bool {
    profiles
        .resolve(profile)
        .is_ok_and(|p| profiles.kind(&p).standing_coverage)
}

/// Read one class file. The name is `<id>@<revision>.toml`, and a record
/// whose own `id` or `revision` disagrees with its name is refused.
fn read_class(profiles: &ProfileRegistry, path: &Utf8Path) -> Option<ClassFile> {
    let stem = path.file_name()?.strip_suffix(".toml")?;
    let (id, rev) = stem.split_once('@')?;
    let revision: u32 = rev.parse().ok()?;
    let bytes = crate::vfs::read(path).ok()?;
    let sha256 = openwarrant_compiler::sha256_hex(&bytes);
    let parsed = standing::parse_in(&String::from_utf8_lossy(&bytes), &|p| {
        may_cover(profiles, p)
    })
    .and_then(|c| {
        if c.id == id && c.revision == revision {
            Ok(c)
        } else {
            Err(vec![Refusal::Bound {
                term: "id",
                why: format!(
                    "the record says {}@{} and its file is named {id}@{revision}",
                    c.id, c.revision
                ),
            }])
        }
    });
    Some(ClassFile {
        id: id.to_owned(),
        revision,
        path: path.to_owned(),
        sha256,
        parsed,
    })
}

/// Every class file, by id then revision.
#[must_use]
pub fn load_all(repo: &Repository) -> Vec<ClassFile> {
    let dir = repo.root.join(DIR);
    let mut out: Vec<ClassFile> = crate::vfs::read_dir_utf8(&dir)
        .map(|rd| {
            rd.into_iter()
                .filter(|p| p.extension() == Some("toml"))
                .filter_map(|p| read_class(&repo.profiles, &p))
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.id.cmp(&b.id).then(a.revision.cmp(&b.revision)));
    out
}

/// The class a `standing://<id>@<rev>` reference names, if its file exists.
#[must_use]
pub fn load(repo: &Repository, reference: &str) -> Option<ClassFile> {
    let (id, rev) = standing::parse_reference(reference)?;
    read_class(&repo.profiles, &class_path(repo, &id, rev))
}

/// The human act on a class, as its signed response records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    pub by: String,
    pub at: String,
    pub principal: String,
}

/// Where a class stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// No acceptance response: proposed, covers nothing.
    Proposed,
    /// A response exists and its signature does not verify over these bytes.
    Unsigned {
        verdict: Verdict,
    },
    Accepted(Signed),
}

fn signed_act(repo: &Repository, act: Act, class: &ClassFile, who_key: &str) -> Option<Verdict> {
    let path = crate::authority_check::response_path(repo, act, &class.subject());
    let text = crate::vfs::read_to_string(&path).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let who = value.get(who_key).and_then(toml::Value::as_str)?.to_owned();
    Some(crate::authority_check::verify(
        repo,
        act,
        &class.subject(),
        &who,
        Some(&class.sha256),
    ))
}

fn response_field(repo: &Repository, act: Act, class: &ClassFile, key: &str) -> Option<String> {
    let path = crate::authority_check::response_path(repo, act, &class.subject());
    let text = crate::vfs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}

/// Whether a human signed this class's acceptance over its exact bytes.
#[must_use]
pub fn standing_of(repo: &Repository, class: &ClassFile) -> Standing {
    match signed_act(repo, Act::AcceptStanding, class, "accepted_by") {
        None => Standing::Proposed,
        Some(Verdict::Signed { principal, .. }) => {
            let by = response_field(repo, Act::AcceptStanding, class, "accepted_by");
            let at = response_field(repo, Act::AcceptStanding, class, "effective_time");
            match (by, at) {
                (Some(by), Some(at)) => Standing::Accepted(Signed { by, at, principal }),
                _ => Standing::Unsigned {
                    verdict: Verdict::Invalid {
                        why: "the signed acceptance names no signer or time".to_owned(),
                    },
                },
            }
        }
        Some(verdict) => Standing::Unsigned { verdict },
    }
}

/// The verified revocation of a class, if a human signed one.
#[must_use]
pub fn revocation_of(repo: &Repository, class: &ClassFile) -> Option<Signed> {
    match signed_act(repo, Act::RevokeStanding, class, "revoked_by")? {
        Verdict::Signed { principal, .. } => Some(Signed {
            by: response_field(repo, Act::RevokeStanding, class, "revoked_by")?,
            at: response_field(repo, Act::RevokeStanding, class, "effective_time")?,
            principal,
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The contract as a class reads it
// ---------------------------------------------------------------------------

/// What [`standing::covers`] reads of one Warrant, from its compiled basis and
/// its records. Nothing is inferred: every field is one the Warrant states.
///
/// # Errors
/// When the Warrant does not compile or a record will not read — a Warrant
/// that cannot be read cannot be covered.
pub fn contract_of(
    repo: &Repository,
    one: &crate::repo::Loaded,
) -> Result<standing::Contract, String> {
    let (Some(basis), Some(v)) = (&one.basis, &one.validated) else {
        return Err("the manifest did not validate, so there is no contract to cover".to_owned());
    };
    let deliverables = repo
        .load_deliverables(&one.dir)
        .map_err(|e| e.to_string())?;
    if let Some((path, why)) = deliverables.failures.first() {
        return Err(format!("{path}: {why}"));
    }
    let assumptions = repo
        .load_rationale(&one.dir)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let mut stages = Vec::new();
    for a in basis.atoms.iter().filter(|a| a.role == "milestones") {
        let graph = openwarrant_core::milestones::parse(&String::from_utf8_lossy(&a.bytes))
            .map_err(|e| format!("{}: {e}", a.source))?;
        stages.extend(graph.stages.into_iter().map(|s| standing::StageBudget {
            id: s.id,
            executor_kind: s.executor_kind.to_string(),
            budget_tokens: s.budget_tokens,
            wall_time_seconds: s.wall_time_seconds,
        }));
    }
    Ok(standing::Contract {
        profile: v.raw.profile.clone(),
        assurance: v.assurance_level.to_string(),
        declared_paths: deliverables
            .records
            .iter()
            .map(|d| d.target_ref.clone())
            .collect(),
        has_adr_atom: basis.atoms.iter().any(|a| a.role == "adr"),
        residual_risks: assumptions
            .iter()
            .filter(|a| {
                a.epistemic_status
                    == openwarrant_core::rationale::EpistemicStatus::AcceptedResidualRisk
            })
            .map(|a| a.id.clone())
            .collect(),
        cited_gates: basis
            .atoms
            .iter()
            .filter(|a| a.role == "assurance")
            .flat_map(|a| {
                openwarrant_core::gate::cited_gate_uris(&String::from_utf8_lossy(&a.bytes))
            })
            .collect(),
        stages,
    })
}

/// A declared path that is a RESOLVED Warrant's `deliverables.toml`: never
/// coverable, and not expressible as a constant glob.
fn resolved_deliverables_file(repo: &Repository, path: &str) -> bool {
    let Some(rest) = path.strip_prefix(&format!("{}/", repo.config.paths.warrants)) else {
        return false;
    };
    let Some((alias, "deliverables.toml")) = rest.split_once('/') else {
        return false;
    };
    repo.warrant_dir(alias)
        .ok()
        .and_then(|d| repo.load_resolution(&d).ok().flatten())
        .is_some()
}

/// Every covered authorization in the corpus: (alias, reference, time).
fn covered_records(repo: &Repository) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for dir in repo.warrant_dirs().unwrap_or_default() {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        let Ok(Some(record)) = repo.load_authorization(&dir) else {
            continue;
        };
        let Some(auth) = record.revision.authorization else {
            continue;
        };
        if let Some(basis) = auth
            .policy_basis
            .filter(|b| b.starts_with(standing::SCHEME))
        {
            out.push((alias, basis, auth.effective_time));
        }
    }
    out
}

/// How many Warrants a class covered strictly before `(at, alias)`.
fn used_before(repo: &Repository, reference: &str, at: &str, alias: &str) -> u32 {
    let t = standing::epoch_seconds(at);
    let n = covered_records(repo)
        .into_iter()
        .filter(|(a, r, when)| {
            r == reference && a != alias && (standing::epoch_seconds(when), a.as_str()) < (t, alias)
        })
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// war standing propose
// ---------------------------------------------------------------------------

fn refusal_diag(r: &Refusal, file: String) -> Diagnostic {
    Diagnostic::error(r.rule(), file, r.to_string())
}

/// `war sign standing propose <file>`: validate a class and place it where
/// `war sign` offers it for one signature. Writes the file's exact bytes,
/// which are what the signature will bind; `--dry-run` writes nothing.
pub fn propose(repo: &Repository, file: &Utf8Path, dry_run: bool) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let bytes = std::fs::read(file).map_err(|source| RepoError::Io {
        context: format!("could not read {file}"),
        source,
    })?;
    let class = match standing::parse_in(&String::from_utf8_lossy(&bytes), &|p| {
        may_cover(&repo.profiles, p)
    }) {
        Ok(c) => c,
        Err(refusals) => {
            for r in &refusals {
                report.push(refusal_diag(r, file.to_string()));
            }
            report.note("Refused; nothing written. A class states every term and reaches no authority path.");
            return Ok(report);
        }
    };
    let now = crate::gate_cmd::receipt::now_rfc3339_public();
    if let Some(r) = standing::expiry_refusal(&class, &now) {
        report.push(refusal_diag(&r, file.to_string()));
        return Ok(report);
    }
    let dest = class_path(repo, &class.id, class.revision);
    match std::fs::read(&dest) {
        Ok(existing) if existing == bytes => {
            report.push(Diagnostic::pass(
                "standing.proposed",
                format!(
                    "{} is already proposed with these bytes → {}",
                    standing::reference(&class.id, class.revision),
                    repo.relative(&dest)
                ),
            ));
            return Ok(report);
        }
        Ok(_) => {
            report.push(Diagnostic::error(
                "standing.revision-exists",
                repo.relative(&dest),
                format!(
                    "{} exists with other bytes. A class revision is immutable once proposed; \
                     a changed class is revision {}",
                    repo.relative(&dest),
                    class.revision + 1
                ),
            ));
            return Ok(report);
        }
        Err(_) => {}
    }
    let sha = openwarrant_compiler::sha256_hex(&bytes);
    if dry_run {
        report.push(Diagnostic::pass(
            "standing.would-propose",
            format!(
                "{} at sha256:{} would be written to {}; nothing written",
                standing::reference(&class.id, class.revision),
                &sha[..12],
                repo.relative(&dest)
            ),
        ));
        return Ok(report);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    if let Err(refused) = crate::compile::atomic::write(&dest, &bytes) {
        report.push(refused.diagnostic());
        return Ok(report);
    }
    report.push(Diagnostic::pass(
        "standing.proposed",
        format!(
            "{} proposed at sha256:{} → {}",
            standing::reference(&class.id, class.revision),
            &sha[..12],
            repo.relative(&dest)
        ),
    ));
    report.note(format!(
        "Proposed, not signed: it covers nothing yet. One human act accepts it: \
         `war sign {} --ssh-sign` (try `--dry-run` first).",
        target(&class.id, class.revision)
    ));
    Ok(report)
}

// ---------------------------------------------------------------------------
// war standing show
// ---------------------------------------------------------------------------

/// Every file under the repository, repository-relative, skipping `.git`,
/// `target` and `node_modules`.
fn tree(repo: &Repository) -> Vec<String> {
    fn walk(root: &Utf8Path, dir: &Utf8Path, out: &mut Vec<String>) {
        let Ok(rd) = dir.read_dir_utf8() else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default();
            if matches!(name, ".git" | "target" | "node_modules") {
                continue;
            }
            if p.is_dir() {
                walk(root, p, out);
            } else if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string());
            }
        }
    }
    let mut out = Vec::new();
    walk(&repo.root, &repo.root, &mut out);
    out.sort();
    out
}

/// What each glob of a class matches in the tree today.
#[must_use]
pub fn matches_today(
    repo: &Repository,
    class: &StandingAuthorization,
) -> Vec<(String, Vec<String>)> {
    let files = tree(repo);
    class
        .paths
        .iter()
        .map(|g| {
            (
                g.clone(),
                files
                    .iter()
                    .filter(|f| standing::glob_match(g, f))
                    .cloned()
                    .collect(),
            )
        })
        .collect()
}

/// One class as `war sign standing show` reports it.
#[derive(Debug, Clone, Serialize)]
pub struct View {
    pub reference: String,
    pub file: String,
    pub sha256: String,
    pub state: String,
    pub signed_by: Option<String>,
    pub signed_at: Option<String>,
    pub revoked_at: Option<String>,
    pub expires_at: Option<String>,
    pub used: usize,
    pub max_warrants: Option<u32>,
    pub refusals: Vec<String>,
    pub matches: BTreeMap<String, Vec<String>>,
}

/// `war sign standing show [<id>]`.
pub fn show(repo: &Repository, id: Option<&str>) -> Result<(Report, Vec<View>), RepoError> {
    let mut report = Report::default();
    let covered = covered_records(repo);
    let mut views = Vec::new();
    for class in load_all(repo).into_iter().filter(|c| {
        id.is_none_or(|i| i == c.id || i == c.reference() || i == target(&c.id, c.revision))
    }) {
        let standing = standing_of(repo, &class);
        let revoked = revocation_of(repo, &class);
        let (state, by, at) = match (&standing, &revoked) {
            (Standing::Accepted(_), Some(r)) => ("revoked", None, Some(r.at.clone())),
            (Standing::Accepted(s), None) => ("accepted", Some(s.by.clone()), Some(s.at.clone())),
            (Standing::Unsigned { .. }, _) => ("unsigned", None, None),
            (Standing::Proposed, _) => ("proposed", None, None),
        };
        let signed = match &standing {
            Standing::Accepted(s) => Some(s.clone()),
            _ => None,
        };
        let used = covered
            .iter()
            .filter(|(_, r, _)| *r == class.reference())
            .count();
        let view = View {
            reference: class.reference(),
            file: repo.relative(&class.path),
            sha256: class.sha256.clone(),
            state: state.to_owned(),
            signed_by: by.or_else(|| signed.as_ref().map(|s| s.by.clone())),
            signed_at: signed.as_ref().map(|s| s.at.clone()).or(at.clone()),
            revoked_at: revoked.as_ref().map(|r| r.at.clone()),
            expires_at: class.parsed.as_ref().ok().map(|c| c.expires_at.clone()),
            used,
            max_warrants: class.parsed.as_ref().ok().map(|c| c.max_warrants),
            refusals: class
                .parsed
                .as_ref()
                .err()
                .map(|rs| rs.iter().map(|r| format!("{}: {r}", r.rule())).collect())
                .unwrap_or_default(),
            matches: class
                .parsed
                .as_ref()
                .map(|c| matches_today(repo, c).into_iter().collect())
                .unwrap_or_default(),
        };
        let line = format!(
            "{}  {}  {} of {} covered  expires {}  {}",
            view.reference,
            view.state,
            view.used,
            view.max_warrants.map_or("?".to_owned(), |m| m.to_string()),
            view.expires_at.as_deref().unwrap_or("?"),
            view.signed_by
                .as_deref()
                .map(|b| format!("signed by {b}"))
                .unwrap_or_default()
        );
        if let Standing::Unsigned { verdict } = &standing {
            report.push(Diagnostic::error(
                "standing.unsigned",
                view.file.clone(),
                format!("{line}: {}", verdict.why()),
            ));
        } else if view.refusals.is_empty() {
            report.push(Diagnostic::pass("standing.shown", line));
        } else {
            for r in &view.refusals {
                report.push(Diagnostic::error(
                    "standing.refused",
                    view.file.clone(),
                    format!("{}: {r}", view.reference),
                ));
            }
        }
        for (g, m) in &view.matches {
            report.push(Diagnostic::pass(
                "standing.covers",
                format!(
                    "{}  covers: {g}  → {} file(s) today{}",
                    view.reference,
                    m.len(),
                    if m.is_empty() {
                        String::new()
                    } else {
                        format!(
                            ": {}{}",
                            m.iter().take(8).cloned().collect::<Vec<_>>().join(", "),
                            if m.len() > 8 { ", …" } else { "" }
                        )
                    }
                ),
            ));
        }
        views.push(view);
    }
    if views.is_empty() {
        report.push(Diagnostic::pass(
            "standing.none",
            format!(
                "no class{} under {DIR}",
                id.map(|i| format!(" {i}")).unwrap_or_default()
            ),
        ));
    }
    Ok((report, views))
}

// ---------------------------------------------------------------------------
// war standing apply
// ---------------------------------------------------------------------------

/// The class a Warrant asks for: `--class`, else the manifest's
/// `[standing] ref`.
fn requested_reference(one: &crate::repo::Loaded, flag: Option<&str>) -> Option<String> {
    if let Some(f) = flag {
        return Some(if f.starts_with(standing::SCHEME) {
            f.to_owned()
        } else {
            format!("{}{}", standing::SCHEME, f.trim_start_matches("standing:"))
        });
    }
    let text = crate::vfs::read_to_string(one.dir.join("manifest.toml")).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get("standing")?
        .get("ref")?
        .as_str()
        .map(str::to_owned)
}

/// `war sign standing apply <alias>`: the coverage check. Inside the class, the
/// Warrant's `authorization.toml` is written with the class's signer as
/// authorizer; outside it, every broken term is named and nothing is written.
pub fn apply(
    repo: &Repository,
    alias: &str,
    class_ref: Option<&str>,
    dry_run: bool,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let dir = repo.warrant_dir(alias)?;
    let authorization_path = dir.join("authorization.toml");
    let before = match crate::compile::atomic::prestate(&authorization_path) {
        Ok(p) => p,
        Err(refused) => {
            report.push(refused.diagnostic());
            return Ok(report);
        }
    };
    let one = repo.load_warrant(&dir)?;
    let at = repo.relative(&dir.join("manifest.toml"));
    let refuse = |report: &mut Report, rule: &str, why: String| {
        report.push(Diagnostic::error(
            rule,
            at.clone(),
            format!("{alias}: {why}"),
        ));
    };
    let Some(reference) = requested_reference(&one, class_ref) else {
        refuse(
            &mut report,
            "standing.no-class",
            "names no class: pass --class standing://<id>@<revision>, or write \
             `[standing] ref = \"standing://<id>@<revision>\"` in the manifest"
                .to_owned(),
        );
        return Ok(report);
    };
    let Some(class) = load(repo, &reference) else {
        refuse(
            &mut report,
            "standing.no-class",
            format!("{reference} names no class file under {DIR}"),
        );
        return Ok(report);
    };
    let terms = match &class.parsed {
        Ok(c) => c.clone(),
        Err(refusals) => {
            for r in refusals {
                refuse(&mut report, r.rule(), format!("{reference}: {r}"));
            }
            return Ok(report);
        }
    };
    let signed = match standing_of(repo, &class) {
        Standing::Accepted(s) => s,
        Standing::Proposed => {
            refuse(
                &mut report,
                "standing.unsigned",
                format!(
                    "{reference} is proposed and no human has signed it; a class with no \
                     signature covers nothing — `war sign {} --ssh-sign`",
                    target(&class.id, class.revision)
                ),
            );
            return Ok(report);
        }
        Standing::Unsigned { verdict } => {
            refuse(
                &mut report,
                "standing.unsigned",
                format!(
                    "{reference}: the acceptance does not verify over the class file's bytes \
                     (sha256:{}): {}",
                    &class.sha256[..12],
                    verdict.why()
                ),
            );
            return Ok(report);
        }
    };
    let now = crate::gate_cmd::receipt::now_rfc3339_public();
    let revoked = revocation_of(repo, &class);
    let used = used_before(repo, &reference, &now, alias);
    let mut refused_any = false;
    if let Err(r) = standing::in_force(
        &terms,
        &now,
        &signed.at,
        revoked.as_ref().map(|r| r.at.as_str()),
        used,
    ) {
        refuse(&mut report, r.rule(), format!("{reference}: {r}"));
        refused_any = true;
    }
    let contract = match contract_of(repo, &one) {
        Ok(c) => c,
        Err(why) => {
            refuse(&mut report, "standing.not-compilable", why);
            return Ok(report);
        }
    };
    if let Err(refusals) = standing::covers(&terms, &contract) {
        for r in refusals {
            refuse(&mut report, r.rule(), format!("{reference}: {r}"));
        }
        refused_any = true;
    }
    for p in &contract.declared_paths {
        if resolved_deliverables_file(repo, p) {
            refuse(
                &mut report,
                "standing.never-coverable",
                format!("{p} is a resolved Warrant's deliverables.toml, which no class may cover"),
            );
            refused_any = true;
        }
    }
    // Q-003 (refuse, as recommended): routine work never takes a path from
    // work in flight. The in-flight Warrant resolves first, or the owner
    // signs the routine change individually.
    let ownership = crate::ownership::Ownership::index(repo)?;
    for p in &contract.declared_paths {
        if let Some(owner) = ownership.in_flight_owner(p, alias) {
            refuse(
                &mut report,
                "standing.in-flight-owner",
                format!(
                    "{p} is owned by {} ({}, authorized {}), which is authorized and not \
                     resolved. A covered Warrant never takes a path from work in flight: \
                     {} resolves first, or this change is signed individually",
                    owner.alias, owner.deliverable_id, owner.authorized_at, owner.alias
                ),
            );
            refused_any = true;
        }
    }
    if refused_any {
        report.note(format!(
            "{alias} is not covered by {reference}; nothing was written. It can still be \
             authorized the ordinary way: `war sign {alias}`."
        ));
        return Ok(report);
    }

    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        refuse(
            &mut report,
            "standing.not-compilable",
            "does not compile".to_owned(),
        );
        return Ok(report);
    };
    let ir = openwarrant_compiler::lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("{alias}: could not compile contract: {e}")))?;
    let digest = ir
        .contract_digest()
        .map_err(|e| RepoError::Message(format!("{alias}: could not digest contract: {e}")))?;
    if let Some(existing) = repo.load_authorization(&dir)? {
        let same = existing.revision.contract_digest == digest
            && existing
                .revision
                .authorization
                .as_ref()
                .and_then(|a| a.policy_basis.as_deref())
                == Some(reference.as_str());
        if same {
            report.push(Diagnostic::pass(
                "standing.already-covered",
                format!(
                    "{alias}: contract {digest} is already authorized under {reference}; nothing \
                     written"
                ),
            ));
        } else {
            refuse(
                &mut report,
                "standing.already-authorized",
                format!(
                    "an authorization is already recorded (revision {}, contract {}); an \
                     authorized revision is immutable (§28.7) and a class does not re-cover it. \
                     A moved contract is an amendment, signed individually",
                    existing.revision.revision, existing.revision.contract_digest
                ),
            );
        }
        return Ok(report);
    }
    let register = repo.load_authority_register()?;
    let owned = crate::ownership::declared_set(repo, &dir)?;
    let set_digest = crate::ownership::set_digest(&owned);
    let authorization = Authorization {
        authorizer: signed.by.clone(),
        actor_kind: register
            .actor(&signed.by)
            .map_or(ActorKind::Human, |a| a.actor_kind),
        acting_role: "authorizer".to_owned(),
        meaning: format!(
            "Authorized under the standing authorization {reference} (class sha256:{}), which \
             {} signed at {}: \"{}\" `war standing apply` found this contract inside every \
             term of the class; the authorizer of record is the class's signer (SAS §28.8, \
             proposed in 1.2.0). It does not resolve the Warrant.",
            class.sha256,
            signed.by,
            signed.at,
            terms.meaning.trim()
        ),
        effective_time: now.clone(),
        policy_basis: Some(reference.clone()),
        independence: Independence::SeparateRole,
    };
    let revision = ContractRevision::draft(digest.clone(), ir.contract_coverage.clone())
        .propose(repo.performer())
        .and_then(|p| p.authorize(authorization))
        .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;
    let record = AuthorizationRecord {
        schema: AUTHORIZATION_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        revision,
        sas_revision: basis.sas.as_ref().map(|p| p.version.clone()),
        deliverable_set_digest: Some(set_digest.clone()),
        owned,
    };
    if dry_run {
        report.push(Diagnostic::pass(
            "standing.would-record",
            format!(
                "{alias}: inside {reference}; contract {digest} would be authorized by {} with \
                 {} path(s) under set {set_digest}. Not written",
                signed.by,
                record.owned.len()
            ),
        ));
        return Ok(report);
    }
    let rendered =
        toml::to_string_pretty(&record).map_err(|e| RepoError::Message(e.to_string()))?;
    if let Err(refused) = crate::compile::atomic::write_if(&authorization_path, rendered, &before) {
        report.push(refused.diagnostic());
        return Ok(report);
    }
    if let Some(v) = &one.validated {
        let payload = serde_json::json!({
            "contract_digest": digest,
            "acting_role": "authorizer",
            "channel": CHANNEL,
            "policy_basis": reference,
            "class_sha256": class.sha256,
            "deliverable_set_digest": set_digest,
            "set_signed": false,
        })
        .to_string();
        crate::journal_cmd::record(
            &dir,
            &v.uuid.to_string(),
            crate::journal_cmd::AUTHORIZATION_RECORDED,
            &format!("person://{}", signed.by),
            &payload,
        )?;
    }
    report.push(Diagnostic::pass(
        "standing.recorded",
        format!(
            "{alias}: contract {digest} authorized under {reference}, authorizer {} → {}",
            signed.by,
            repo.relative(&authorization_path)
        ),
    ));
    Ok(report)
}

// ---------------------------------------------------------------------------
// Re-derivation: authority_check and war check
// ---------------------------------------------------------------------------

/// Why a covered record does not stand, or `Ok` with the class's acceptance.
fn rederive(
    repo: &Repository,
    alias: &str,
    record: &AuthorizationRecord,
) -> Result<(ClassFile, Signed), (&'static str, String)> {
    let Some(auth) = &record.revision.authorization else {
        return Err(("standing.unsigned", "no authorization table".to_owned()));
    };
    let reference = auth.policy_basis.clone().unwrap_or_default();
    let Some(class) = load(repo, &reference) else {
        return Err((
            "standing.unsigned",
            format!("{reference} names no class file under {DIR}; nothing signed this record"),
        ));
    };
    let terms = match &class.parsed {
        Ok(c) => c.clone(),
        Err(rs) => {
            let first = &rs[0];
            return Err((first.rule(), format!("{reference}: {first}")));
        }
    };
    let signed = match standing_of(repo, &class) {
        Standing::Accepted(s) => s,
        Standing::Proposed => {
            return Err((
                "standing.unsigned",
                format!(
                    "{reference} carries no signed acceptance; a class nobody signed covers nothing"
                ),
            ));
        }
        Standing::Unsigned { verdict } => {
            return Err((
                "standing.unsigned",
                format!(
                    "{reference}: the acceptance does not verify over the class file as it \
                     stands (sha256:{}): {}",
                    &class.sha256[..12],
                    verdict.why()
                ),
            ));
        }
    };
    if signed.by != auth.authorizer {
        return Err((
            "standing.unsigned",
            format!(
                "the record names {} as authorizer and {reference} was signed by {}",
                auth.authorizer, signed.by
            ),
        ));
    }
    let revoked = revocation_of(repo, &class);
    let used = used_before(repo, &reference, &auth.effective_time, alias);
    if let Err(r) = standing::in_force(
        &terms,
        &auth.effective_time,
        &signed.at,
        revoked.as_ref().map(|r| r.at.as_str()),
        used,
    ) {
        return Err((
            r.rule(),
            format!("{reference}, as stamped at {}: {r}", auth.effective_time),
        ));
    }
    Ok((class, signed))
}

/// `authority_check`'s answer for an authorization whose `policy_basis` is a
/// class: `None` when the record is not a covered one (the ordinary verdict
/// stands), else whether the class, its signature, the record's stamp and the
/// Warrant as it compiles now all hold.
#[must_use]
pub fn covered_verdict(
    repo: &Repository,
    alias: &str,
    actor: &str,
    bound_digest: Option<&str>,
) -> Option<Verdict> {
    let dir = repo.warrant_dir(alias).ok()?;
    let record = repo.load_authorization(&dir).ok()??;
    let auth = record.revision.authorization.as_ref()?;
    let reference = auth.policy_basis.as_deref()?;
    if !reference.starts_with(standing::SCHEME) {
        return None;
    }
    let unsigned = |why: String| Some(Verdict::Unsigned { why });
    if auth.authorizer != actor {
        return unsigned(format!(
            "{alias}: the covered record names {} and the act was asked of {actor}",
            auth.authorizer
        ));
    }
    if bound_digest
        .is_some_and(|d| d.trim_start_matches("sha256:") != record.revision.contract_digest)
    {
        return unsigned(format!(
            "{alias}: the covered record binds contract {} and the question is about another",
            record.revision.contract_digest
        ));
    }
    let (class, signed) = match rederive(repo, alias, &record) {
        Ok(ok) => ok,
        Err((rule, why)) => return unsigned(format!("{alias} ({rule}): {why}")),
    };
    // Re-derived against the contract as it compiles now. A covered record
    // is believed only while the Warrant it covers is still the one on disk
    // and still inside the class; a moved contract is not guessed about.
    let one = repo.load_warrant(&dir).ok()?;
    let current = match (&one.basis, &one.validated) {
        (Some(b), Some(v)) => openwarrant_compiler::lower(b, v)
            .ok()
            .and_then(|ir| ir.contract_digest().ok()),
        _ => None,
    };
    if current.as_deref() != Some(record.revision.contract_digest.as_str()) {
        return unsigned(format!(
            "{alias}: covered at contract {}, and the Warrant now compiles to {}; a covered \
             record is re-derived from the contract it covers, and that contract is no longer \
             on disk",
            record.revision.contract_digest,
            current.as_deref().unwrap_or("nothing")
        ));
    }
    let terms = class.parsed.as_ref().ok()?;
    match contract_of(repo, &one) {
        Err(why) => unsigned(format!("{alias}: {why}")),
        Ok(contract) => match standing::covers(terms, &contract) {
            Ok(()) => Some(Verdict::Signed {
                principal: signed.principal,
                response: format!(
                    "{} (standing, class sha256:{}, signed {})",
                    class.reference(),
                    &class.sha256[..12],
                    signed.at
                ),
            }),
            Err(rs) => unsigned(format!(
                "{alias} is no longer inside {}: {}",
                class.reference(),
                rs.iter()
                    .map(|r| format!("{}: {r}", r.rule()))
                    .collect::<Vec<_>>()
                    .join("; ")
            )),
        },
    }
}

/// `war check`'s `standing.*` rules for one covered Warrant. Never trusted:
/// the class, its signature, the stamp and the Warrant as it stands now are
/// re-derived on every run.
pub fn check(
    repo: &Repository,
    one: &crate::repo::Loaded,
    record: &AuthorizationRecord,
    report: &mut Report,
) {
    let Some(auth) = &record.revision.authorization else {
        return;
    };
    let Some(reference) = auth.policy_basis.as_deref() else {
        return;
    };
    if !reference.starts_with(standing::SCHEME) {
        return;
    }
    let alias = one.alias();
    let file = repo.relative(&one.dir.join("authorization.toml"));
    if record.revision.state != RevisionState::Authorized {
        return;
    }
    // A record stamped before a revocation stands (§31); one stamped after
    // it is an error like any other stamp outside the class's bounds.
    if let Some(class) = load(repo, reference)
        && let Some(revoked) = revocation_of(repo, &class)
        && standing::epoch_seconds(&revoked.at) > standing::epoch_seconds(&auth.effective_time)
    {
        report.push(Diagnostic::warn(
            "standing.revoked",
            file.clone(),
            format!(
                "{alias}: {reference} was revoked by {} at {}; this record was stamped at {} and \
                 stands (§31), and the class covers nothing new",
                revoked.by, revoked.at, auth.effective_time
            ),
        ));
    }
    let terms = match rederive(repo, &alias, record) {
        Ok((class, signed)) => {
            report.push(Diagnostic::pass(
                "standing.signed",
                format!(
                    "{alias}: {reference} (sha256:{}) signed by {} at {}",
                    &class.sha256[..12],
                    signed.by,
                    signed.at
                ),
            ));
            class.parsed.ok()
        }
        Err((rule, why)) => {
            report.push(Diagnostic::error(
                rule,
                file.clone(),
                format!("{alias}: {why}"),
            ));
            load(repo, reference).and_then(|c| c.parsed.ok())
        }
    };
    let Some(terms) = terms else {
        return;
    };
    match contract_of(repo, one) {
        Err(why) => report.push(Diagnostic::error(
            "standing.outside-class",
            file,
            format!("{alias}: the Warrant cannot be read against {reference}: {why}"),
        )),
        Ok(contract) => match standing::covers(&terms, &contract) {
            Ok(()) => report.push(Diagnostic::pass(
                "standing.inside-class",
                format!("{alias}: inside every term of {reference}, as it stands"),
            )),
            Err(rs) => {
                for r in rs {
                    report.push(Diagnostic::error(
                        "standing.outside-class",
                        file.clone(),
                        format!("{alias}: {reference}, {}: {r}", r.rule()),
                    ));
                }
            }
        },
    }
}

/// `war check`'s rules over the class files themselves: a class that reaches
/// authority, or whose acceptance does not verify over its bytes.
pub fn check_classes(repo: &Repository, report: &mut Report) {
    for class in load_all(repo) {
        let file = repo.relative(&class.path);
        if let Err(rs) = &class.parsed {
            for r in rs {
                report.push(Diagnostic::error(
                    r.rule(),
                    file.clone(),
                    format!("{}: {r}", class.reference()),
                ));
            }
        }
        match standing_of(repo, &class) {
            Standing::Proposed => report.push(Diagnostic::pass(
                "standing.proposed",
                format!(
                    "{}: proposed, covers nothing until `war sign {}`",
                    class.reference(),
                    target(&class.id, class.revision)
                ),
            )),
            Standing::Unsigned { verdict } => report.push(Diagnostic::error(
                "standing.unsigned",
                file,
                format!(
                    "{}: the acceptance does not verify over the class file's bytes \
                     (sha256:{}), so the class covers nothing: {}",
                    class.reference(),
                    &class.sha256[..12],
                    verdict.why()
                ),
            )),
            Standing::Accepted(s) => report.push(Diagnostic::pass(
                "standing.signed",
                format!("{}: signed by {} at {}", class.reference(), s.by, s.at),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// The human acts: accept and revoke (driven by `war sign`)
// ---------------------------------------------------------------------------

/// What is put to the signer of a class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptRequest {
    pub schema: String,
    pub id: String,
    pub revision: u32,
    pub reference: String,
    pub sha256: String,
    pub file: String,
    /// The class as parsed, or `None` when it is refused.
    pub class: Option<StandingAuthorization>,
    /// Each glob, and what it matches in the tree today.
    pub covers: Vec<(String, Vec<String>)>,
    /// Refusals of the class itself, by rule.
    pub refusals: Vec<String>,
    pub eligible_acceptors: Vec<String>,
    /// Register holders who may NOT sign, with why (agent by kind, the
    /// performer as `SelfAct`).
    pub refused_signers: Vec<(String, String)>,
}

/// What the signer returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptResponse {
    pub schema: String,
    pub id: String,
    pub revision: u32,
    /// The class file's exact sha256.
    pub sha256: String,
    pub accepted_by: String,
    pub acting_role: String,
    pub meaning: String,
    pub effective_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_via: Option<String>,
}

/// What is put to the signer of a revocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeRequest {
    pub schema: String,
    pub id: String,
    pub revision: u32,
    pub reference: String,
    pub sha256: String,
    /// Warrants the class already covers; their records stand (§31).
    pub covered: Vec<String>,
    pub eligible_acceptors: Vec<String>,
    pub refused_signers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeResponse {
    pub schema: String,
    pub id: String,
    pub revision: u32,
    pub sha256: String,
    pub revoked_by: String,
    pub acting_role: String,
    pub meaning: String,
    pub effective_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_via: Option<String>,
}

/// Who may sign, and who may not with why.
type Signers = (Vec<String>, Vec<(String, String)>);

/// Who may sign a class act, and who may not and why: the authorizer role,
/// through `may_authorize`, which refuses an agent by kind and the performer
/// as `SelfAct`.
fn signers(repo: &Repository) -> Result<Signers, RepoError> {
    let register = repo.load_authority_register()?;
    let performer = repo.performer();
    let mut ok = Vec::new();
    let mut refused = Vec::new();
    for a in &register.assignments {
        match a.may_authorize(&performer) {
            Ok(()) => ok.push(a.actor.clone()),
            Err(e) => refused.push((a.actor.clone(), authority_word(&e))),
        }
    }
    Ok((ok, refused))
}

fn authority_word(e: &openwarrant_core::authority::AuthorityError) -> String {
    use openwarrant_core::authority::AuthorityError as E;
    let kind = match e {
        E::AgentProhibited { .. } => "AgentProhibited",
        E::SelfAct { .. } => "SelfAct",
        E::RoleNotHeld { .. } => "RoleNotHeld",
        _ => "NotPermitted",
    };
    format!("{kind}: {e}")
}

/// The acceptance request for one class revision.
pub fn accept_request(repo: &Repository, class: &ClassFile) -> Result<AcceptRequest, RepoError> {
    let (eligible, refused) = signers(repo)?;
    let now = crate::gate_cmd::receipt::now_rfc3339_public();
    let mut refusals: Vec<String> = match &class.parsed {
        Ok(_) => Vec::new(),
        Err(rs) => rs.iter().map(|r| format!("{}: {r}", r.rule())).collect(),
    };
    if let Ok(c) = &class.parsed
        && let Some(r) = standing::expiry_refusal(c, &now)
    {
        refusals.push(format!("{}: {r}", r.rule()));
    }
    Ok(AcceptRequest {
        schema: ACCEPT_REQUEST_SCHEMA.to_owned(),
        id: class.id.clone(),
        revision: class.revision,
        reference: class.reference(),
        sha256: class.sha256.clone(),
        file: repo.relative(&class.path),
        class: class.parsed.as_ref().ok().cloned(),
        covers: class
            .parsed
            .as_ref()
            .map(|c| matches_today(repo, c))
            .unwrap_or_default(),
        refusals,
        eligible_acceptors: eligible,
        refused_signers: refused,
    })
}

/// Every class revision awaiting its acceptance: proposed, or carrying an
/// acceptance that does not verify. Never one already revoked.
pub fn pending_acceptances(repo: &Repository) -> Result<Vec<AcceptRequest>, RepoError> {
    let mut out = Vec::new();
    for class in load_all(repo) {
        if matches!(standing_of(repo, &class), Standing::Accepted(_)) {
            continue;
        }
        out.push(accept_request(repo, &class)?);
    }
    Ok(out)
}

/// Revocation requests for every signed, unrevoked class revision. Offered
/// only under `war sign --revoke`: a sweep never revokes.
pub fn revocable(repo: &Repository) -> Result<Vec<RevokeRequest>, RepoError> {
    let (eligible, refused) = signers(repo)?;
    let covered = covered_records(repo);
    let mut out = Vec::new();
    for class in load_all(repo) {
        if !matches!(standing_of(repo, &class), Standing::Accepted(_))
            || revocation_of(repo, &class).is_some()
        {
            continue;
        }
        out.push(RevokeRequest {
            schema: REVOKE_REQUEST_SCHEMA.to_owned(),
            id: class.id.clone(),
            revision: class.revision,
            reference: class.reference(),
            sha256: class.sha256.clone(),
            covered: covered
                .iter()
                .filter(|(_, r, _)| *r == class.reference())
                .map(|(a, _, _)| a.clone())
                .collect(),
            eligible_acceptors: eligible.clone(),
            refused_signers: refused.clone(),
        });
    }
    Ok(out)
}

/// The class file a pending act is about.
#[must_use]
pub fn record_path(repo: &Repository, id: &str, revision: u32) -> Utf8PathBuf {
    class_path(repo, id, revision)
}

fn signer_refusal(
    repo: &Repository,
    who: &str,
    report: &mut Report,
    at: &str,
) -> Result<Option<ActorKind>, RepoError> {
    let register = repo.load_authority_register()?;
    let Some(a) = register.actor(who) else {
        report.push(Diagnostic::error(
            "standing.unknown-actor",
            at.to_owned(),
            format!("{who:?} holds no role assignment in docs/authority/roles.toml"),
        ));
        return Ok(None);
    };
    if let Err(e) = a.may_authorize(&repo.performer()) {
        report.push(Diagnostic::error(
            "standing.not-permitted",
            at.to_owned(),
            authority_word(&e),
        ));
        return Ok(None);
    }
    Ok(Some(a.actor_kind))
}

/// Ingest a signed acceptance of a class. Every refusal runs in both modes;
/// `DryRun` stops where the real act would record. The signed response IS
/// the record: nothing else is written.
pub fn accept_ingest_with(
    repo: &Repository,
    id: &str,
    revision: u32,
    response_path: &Utf8Path,
    mode: crate::sign::IngestMode,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let at = response_path.to_string();
    let refuse = |report: &mut Report, rule: &str, why: String| {
        report.push(Diagnostic::error(rule, at.clone(), why));
    };
    let Some(class) = read_class(&repo.profiles, &class_path(repo, id, revision)) else {
        refuse(
            &mut report,
            "standing.no-class",
            format!("no class file {id}@{revision}.toml under {DIR}"),
        );
        return Ok(report);
    };
    let text = std::fs::read_to_string(response_path).map_err(|source| RepoError::Io {
        context: format!("could not read {response_path}"),
        source,
    })?;
    let response: AcceptResponse = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            refuse(&mut report, "standing.response-malformed", e.to_string());
            return Ok(report);
        }
    };
    if response.schema != ACCEPT_RESPONSE_SCHEMA {
        refuse(
            &mut report,
            "standing.response-schema",
            format!("unknown schema {:?}", response.schema),
        );
        return Ok(report);
    }
    if response.id != id || response.revision != revision {
        refuse(
            &mut report,
            "standing.response-class",
            format!(
                "the response names {}@{}, ingesting {id}@{revision}",
                response.id, response.revision
            ),
        );
        return Ok(report);
    }
    if let Err(e) = openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time) {
        refuse(
            &mut report,
            "standing.effective-time",
            format!("effective_time {:?}: {e}", response.effective_time),
        );
        return Ok(report);
    }
    if response.sha256 != class.sha256 {
        refuse(
            &mut report,
            "standing.stale-digest",
            format!(
                "the response signs sha256:{} and the class file is now sha256:{}; what was \
                 drafted is what is signed — draft and sign again",
                response.sha256, class.sha256
            ),
        );
        return Ok(report);
    }
    let terms = match &class.parsed {
        Ok(c) => c.clone(),
        Err(rs) => {
            for r in rs {
                refuse(&mut report, r.rule(), format!("{}: {r}", class.reference()));
            }
            return Ok(report);
        }
    };
    if let Some(r) = standing::expiry_refusal(&terms, &response.effective_time) {
        refuse(&mut report, r.rule(), format!("{}: {r}", class.reference()));
        return Ok(report);
    }
    if signer_refusal(repo, &response.accepted_by, &mut report, &at)?.is_none() {
        return Ok(report);
    }
    let earlier = crate::authority_check::verify_excluding(
        repo,
        Act::AcceptStanding,
        &class.subject(),
        &response.accepted_by,
        Some(&class.sha256),
        Some(response_path),
    );
    if earlier.is_signed() {
        refuse(
            &mut report,
            "standing.already-accepted",
            format!(
                "{} is already signed; a class is accepted once",
                class.reference()
            ),
        );
        return Ok(report);
    }
    if mode == crate::sign::IngestMode::DryRun {
        report.push(Diagnostic::pass(
            "standing.would-record",
            format!(
                "{} (sha256:{}) would be accepted by {}; it would then cover Warrants inside \
                 it until {} or {} of them. Nothing written",
                class.reference(),
                &class.sha256[..12],
                response.accepted_by,
                terms.expires_at,
                terms.max_warrants
            ),
        ));
        return Ok(report);
    }
    report.push(Diagnostic::pass(
        "standing.accepted",
        format!(
            "{} accepted by {} acting as {} at {}; the signed response is the record",
            class.reference(),
            response.accepted_by,
            response.acting_role,
            response.effective_time
        ),
    ));
    Ok(report)
}

/// Ingest a signed revocation. The class must be signed and not already
/// revoked; after it the class covers nothing new, and every record stamped
/// before it stands (§31).
pub fn revoke_ingest_with(
    repo: &Repository,
    id: &str,
    revision: u32,
    response_path: &Utf8Path,
    mode: crate::sign::IngestMode,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let at = response_path.to_string();
    let refuse = |report: &mut Report, rule: &str, why: String| {
        report.push(Diagnostic::error(rule, at.clone(), why));
    };
    let Some(class) = read_class(&repo.profiles, &class_path(repo, id, revision)) else {
        refuse(
            &mut report,
            "standing.no-class",
            format!("no class file {id}@{revision}.toml under {DIR}"),
        );
        return Ok(report);
    };
    let text = std::fs::read_to_string(response_path).map_err(|source| RepoError::Io {
        context: format!("could not read {response_path}"),
        source,
    })?;
    let response: RevokeResponse = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            refuse(&mut report, "standing.response-malformed", e.to_string());
            return Ok(report);
        }
    };
    if response.schema != REVOKE_RESPONSE_SCHEMA
        || response.id != id
        || response.revision != revision
    {
        refuse(
            &mut report,
            "standing.response-class",
            format!(
                "the response ({}, {}@{}) is not a revocation of {id}@{revision}",
                response.schema, response.id, response.revision
            ),
        );
        return Ok(report);
    }
    if let Err(e) = openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time) {
        refuse(
            &mut report,
            "standing.effective-time",
            format!("effective_time {:?}: {e}", response.effective_time),
        );
        return Ok(report);
    }
    if response.sha256 != class.sha256 {
        refuse(
            &mut report,
            "standing.stale-digest",
            format!(
                "the response revokes sha256:{} and the class file is sha256:{}",
                response.sha256, class.sha256
            ),
        );
        return Ok(report);
    }
    if !matches!(standing_of(repo, &class), Standing::Accepted(_)) {
        refuse(
            &mut report,
            "standing.unsigned",
            format!(
                "{} was never signed; there is nothing to revoke",
                class.reference()
            ),
        );
        return Ok(report);
    }
    if signer_refusal(repo, &response.revoked_by, &mut report, &at)?.is_none() {
        return Ok(report);
    }
    let earlier = crate::authority_check::verify_excluding(
        repo,
        Act::RevokeStanding,
        &class.subject(),
        &response.revoked_by,
        Some(&class.sha256),
        Some(response_path),
    );
    if earlier.is_signed() {
        refuse(
            &mut report,
            "standing.already-revoked",
            format!("{} is already revoked", class.reference()),
        );
        return Ok(report);
    }
    if mode == crate::sign::IngestMode::DryRun {
        report.push(Diagnostic::pass(
            "standing.would-record",
            format!(
                "{} would be revoked by {}; records already made under it stand (§31). \
                 Nothing written",
                class.reference(),
                response.revoked_by
            ),
        ));
        return Ok(report);
    }
    report.push(Diagnostic::pass(
        "standing.revoked-recorded",
        format!(
            "{} revoked by {} at {}; it covers nothing new, and records made before it stand \
             (§31)",
            class.reference(),
            response.revoked_by,
            response.effective_time
        ),
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subjects_and_targets_are_stable() {
        // The enforcement finds a signature by this name; a rename here would
        // silently make every class unsigned.
        assert_eq!(subject("routine-tui", 2), "STANDING-routine-tui@2");
        assert_eq!(target("routine-tui", 2), "standing:routine-tui@2");
    }

    #[test]
    fn a_class_file_whose_name_disagrees_with_its_record_is_refused() {
        let tmp = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf-8 temp dir")
            .join(format!("war-standing-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("temp dir");
        let text = r#"schema = "oh.war/standing-authorization/v1"
id = "a"
revision = 1
meaning = "m"
paths = ["src/**"]
profile = "delivery"
assurance = "basic"
gates = ["gate://ops.conformance.plants@1.1.0", "gate://software.repo.war-check@1.0.0"]
expires_at = "2026-12-01T00:00:00Z"
max_warrants = 1

[budget]
budget_tokens = 1
wall_time_seconds = 1
max_stages = 1
max_deliverables = 1
"#;
        let good = tmp.join("a@1.toml");
        let bad = tmp.join("b@1.toml");
        std::fs::write(&good, text).expect("write");
        std::fs::write(&bad, text).expect("write");
        assert!(
            read_class(&ProfileRegistry::builtin(), &good)
                .expect("read")
                .parsed
                .is_ok()
        );
        assert!(
            read_class(&ProfileRegistry::builtin(), &bad)
                .expect("read")
                .parsed
                .is_err()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
