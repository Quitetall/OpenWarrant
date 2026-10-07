// SPDX-License-Identifier: Apache-2.0
//! `war model` — the compiled corpus as one document, `oh.war/model/v1`
//! (OW-WAR-0148 M1; OW-ADR-0031).
//!
//! ```text
//! oh.war/model/v1 = { schema, basis_digest, records[], relations[], states[], diagnostics[] }
//! ```
//!
//! Built from the [`Corpus`] every other reader shares, so it says nothing a
//! builder does not already derive; it only gives each thing an address.
//! Shaped to export to Liminal without translation (OW-ADR-0031): a record is
//! a Node, a relation a Relation, a record's `revision` a member of a
//! Workspace Basis, and `governed_by` its Jurisdiction.
//!
//! - **Records:** every Warrant, obligation, deliverable, stage, question,
//!   roadmap phase, SAS requirement, ticket and ticket item. A record's id
//!   is global: a Warrant's own records are `<alias>/<id>`, a ticket's items
//!   `<ticket>/<item>`.
//! - **Revision:** the digest of the record's own bytes where it has them
//!   (a Warrant's compiled contract, a question's file, an item's line);
//!   otherwise the digest of the atom or file that holds it.
//! - **Read in place** (OW-WAR-0148 M10): each OpenSpec change (`change`)
//!   or Spec Kit feature (`feature`) a `[[adapters]]` entry names, its tasks
//!   (`item`) and its requirements, outcomes and stories, each with the
//!   revision of its own bytes, and the relations its folder states.
//! - **Authored records** (OW-WAR-0148 M3): every record of a record atom
//!   under `docs/records/`, with its type (a profile noun) and the revision
//!   of its own byte span; and the relations documents author — record
//!   atoms' relation lines, obligations' `evaluates`, ticket items'
//!   `implements` — each with the revision it pins, if any.
//! - **Store types** (OW-WAR-0148 M18): the roadmap record
//!   (`<prefix>-ROADMAP`, type `roadmap`) its phases are `part_of`, with
//!   each phase's `depends_on`; the specification (`<NS>-SAS`, type `spec`),
//!   each numbered section and subsection (`<NS>-SAS-43`, `<NS>-SAS-43.5`,
//!   type `section`, the revision of its own byte span) and each §106
//!   requirement `part_of` it; each ADR (its alias, type `adr`) with its
//!   front matter's `status`, its `supersedes`, and the Warrants it governs
//!   as the inert `adr.governs`. Each only where the type that reads the
//!   store selects `structure`, related only under `links`.
//! - **Instruction sections** (M16): each `##` section of the root
//!   `CLAUDE.md` and `AGENTS.md` (and configured nested files), id
//!   `md:<file>#<slug>`, type `instruction`, its revision that of its own
//!   byte span. The managed `openwarrant` block is none of them.
//! - **Relations:** `part_of`, `parent`, `supersedes`, `roadmap`,
//!   `implements`, `depends_on`, `promoted_to`. A relation whose target is
//!   not a record here is kept AND reported (`model.relation-target-unknown`):
//!   never silently dropped.
//! - **States:** what the builders already derive — a Warrant's §24 state,
//!   its rung and currency, an obligation's disposition, a phase's
//!   achievement, a checklist's done/open. Each says whether it is
//!   `recorded` or `computed`. Beside them, the kernel states
//!   (OW-WAR-0148 M4): each fixed state that holds, of kind `computed` or
//!   `authenticated` with its RQ-032 `facet`, and each declared state
//!   entered, of kind `declared` with what it `refines` and whether it has
//!   `lapsed`. An item's `in_progress` reads the claim locks, the one input
//!   outside the tree; a declared state reads the journals.
//! - **basis_digest:** sha256 over the sorted `(id, revision)` pairs: the
//!   same tree gives the same digest and the same bytes.
//!
//! Nothing here is evaluated twice and nothing is written.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use openwarrant_core::Capability as C;
use openwarrant_core::projection::Store;

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const SCHEMA: &str = "oh.war/model/v1";

/// The compiled model.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// `oh.war/model/v1`.
    pub schema: String,
    /// `sha256:` over the sorted `(id, revision)` pairs of every record.
    pub basis_digest: String,
    /// Sorted by id.
    pub records: Vec<Record>,
    /// Sorted by (from, kind, to).
    pub relations: Vec<Relation>,
    /// Sorted by (record, kind).
    pub states: Vec<State>,
    /// Sorted by (rule, record, message).
    pub diagnostics: Vec<ModelDiagnostic>,
}

