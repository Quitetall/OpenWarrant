// SPDX-License-Identifier: Apache-2.0
//! One loader for every layout of a Format 1.0 work document
//! (`docs/design/one-format.md`). The layout is decided here and nowhere
//! else: the roots, which file in a directory is the document, where its
//! state goes, and where new work is written.
//!
//! | layout | the document | its state |
//! |---|---|---|
//! | master | `docs/work/<id>/<id>-<slug>.md` (or `<id>.md`) | `docs/work/<id>/` |
//! | file | `<root>/<id>.md` | `<root>/<id>/` |
//! | directory | `<root>/<id>/warrant.md` | `<root>/<id>/` |
//!
//! The id is identity: a hash id for new work (`w-<hex>` a work order,
//! `d-<hex>` a document; the prefix shows the kind, never the tier), an alias for
//! older work, or one of the fixed singleton ids ([`SINGLETONS`]). The slug is
//! cosmetic: an item is found by id whatever its slug, and renaming it moves
//! no digest, because the state directory is named by the id alone and no path
//! is inside a contract. The `docs/warrants/` and `docs/tickets/` roots stay
//! readable where they are; nothing signed moves.
//!
//! [`load`] lowers the document, in memory, to the Compilation Basis the acts
//! already consume. The manifest bytes are the document's exact bytes, so the
//! contract digest binds every byte; the sources are ids, never paths, so the
//! digest is the same in every layout and `war plan expand` moves nothing a
//! signature covers. Nothing here writes except [`expand`].
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

/// Where every master lives, and where new work is written.
pub const WORK_ROOT: &str = "docs/work";

/// The ids that are not hashes: one Spec and one Roadmap, each a master
/// like any other (`docs/work/spec/spec.md`, `docs/work/roadmap/roadmap.md`).
pub const SINGLETONS: [&str; 2] = ["spec", "roadmap"];

/// Where a document's bytes are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// `docs/work/<id>/<id>-<slug>.md`.
    Master,
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

/// `docs/work/` in `repo`.
#[must_use]
pub fn work_root(repo: &Repository) -> Utf8PathBuf {
    repo.root.join(WORK_ROOT)
}

/// The work roots, in the order an id is looked up. A level never chooses
/// the root: an item found in any is the same kind of thing.
#[must_use]
pub fn roots(repo: &Repository) -> Vec<Utf8PathBuf> {
    let mut out = vec![work_root(repo), repo.warrants_dir()];
    let tickets = crate::format_doc::tickets_dir(repo);
    if !out.contains(&tickets) {
        out.push(tickets);
    }
    out
}

/// Whether `name` can be an item's id: an alias, a hash id or a singleton.
#[must_use]
pub fn is_id(name: &str) -> bool {
    crate::format_doc::is_alias(name) || SINGLETONS.contains(&name)
}

/// The value of `key` in a document's frontmatter, read without parsing it.
fn frontmatter_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    for line in lines {
        let line = line.trim_end();
        if line == "---" {
            return None;
        }
        if let Some(v) = line.strip_prefix(key).and_then(|r| r.strip_prefix(':')) {
            return Some(v.trim().trim_matches(['"', '\'']));
        }
    }
    None
}

/// A slug for `title`: lowercase ASCII letters and digits joined by `-`, at
/// most 48 bytes. Cosmetic: nothing reads identity from it.
#[must_use]
pub fn slug(title: &str) -> String {
    let mut out = String::new();
    for word in title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        if out.len() + word.len() + 1 > 48 {
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(&word.to_ascii_lowercase());
    }
    out
}

/// What a directory holds as the document of `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Held {
    None,
    One(Utf8PathBuf),
    /// More than one file claims to be it: no subject, and `war check` says so.
    Many(Vec<Utf8PathBuf>),
}

/// The document directory `dir` holds for `id`: `warrant.md`, `ticket.md`,
/// `<id>.md`, or `<id>-<slug>.md` whose frontmatter `alias` is `id` (so a
/// slug never decides identity).
#[must_use]
pub fn document_in(dir: &Utf8Path, id: &str) -> Held {
    let mut found = Vec::new();
    for name in ["warrant.md", "ticket.md"] {
        if crate::vfs::is_file(dir.join(name)) {
            found.push(dir.join(name));
        }
    }
    let prefix = format!("{id}-");
    let mut named: Vec<Utf8PathBuf> = crate::vfs::read_dir_utf8(dir)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                n == format!("{id}.md") || (n.starts_with(&prefix) && n.ends_with(".md"))
            }) && crate::vfs::is_file(p)
        })
        .filter(|p| {
            crate::vfs::read(p).is_ok_and(|b| {
                let text = String::from_utf8_lossy(&b);
                format::declares_format(&text) && frontmatter_value(&text, "alias") == Some(id)
            })
        })
        .collect();
    named.sort();
    found.extend(named);
    match found.len() {
        0 => Held::None,
        1 => Held::One(found.remove(0)),
        _ => Held::Many(found),
    }
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

