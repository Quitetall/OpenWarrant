// SPDX-License-Identifier: Apache-2.0
//! One loader for both layouts of a Format 1.0 work document
//! (`docs/design/one-format.md`).
//!
//! A Warrant document is `<root>/<id>.md` or `<root>/<id>/warrant.md`. Its
//! state (authorization, evidence, verdicts, resolution, `deliverables.toml`,
//! `generated/`) lives in `<root>/<id>/` in both layouts (FORMAT-1.0 §4.3), so
//! every act that takes a Warrant directory takes that one.
//!
//! [`load`] lowers the document, in memory, to the Compilation Basis the acts
//! already consume. The manifest bytes are the document's exact bytes, so the
//! contract digest binds every byte; the sources are ids, never paths, so the
//! digest is the same in both layouts and `war plan expand` moves nothing a
//! signature covers. Nothing here writes.
//!
//! A legacy Warrant (`manifest.toml` and atoms) is not read here; nor is a
//! document of profile `ticket`, which no formal act takes.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::lower::{AtomSource, CompilationBasis, ScopeSource};
use openwarrant_core::format::{self, Document, FieldValue, Outcome, Unit};
use openwarrant_core::manifest::AtomEntry;
use openwarrant_core::{MANIFEST_SCHEMA, Manifest};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{Loaded, RepoError, Repository};

/// Where a document's bytes are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// `<root>/<id>.md`.
    File,
    /// `<root>/<id>/warrant.md`.
    Directory,
}

/// A Format 1.0 Warrant document found in a work root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub alias: String,
    /// The document's bytes.
    pub file: Utf8PathBuf,
    /// `<root>/<id>/`: where its acts' records go. May not exist yet.
    pub state: Utf8PathBuf,
    pub layout: Layout,
}

/// The work roots, in the order an id is looked up. A level never chooses
/// the root: an item found in either is the same kind of thing.
#[must_use]
pub fn roots(repo: &Repository) -> Vec<Utf8PathBuf> {
    let mut out = vec![repo.warrants_dir()];
    let tickets = crate::format_doc::tickets_dir(repo);
    if !out.contains(&tickets) {
        out.push(tickets);
    }
    out
}

/// Whether the file at `path` is a Format 1.0 document a formal act takes:
/// it declares `format:` and a profile other than `ticket`.
fn is_formal(path: &Utf8Path) -> bool {
    let Ok(bytes) = crate::vfs::read(path) else {
        return false;
    };
    let text = String::from_utf8_lossy(&bytes);
    format::declares_format(&text)
        && crate::format_doc::frontmatter_profile(&text).is_some_and(|p| p != "ticket")
}

/// `alias` in `root`, in either layout. A directory holding a manifest is a
/// legacy Warrant and is not a document. Both layouts at once is refused by
/// `war check` (`format.document-duplicate`) and is not a subject here.
fn in_root(root: &Utf8Path, alias: &str) -> Option<Found> {
    if alias.is_empty() || alias.contains(['/', '\\']) || alias.starts_with('.') {
        return None;
    }
    let state = root.join(alias);
    if crate::vfs::is_file(state.join("manifest.toml"))
        && !crate::format_doc::is_document_dir(&state)
    {
        return None;
    }
    let file = root.join(format!("{alias}.md"));
    let inner = state.join("warrant.md");
    match (crate::vfs::is_file(&file), crate::vfs::is_file(&inner)) {
        (true, false) if is_formal(&file) => Some(Found {
            alias: alias.to_owned(),
            file,
            state,
            layout: Layout::File,
        }),
        (false, true) if is_formal(&inner) => Some(Found {
            alias: alias.to_owned(),
            file: inner,
            state,
            layout: Layout::Directory,
        }),
        _ => None,
    }
}

/// The document `alias` names, in any work root.
#[must_use]
pub fn find(repo: &Repository, alias: &str) -> Option<Found> {
    roots(repo).iter().find_map(|r| in_root(r, alias))
}

/// The document whose state directory is `dir`.
#[must_use]
pub fn of_state(dir: &Utf8Path) -> Option<Found> {
    let alias = dir.file_name()?;
    in_root(dir.parent()?, alias)
}

/// Every document a formal act takes, by state directory, sorted.
#[must_use]
pub fn all(repo: &Repository) -> Vec<Found> {
    let mut out = Vec::new();
    for root in roots(repo) {
        let Ok(entries) = crate::vfs::read_dir(&root) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .map(|n| n.strip_suffix(".md").map_or(n.clone(), str::to_owned))
            .filter(|n| crate::format_doc::is_alias(n))
            .collect();
        names.sort();
        names.dedup();
        out.extend(names.iter().filter_map(|n| in_root(&root, n)));
    }
    out.sort_by(|a, b| a.state.cmp(&b.state));
    out
}