/// One addressable record (a Liminal Node).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// Global: `OW-WAR-0001`, `OW-WAR-0001/OBL-002`, `t-3f2a/i-9c01`.
    pub id: String,
    /// `warrant`, `obligation`, `deliverable`, `stage`, `question`, `phase`,
    /// `requirement`, `ticket` or `item`; a store's records (OW-WAR-0148
    /// M18): `roadmap`, `spec`, `section` or `adr`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Repository-relative path of the file that holds it.
    pub source: String,
    /// `sha256:<hex>` of its own bytes, else of the file that holds it; a
    /// Warrant's is its contract digest as it compiles now.
    pub revision: String,
    /// Who governs it (a Liminal Jurisdiction), where that is defined: a
    /// Warrant's authorizer once authorized; a Warrant's own records, their
    /// Warrant; a deliverable, the Warrant that governs its path now
    /// (OW-ADR-0021); a phase, whoever accepted the roadmap; an item, its
    /// ticket. Absent where nobody does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governed_by: Option<String>,
}

/// `{from, kind, to}` (a Liminal Relation).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub from: String,
    /// `part_of`, `parent`, `supersedes`, `roadmap`, `implements`,
    /// `depends_on` or `promoted_to`; from authored relations (OW-WAR-0148
    /// M3), any core kind or a namespaced one (`x.mentions`), which is
    /// carried and drives nothing.
    pub kind: String,
    pub to: String,
    /// The revision of `to` the relation pins (`REQ-pr1@sha256:…`), when it
    /// pins one. A pin that is not `to`'s revision now reads stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_revision: Option<String>,
}

/// A state a builder derives for a record, or a kernel state that holds for
/// it (OW-WAR-0148 M4).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub record: String,
    /// What the builders derive: `phase`, `rung`, `currency`,
    /// `disposition`, `achieved`, `checklist`, or an ADR's `status` as its
    /// front matter states it. A kernel state (OW-ADR-0031) is `computed`,
    /// `authenticated` or `declared`, and its `value` is the state's name.
    pub kind: String,
    pub value: String,
    /// `recorded` (read from a record of an act), `computed`, or `authored`:
    /// a declared state's entry in a journal, an ADR's status in its front
    /// matter.
    pub provenance: String,
    /// A fixed kernel state's dimension in SAS §24 / RQ-032's decomposition:
    /// `phase`, `outcome`, `currency` or `standing`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facet: Option<String>,
    /// A declared state's fixed parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refines: Option<String>,
    /// A declared state whose parent stopped holding: still on record, and
    /// no longer held.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lapsed: bool,
}

/// A finding about the model itself.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDiagnostic {
    pub rule: String,
    /// The record it is about, or the path when no record could be formed.
    pub record: String,
    pub message: String,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes))
}

fn file_digest(path: &Utf8Path) -> Option<String> {
    crate::vfs::read(path).ok().map(|b| digest(&b))
}

fn word<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(str::to_owned))
        .unwrap_or_default()
}

#[derive(Default)]
struct Builder {
    records: Vec<Record>,
    relations: BTreeSet<Relation>,
    states: BTreeSet<State>,
    diagnostics: BTreeSet<ModelDiagnostic>,
}

impl Builder {
    fn record(
        &mut self,
        id: String,
        kind: &str,
        source: String,
        revision: String,
        governed_by: Option<String>,
    ) {
        self.records.push(Record {
            id,
            kind: kind.to_owned(),
            source,
            revision,
            governed_by,
        });
    }
    fn relate(&mut self, from: &str, kind: &str, to: &str) {
        self.relate_pinned(from, kind, to, None);
    }
    fn relate_pinned(&mut self, from: &str, kind: &str, to: &str, pin: Option<String>) {
        self.relations.insert(Relation {
            from: from.to_owned(),
            kind: kind.to_owned(),
            to: to.to_owned(),
            to_revision: pin,
        });
    }
    fn state(&mut self, record: &str, kind: &str, value: String, recorded: bool) {
        self.states.insert(State {
            record: record.to_owned(),
            kind: kind.to_owned(),
            value,
            provenance: if recorded { "recorded" } else { "computed" }.to_owned(),
            facet: None,
            refines: None,
            lapsed: false,
        });
    }
    fn diagnose(&mut self, rule: &str, record: &str, message: String) {
        self.diagnostics.insert(ModelDiagnostic {
            rule: rule.to_owned(),
            record: record.to_owned(),
            message,
        });
    }
}