/// `alias` in `root`, in any layout. A directory holding a manifest is a
/// legacy Warrant and is not a document. Two documents for one id is refused
/// by `war check` (`format.document-duplicate`) and is not a subject here.
fn in_root(root: &Utf8Path, alias: &str) -> Option<Found> {
    if alias.is_empty() || alias.contains(['/', '\\']) || alias.starts_with('.') {
        return None;
    }
    let state = root.join(alias);
    let inner = document_in(&state, alias);
    if crate::vfs::is_file(state.join("manifest.toml")) && inner == Held::None {
        return None;
    }
    let file = root.join(format!("{alias}.md"));
    let master = root.ends_with(WORK_ROOT);
    let (file, layout) = match (crate::vfs::is_file(&file) && !master, inner) {
        (true, Held::None) => (file, Layout::File),
        (false, Held::One(inner)) => {
            let layout = if master {
                Layout::Master
            } else {
                Layout::Directory
            };
            (inner, layout)
        }
        _ => return None,
    };
    is_formal(&file).then(|| Found {
        alias: alias.to_owned(),
        file,
        state,
        layout,
    })
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

/// Whether `dir` is a document's state directory rather than a legacy
/// Warrant (a `manifest.toml` beside a document holds tool settings only,
/// FORMAT-1.0 §4.2): a reader that models documents itself skips it.
#[must_use]
pub fn is_document_state(dir: &Utf8Path) -> bool {
    of_state(dir).is_some()
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
            .filter(|n| is_id(n))
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
    let parent = path.parent()?;
    let found = of_state(parent).or_else(|| in_root(parent, path.file_stem()?))?;
    (found.file == path && has_act(&found)).then(|| {
        format!(
            "subject.signed-bytes: {} has a recorded act, and its bytes are what that \
             signature covers. Progress on it lives in {}/, not in the document; a change \
             to the document is a new revision for a person to authorize",
            found.file, found.state
        )
    })
}

/// Where a new master for `id` titled `title` is written.
#[must_use]
pub fn master_path(repo: &Repository, id: &str, title: &str) -> Utf8PathBuf {
    let dir = work_root(repo).join(id);
    if SINGLETONS.contains(&id) {
        return dir.join(format!("{id}.md"));
    }
    let slug = slug(title);
    if slug.is_empty() {
        dir.join(format!("{id}.md"))
    } else {
        dir.join(format!("{id}-{slug}.md"))
    }
}

/// `war plan new <title>`: a new master, `docs/work/<id>/<id>-<slug>.md`, its
/// id `w-` and the hash of a fresh UUIDv7 (merge-safe; two branches never take one).
/// The bytes are those `war plan new --format 1.0` writes, with this id.
pub fn create_master(
    repo: &Repository,
    title: &str,
    delivery: bool,
) -> Result<(String, Utf8PathBuf), RepoError> {
    let title = title.trim();
    if title.is_empty() || title.contains('\n') {
        return Err(RepoError::Message(
            "new.title: a work order needs a one-line title".to_owned(),
        ));
    }
    let taken = all(repo).len() + count_entries(&work_root(repo));
    for attempt in 0..16usize {
        let uuid = openwarrant_core::WarUuid::mint().to_string();
        let len = openwarrant_core::ticket::ticket_id_len(taken) + attempt / 4;
        let id = openwarrant_core::ticket::hash_id("w-", &uuid, len);
        if roots(repo).iter().any(|r| {
            crate::vfs::exists(r.join(&id)) || crate::vfs::exists(r.join(format!("{id}.md")))
        }) {
            continue;
        }
        let path = master_path(repo, &id, title);
        let dir = path.parent().unwrap_or(&path).to_owned();
        std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
            context: format!("could not create {dir}"),
            source,
        })?;
        let text = crate::new::format_template(&id, &uuid, title, delivery);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write as _;
                file.write_all(text.as_bytes())
                    .map_err(|source| RepoError::Io {
                        context: format!("could not write {path}"),
                        source,
                    })?;
                return Ok((id, path));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not create {path}"),
                    source,
                });
            }
        }
    }
    Err(RepoError::Message(
        "new.id: could not mint an unused id after 16 attempts".to_owned(),
    ))
}