/// Whether an act is recorded for the subject: from then on its bytes are
/// what a signature covers and progress lives in its state, never in it.
#[must_use]
pub fn has_act(found: &Found) -> bool {
    ["authorization.toml", "resolution.toml"]
        .iter()
        .any(|f| crate::vfs::is_file(found.state.join(f)))
}

/// The refusal a write to a document under a recorded act gets, or `None`
/// when `path` is no such document.
#[must_use]
pub fn signed_bytes_refusal(path: &Utf8Path) -> Option<String> {
    let found = if path.file_name() == Some("warrant.md") {
        of_state(path.parent()?)
    } else {
        let alias = path.file_stem()?;
        in_root(path.parent()?, alias)
    }?;
    (found.file == path && has_act(&found)).then(|| {
        format!(
            "subject.signed-bytes: {} has a recorded act, and its bytes are what that \
             signature covers. Progress on it lives in {}/, not in the document; a change \
             to the document is a new revision for a person to authorize",
            found.file, found.state
        )
    })
}

fn field<'a>(doc: &'a Document, key: &str) -> Option<&'a str> {
    doc.fields.get(key).and_then(FieldValue::as_str)
}

fn atom_frontmatter(uuid: &str, role: &str, ordinal: u32) -> String {
    format!(
        "---\nschema: oh.war/atom/v1\nwarrant_uuid: {uuid}\nrole: {role}\njurisdiction: \
         authored\norder: {ordinal}\nclassification: internal\n---\n\n"
    )
}

fn section<'a>(doc: &'a Document, id: &str) -> Option<&'a Unit> {
    doc.units.iter().find(|u| u.is_section() && u.id == id)
}

/// The exact span of every unit in section `id`, the section included.
fn section_bytes(doc: &Document, bytes: &[u8], id: &str) -> Option<Vec<u8>> {
    let start = section(doc, id)?.start;
    let end = doc
        .units
        .iter()
        .filter(|u| u.is_section() && u.start > start)
        .map(|u| u.start)
        .min()
        .unwrap_or(bytes.len());
    bytes.get(start..end).map(<[u8]>::to_vec)
}

/// The obligation records, restated as the assurance atom's
/// `### OBL-NNN — statement` blocks with `- **field:**` bullets.
fn assurance_text(doc: &Document) -> String {
    let mut out = String::from("# Assurance\n");
    for u in doc.units.iter().filter(|u| u.record_type == "obligation") {
        out.push_str(&format!("\n### {} \u{2014} {}\n\n", u.id, u.title));
        for (k, v) in &u.fields {
            let v = match v {
                FieldValue::One(s) => s.clone(),
                FieldValue::Many(m) => m.join(", "),
            };
            out.push_str(&format!("- **{k}:** {}\n", v.replace('\n', " ")));
        }
        if !u.body.trim().is_empty() {
            out.push('\n');
            out.push_str(u.body.trim_end());
            out.push('\n');
        }
    }
    out
}

fn yaml(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_owned())
}

fn yaml_list(items: &[String]) -> String {
    format!(
        "[{}]",
        items.iter().map(|i| yaml(i)).collect::<Vec<_>>().join(", ")
    )
}

/// Milestone records as a `oh.war/milestones/v1` atom: each obligation
/// `part_of` a milestone is one of its `obligation_refs`. Stages are not
/// lowered; [`load`] says so.
fn milestones_text(doc: &Document) -> String {
    let mut out = format!("schema: {}\n", yaml("oh.war/milestones/v1"));
    let milestones: Vec<&Unit> = doc
        .units
        .iter()
        .filter(|u| u.record_type == "milestone")
        .collect();
    if milestones.is_empty() {
        return out;
    }
    out.push_str("\nmilestones:\n");
    for m in milestones {
        let refs: Vec<String> = doc
            .units
            .iter()
            .filter(|u| {
                u.record_type == "obligation"
                    && u.relations.iter().any(|r| {
                        r.kind == "part_of"
                            && (r.target.id == m.id || r.target.id.ends_with(&format!("/{}", m.id)))
                    })
            })
            .map(|u| u.id.clone())
            .collect();
        out.push_str(&format!(
            "  - id: {}\n    title: {}\n    obligation_refs: {}\n",
            yaml(&m.id),
            yaml(&m.title),
            yaml_list(&refs)
        ));
    }
    out
}

