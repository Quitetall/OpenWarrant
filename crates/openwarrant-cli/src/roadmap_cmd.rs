// SPDX-License-Identifier: Apache-2.0
//! `war roadmap` and the roadmap record's checks (OW-WAR-0114, OW-ADR-0023).
//!
//! The record lives under `config.paths.roadmap` (`docs/roadmap/`):
//! `roadmap.toml`, its atoms, and `revisions/<n>.toml`. This module loads
//! it, holds Warrants to it (`roadmap.*` rules), and answers three
//! commands:
//!
//! - `war roadmap`: phases in dependency order, each with its exit, its
//!   members and whether it is achieved, read from `status::build` so the
//!   numbers are the ones `war status` reports.
//! - `war roadmap assign <alias> <phase>`: writes the ref on an **unsigned**
//!   Warrant. A signed one is refused by name: its ref is inside the
//!   contract digest, and moving it is an amendment.
//! - `war roadmap propose`: records the atoms as they stand as a proposed
//!   revision, which the queue then offers for one signature.
//!
//! # Placements
//!
//! `[[placement]]` in `roadmap.toml` places a SIGNED Warrant that names no
//! phase, without touching its signed manifest (OW-WAR-0114 A-002); the ref
//! is applied at that Warrant's next amendment. A placement for a Warrant
//! that has its own ref, or that is unsigned (use `assign`), is refused.
//! It is a migration shim for history, not a member list: a new Warrant
//! states its phase itself.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::sha256_hex;
use openwarrant_core::roadmap::{
    Phases, ROADMAP_REVISION_SCHEMA, ROADMAP_SCHEMA, RoadmapError, RoadmapRevision, parse_phases,
};
use openwarrant_core::sas::SasRevisionState;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// `roadmap.toml` as this command reads it: the core manifest plus the
/// placement shim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub uuid: String,
    pub program: String,
    pub prefix: String,
    #[serde(default)]
    pub atoms: Vec<openwarrant_core::roadmap::RoadmapAtomRef>,
    #[serde(default, rename = "placement", skip_serializing_if = "Vec::is_empty")]
    pub placements: Vec<Placement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub warrant: String,
    /// `<PREFIX>-PHASE-<N>`.
    pub phase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
}

/// The record, loaded.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub dir: Utf8PathBuf,
    pub manifest: Manifest,
    pub phases: Phases,
    /// sha256 over the manifest and every atom, in ordinal order.
    pub digest: String,
    pub revisions: Vec<RoadmapRevision>,
}

impl Loaded {
    /// The newest accepted revision.
    #[must_use]
    pub fn accepted(&self) -> Option<&RoadmapRevision> {
        self.revisions
            .iter()
            .filter(|r| r.state == SasRevisionState::Accepted)
            .max_by_key(|r| r.revision)
    }

    /// The proposed revision whose digest is the tree's, if one awaits a
    /// signature.
    #[must_use]
    pub fn pending(&self) -> Option<&RoadmapRevision> {
        self.revisions
            .iter()
            .filter(|r| r.state == SasRevisionState::Proposed && r.sha256 == self.digest)
            .max_by_key(|r| r.revision)
    }

    /// Whether the tree is what a human accepted.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.accepted().is_some_and(|r| r.sha256 == self.digest)
    }

    #[must_use]
    pub fn phase_titles(&self) -> BTreeMap<String, String> {
        self.phases
            .phases
            .iter()
            .map(|p| (p.id.clone(), p.title.clone()))
            .collect()
    }

    #[must_use]
    pub fn placement_of(&self, alias: &str) -> Option<&Placement> {
        self.manifest.placements.iter().find(|p| p.warrant == alias)
    }
}

/// Why the record could not be loaded, kept apart so a cycle is reported as
/// a cycle and not as "malformed".
#[derive(Debug)]
pub enum LoadError {
    Io(String),
    Phases(RoadmapError),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) => f.write_str(s),
            Self::Phases(e) => write!(f, "{e}"),
        }
    }
}

#[must_use]
pub fn dir(repo: &Repository) -> Utf8PathBuf {
    repo.root.join(&repo.config.paths.roadmap)
}