fn count_entries(dir: &Utf8Path) -> usize {
    crate::vfs::read_dir_utf8(dir).map_or(0, |v| v.len())
}

/// What [`expand`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expanded {
    pub alias: String,
    pub from: Utf8PathBuf,
    pub to: Utf8PathBuf,
}

/// `war plan expand <id>`: a file document, `<root>/<id>.md`, becomes the
/// directory form, `<root>/<id>/<id>-<slug>.md`, beside the state already
/// in `<root>/<id>/`. The bytes move unchanged and the id stays, so the
/// contract digest, and every signature over it, stands. Nothing else moves.
pub fn expand(repo: &Repository, id: &str) -> Result<Expanded, RepoError> {
    let found = find(repo, id).ok_or_else(|| {
        RepoError::Message(format!(
            "expand.unknown: no single-file Warrant document is named {id}"
        ))
    })?;
    if found.layout != Layout::File {
        return Err(RepoError::Message(format!(
            "expand.already: {id} is already a directory, {}",
            repo.relative(&found.file)
        )));
    }
    let bytes = crate::vfs::read(&found.file).map_err(|source| RepoError::Io {
        context: format!("could not read {}", found.file),
        source,
    })?;
    let doc = format::parse(&bytes);
    let title = doc.title.unwrap_or_default();
    let name = match slug(&title).as_str() {
        "" => format!("{id}.md"),
        s => format!("{id}-{s}.md"),
    };
    let to = found.state.join(name);
    std::fs::create_dir_all(&found.state).map_err(|source| RepoError::Io {
        context: format!("could not create {}", found.state),
        source,
    })?;
    if crate::vfs::exists(&to) {
        return Err(RepoError::Message(format!(
            "expand.occupied: {} already exists; nothing moved",
            repo.relative(&to)
        )));
    }
    std::fs::rename(&found.file, &to).map_err(|source| RepoError::Io {
        context: format!("could not move {} to {to}", found.file),
        source,
    })?;
    Ok(Expanded {
        alias: id.to_owned(),
        from: found.file,
        to,
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
            format!(
                "the file is named {} but declares alias {alias}",
                found.alias
            ),
        ));
        return Ok(none(report));
    }
    let uuid = field(&doc, "uuid").unwrap_or_default().to_owned();
    if doc.units.iter().any(|u| u.record_type == "stage") {
        report.push(Diagnostic::unknown(
            "subject.stages-unlowered",
            source.clone(),
            "stage records are not lowered to a dispatch basis: `war admin dispatch` reads stages \
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
    let validated =
        match manifest.validate_in(Some(repo.config.project.namespace.as_str()), &repo.profiles) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_is_lowercase_words_and_never_identity() {
        assert_eq!(slug("A note is Delivered!"), "a-note-is-delivered");
        assert_eq!(slug("  "), "");
        assert!(slug(&"word ".repeat(40)).len() <= 48);
    }

    #[test]
    fn a_directory_holds_one_document_by_its_alias_not_its_slug() {
        let tmp = tempfile::tempdir().expect("tmp");
        let dir = Utf8Path::from_path(tmp.path())
            .expect("utf8")
            .join("w-3c16");
        std::fs::create_dir_all(&dir).expect("dir");
        let doc = |alias: &str| {
            format!("---\nformat: openwarrant/1.0\nprofile: delivery\nalias: {alias}\n---\n# T\n")
        };
        assert_eq!(document_in(&dir, "w-3c16"), Held::None);
        std::fs::write(dir.join("w-3c16-some-slug.md"), doc("w-3c16")).expect("write");
        // A file whose name has the prefix but whose alias is another id is not it.
        std::fs::write(dir.join("w-3c16-other.md"), doc("w-9999")).expect("write");
        assert_eq!(
            document_in(&dir, "w-3c16"),
            Held::One(dir.join("w-3c16-some-slug.md"))
        );
        std::fs::write(dir.join("w-3c16-copy.md"), doc("w-3c16")).expect("write");
        assert!(matches!(document_in(&dir, "w-3c16"), Held::Many(v) if v.len() == 2));
    }
}
