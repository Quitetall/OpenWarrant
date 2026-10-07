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
//! - **Authored records** (OW-WAR-0148 M3): every record of a record atom
//!   under `docs/records/`, with its type (a profile noun) and the revision
//!   of its own byte span; and the relations documents author — record
//!   atoms' relation lines, obligations' `evaluates`, ticket items'
//!   `implements` — each with the revision it pins, if any.
//! - **Relations:** `part_of`, `parent`, `supersedes`, `roadmap`,
//!   `implements`, `depends_on`, `promoted_to`. A relation whose target is
//!   not a record here is kept AND reported (`model.relation-target-unknown`):
//!   never silently dropped.
//! - **States:** what the builders already derive — a Warrant's §24 state,
//!   its rung and currency, an obligation's disposition, a phase's
//!   achievement, a checklist's done/open. Each says whether it is
//!   `recorded` or `computed`.
//! - **basis_digest:** sha256 over the sorted `(id, revision)` pairs: the
//!   same tree gives the same digest and the same bytes.
//!
//! Nothing here is evaluated twice and nothing is written.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::RepoError;

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
    /// `requirement`, `ticket` or `item`.
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

/// A state a builder derives for a record.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub record: String,
    /// `phase`, `rung`, `currency`, `disposition`, `achieved` or `checklist`.
    pub kind: String,
    pub value: String,
    /// `recorded` (read from a record of an act) or `computed`.
    pub provenance: String,
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
    match corpus.roadmap() {
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
        }
        Ok(None) => {
            // No record: the phases are SAS §98's, as status reads them.
            let (source, rev) = repo
                .sas_document()
                .map(|(p, bytes)| (repo.relative(&p), digest(&bytes)))
                .unwrap_or_else(|_| ("SAS §98".to_owned(), digest(b"builtin:sas-98")));
            for id in objectives.keys() {
                b.record(id.clone(), "phase", source.clone(), rev.clone(), None);
            }
        }
        Err(e) => b.diagnose("model.roadmap-unreadable", "roadmap", e.to_string()),
    }
    for (id, o) in &objectives {
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

    // ---- SAS §106 requirements: the targets of `implements`.
    if let Ok((path, bytes)) = repo.sas_document() {
        let (source, rev) = (repo.relative(&path), digest(&bytes));
        for r in status.requirements.iter().filter(|r| r.title.is_some()) {
            b.record(
                r.requirement.canonical(),
                "requirement",
                source.clone(),
                rev.clone(),
                None,
            );
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
                if let Some(w) = &t.manifest.promoted_to {
                    b.relate(&tid, "promoted_to", w);
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
                    b.relate(&id, "part_of", &tid);
                    b.state(
                        &id,
                        "checklist",
                        if item.done { "done" } else { "open" }.to_owned(),
                        false,
                    );
                    for blocker in &item.after {
                        let to = match blocker {
                            openwarrant_core::ticket::Blocker::Item { item } => {
                                format!("{tid}/{item}")
                            }
                            openwarrant_core::ticket::Blocker::Ticket { ticket } => ticket.clone(),
                            openwarrant_core::ticket::Blocker::ItemOf { ticket, item } => {
                                format!("{ticket}/{item}")
                            }
                        };
                        b.relate(&id, "depends_on", &to);
                    }
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