/// `Ok(None)` when the program has no roadmap record yet (a repository
/// mid-migration keeps reading SAS §98).
pub fn load(repo: &Repository) -> Result<Option<Loaded>, LoadError> {
    let dir = dir(repo);
    let manifest_path = dir.join("roadmap.toml");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let read =
        |p: &Utf8Path| fs::read(p).map_err(|e| LoadError::Io(format!("could not read {p}: {e}")));
    let manifest_bytes = read(&manifest_path)?;
    let manifest: Manifest = toml::from_str(&String::from_utf8_lossy(&manifest_bytes))
        .map_err(|e| LoadError::Io(format!("{manifest_path}: {e}")))?;
    if manifest.schema != ROADMAP_SCHEMA {
        return Err(LoadError::Io(format!(
            "{manifest_path}: schema {:?}, expected {ROADMAP_SCHEMA:?}",
            manifest.schema
        )));
    }
    let mut atoms = manifest.atoms.clone();
    atoms.sort_by_key(|a| a.ordinal);
    let mut preimage = format!("roadmap.toml\0{}\n", sha256_hex(&manifest_bytes));
    let mut phases_src = None;
    for a in &atoms {
        let p = dir.join(&a.path);
        let bytes = read(&p)?;
        preimage.push_str(&format!("{}\0{}\n", a.path, sha256_hex(&bytes)));
        if a.role == "phases" {
            phases_src = Some(String::from_utf8_lossy(&bytes).into_owned());
        }
    }
    let phases_src = phases_src
        .ok_or_else(|| LoadError::Io(format!("{manifest_path}: no atom with role `phases`")))?;
    let phases = parse_phases(&phases_src, &manifest.prefix).map_err(LoadError::Phases)?;
    let revisions = load_revisions(&dir).map_err(LoadError::Io)?;
    Ok(Some(Loaded {
        digest: sha256_hex(preimage.as_bytes()),
        dir,
        manifest,
        phases,
        revisions,
    }))
}

fn load_revisions(dir: &Utf8Path) -> Result<Vec<RoadmapRevision>, String> {
    let rdir = dir.join("revisions");
    let Ok(rd) = fs::read_dir(&rdir) else {
        return Ok(vec![]);
    };
    let mut out = Vec::new();
    for e in rd.filter_map(Result::ok) {
        let Ok(p) = Utf8PathBuf::from_path_buf(e.path()) else {
            continue;
        };
        if p.extension() != Some("toml") {
            continue;
        }
        let text = fs::read_to_string(&p).map_err(|e| format!("could not read {p}: {e}"))?;
        let r: RoadmapRevision = toml::from_str(&text).map_err(|e| format!("{p}: {e}"))?;
        if r.schema != ROADMAP_REVISION_SCHEMA {
            return Err(format!("{p}: schema {:?}", r.schema));
        }
        if r.state == SasRevisionState::Accepted && r.acceptance.is_none() {
            return Err(format!("{p}: accepted with no acceptance recorded"));
        }
        out.push(r);
    }
    out.sort_by_key(|r| r.revision);
    Ok(out)
}

#[must_use]
pub fn revision_path(loaded: &Loaded, n: u32) -> Utf8PathBuf {
    loaded.dir.join("revisions").join(format!("{n}.toml"))
}

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

