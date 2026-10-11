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
//! - **Documents** (OW-WAR-0148 M18): each development document the index
//!   reads that nothing else reads as records, `doc:<path>`, type
//!   `document` with the state `untyped`, or, adopted in place, of its
//!   document type with the state `adopted`.
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
pub use openwarrant_core::format::FieldValue;
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
    /// FORMAT-1.0 §8: the highest format version this reader implements,
    /// `openwarrant/1.0`. Additive (COMPATIBILITY.md): absent from models
    /// written before it.
    #[serde(default)]
    pub format: String,
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
    /// M18): `roadmap`, `spec`, `section` or `adr`; an indexed document no
    /// type reads, `document`, and one adopted, its document type's name.
    #[serde(rename = "type")]
    pub kind: String,
    /// FORMAT-1.0 §8: the heading's title; absent only for a legacy record
    /// with none (a records/v1 record, a store record).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// FORMAT-1.0 §8: the global id of the work document that holds it (a
    /// Warrant's or ticket's own records); absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    /// FORMAT-1.0 §8: the global id of the last section opened before it in
    /// its document; absent if none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Repository-relative path of the file that holds it.
    pub source: String,
    /// FORMAT-1.0 §8: 1-based line of its heading; 1 for a legacy record
    /// that is a whole file or has no line of its own.
    #[serde(default)]
    pub line: usize,
    /// FORMAT-1.0 §8: `[start, end]` byte offsets in `source`; the whole
    /// file for a legacy record that has no span of its own.
    #[serde(default)]
    pub span: [usize; 2],
    /// `sha256:<hex>` of its own bytes, else of the file that holds it; a
    /// Warrant's is its contract digest as it compiles now.
    pub revision: String,
    /// FORMAT-1.0 §9: `sha256:` over its meaning, domain
    /// `oh.war/record-content/v1`: the same record has the same
    /// `content_digest` in every encoding.
    #[serde(default)]
    pub content_digest: String,
    /// FORMAT-1.0 §8: name → string, or array of strings for a frontmatter
    /// list. `{}` when none.
    #[serde(default)]
    pub fields: BTreeMap<String, FieldValue>,
    /// FORMAT-1.0 §8: the body text, exactly; `""` when none.
    #[serde(default)]
    pub body: String,
    /// FORMAT-1.0 §7.2, §8: dotted (extension) names → value, carried and
    /// inert. `{}` when none.
    #[serde(default)]
    pub ext: BTreeMap<String, FieldValue>,
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
    /// `disposition`, `achieved`, `checklist`, an ADR's `status` as its
    /// front matter states it, or an indexed document's `document`
    /// (`untyped`, or `adopted` in docs/types.toml). A kernel state (OW-ADR-0031) is `computed`,
    /// `authenticated` or `declared`, and its `value` is the state's name.
    pub kind: String,
    pub value: String,
    /// `recorded` (read from a record of an act), `computed`, or `authored`:
    /// a declared state's entry in a journal, an ADR's status in its front
    /// matter, a document's adoption.
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
            title: None,
            document: None,
            section: None,
            source,
            line: 0,
            span: [0, 0],
            revision,
            content_digest: String::new(),
            fields: BTreeMap::new(),
            body: String::new(),
            ext: BTreeMap::new(),
            governed_by,
        });
    }
    /// The record pushed last, to give it content.
    fn last(&mut self) -> &mut Record {
        let n = self.records.len() - 1;
        &mut self.records[n]
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
        // FORMAT-1.0 §12: the manifest's identity as the document's fields,
        // `enterprise_id` as the federation extension `kf.enterprise_id`
        // (omitted when empty). The revision above is unchanged.
        {
            let m = &basis.manifest;
            let r = b.last();
            r.title = Some(m.title.clone());
            r.line = 1;
            r.span = [0, basis.manifest_bytes.len()];
            r.fields.insert("uuid".to_owned(), fv(&m.uuid));
            r.fields.insert("alias".to_owned(), fv(&m.local_alias));
            r.fields.insert("profile".to_owned(), fv(&m.profile));
            if let Some(level) = &m.assurance_level {
                r.fields.insert("assurance_level".to_owned(), fv(level));
            }
            if !m.enterprise_id.is_empty() {
                r.ext
                    .insert("kf.enterprise_id".to_owned(), fv(&m.enterprise_id));
            }
        }
        let atoms_of = |role: &str| -> Vec<(String, String, &openwarrant_compiler::AtomSource)> {
            basis
                .atoms
                .iter()
                .filter(|a| a.role == role)
                .map(|a| (repo.relative(&one.dir.join(&a.source)), digest(&a.bytes), a))
                .collect()
        };
        // FORMAT-1.0 §12: the intent, basis and work order atoms are the
        // sections `S-intent`, `S-basis` and `S-work-order`, each with the
        // revision of its own atom's bytes.
        for (role, sid, name) in LEGACY_SECTIONS {
            for (source, rev, atom) in atoms_of(role) {
                let id = format!("{alias}/{sid}");
                b.record(id.clone(), "section", source, rev, Some(alias.clone()));
                lower_atom_section(b.last(), &atom.bytes, name, &alias);
                b.relate(&id, "part_of", &alias);
            }
        }
        for (source, rev, atom) in atoms_of("assurance") {
            let text = String::from_utf8_lossy(&atom.bytes);
            if let Ok(set) = openwarrant_core::obligation::parse(&text) {
                let spans = obligation_spans(&text);
                for o in set.obligations {
                    let id = format!("{alias}/{}", o.id);
                    b.record(
                        id.clone(),
                        "obligation",
                        source.clone(),
                        rev.clone(),
                        Some(alias.clone()),
                    );
                    lower_obligation(b.last(), &o, spans.get(o.id.as_str()), &alias);
                    b.relate(&id, "part_of", &alias);
                }
            }
        }
        for (source, rev, atom) in atoms_of("milestones") {
            let text = String::from_utf8_lossy(&atom.bytes);
            if let Ok(g) = openwarrant_core::milestones::parse(&text) {
                // FORMAT-1.0 §4.1, §12: each milestone a record; a stage
                // and an obligation `part_of` the milestone that names it.
                for m in &g.milestones {
                    let id = format!("{alias}/{}", m.id);
                    b.record(
                        id.clone(),
                        "milestone",
                        source.clone(),
                        rev.clone(),
                        Some(alias.clone()),
                    );
                    let r = b.last();
                    r.title.clone_from(&m.title);
                    r.document = Some(alias.clone());
                    r.line = yaml_id_line(&text, &m.id);
                    b.relate(&id, "part_of", &alias);
                    for d in &m.depends_on {
                        b.relate(&id, "depends_on", &format!("{alias}/{d}"));
                    }
                    for s in m.stage_refs.iter().chain(&m.obligation_refs) {
                        b.relate(&format!("{alias}/{s}"), "part_of", &id);
                    }
                }
                for s in g.stages {
                    let id = format!("{alias}/{}", s.id);
                    b.record(
                        id.clone(),
                        "stage",
                        source.clone(),
                        rev.clone(),
                        Some(alias.clone()),
                    );
                    lower_stage(b.last(), &s, &text, &alias);
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
                b.last().document = Some(alias.clone());
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
                b.last().document = Some(alias.clone());
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
            // FORMAT-1.0 §12: one record, `adr_uuid` → `uuid`, `status` a
            // field, its `# ` heading the title.
            {
                let r = b.last();
                let (title, body) = split_title(&a.body);
                r.title = title;
                r.body = body;
                r.fields.insert("uuid".to_owned(), fv(&a.uuid));
                r.fields.insert("status".to_owned(), fv(a.status.as_str()));
            }
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
                lower_ticket(b.last(), &t.manifest);
                // FORMAT-1.0 §12: the intent atom `S-intent`; the checklist
                // `S-checklist`, its lines unchanged.
                if let Some(path) = t.manifest.atom_path("intent") {
                    let path = t.dir.join(path);
                    if let Ok(bytes) = crate::vfs::read(&path) {
                        let id = format!("{tid}/S-intent");
                        b.record(
                            id.clone(),
                            "section",
                            repo.relative(&path),
                            digest(&bytes),
                            Some(tid.clone()),
                        );
                        lower_atom_section(b.last(), &bytes, "Intent", &tid);
                        b.relate(&id, "part_of", &tid);
                    }
                }
                let checklist_id = format!("{tid}/S-checklist");
                b.record(
                    checklist_id.clone(),
                    "section",
                    repo.relative(&t.checklist_path),
                    digest(t.checklist_text.as_bytes()),
                    Some(tid.clone()),
                );
                lower_atom_section(b.last(), t.checklist_text.as_bytes(), "Checklist", &tid);
                b.relate(&checklist_id, "part_of", &tid);
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
                    {
                        let start: usize = lines.iter().take(item.line).map(|l| l.len()).sum();
                        let r = b.last();
                        r.title = Some(item.text.clone());
                        r.document = Some(tid.clone());
                        r.section = Some(checklist_id.clone());
                        r.line = item.line + 1;
                        r.span = [start, start + line.len()];
                    }
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
        lower_span(b.last(), r);
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
        lower_span(b.last(), r);
    }
    for f in &authored.instruction_faults {
        b.diagnose(f.rule, &format!("{}:{}", f.file, f.line), f.message.clone());
    }

    // ---- The development-document index (OW-WAR-0148 M18): every
    // indexed document nothing reads as records is one, `doc:<path>` of
    // type `document`, state `untyped`; an adopted one is of its type,
    // state `adopted` (authored in docs/types.toml). The rest are already
    // records in their own form.
    let index = corpus.documents();
    for d in &index.docs {
        let (kind, value, provenance) = match &d.governor {
            crate::doc_index::Governor::Untyped => {
                (crate::doc_index::UNTYPED_TYPE, "untyped", "computed")
            }
            crate::doc_index::Governor::Adopted(t) => (t.as_str(), "adopted", "authored"),
            _ => continue,
        };
        let id = d.id();
        b.record(
            id.clone(),
            kind,
            d.path.clone(),
            file_digest(&repo.root.join(&d.path)).unwrap_or_else(|| digest(b"")),
            None,
        );
        b.states.insert(State {
            record: id,
            kind: "document".to_owned(),
            value: value.to_owned(),
            provenance: provenance.to_owned(),
            facet: None,
            refines: None,
            lapsed: false,
        });
    }
    for f in &index.faults {
        b.diagnose(f.rule, &f.file, f.message.clone());
    }

    // ---- Format 1.0 documents (FORMAT-1.0 §4, §8; OW-ADR-0032): each
    // admitted document's records, relations and item states, lowered into
    // the same model as the legacy encodings above. A refused document, or
    // one UNKNOWN as a whole, adds no record: only its findings.
    let docs = corpus.format_docs();
    for f in &docs.faults {
        b.diagnose(f.rule, &format!("{}:{}", f.file, f.line), f.message.clone());
    }
    for d in &docs.docs {
        for f in &d.doc.findings {
            b.diagnose(
                f.rule,
                &format!("{}:{}", d.source, f.line),
                f.message.clone(),
            );
        }
        if d.admitted()
            && let (Some(alias), Some(uuid)) = (&d.doc.alias, d.doc.fields.get("uuid"))
            && let Some(u) = uuid.as_str()
        {
            uuid_alias.insert(u.to_owned(), alias.clone());
        }
    }
    for d in docs.docs.iter().filter(|d| d.admitted()) {
        lower_document(&mut b, d, &uuid_alias);
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

    finish_content(&mut b, repo);

    let pairs: Vec<(&str, &str)> = b
        .records
        .iter()
        .map(|r| (r.id.as_str(), r.revision.as_str()))
        .collect();
    let basis = serde_jcs::to_string(&pairs)
        .map_err(|e| RepoError::Message(format!("could not canonicalize the model basis: {e}")))?;
    Ok(Model {
        schema: SCHEMA.to_owned(),
        format: openwarrant_core::format::VERSION.to_owned(),
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

// ---- FORMAT-1.0: content (§8, §9) and legacy lowering (§12) ---------------

fn fv(s: impl Into<String>) -> FieldValue {
    FieldValue::One(s.into())
}

/// The atoms a directory Warrant's sections come from (FORMAT-1.0 §12):
/// (role, section id, default title).
const LEGACY_SECTIONS: [(&str, &str, &str); 3] = [
    ("intent", "S-intent", "Intent"),
    ("basis", "S-basis", "Basis"),
    ("work_order", "S-work-order", "Work order"),
];

fn skip_blank_lines(mut text: &str) -> &str {
    while let Some((line, rest)) = text.split_once('\n') {
        if line.trim().is_empty() {
            text = rest;
        } else {
            break;
        }
    }
    text
}

/// A legacy text's title and body: its first line, when that line (after
/// blank lines) is a `# ` heading, and what follows, leading blank lines
/// removed (as a Format 1.0 unit's separating blank line is no body).
fn split_title(text: &str) -> (Option<String>, String) {
    let rest = skip_blank_lines(text);
    match rest.strip_prefix("# ") {
        Some(h) => {
            let (line, after) = h.split_once('\n').unwrap_or((h, ""));
            (
                Some(line.trim().to_owned()),
                skip_blank_lines(after).to_owned(),
            )
        }
        None => (None, rest.to_owned()),
    }
}

/// A legacy atom as a section (FORMAT-1.0 §12): its `# ` heading the title
/// (else `default`), the rest of its body the body, its frontmatter's dotted
/// keys `ext` (DV-12). Its span is the whole atom, whose bytes its revision
/// already is.
fn lower_atom_section(r: &mut Record, bytes: &[u8], default: &str, document: &str) {
    let text = String::from_utf8_lossy(bytes);
    let mut offset = 0;
    if text.starts_with("---")
        && let Ok(fm) = openwarrant_core::frontmatter::parse(&text)
    {
        offset = fm.body_offset;
        for key in fm.keys().filter(|k| openwarrant_core::format::is_dotted(k)) {
            let value = match fm.get(key) {
                Some(openwarrant_core::frontmatter::Value::List(v)) => FieldValue::Many(v.clone()),
                Some(openwarrant_core::frontmatter::Value::Scalar(s)) => fv(s),
                None => continue,
            };
            r.ext.insert(key.to_owned(), value);
        }
    }
    let rest = text.get(offset..).unwrap_or_default();
    let (title, body) = split_title(rest);
    let heading = rest
        .lines()
        .position(|l| l.starts_with("# "))
        .filter(|_| title.is_some());
    r.title = Some(title.unwrap_or_else(|| default.to_owned()));
    r.body = body;
    r.document = Some(document.to_owned());
    r.line = text.get(..offset).unwrap_or_default().lines().count() + heading.map_or(1, |n| n + 1);
    r.span = [0, bytes.len()];
}

/// Each `### OBL-…` block of an assurance atom: id → (1-based line, start,
/// end), the block running to the next `###` or `##` heading.
fn obligation_spans(text: &str) -> BTreeMap<String, (usize, usize, usize)> {
    let mut out = BTreeMap::new();
    let mut open: Option<(String, usize, usize)> = None;
    let mut pos = 0;
    for (n, raw) in text.split_inclusive('\n').enumerate() {
        let t = raw.trim();
        if t.starts_with("### ") || t.starts_with("## ") {
            if let Some((id, line, start)) = open.take() {
                out.entry(id).or_insert((line, start, pos));
            }
            if let Some(rest) = t.strip_prefix("### ") {
                let id = rest
                    .split_once('\u{2014}')
                    .or_else(|| rest.split_once(" - "))
                    .map_or(rest, |(id, _)| id)
                    .trim();
                if id.starts_with("OBL-") {
                    open = Some((id.to_owned(), n + 1, pos));
                }
            }
        }
        pos += raw.len();
    }
    if let Some((id, line, start)) = open {
        out.entry(id).or_insert((line, start, pos));
    }
    out
}

/// A legacy obligation (FORMAT-1.0 §12): its statement the title, its
/// `- **key:**` bullets fields. An authored `disposition` is state, not
/// content (§3.7), and is no field.
fn lower_obligation(
    r: &mut Record,
    o: &openwarrant_core::obligation::Obligation,
    span: Option<&(usize, usize, usize)>,
    document: &str,
) {
    r.title = Some(o.statement.clone());
    r.document = Some(document.to_owned());
    r.fields.insert("scope".to_owned(), fv(&o.scope));
    r.fields.insert("evidence".to_owned(), fv(&o.evidence));
    if let Some(k) = o.scope_kind {
        r.fields.insert("scope_kind".to_owned(), fv(k.as_str()));
    }
    if let Some(n) = &o.note {
        r.fields.insert("note".to_owned(), fv(n));
    }
    if let Some(v) = &o.origin {
        r.fields.insert("origin".to_owned(), fv(word(v)));
    }
    if let Some(v) = &o.admissibility {
        r.fields.insert("admissibility".to_owned(), fv(word(v)));
    }
    if let Some(&(line, start, end)) = span {
        r.line = line;
        r.span = [start, end];
    }
}

/// The 1-based line of a YAML list entry `id: <id>`, else 1.
fn yaml_id_line(text: &str, id: &str) -> usize {
    let quoted = format!("\"{id}\"");
    text.lines()
        .position(|l| {
            let t = l.trim_start().trim_start_matches("- ");
            t.starts_with("id:") && (t.contains(&quoted) || t.trim_end().ends_with(id))
        })
        .map_or(1, |n| n + 1)
}

/// A legacy stage (FORMAT-1.0 §4.1, §12): `executor`, `tier`,
/// `budget_tokens` and `context` fields.
fn lower_stage(
    r: &mut Record,
    s: &openwarrant_core::milestones::Stage,
    text: &str,
    document: &str,
) {
    r.title.clone_from(&s.title);
    r.document = Some(document.to_owned());
    r.line = yaml_id_line(text, &s.id);
    r.fields.insert(
        "executor".to_owned(),
        fv(s.executor_ref
            .clone()
            .unwrap_or_else(|| s.executor_kind.to_string())),
    );
    r.fields
        .insert("tier".to_owned(), fv(s.responsibility_tier.to_string()));
    if let Some(n) = s.budget_tokens {
        r.fields
            .insert("budget_tokens".to_owned(), fv(n.to_string()));
    }
    let context: Vec<&str> = s
        .context_atoms
        .iter()
        .chain(&s.context_sections)
        .chain(&s.context_artifacts)
        .chain(&s.context_external)
        .map(String::as_str)
        .collect();
    if !context.is_empty() {
        r.fields
            .insert("context".to_owned(), fv(context.join(", ")));
    }
}

/// A legacy ticket (FORMAT-1.0 §12): its manifest keys as the document's
/// fields, its title the title.
fn lower_ticket(r: &mut Record, m: &openwarrant_core::ticket::TicketManifest) {
    r.title = Some(m.title.clone());
    let f = &mut r.fields;
    f.insert("alias".to_owned(), fv(&m.id));
    f.insert("uuid".to_owned(), fv(&m.uuid));
    f.insert("profile".to_owned(), fv(&m.profile));
    f.insert("priority".to_owned(), fv(m.priority.to_string()));
    f.insert("created_at".to_owned(), fv(&m.created_at));
    f.insert("created_by".to_owned(), fv(&m.created_by));
    if let Some(k) = &m.kind {
        f.insert("type".to_owned(), fv(k));
    }
    if !m.labels.is_empty() {
        f.insert("labels".to_owned(), FieldValue::Many(m.labels.clone()));
    }
    if let Some(v) = &m.imported_from {
        f.insert("imported_from".to_owned(), fv(v));
    }
    if let Some(v) = m.issue {
        f.insert("issue".to_owned(), fv(v.to_string()));
    }
    if let Some(v) = &m.issue_url {
        f.insert("issue_url".to_owned(), fv(v));
    }
    if let Some(v) = &m.due {
        f.insert("due".to_owned(), fv(v));
    }
}

/// A record atom's or instruction section's own span, title and body.
fn lower_span(r: &mut Record, rec: &crate::records::Record) {
    r.title.clone_from(&rec.title);
    r.line = rec.line;
    r.span = rec.span;
    r.body.clone_from(&rec.body);
}

/// One admitted Format 1.0 document into the builder (FORMAT-1.0 §4.1,
/// §5): a work document's own record (its alias, type `warrant` or
/// `ticket`, the frontmatter as fields and the preamble as body, its span
/// the frontmatter and preamble), each unit and item, ids local to a work
/// document made global as `<alias>/<id>`, each relation's target resolved
/// the same way, and each item's `checklist` state.
fn lower_document(
    b: &mut Builder,
    d: &crate::format_doc::Loaded,
    uuid_alias: &BTreeMap<String, String>,
) {
    use openwarrant_core::format::sha256;
    let doc = &d.doc;
    let work = doc.alias.clone();
    let mut local: BTreeSet<&str> = BTreeSet::new();
    for u in &doc.units {
        local.insert(u.id.as_str());
        for it in &u.items {
            if let Some(i) = &it.id {
                local.insert(i.as_str());
            }
        }
    }
    let global = |id: &str| -> String {
        match &work {
            Some(a) => format!("{a}/{id}"),
            None => id.to_owned(),
        }
    };
    // A bare item id is always this document's (§5: `item-ref = item-id |
    // alias "/" item-id`), so it is made global even when no item has it.
    let resolve = |t: &str| -> String {
        if work.is_some() && (local.contains(t) || openwarrant_core::ticket::is_item_id(t)) {
            global(t)
        } else if let Some(u) = t.strip_prefix("war://") {
            uuid_alias.get(u).cloned().unwrap_or_else(|| t.to_owned())
        } else {
            t.to_owned()
        }
    };
    let span = |s: usize, e: usize| sha256(d.bytes.get(s..e).unwrap_or_default());
    if let Some(alias) = &work {
        b.record(
            alias.clone(),
            doc.record_type(),
            d.source.clone(),
            span(0, doc.head_end),
            None,
        );
        let r = b.last();
        r.title.clone_from(&doc.title);
        r.line = 1;
        r.span = [0, doc.head_end];
        r.fields.clone_from(&doc.fields);
        r.ext.clone_from(&doc.ext);
        r.body.clone_from(&doc.body);
    }
    let mut all_done = true;
    let mut any_item = false;
    for u in &doc.units {
        let id = global(&u.id);
        b.record(
            id.clone(),
            &u.record_type,
            d.source.clone(),
            span(u.start, u.end),
            work.clone(),
        );
        let r = b.last();
        r.title = Some(u.title.clone());
        r.document.clone_from(&work);
        // A section is in no section; a record, in the last one opened before it.
        r.section = if u.is_section() {
            None
        } else {
            u.section.as_deref().map(global)
        };
        r.line = u.line;
        r.span = [u.start, u.end];
        r.fields.clone_from(&u.fields);
        r.ext.clone_from(&u.ext);
        r.body.clone_from(&u.body);
        if let Some(a) = &work {
            b.relate(&id, "part_of", a);
        }
        for rel in &u.relations {
            b.relate_pinned(
                &id,
                &rel.kind,
                &resolve(&rel.target.id),
                rel.target.pin.clone(),
            );
        }
        let item_section = if u.is_section() {
            Some(id.clone())
        } else {
            u.section.as_deref().map(global)
        };
        for it in &u.items {
            let Some(iid) = &it.id else { continue };
            any_item = true;
            all_done &= it.done;
            let item_id = global(iid);
            b.record(
                item_id.clone(),
                "item",
                d.source.clone(),
                span(it.start, it.end),
                work.clone().or_else(|| Some(id.clone())),
            );
            let r = b.last();
            r.title = Some(it.text.clone());
            r.document.clone_from(&work);
            r.section.clone_from(&item_section);
            r.line = it.line;
            r.span = [it.start, it.end];
            if let Some(a) = &work {
                b.relate(&item_id, "part_of", a);
            }
            for after in &it.after {
                b.relate(&item_id, "depends_on", &resolve(after));
            }
            for t in &it.implements {
                b.relate_pinned(&item_id, "implements", &resolve(&t.id), t.pin.clone());
            }
            b.state(
                &item_id,
                "checklist",
                if it.done { "done" } else { "open" }.to_owned(),
                false,
            );
        }
    }
    if let Some(alias) = &work
        && doc.record_type() == "ticket"
    {
        b.state(
            alias,
            "checklist",
            if any_item && all_done { "done" } else { "open" }.to_owned(),
            false,
        );
    }
}

/// FORMAT-1.0 §8, §9, after every record and relation is known: a record
/// with no span of its own spans its whole file (line 1), and each record's
/// `content_digest` over `{id, type, title, fields, relations, body}`. Its
/// relations are its outgoing ones in the model except the containment
/// `part_of <document>` the reader adds for every record of a work
/// document, which no encoding authors.
fn finish_content(b: &mut Builder, repo: &Repository) {
    use openwarrant_core::format::{ContentRelation, content_digest};
    let mut sizes: BTreeMap<String, usize> = BTreeMap::new();
    for r in &mut b.records {
        if r.line == 0 {
            r.line = 1;
            let n = *sizes.entry(r.source.clone()).or_insert_with(|| {
                crate::vfs::metadata(repo.root.join(&r.source))
                    .ok()
                    .filter(|m| !m.is_dir())
                    .and_then(|m| usize::try_from(m.len()).ok())
                    .unwrap_or(0)
            });
            r.span = [0, n];
        }
    }
    let mut out: BTreeMap<&str, Vec<&Relation>> = BTreeMap::new();
    for rel in &b.relations {
        out.entry(rel.from.as_str()).or_default().push(rel);
    }
    let digests: Vec<String> = b
        .records
        .iter()
        .map(|r| {
            let rels: Vec<ContentRelation> = out
                .get(r.id.as_str())
                .into_iter()
                .flatten()
                .filter(|rel| !(rel.kind == "part_of" && Some(&rel.to) == r.document.as_ref()))
                .map(|rel| ContentRelation {
                    kind: rel.kind.clone(),
                    to: rel.to.clone(),
                    to_revision: rel.to_revision.clone(),
                })
                .collect();
            content_digest(
                &r.id,
                &r.kind,
                r.title.as_deref().unwrap_or(""),
                &r.fields,
                &rels,
                &r.body,
            )
        })
        .collect();
    for (r, d) in b.records.iter_mut().zip(digests) {
        r.content_digest = d;
    }
}