/// Lower `found` to a Compilation Basis. A document refused or read
/// `UNKNOWN` as a whole has no basis, as a manifest that does not validate
/// has none; `war check` names why.
pub fn load(repo: &Repository, found: &Found) -> Result<Loaded, RepoError> {
    let source = repo.relative(&found.file);
    let bytes = crate::vfs::read(&found.file).map_err(|e| RepoError::Io {
        context: format!("could not read {}", found.file),
        source: e,
    })?;
    let profiles = crate::format_doc::profiles(repo);
    let doc = format::parse_with(&bytes, &profiles, &source);
    let mut report = Report::default();
    let none = |report| Loaded {
        dir: found.state.clone(),
        basis: None,
        validated: None,
        report,
    };
    if doc.unknown() {
        report.push(Diagnostic::unknown(
            "subject.unreadable",
            source,
            "the document reads UNKNOWN as a whole (FORMAT-1.0 §6); no act can be computed \
             over it",
        ));
        return Ok(none(report));
    }
    if doc.refused() {
        let why = doc
            .findings
            .iter()
            .filter(|f| f.outcome == Outcome::Refusal)
            .map(|f| format!("{} (line {})", f.rule, f.line))
            .collect::<Vec<_>>()
            .join(", ");
        report.push(Diagnostic::error(
            "subject.refused",
            source,
            format!("the document is refused ({why}); `war check` names each finding"),
        ));
        return Ok(none(report));
    }
    let alias = doc.alias.clone().unwrap_or_else(|| found.alias.clone());
    if alias != found.alias {
        report.push(Diagnostic::error(
            "subject.alias-mismatch",
            source,
            format!("the file is named {} but declares alias {alias}", found.alias),
        ));
        return Ok(none(report));
    }
    let uuid = field(&doc, "uuid").unwrap_or_default().to_owned();
    if doc.units.iter().any(|u| u.record_type == "stage") {
        report.push(Diagnostic::unknown(
            "subject.stages-unlowered",
            source.clone(),
            "stage records are not lowered to a dispatch basis: `war dispatch` reads stages \
             from a legacy milestones atom, which a Format 1.0 document does not have",
        ));
    }

    let delivery = doc.profile != "decision";
    let mut plan: Vec<(u32, &str, Vec<u8>)> = vec![
        (
            10,
            "intent",
            section_bytes(&doc, &bytes, "S-intent").unwrap_or_default(),
        ),
        (
            20,
            "basis",
            section_bytes(&doc, &bytes, "S-basis").unwrap_or_default(),
        ),
    ];
    if delivery {
        plan.push((
            40,
            "work_order",
            section_bytes(&doc, &bytes, "S-work-order").unwrap_or_default(),
        ));
        plan.push((45, "milestones", milestones_text(&doc).into_bytes()));
    } else {
        plan.push((
            30,
            "adr",
            section_bytes(&doc, &bytes, "S-decision").unwrap_or_default(),
        ));
    }
    plan.push((60, "assurance", assurance_text(&doc).into_bytes()));

    let mut atoms = Vec::new();
    let mut entries = Vec::new();
    for (ordinal, role, body) in plan {
        let path = format!("{alias}#{role}");
        let mut atom = if role == "milestones" {
            Vec::new()
        } else {
            atom_frontmatter(&uuid, role, ordinal).into_bytes()
        };
        atom.extend_from_slice(&body);
        entries.push(AtomEntry {
            ordinal,
            role: role.to_owned(),
            path: Some(path.clone()),
            r#ref: None,
            required: true,
        });
        atoms.push(AtomSource {
            ordinal,
            role: role.to_owned(),
            jurisdiction: "authored".to_owned(),
            source: path,
            bytes: atom,
            required: true,
        });
    }
    let manifest = Manifest {
        schema: MANIFEST_SCHEMA.to_owned(),
        uuid,
        local_alias: alias.clone(),
        enterprise_id: String::new(),
        title: doc.title.clone().unwrap_or_default(),
        profile: doc.profile.clone(),
        profile_digest: None,
        assurance_level: field(&doc, "assurance_level").map(str::to_owned),
        implements: vec![],
        roadmap: vec![],
        parents: vec![],
        supersedes: vec![],
        currency: None,
        atoms: entries,
    };
    let validated = match manifest
        .validate_in(Some(repo.config.project.namespace.as_str()), &repo.profiles)
    {
        Ok(v) => v,
        Err(e) => {
            report.push(Diagnostic::error("subject.invalid", source, e.to_string()));
            return Ok(none(report));
        }
    };
    let scope_path = found.state.join("scope.toml");
    let scope = crate::vfs::read(&scope_path).ok().map(|b| ScopeSource {
        source: format!("{alias}/scope.toml"),
        bytes: b,
    });
    Ok(Loaded {
        dir: found.state.clone(),
        basis: Some(CompilationBasis {
            manifest,
            manifest_source: alias,
            manifest_bytes: bytes,
            atoms,
            scope,
            sas: repo.sas_pin_for(&found.state)?,
        }),
        validated: Some(validated),
        report,
    })
}