/// The record's own rules, and the ones that need the whole corpus.
/// Per-ref rules (`roadmap.unknown-phase`) run in `check_traceability`,
/// beside the ref's grammar.
pub fn check(repo: &Repository, corpus: &[crate::repo::Loaded], report: &mut Report) {
    let file = repo.relative(&dir(repo).join("roadmap.toml"));
    let loaded = match load(repo) {
        Ok(None) => return,
        Ok(Some(l)) => l,
        Err(LoadError::Phases(e @ RoadmapError::Cycle { .. })) => {
            report.push(Diagnostic::error("roadmap.cycle", file, e.to_string()));
            return;
        }
        Err(e) => {
            report.push(Diagnostic::error("roadmap.malformed", file, e.to_string()));
            return;
        }
    };
    check_signatures(repo, &loaded, report);
    report.push(Diagnostic::pass(
        "roadmap.valid",
        format!(
            "roadmap: {} phases, acyclic, digest sha256:{}",
            loaded.phases.phases.len(),
            &loaded.digest[..12]
        ),
    ));

    if loaded.is_accepted() {
        let rev = loaded.accepted().map_or(0, |r| r.revision);
        report.push(Diagnostic::pass(
            "roadmap.accepted",
            format!("roadmap: revision {rev} is accepted and is the tree"),
        ));
    } else if let Some(p) = loaded.pending() {
        report.push(Diagnostic::warn(
            "roadmap.unaccepted",
            repo.relative(&revision_path(&loaded, p.revision)),
            format!(
                "roadmap: revision {} is proposed and awaits one signature: `war sign roadmap --ssh-sign` (try `--dry-run` first)",
                p.revision
            ),
        ));
    } else {
        report.push(Diagnostic::warn(
            "roadmap.unaccepted",
            file.clone(),
            format!(
                "roadmap: the atoms (sha256:{}) are not an accepted revision; record them with `war roadmap propose`, then a human accepts with `war sign roadmap --ssh-sign`",
                &loaded.digest[..12]
            ),
        ));
    }

    // Which Warrants are replaced — by relation, not by a field.
    let superseded: BTreeSet<String> = corpus
        .iter()
        .filter_map(|w| w.validated.as_ref())
        .flat_map(|v| v.raw.supersedes.iter())
        .map(|s| s.r#ref.trim_start_matches("war://").to_owned())
        .collect();
    let by_alias: BTreeMap<String, &crate::repo::Loaded> =
        corpus.iter().map(|w| (w.alias(), w)).collect();

    let mut placed = BTreeSet::new();
    for p in &loaded.manifest.placements {
        let why = match by_alias.get(&p.warrant) {
            None => Some((
                "roadmap.placement-unknown",
                format!("names {}, which is not in this repository", p.warrant),
            )),
            Some(w)
                if w.basis
                    .as_ref()
                    .is_some_and(|b| !b.manifest.roadmap.is_empty()) =>
            {
                Some((
                    "roadmap.placement-redundant",
                    format!(
                        "{} states its own phase; a placement would be a second answer",
                        p.warrant
                    ),
                ))
            }
            Some(w) if !w.dir.join("authorization.toml").is_file() => Some((
                "roadmap.placement-unsigned",
                format!(
                    "{} is unsigned; give it its own ref with `war roadmap assign {} {}`",
                    p.warrant, p.warrant, p.phase
                ),
            )),
            Some(_) if loaded.phases.get(&p.phase).is_none() => Some((
                "roadmap.unknown-phase",
                format!(
                    "places {} in {}, which the roadmap does not declare",
                    p.warrant, p.phase
                ),
            )),
            Some(_) => None,
        };
        match why {
            Some((rule, msg)) => report.push(Diagnostic::error(
                rule,
                file.clone(),
                format!("roadmap: placement {msg}"),
            )),
            None => {
                placed.insert(p.warrant.clone());
            }
        }
    }

    let mut unassigned = 0usize;
    for w in corpus {
        let Some(v) = w.validated.as_ref() else {
            continue;
        };
        let Some(b) = w.basis.as_ref() else { continue };
        if !b.manifest.roadmap.is_empty()
            || placed.contains(&w.alias())
            || superseded.contains(&v.raw.uuid)
        {
            continue;
        }
        unassigned += 1;
        let alias = w.alias();
        let signed = w.dir.join("authorization.toml").is_file();
        report.push(Diagnostic::warn(
            "roadmap.unassigned",
            repo.relative(&w.dir.join("manifest.toml")),
            if signed {
                format!(
                    "{alias}: names no roadmap phase (§6.4) and is signed, so its manifest cannot take a ref until it is amended; place it with a `[[placement]]` in roadmap.toml and accept the revision (`war roadmap` shows the phases)"
                )
            } else {
                format!(
                    "{alias}: names no roadmap phase (§6.4); `war roadmap` lists the phases, and `war roadmap assign {alias} <phase>` writes the ref"
                )
            },
        ));
    }
    if unassigned == 0 {
        report.push(Diagnostic::pass(
            "roadmap.assigned",
            format!(
                "roadmap: every current Warrant names a phase ({} by placement, pending their next amendment)",
                placed.len()
            ),
        ));
    }
}

/// `roadmap.unknown-phase` for one parsed ref, when a record exists.
pub fn check_ref(
    loaded: &Loaded,
    alias: &str,
    r: &openwarrant_core::traceability::RoadmapRef,
    file: &str,
    report: &mut Report,
) -> bool {
    let id = format!("{}-PHASE-{}", r.prefix, r.phase);
    if loaded.phases.get(&id).is_some() {
        return true;
    }
    report.push(Diagnostic::error(
        "roadmap.unknown-phase",
        file.to_owned(),
        format!(
            "{alias}: roadmap://{id} names a phase the roadmap does not declare; `war roadmap` lists the phases"
        ),
    ));
    false
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// What `war roadmap --json` returns.
#[derive(Debug, Clone, Serialize)]
pub struct View {
    pub schema: &'static str,
    pub program: String,
    pub digest: String,
    pub accepted_revision: Option<u32>,
    pub accepted: bool,
    pub pending_revision: Option<u32>,
    pub phases: Vec<PhaseView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhaseView {
    pub id: String,
    pub title: String,
    pub tier: Option<String>,
    pub depends_on: Vec<String>,
    pub exit: String,
    pub exit_warrant: Option<String>,
    pub achieved: String,
    pub members: Vec<String>,
    pub open: Vec<String>,
}

pub fn view(repo: &Repository) -> Result<(Report, View), RepoError> {
    let mut report = Report::default();
    let loaded = match load(repo) {
        Ok(Some(l)) => l,
        Ok(None) => {
            return Err(RepoError::Message(format!(
                "no roadmap record at {}; this program still reads SAS §98",
                repo.relative(&dir(repo))
            )));
        }
        Err(e) => return Err(RepoError::Message(e.to_string())),
    };
    let status = crate::status::build(repo)?;
    let tier_title: BTreeMap<&str, &str> = loaded
        .phases
        .tiers
        .iter()
        .map(|t| (t.id.as_str(), t.title.as_str()))
        .collect();
    let mut phases = Vec::new();
    for p in loaded.phases.in_dependency_order() {
        let obj = status.objectives.iter().find(|o| {
            o.roadmap_ref
                .as_ref()
                .is_some_and(|r| format!("{}-PHASE-{}", r.prefix, r.phase) == p.id)
        });
        phases.push(PhaseView {
            id: p.id.clone(),
            title: p.title.clone(),
            tier: p
                .priority
                .as_ref()
                .map(|t| format!("{t} {}", tier_title.get(t.as_str()).unwrap_or(&""))),
            depends_on: p.depends_on.clone(),
            exit: p.exit.clone(),
            exit_warrant: obj.and_then(|o| o.exit_warrant.clone()),
            achieved: obj.map_or_else(
                || "not derivable".to_owned(),
                |o| achieved_word(&o.achieved),
            ),
            members: obj.map(|o| o.warrants.clone()).unwrap_or_default(),
            open: p.open.clone(),
        });
    }
    report.push(Diagnostic::pass(
        "roadmap.shown",
        format!("{} phases", phases.len()),
    ));
    Ok((
        report,
        View {
            schema: "oh.war/roadmap-view/v2",
            program: loaded.manifest.program.clone(),
            digest: loaded.digest.clone(),
            accepted_revision: loaded.accepted().map(|r| r.revision),
            accepted: loaded.is_accepted(),
            pending_revision: loaded.pending().map(|r| r.revision),
            phases,
        },
    ))
}

fn achieved_word(a: &openwarrant_core::status::Achieved) -> String {
    use openwarrant_core::status::Achieved as A;
    match a {
        A::Recorded => "achieved".to_owned(),
        A::ExitWarrantWouldSatisfy => "exit Warrant would satisfy; awaiting resolution".to_owned(),
        A::Blocked { by } => format!("blocked by {}", by.len()),
        A::NotDerivable { why } => format!("not derivable: {why}"),
    }
}

#[must_use]
pub fn render(v: &View) -> String {
    let mut s = format!(
        "{} roadmap — {}\n",
        v.program,
        if v.accepted {
            format!("revision {} accepted", v.accepted_revision.unwrap_or(0))
        } else if let Some(p) = v.pending_revision {
            format!("revision {p} proposed, awaiting `war sign roadmap --ssh-sign`")
        } else {
            "not accepted — `war roadmap propose`, then `war sign roadmap --ssh-sign`".to_owned()
        }
    );
    for p in &v.phases {
        s.push_str(&format!(
            "\n{}  {}{}\n    exit: {}\n    {} Warrant(s){} · {}\n",
            p.id,
            p.title,
            p.tier
                .as_ref()
                .map_or(String::new(), |t| format!("  [tier {t}]")),
            p.exit,
            p.members.len(),
            p.exit_warrant
                .as_ref()
                .map_or(String::new(), |e| format!(", exit {e}")),
            p.achieved,
        ));
        if !p.depends_on.is_empty() {
            s.push_str(&format!("    after: {}\n", p.depends_on.join(", ")));
        }
        if !p.open.is_empty() {
            s.push_str(&format!("    no Warrant yet: {}\n", p.open.join(", ")));
        }
    }
    s
}

/// `war roadmap assign <alias> <phase>[/slug]`: the ref on an unsigned
/// Warrant. Refused by name for a signed one.
pub fn assign(repo: &Repository, alias: &str, target: &str) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let loaded = load(repo)
        .map_err(|e| RepoError::Message(e.to_string()))?
        .ok_or_else(|| {
            RepoError::Message("no roadmap record; nothing to assign into".to_owned())
        })?;
    let (phase, slug) = match target.split_once('/') {
        Some((p, s)) => (p, Some(s)),
        None => (target, None),
    };
    let phase = phase.trim_start_matches("roadmap://");
    let dir = repo.warrant_dir(alias)?;
    let manifest_path = dir.join("manifest.toml");
    if loaded.phases.get(phase).is_none() {
        report.push(Diagnostic::error(
            "roadmap.unknown-phase",
            repo.relative(&manifest_path),
            format!("{phase} is not a phase of the roadmap; `war roadmap` lists them"),
        ));
        return Ok(report);
    }
    if dir.join("authorization.toml").is_file() {
        report.push(Diagnostic::error(
            "roadmap.assign-signed",
            repo.relative(&manifest_path),
            format!(
                "{alias} is signed: its roadmap ref is inside the contract digest. Place it with a `[[placement]]` in roadmap.toml (a roadmap revision), or add the ref in its next amendment"
            ),
        ));
        return Ok(report);
    }
    let text = fs::read_to_string(&manifest_path).map_err(|source| RepoError::Io {
        context: format!("could not read {manifest_path}"),
        source,
    })?;
    if text.contains("\n[[roadmap]]") {
        report.push(Diagnostic::error(
            "roadmap.assign-exists",
            repo.relative(&manifest_path),
            format!("{alias} already names a phase; edit its [[roadmap]] entry to move it"),
        ));
        return Ok(report);
    }
    let r = match slug {
        Some(s) => format!("roadmap://{phase}/{s}"),
        None => format!("roadmap://{phase}"),
    };
    openwarrant_core::traceability::RoadmapRef::parse(&r)
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let block = format!("\n[[roadmap]]\nref = \"{r}\"\n");
    let new = match text.find("\n[[atoms]]") {
        Some(at) => format!("{}{block}{}", &text[..at], &text[at..]),
        None => format!("{text}{block}"),
    };
    fs::write(&manifest_path, new).map_err(|source| RepoError::Io {
        context: format!("could not write {manifest_path}"),
        source,
    })?;
    report.push(Diagnostic::pass(
        "roadmap.assigned",
        format!("{alias} → {r}"),
    ));
    Ok(report)
}

/// `war roadmap propose`: the atoms as they stand, as the next revision.
pub fn propose(repo: &Repository) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let loaded = load(repo)
        .map_err(|e| RepoError::Message(e.to_string()))?
        .ok_or_else(|| RepoError::Message("no roadmap record to propose".to_owned()))?;
    if let Some(r) = loaded
        .revisions
        .iter()
        .rev()
        .find(|r| r.sha256 == loaded.digest)
    {
        report.push(Diagnostic::pass(
            "roadmap.proposed",
            format!(
                "revision {} already records these atoms ({})",
                r.revision, r.state
            ),
        ));
        return Ok(report);
    }
    let n = loaded.revisions.last().map_or(1, |r| r.revision + 1);
    let predecessor = loaded.accepted();
    let rec = RoadmapRevision {
        schema: ROADMAP_REVISION_SCHEMA.to_owned(),
        revision: n,
        sha256: loaded.digest.clone(),
        state: SasRevisionState::Proposed,
        predecessor: predecessor.map(|p| p.revision),
        phases: loaded.phase_titles(),
        acceptance: None,
    };
    let diff = openwarrant_core::roadmap::PhaseDiff::between(
        &predecessor.map(|p| p.phases.clone()).unwrap_or_default(),
        &rec.phases,
    );
    let path = revision_path(&loaded, n);
    fs::create_dir_all(loaded.dir.join("revisions")).map_err(|source| RepoError::Io {
        context: "could not create revisions/".to_owned(),
        source,
    })?;
    fs::write(
        &path,
        toml::to_string_pretty(&rec).map_err(|e| RepoError::Message(e.to_string()))?,
    )
    .map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })?;
    report.push(Diagnostic::pass(
        "roadmap.proposed",
        format!(
            "revision {n} proposed at sha256:{} → {}; {}",
            &loaded.digest[..12],
            repo.relative(&path),
            diff.summary()
        ),
    ));
    report.note(
        "Proposed, not accepted. One human act accepts it: `war sign roadmap --ssh-sign` (try `--dry-run` first).".to_owned(),
    );
    Ok(report)
}