/// Build the model from the corpus.
pub fn build(corpus: &Corpus) -> Result<Model, RepoError> {
    let repo = corpus.repo();
    let status = corpus.status()?;
    let mut b = Builder::default();
    let mut uuid_alias: BTreeMap<String, String> = BTreeMap::new();
    for e in corpus.entries()? {
        if let Some(v) = e.ok().and_then(|l| l.validated.as_ref()) {
            uuid_alias.insert(v.uuid.to_string(), v.alias.to_string());
        }
    }
    let ownership = corpus.ownership().ok();

    // ---- Warrants and their records.
    for e in corpus.entries()? {
        let one = match e.loaded() {
            Ok(one) => one,
            Err(err) => {
                b.diagnose(
                    "model.warrant-unreadable",
                    &repo.relative(&e.dir),
                    err.to_string(),
                );
                continue;
            }
        };
        let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
            b.diagnose(
                "model.warrant-invalid",
                &repo.relative(&e.dir),
                format!(
                    "{}: the manifest did not validate, so it is no record",
                    e.name()
                ),
            );
            continue;
        };
        let alias = validated.alias.to_string();
        let auth = repo.load_authorization(&one.dir).ok().flatten();
        let authorizer = auth.as_ref().and_then(|a| {
            (a.revision.state == openwarrant_core::RevisionState::Authorized)
                .then(|| {
                    a.revision
                        .authorization
                        .as_ref()
                        .map(|x| x.authorizer.clone())
                })
                .flatten()
        });
        b.record(
            alias.clone(),
            "warrant",
            repo.relative(&one.dir.join("manifest.toml")),
            match e.contract_digest() {
                Some(d) => format!("sha256:{d}"),
                None => digest(&basis.manifest_bytes),
            },
            authorizer,
        );
        let atoms_of = |role: &str| -> Vec<(String, String, &openwarrant_compiler::AtomSource)> {
            basis
                .atoms
                .iter()
                .filter(|a| a.role == role)
                .map(|a| (repo.relative(&one.dir.join(&a.source)), digest(&a.bytes), a))
                .collect()
        };
        for (source, rev, atom) in atoms_of("assurance") {
            if let Ok(set) =
                openwarrant_core::obligation::parse(&String::from_utf8_lossy(&atom.bytes))
            {
                for o in set.obligations {
                    let id = format!("{alias}/{}", o.id);
                    b.record(
                        id.clone(),
                        "obligation",
                        source.clone(),
                        rev.clone(),
                        Some(alias.clone()),
                    );
                    b.relate(&id, "part_of", &alias);
                }
            }
        }
        for (source, rev, atom) in atoms_of("milestones") {
            if let Ok(g) =
                openwarrant_core::milestones::parse(&String::from_utf8_lossy(&atom.bytes))
            {
                for s in g.stages {
                    let id = format!("{alias}/{}", s.id);
                    b.record(
                        id.clone(),
                        "stage",
                        source.clone(),
                        rev.clone(),
                        Some(alias.clone()),
                    );
                    b.relate(&id, "part_of", &alias);
                }
            }
        }
        let deliverables_path = one.dir.join("deliverables.toml");
        if let (Ok(set), Some(rev)) = (
            repo.load_deliverables(&one.dir),
            file_digest(&deliverables_path),
        ) {
            for d in &set.records {
                let id = format!("{alias}/{}", d.id);
                let governor = ownership
                    .and_then(|o| o.current(&d.target_ref))
                    .map_or_else(|| alias.clone(), |o| o.alias.clone());
                b.record(
                    id.clone(),
                    "deliverable",
                    repo.relative(&deliverables_path),
                    rev.clone(),
                    Some(governor),
                );
                b.relate(&id, "part_of", &alias);
            }
        }
        if let Ok(rd) = crate::vfs::read_dir(one.dir.join("questions")) {
            let mut paths: Vec<_> = rd
                .filter_map(Result::ok)
                .filter_map(|x| camino::Utf8PathBuf::from_path_buf(x.path()).ok())
                .filter(|p| p.extension() == Some("toml"))
                .collect();
            paths.sort();
            for p in paths {
                let Ok(bytes) = crate::vfs::read(&p) else {
                    continue;
                };
                let qid = toml::from_str::<toml::Value>(&String::from_utf8_lossy(&bytes))
                    .ok()
                    .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_owned));
                let Some(qid) = qid else {
                    b.diagnose(
                        "model.question-unreadable",
                        &repo.relative(&p),
                        "no `id` could be read".to_owned(),
                    );
                    continue;
                };
                let id = format!("{alias}/{qid}");
                b.record(
                    id.clone(),
                    "question",
                    repo.relative(&p),
                    digest(&bytes),
                    Some(alias.clone()),
                );
                b.relate(&id, "part_of", &alias);
            }
        }
        // Relations the manifest declares.
        let target = |r: &str| -> String {
            let bare = r.strip_prefix("war://").unwrap_or(r);
            uuid_alias
                .get(bare)
                .cloned()
                .unwrap_or_else(|| r.to_owned())
        };
        for p in &basis.manifest.parents {
            b.relate(&alias, "parent", &target(&p.r#ref));
        }
        for s in &basis.manifest.supersedes {
            b.relate(&alias, "supersedes", &target(&s.r#ref));
        }
        for r in &basis.manifest.roadmap {
            match openwarrant_core::traceability::RoadmapRef::parse(&r.r#ref) {
                Ok(rr) => b.relate(
                    &alias,
                    "roadmap",
                    &format!("{}-PHASE-{}", rr.prefix, rr.phase),
                ),
                Err(_) => b.relate(&alias, "roadmap", &r.r#ref),
            }
        }
        for i in &basis.manifest.implements {
            match openwarrant_core::traceability::RequirementRef::parse(&i.r#ref) {
                Ok(rq) => b.relate(&alias, "implements", &rq.canonical()),
                Err(_) => b.relate(&alias, "implements", &i.r#ref),
            }
        }
    }

    // ---- States the status already derived.
    for w in &status.warrants {
        if let Some(s) = &w.state {
            b.state(
                &w.alias,
                "phase",
                word(&s.phase),
                word(&s.provenance) == "recorded",
            );
        }
        b.state(&w.alias, "rung", word(&w.rung), false);
        b.state(
            &w.alias,
            "currency",
            corpus.currencies().of(&w.alias).to_string(),
            false,
        );
        for o in &w.obligations {
            b.state(
                &format!("{}/{}", w.alias, o.id),
                "disposition",
                o.disposition.clone(),
                false,
            );
        }
    }

    // ---- Kernel states (OW-WAR-0148 M4): each fixed state that holds,
    // computed or authenticated, and each declared state entered, held or
    // lapsed. Beside the builders' states above, never in place of them.
    b.states.extend(crate::states::model_states(corpus)?);

    // ---- Roadmap phases.
    let objectives: BTreeMap<String, &openwarrant_core::status::ObjectiveStatus> = status
        .objectives
        .iter()
        .filter_map(|o| {
            o.roadmap_ref
                .as_ref()
                .map(|r| (format!("{}-PHASE-{}", r.prefix, r.phase), o))
        })
        .collect();
    // Status lists a phase nobody declared so that the Warrants naming it stay
    // visible. It is no record of the SAS or of the roadmap record, and holds
    // no state: a relation to it stays `model.relation-target-unknown`.
    let mut undeclared: BTreeSet<&str> = BTreeSet::new();
    // OW-WAR-0148 M18: the `roadmap` type's records, read from its store
    // under `structure`, related under `links`.
    let roadmap_caps = crate::types::caps(repo, Store::Roadmap);
    let roadmap_record = if roadmap_caps.has(C::Structure) {
        corpus.roadmap()
    } else {
        Ok(None)
    };
    match roadmap_record {
        Ok(Some(rm)) => {
            let atom = rm
                .manifest
                .atoms
                .iter()
                .filter(|a| a.role == "phases")
                .min_by_key(|a| a.ordinal)
                .map(|a| rm.dir.join(&a.path));
            let source = atom.as_ref().map_or_else(
                || repo.relative(&rm.dir.join("roadmap.toml")),
                |p| repo.relative(p),
            );
            let rev = atom
                .as_deref()
                .and_then(file_digest)
                .unwrap_or_else(|| format!("sha256:{}", rm.digest.trim_start_matches("sha256:")));
            let accepted_by = rm
                .accepted()
                .and_then(|r| r.acceptance.as_ref().map(|a| a.accepted_by.clone()));
            for p in &rm.phases.phases {
                b.record(
                    p.id.clone(),
                    "phase",
                    source.clone(),
                    rev.clone(),
                    accepted_by.clone(),
                );
            }
            for pl in &rm.manifest.placements {
                b.relate(&pl.warrant, "roadmap", &pl.phase);
            }
            let roadmap = roadmap_id(rm);
            b.record(
                roadmap.clone(),
                "roadmap",
                repo.relative(&rm.dir.join("roadmap.toml")),
                format!("sha256:{}", rm.digest.trim_start_matches("sha256:")),
                accepted_by.clone(),
            );
            if roadmap_caps.has(C::Links) {
                for p in &rm.phases.phases {
                    b.relate(&p.id, "part_of", &roadmap);
                    for d in &p.depends_on {
                        b.relate(&p.id, "depends_on", d);
                    }
                }
            }
            let ids: BTreeSet<&str> = rm.phases.phases.iter().map(|p| p.id.as_str()).collect();
            undeclared.extend(
                objectives
                    .keys()
                    .map(String::as_str)
                    .filter(|id| !ids.contains(id)),
            );
        }
        Ok(None) => {
            // No record: the phases are SAS §98's, as status reads them.
            let (source, rev) = repo
                .sas_document()
                .map(|(p, bytes)| (repo.relative(&p), digest(&bytes)))
                .unwrap_or_else(|_| ("SAS §98".to_owned(), digest(b"builtin:sas-98")));
            let declared: BTreeSet<u8> = crate::status::sas_phases(repo)
                .iter()
                .map(|(n, _, _)| *n)
                .collect();
            for (id, o) in &objectives {
                if o.roadmap_ref
                    .as_ref()
                    .is_some_and(|r| declared.contains(&r.phase))
                {
                    b.record(id.clone(), "phase", source.clone(), rev.clone(), None);
                } else {
                    undeclared.insert(id);
                }
            }
        }
        Err(e) => b.diagnose("model.roadmap-unreadable", "roadmap", e.to_string()),
    }
    for (id, o) in objectives
        .iter()
        .filter(|(id, _)| !undeclared.contains(id.as_str()))
    {
        let value = match &o.achieved {
            openwarrant_core::status::Achieved::Recorded => "achieved".to_owned(),
            openwarrant_core::status::Achieved::ExitWarrantWouldSatisfy => {
                "exit_would_satisfy".to_owned()
            }
            openwarrant_core::status::Achieved::Blocked { .. } => "blocked".to_owned(),
            openwarrant_core::status::Achieved::NotDerivable { .. } => "not_derivable".to_owned(),
        };
        b.state(id, "achieved", value, false);
    }

    // ---- The `spec` type (OW-WAR-0148 M18): the specification, its
    // numbered sections and subsections (each with its own span's revision)
    // and its §106 requirements, the targets of `implements`.
    let spec_caps = crate::types::caps(repo, Store::Sas);
    if spec_caps.has(C::Structure)
        && let Ok((path, bytes)) = repo.sas_document()
    {
        let (source, rev) = (repo.relative(&path), digest(&bytes));
        let text = String::from_utf8_lossy(&bytes);
        let spec = spec_id(repo, &text);
        let links = spec_caps.has(C::Links);
        b.record(
            spec.clone(),
            "spec",
            source.clone(),
            rev.clone(),
            spec_acceptor(corpus, &rev),
        );
        for section in openwarrant_core::sas_sections::split(&bytes)
            .iter()
            .filter(|s| s.kind == openwarrant_core::sas_sections::SectionKind::Numbered)
        {
            let id = format!("{spec}-{}", section.id);
            b.record(
                id.clone(),
                "section",
                source.clone(),
                format!("sha256:{}", section.sha256),
                None,
            );
            if links {
                b.relate(&id, "part_of", &spec);
            }
            for sub in &section.subsections {
                let sid = format!("{spec}-{}", sub.id);
                b.record(
                    sid.clone(),
                    "section",
                    source.clone(),
                    format!("sha256:{}", sub.sha256),
                    None,
                );
                if links {
                    b.relate(&sid, "part_of", &id);
                }
            }
        }
        for r in status.requirements.iter().filter(|r| r.title.is_some()) {
            let id = r.requirement.canonical();
            b.record(id.clone(), "requirement", source.clone(), rev.clone(), None);
            if links {
                b.relate(&id, "part_of", &spec);
            }
        }
    }

    // ---- The `adr` type (OW-WAR-0148 M18): one record per ADR atom, with
    // the status its own front matter states (authored, never signed), the
    // decisions it supersedes, and the Warrants it governs, carried as the
    // namespaced `adr.governs` (inert).
    let adr_caps = crate::types::caps(repo, Store::Adr);
    if adr_caps.has(C::Structure)
        && let Ok(adrs) = corpus.adrs()
    {
        let adr_target =
            |r: &str| -> String { adr_ref(&adrs.records, r).unwrap_or_else(|| r.to_owned()) };
        let aliases: BTreeSet<&str> = uuid_alias.values().map(String::as_str).collect();
        for a in &adrs.records {
            b.record(
                a.local_alias.clone(),
                "adr",
                a.source.clone(),
                file_digest(&repo.root.join(&a.source))
                    .unwrap_or_else(|| digest(a.body.as_bytes())),
                None,
            );
            b.states.insert(State {
                record: a.local_alias.clone(),
                kind: "status".to_owned(),
                value: a.status.as_str().to_owned(),
                provenance: "authored".to_owned(),
                facet: None,
                refines: None,
                lapsed: false,
            });
            if adr_caps.has(C::Links) {
                for s in &a.supersedes {
                    b.relate(&a.local_alias, "supersedes", &adr_target(s));
                }
                if let Some(by) = &a.superseded_by {
                    b.relate(&adr_target(by), "supersedes", &a.local_alias);
                }
                for g in &a.governs {
                    // `war://<uuid>`, or `war://<alias>` as some ADRs write it.
                    let bare = g.strip_prefix("war://").unwrap_or(g);
                    let to = uuid_alias.get(bare).cloned().unwrap_or_else(|| {
                        if aliases.contains(bare) {
                            bare.to_owned()
                        } else {
                            g.clone()
                        }
                    });
                    b.relate(&a.local_alias, "adr.governs", &to);
                }
            }
        }
    }

    // ---- Tickets and their items.
    match corpus.tickets() {
        Ok((tickets, faults)) => {
            for f in faults {
                b.diagnose(
                    "model.ticket-unreadable",
                    f.file.as_deref().unwrap_or("tickets"),
                    f.message.clone(),
                );
            }
            for t in tickets {
                let tid = t.id().to_owned();
                let manifest = t.dir.join("manifest.toml");
                b.record(
                    tid.clone(),
                    "ticket",
                    repo.relative(&manifest),
                    file_digest(&manifest).unwrap_or_else(|| digest(t.checklist_text.as_bytes())),
                    None,
                );
                // OW-WAR-0148 M5: the ticket's relations as the ticket store
                // itself reads them (`part_of`, `depends_on`, `promoted_to`).
                for r in openwarrant_core::ticket::kernel_relations(&t.manifest, &t.checklist) {
                    b.relate(&r.from, r.kind, &r.to);
                }
                let lines: Vec<&str> = t.checklist_text.split_inclusive('\n').collect();
                let mut all_done = !t.checklist.items.is_empty();
                for item in &t.checklist.items {
                    all_done &= item.done;
                    let Some(iid) = &item.id else { continue };
                    let id = format!("{tid}/{iid}");
                    let line = lines.get(item.line).copied().unwrap_or_default();
                    b.record(
                        id.clone(),
                        "item",
                        repo.relative(&t.checklist_path),
                        digest(line.as_bytes()),
                        Some(tid.clone()),
                    );
                    b.state(
                        &id,
                        "checklist",
                        if item.done { "done" } else { "open" }.to_owned(),
                        false,
                    );
                }
                b.state(
                    &tid,
                    "checklist",
                    if all_done { "done" } else { "open" }.to_owned(),
                    false,
                );
            }
        }
        Err(e) => b.diagnose("model.tickets-unreadable", "tickets", e.to_string()),
    }

    // ---- Read in place (OW-WAR-0148 M10): each OpenSpec change or Spec
    // Kit feature an `[[adapters]]` entry names, its tasks as items and its
    // requirements, outcomes and stories as records, with their relations;
    // what its folder does not let be read is a diagnostic by rule.
    let adapted = corpus.adapters();
    for f in adapted.faults() {
        b.diagnose(f.rule, &f.place(), f.message.clone());
    }
    for tree in &adapted.trees {
        let kind = match tree.kind {
            "openspec" => "change",
            _ => "feature",
        };
        for w in &tree.warrants {
            b.record(
                w.id.clone(),
                kind,
                w.source.clone(),
                w.revision.clone(),
                None,
            );
            b.state(&w.id, "checklist", w.state().to_owned(), false);
            for t in &w.tasks {
                b.record(
                    t.id.clone(),
                    "item",
                    w.tasks_file.clone().unwrap_or_default(),
                    t.revision.clone(),
                    Some(w.id.clone()),
                );
                b.state(
                    &t.id,
                    "checklist",
                    if t.done { "done" } else { "open" }.to_owned(),
                    false,
                );
            }
        }
        for r in &tree.records {
            b.record(
                r.id.clone(),
                r.kind,
                r.source.clone(),
                r.revision.clone(),
                None,
            );
        }
        for (from, kind, to) in &tree.relations {
            b.relate(from, kind, to);
        }
    }

    // ---- Record atoms and authored relations (OW-WAR-0148 M3): records
    // join the others, relations join the existing kinds, and each refusal
    // is a diagnostic under its own rule.
    let authored = corpus.records();
    for r in &authored.records {
        b.record(
            r.id.clone(),
            &r.record_type,
            r.source.clone(),
            r.revision.clone(),
            None,
        );
    }
    for r in &authored.relations {
        b.relate_pinned(&r.from, r.kind.as_str(), &r.target.id, r.target.pin.clone());
    }
    for f in &authored.faults {
        b.diagnose(f.rule, &format!("{}:{}", f.file, f.line), f.message.clone());
    }
    // ---- Instruction sections (M16): each `##` section of CLAUDE.md and
    // AGENTS.md, type `instruction`, with its own span's revision. Nobody
    // governs them; a relation names one as `md:<file>#<slug>`.
    for r in &authored.instructions {
        b.record(
            r.id.clone(),
            &r.record_type,
            r.source.clone(),
            r.revision.clone(),
            None,
        );
    }
    for f in &authored.instruction_faults {
        b.diagnose(f.rule, &format!("{}:{}", f.file, f.line), f.message.clone());
    }

    // ---- Every relation names a record, or is reported.
    b.records.sort_by(|x, y| x.id.cmp(&y.id));
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut duplicate = Vec::new();
    for r in &b.records {
        if !seen.insert(&r.id) {
            duplicate.push((r.id.clone(), r.source.clone()));
        }
    }
    for (id, source) in duplicate {
        b.diagnose(
            "model.record-duplicate",
            &id,
            format!("{id} is declared more than once (again in {source})"),
        );
    }
    let ids: BTreeSet<String> = b.records.iter().map(|r| r.id.clone()).collect();
    let unknown: Vec<Relation> = b
        .relations
        .iter()
        .filter(|r| !ids.contains(&r.to) || !ids.contains(&r.from))
        .cloned()
        .collect();
    for r in unknown {
        let missing = if ids.contains(&r.to) { &r.from } else { &r.to };
        b.diagnose(
            "model.relation-target-unknown",
            &r.from,
            format!("{} {} {}: {missing} is not a record of this corpus; the relation is kept, not dropped", r.from, r.kind, r.to),
        );
    }

    let pairs: Vec<(&str, &str)> = b
        .records
        .iter()
        .map(|r| (r.id.as_str(), r.revision.as_str()))
        .collect();
    let basis = serde_jcs::to_string(&pairs)
        .map_err(|e| RepoError::Message(format!("could not canonicalize the model basis: {e}")))?;
    Ok(Model {
        schema: SCHEMA.to_owned(),
        basis_digest: digest(basis.as_bytes()),
        records: b.records,
        relations: b.relations.into_iter().collect(),
        states: b.states.into_iter().collect(),
        diagnostics: b.diagnostics.into_iter().collect(),
    })
}

/// `war model`: the model, and a report naming each of its diagnostics.
pub fn run(corpus: &Corpus) -> Result<(Report, Model), RepoError> {
    let model = build(corpus)?;
    let mut report = Report::default();
    for d in &model.diagnostics {
        report.push(Diagnostic::warn(
            d.rule.clone(),
            d.record.clone(),
            d.message.clone(),
        ));
    }
    report.push(Diagnostic::pass("model.built", summary(&model)));
    Ok((report, model))
}

#[must_use]
pub fn summary(m: &Model) -> String {
    format!(
        "{} record(s), {} relation(s), {} state(s), {} diagnostic(s); basis {}",
        m.records.len(),
        m.relations.len(),
        m.states.len(),
        m.diagnostics.len(),
        m.basis_digest
    )
}

// ---- OW-WAR-0148 M18: the store types' record ids ---------------------------

/// The roadmap record's id: `<prefix>-ROADMAP` (`OW-ROADMAP`), beside its
/// phases' `<prefix>-PHASE-<n>`.
#[must_use]
pub fn roadmap_id(rm: &crate::roadmap_cmd::Loaded) -> String {
    format!("{}-ROADMAP", rm.manifest.prefix)
}

/// The specification's id: `<NS>-SAS` (`WAR-SAS`), the namespace its §106
/// requirements carry (`WAR-SAS-RQ-001`), else the program's. Its sections
/// are `<NS>-SAS-<n>` and `<NS>-SAS-<n>.<m>`, as `sas://` cites them.
#[must_use]
pub fn spec_id(repo: &Repository, text: &str) -> String {
    let ns = openwarrant_core::sas_sections::namespace(text)
        .unwrap_or_else(|| repo.config.project.namespace.as_str().to_owned());
    format!("{ns}-SAS")
}

/// Who accepted the revision whose digest is `revision` (`sha256:<hex>`), if
/// one is accepted and the `spec` type selects `acceptance`.
fn spec_acceptor(corpus: &Corpus, revision: &str) -> Option<String> {
    if !crate::types::has(corpus.repo(), Store::Sas, C::Acceptance) {
        return None;
    }
    let hex = revision.strip_prefix("sha256:").unwrap_or(revision);
    corpus
        .sas_revisions()
        .ok()?
        .iter()
        .find(|r| r.sha256 == hex && r.state == openwarrant_core::sas::SasRevisionState::Accepted)
        .and_then(|r| r.acceptance.as_ref().map(|a| a.accepted_by.clone()))
}

/// The specification's id while its bytes are an accepted revision's and
/// the `spec` type selects `structure` and `acceptance`: the fixed state
/// `accepted` (authenticated: it holds on the revision a human accepted).
#[must_use]
pub fn spec_accepted(corpus: &Corpus) -> Option<String> {
    let repo = corpus.repo();
    if !crate::types::has(repo, Store::Sas, C::Structure) {
        return None;
    }
    let (_, bytes) = repo.sas_document().ok()?;
    spec_acceptor(corpus, &digest(&bytes))?;
    Some(spec_id(repo, &String::from_utf8_lossy(&bytes)))
}

/// Every ADR another supersedes, by `supersedes` or `superseded_by`, where
/// the `adr` type selects `structure` and `links`: the fixed state
/// `superseded`, computed from the relation (OW-ADR-0022), never from the
/// status an ADR's front matter states.
#[must_use]
pub fn adrs_superseded(corpus: &Corpus) -> Vec<String> {
    let repo = corpus.repo();
    if !(crate::types::has(repo, Store::Adr, C::Structure)
        && crate::types::has(repo, Store::Adr, C::Links))
    {
        return Vec::new();
    }
    let Ok(adrs) = corpus.adrs() else {
        return Vec::new();
    };
    let of = |r: &str| adr_ref(&adrs.records, r);
    let mut out: BTreeSet<String> = BTreeSet::new();
    for a in &adrs.records {
        if a.superseded_by.as_deref().and_then(of).is_some() {
            out.insert(a.local_alias.clone());
        }
        for s in &a.supersedes {
            if let Some(old) = of(s) {
                out.insert(old);
            }
        }
    }
    out.into_iter().collect()
}

/// The ADR an `adr://` reference names, by its UUID or its alias, as its
/// alias: `None` when it names no ADR of the store.
fn adr_ref(adrs: &[openwarrant_core::AdrRecord], r: &str) -> Option<String> {
    let bare = r.strip_prefix("adr://").unwrap_or(r);
    adrs.iter()
        .find(|a| a.uuid == bare || a.local_alias == bare)
        .map(|a| a.local_alias.clone())
}