// ---------------------------------------------------------------------------
// Acceptance (M3): the SAS acceptance path, for a roadmap revision
// ---------------------------------------------------------------------------

pub const ACCEPT_REQUEST_SCHEMA: &str = "oh.war/roadmap-acceptance-request/v1";
pub const ACCEPT_RESPONSE_SCHEMA: &str = "oh.war/roadmap-acceptance-response/v1";

/// The subject a roadmap revision's signed response is named for.
#[must_use]
pub fn subject(revision: u32) -> String {
    format!("ROADMAP-{revision}")
}

/// What is put to the acceptor: the revision, its digest, and what moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptRequest {
    pub schema: String,
    pub revision: u32,
    pub sha256: String,
    pub predecessor: Option<u32>,
    pub diff: openwarrant_core::roadmap::PhaseDiff,
    pub phase_count: usize,
    pub eligible_acceptors: Vec<String>,
}

/// What the acceptor returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptResponse {
    pub schema: String,
    pub revision: u32,
    /// Must equal the record's AND the tree's digest now.
    pub sha256: String,
    pub accepted_by: String,
    pub acting_role: String,
    pub meaning: String,
    pub effective_time: String,
}

fn find_revision(loaded: &Loaded, n: u32) -> Result<&RoadmapRevision, RepoError> {
    loaded
        .revisions
        .iter()
        .find(|r| r.revision == n)
        .ok_or_else(|| RepoError::Message(format!("no roadmap revision {n} on record")))
}

pub fn accept_request(repo: &Repository, n: u32) -> Result<AcceptRequest, RepoError> {
    let loaded = load(repo)
        .map_err(|e| RepoError::Message(e.to_string()))?
        .ok_or_else(|| RepoError::Message("no roadmap record".to_owned()))?;
    let rec = find_revision(&loaded, n)?;
    let before = rec
        .predecessor
        .and_then(|p| loaded.revisions.iter().find(|r| r.revision == p))
        .map(|r| r.phases.clone())
        .unwrap_or_default();
    let register = repo.load_authority_register()?;
    Ok(AcceptRequest {
        schema: ACCEPT_REQUEST_SCHEMA.to_owned(),
        revision: n,
        sha256: rec.sha256.clone(),
        predecessor: rec.predecessor,
        diff: openwarrant_core::roadmap::PhaseDiff::between(&before, &rec.phases),
        phase_count: rec.phases.len(),
        eligible_acceptors: register
            .holders(openwarrant_core::authority::ActorRole::Authorizer)
            .filter(|a| a.may_authorize(&repo.performer()).is_ok())
            .map(|a| a.actor.clone())
            .collect(),
    })
}

/// The proposed revision the tree matches, as a pending act.
pub fn pending_request(repo: &Repository) -> Result<Option<AcceptRequest>, RepoError> {
    let Ok(Some(loaded)) = load(repo) else {
        return Ok(None);
    };
    match loaded.pending() {
        Some(p) => accept_request(repo, p.revision).map(Some),
        None => Ok(None),
    }
}

/// Ingest a signed acceptance. `DryRun` judges everything and writes nothing.
pub fn accept_ingest_with(
    repo: &Repository,
    n: u32,
    response_path: &Utf8Path,
    mode: crate::sign::IngestMode,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let refuse = |report: &mut Report, rule: &str, why: String| {
        report.push(Diagnostic::error(rule, response_path.to_string(), why));
    };
    let loaded = load(repo)
        .map_err(|e| RepoError::Message(e.to_string()))?
        .ok_or_else(|| RepoError::Message("no roadmap record".to_owned()))?;
    let rec = find_revision(&loaded, n)?.clone();
    let text = fs::read_to_string(response_path).map_err(|source| RepoError::Io {
        context: format!("could not read {response_path}"),
        source,
    })?;
    let response: AcceptResponse = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            refuse(&mut report, "roadmap.response-malformed", e.to_string());
            return Ok(report);
        }
    };
    if response.schema != ACCEPT_RESPONSE_SCHEMA {
        refuse(
            &mut report,
            "roadmap.response-schema",
            format!("unknown schema {:?}", response.schema),
        );
        return Ok(report);
    }
    if response.revision != n {
        refuse(
            &mut report,
            "roadmap.response-revision",
            format!(
                "response names revision {}, ingesting {n}",
                response.revision
            ),
        );
        return Ok(report);
    }
    if let Err(e) = openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time) {
        refuse(
            &mut report,
            "roadmap.effective-time",
            format!("effective_time {:?}: {e}", response.effective_time),
        );
        return Ok(report);
    }
    if response.sha256 != rec.sha256 || loaded.digest != rec.sha256 {
        refuse(
            &mut report,
            "roadmap.stale-digest",
            format!(
                "response signs {}, revision {n} records {}, the roadmap is now {}; the atoms moved after proposal — `war roadmap propose` again and sign that",
                response.sha256, rec.sha256, loaded.digest
            ),
        );
        return Ok(report);
    }
    if rec.state == SasRevisionState::Accepted {
        refuse(
            &mut report,
            "roadmap.already-accepted",
            format!("revision {n} is already accepted"),
        );
        return Ok(report);
    }
    let register = repo.load_authority_register()?;
    let Some(assignment) = register.actor(&response.accepted_by) else {
        refuse(
            &mut report,
            "roadmap.unknown-actor",
            format!(
                "{:?} holds no role assignment in docs/authority/roles.toml",
                response.accepted_by
            ),
        );
        return Ok(report);
    };
    if let Err(e) = assignment.may_authorize(&repo.performer()) {
        refuse(&mut report, "roadmap.not-permitted", e.to_string());
        return Ok(report);
    }
    let path = revision_path(&loaded, n);
    if mode == crate::sign::IngestMode::DryRun {
        report.push(Diagnostic::pass(
            "roadmap.would-record",
            format!(
                "roadmap revision {n} would be accepted by {} → {}; nothing written",
                response.accepted_by,
                repo.relative(&path)
            ),
        ));
        return Ok(report);
    }
    let accepted = RoadmapRevision {
        state: SasRevisionState::Accepted,
        acceptance: Some(openwarrant_core::sas::SasAcceptance {
            accepted_by: response.accepted_by.clone(),
            actor_kind: assignment.actor_kind,
            acting_role: response.acting_role.clone(),
            meaning: response.meaning.clone(),
            effective_time: response.effective_time.clone(),
            adr_ref: None,
        }),
        ..rec
    };
    fs::write(
        &path,
        toml::to_string_pretty(&accepted).map_err(|e| RepoError::Message(e.to_string()))?,
    )
    .map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })?;
    report.push(Diagnostic::pass(
        "roadmap.accepted",
        format!(
            "roadmap revision {n} accepted by {} acting as {} → {}",
            response.accepted_by,
            response.acting_role,
            repo.relative(&path)
        ),
    ));
    Ok(report)
}

/// `authority.*` for every accepted revision: believed because a human
/// signed it, not because the file says so.
pub fn check_signatures(repo: &Repository, loaded: &Loaded, report: &mut Report) {
    for rev in loaded
        .revisions
        .iter()
        .filter(|r| r.state == SasRevisionState::Accepted)
    {
        let Some(a) = &rev.acceptance else { continue };
        let s = subject(rev.revision);
        let v = crate::authority_check::verify(
            repo,
            crate::authority_check::Act::AcceptRoadmap,
            &s,
            &a.accepted_by,
            Some(&rev.sha256),
        );
        if v.is_signed() {
            report.push(Diagnostic::pass(v.rule(), format!("{s}: {}", v.why())));
        } else {
            report.push(Diagnostic::error(
                v.rule(),
                repo.relative(&revision_path(loaded, rev.revision)),
                format!("{s}: {}", v.why()),
            ));
        }
    }
}
