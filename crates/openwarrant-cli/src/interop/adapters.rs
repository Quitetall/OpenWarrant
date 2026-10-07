// SPDX-License-Identifier: Apache-2.0
//! Read-in-place adapters (OW-WAR-0148 M10): an OpenSpec or Spec Kit
//! folder seen as Warrants and records, where it is, never converted and
//! never written.
//!
//! ```toml
//! [[adapters]]
//! kind = "openspec"      # or "speckit"
//! path = "openspec"      # relative to the repository root, or absolute
//! ```
//!
//! With an entry, `war view warrants`, `war show <id>`, `war status`,
//! `war plan model`, `war plan impact` and `war check` read the folder on every call
//! with the same readers `war admin import` uses ([`super::openspec`],
//! [`super::speckit`]): a repository that keeps working in those tools gets
//! the list, the status and impact analysis without switching. Nothing here
//! opens a file for writing.
//!
//! Reported by rule: an entry that does not parse (`adapter.config`), a
//! kind this build does not read (`adapter.kind-unknown`), a path that is
//! not there (`adapter.path-missing`), and everything the readers refuse
//! (`openspec.tasks-malformed`, `speckit.tasks-malformed`, ...). `war check`
//! makes each an error; the list, the model and the status carry each as a
//! warning beside what could be read.

use camino::Utf8PathBuf;
use serde::Deserialize;

use super::{Fault, Foreign, Tree};
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::Repository;
use crate::ticket::Outcome;
use crate::warrants::Row;

/// The kinds a folder can be read in place as.
pub const KINDS: &[&str] = crate::warrants::READ_IN_PLACE_KINDS;

/// One `[[adapters]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub kind: String,
    pub path: String,
}

/// Every configured folder, read.
#[derive(Debug, Clone, Default)]
pub struct Adapted {
    pub entries: Vec<Entry>,
    pub trees: Vec<Tree>,
    /// Faults about the configuration itself (the readers' are in each
    /// tree).
    pub faults: Vec<Fault>,
}

/// Whether `id` is shaped like a read-in-place Warrant's or record's id.
#[must_use]
pub fn is_adapter_ref(id: &str) -> bool {
    crate::warrants::kind_of(id) == crate::warrants::IdKind::ReadInPlace
}

/// The `[[adapters]]` entries of `openwarrant.toml`, read on their own: the
/// core configuration parser ignores a table it does not know.
pub fn entries(repo: &Repository) -> Result<Vec<Entry>, Fault> {
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        adapters: Vec<Entry>,
    }
    let path = repo.root.join(crate::init::CONFIG_FILE);
    let text = crate::vfs::read_to_string(&path).map_err(|e| {
        Fault::new(
            "adapter.config",
            crate::init::CONFIG_FILE,
            0,
            format!("could not read it: {e}"),
        )
    })?;
    toml::from_str::<File>(&text)
        .map(|f| f.adapters)
        .map_err(|e| {
            Fault::new(
                "adapter.config",
                crate::init::CONFIG_FILE,
                0,
                format!("[[adapters]]: {e}"),
            )
        })
}

/// Read every configured folder. Never fails: what cannot be read is a
/// fault inside, by rule.
#[must_use]
pub fn load(repo: &Repository) -> Adapted {
    let mut out = Adapted::default();
    let entries = match entries(repo) {
        Ok(e) => e,
        Err(f) => {
            out.faults.push(f);
            return out;
        }
    };
    for e in &entries {
        let path = super::resolve(repo, camino::Utf8Path::new(&e.path));
        match e.kind.as_str() {
            super::openspec::KIND | super::speckit::KIND if !crate::vfs::exists(&path) => {
                out.faults.push(Fault::new(
                    "adapter.path-missing",
                    crate::init::CONFIG_FILE,
                    0,
                    format!(
                        "[[adapters]] kind = {:?} path = {:?}: nothing is at {}",
                        e.kind,
                        e.path,
                        super::shown(repo, &path)
                    ),
                ));
            }
            super::openspec::KIND => match super::openspec::locate(&path) {
                Some(root) => out.trees.push(super::openspec::read(repo, &root)),
                None => out.faults.push(Fault::new(
                    "openspec.missing",
                    &super::shown(repo, &path),
                    0,
                    "neither changes/ nor specs/, nor an openspec/ folder holding them, is here",
                )),
            },
            super::speckit::KIND => match super::speckit::locate(&path) {
                Some(root) => out.trees.push(super::speckit::read(repo, &root)),
                None => out.faults.push(Fault::new(
                    "speckit.missing",
                    &super::shown(repo, &path),
                    0,
                    "no specs/ folder, nor a feature folder (spec.md, plan.md or tasks.md), is \
                     here",
                )),
            },
            other => out.faults.push(Fault::new(
                "adapter.kind-unknown",
                crate::init::CONFIG_FILE,
                0,
                format!(
                    "[[adapters]] kind = {other:?}; this build reads {}",
                    KINDS.join(" and ")
                ),
            )),
        }
    }
    out.entries = entries;
    out
}

impl Adapted {
    /// Whether no adapter is configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.faults.is_empty()
    }

    /// Every fault, the configuration's first, then each folder's.
    pub fn faults(&self) -> impl Iterator<Item = &Fault> {
        self.faults
            .iter()
            .chain(self.trees.iter().flat_map(|t| t.faults.iter()))
    }

    /// Every fault as a warning, for the readers that list what they could.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.faults().map(Fault::warn).collect()
    }

    /// Every Warrant read in place, as `war view warrants` lists it.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        self.trees
            .iter()
            .flat_map(|t| {
                t.warrants.iter().map(move |w| {
                    let (done, total) = w.progress();
                    Row {
                        id: w.id.clone(),
                        title: w.title.clone(),
                        profile: t.kind.to_owned(),
                        encoding: "read_in_place",
                        state: w.state().to_owned(),
                        provenance: "computed",
                        done: Some(done),
                        total: Some(total),
                        dir: w
                            .source
                            .rsplit_once('/')
                            .map_or_else(|| w.source.clone(), |(d, _)| d.to_owned()),
                    }
                })
            })
            .collect()
    }

    /// The Warrant `id` names (itself, or one of its tasks), with its tree.
    #[must_use]
    pub fn warrant(&self, id: &str) -> Option<(&Tree, &Foreign)> {
        let head = id.split_once('/').map_or(id, |(h, _)| h);
        self.trees
            .iter()
            .find_map(|t| t.warrants.iter().find(|w| w.id == head).map(|w| (t, w)))
    }

    /// Where `id` is declared, if a folder declares it: (the kind, the file).
    #[must_use]
    pub fn source_of(&self, id: &str) -> Option<(&'static str, String)> {
        for t in &self.trees {
            if let Some(w) = t.warrants.iter().find(|w| w.id == id) {
                return Some((t.kind, w.source.clone()));
            }
            if let Some((w, _)) = t.tasks().find(|(_, task)| task.id == id) {
                return Some((t.kind, w.tasks_file.clone().unwrap_or_default()));
            }
            if let Some(r) = t.records.iter().find(|r| r.id == id) {
                return Some((t.kind, r.source.clone()));
            }
        }
        None
    }
}

/// `war show <id>` (and `war status <id>`) for a Warrant read in place, or
/// one of its records: the Warrant as a person reads it, from the folder
/// as it is now. Writes nothing.
#[must_use]
pub fn show(repo: &Repository, id: &str) -> Outcome {
    let adapted = load(repo);
    let Some((tree, w)) = adapted.warrant(id) else {
        if let Some(r) = adapted
            .trees
            .iter()
            .flat_map(|t| t.records.iter())
            .find(|r| r.id == id)
        {
            let changed_by: Vec<String> = adapted
                .trees
                .iter()
                .flat_map(|t| t.relations.iter())
                .filter(|(_, _, to)| to == id)
                .map(|(from, kind, _)| format!("{from} ({kind})"))
                .collect();
            let mut md = format!(
                "# {} · {}\n\n{}\n\nDeclared in `{}`, line {} · {}\n",
                r.id, r.kind, r.title, r.source, r.line, r.revision
            );
            if !changed_by.is_empty() {
                md.push_str(&format!("\nNamed by: {}\n", changed_by.join(", ")));
            }
            md.push_str(&format!(
                "\n`war plan impact {id}` lists what a change to it reaches.\n"
            ));
            return Outcome::ok(
                md.trim_end().to_owned(),
                serde_json::json!({
                    "schema": "oh.war/read-in-place-show/v1",
                    "record": {"id": r.id, "type": r.kind, "title": r.title, "source": r.source,
                               "line": r.line, "revision": r.revision},
                    "named_by": changed_by,
                    "markdown": md,
                }),
            );
        }
        let mut out = Outcome::refused(
            "adapter.unknown",
            String::new(),
            if adapted.is_empty() {
                format!(
                    "{id:?} names a Warrant read in place, and no `[[adapters]]` entry in \
                     openwarrant.toml reads one; docs/TYPES.md, \"Read in place\", says how"
                )
            } else {
                format!(
                    "no Warrant read in place is {id:?}; `war view warrants --type openspec` (or \
                     speckit) lists them"
                )
            },
        );
        for d in adapted.diagnostics() {
            out.report.push(d);
        }
        return out;
    };
    let (done, total) = w.progress();
    let what = match tree.kind {
        "openspec" => "OpenSpec change",
        _ => "Spec Kit feature",
    };
    let mut md = format!("# {} — {}\n\n", w.id, w.title);
    md.push_str(&format!(
        "{} · {done}/{total} tasks done · read in place from `{}` ({what}; nothing here is \
         written)\n",
        w.state().replace('_', " "),
        w.source
    ));
    if let Some(c) = &w.created {
        md.push_str(&format!("created {}\n", &c[..c.len().min(10)]));
    }
    if !w.description.is_empty() {
        md.push('\n');
        md.push_str(&w.description);
        md.push('\n');
    }
    md.push_str("\n## Tasks\n\n");
    if w.tasks.is_empty() {
        md.push_str("(no tasks)\n");
    }
    for t in &w.tasks {
        md.push_str(&format!(
            "- [{}] {} (`{}`)\n",
            if t.done { 'x' } else { ' ' },
            t.text,
            t.id
        ));
    }
    if !w.changes.is_empty() {
        md.push_str("\n## Requirements it changes\n\n");
        for c in &w.changes {
            md.push_str(&format!("- {c}\n"));
        }
    }
    let own: Vec<&super::ForeignRecord> = tree
        .records
        .iter()
        .filter(|r| r.id.starts_with(&format!("{}/", w.id)))
        .collect();
    if !own.is_empty() {
        md.push_str("\n## Records\n\n");
        for r in &own {
            md.push_str(&format!("- `{}` · {}: {}\n", r.id, r.kind, r.title));
        }
    }
    let faults: Vec<&Fault> = tree
        .faults
        .iter()
        .filter(|f| {
            let dir = super::shown(repo, &w.dir);
            f.file.starts_with(&format!("{dir}/"))
        })
        .collect();
    let mut out = Outcome::ok(
        md.trim_end().to_owned(),
        serde_json::json!({
            "schema": "oh.war/read-in-place-show/v1",
            "warrant": adapted.rows().into_iter().find(|r| r.id == w.id),
            "description": w.description,
            "tasks": w.tasks.iter().map(|t| serde_json::json!({
                "id": t.id, "text": t.text, "done": t.done, "line": t.line,
                "after": t.after,
            })).collect::<Vec<_>>(),
            "changes": w.changes,
            "records": own.iter().map(|r| serde_json::json!({
                "id": r.id, "type": r.kind, "title": r.title, "source": r.source,
            })).collect::<Vec<_>>(),
            "markdown": md,
        }),
    );
    for f in faults {
        out.report.push(f.warn());
    }
    out
}

/// `war check`: every fault of every configured folder, as an error; with
/// `only`, the faults of that Warrant's folder. Silent without adapters.
pub fn check(repo: &Repository, only: Option<&str>, report: &mut Report) {
    let adapted = load(repo);
    if adapted.is_empty() {
        return;
    }
    let scope: Option<String> = match only {
        None => None,
        Some(id) => match adapted.warrant(id) {
            Some((_, w)) => Some(format!("{}/", super::shown(repo, &w.dir))),
            None => {
                report.push(Diagnostic::error(
                    "adapter.unknown",
                    String::new(),
                    format!("no Warrant read in place is {id:?}"),
                ));
                return;
            }
        },
    };
    let mut n = 0;
    for f in adapted.faults() {
        if scope.as_deref().is_some_and(|s| !f.file.starts_with(s)) {
            continue;
        }
        n += 1;
        report.push(f.error());
    }
    if n == 0 {
        let (warrants, tasks, records) = adapted.trees.iter().fold((0, 0, 0), |(w, t, r), tree| {
            (
                w + tree.warrants.len(),
                t + tree.tasks().count(),
                r + tree.records.len(),
            )
        });
        report.push(Diagnostic::pass(
            "adapter.read",
            format!(
                "read in place, nothing written: {warrants} Warrant(s), {tasks} task(s), \
                 {records} record(s) from {} folder(s)",
                adapted.trees.len()
            ),
        ));
    }
}

/// The section `war status` prints after the corpus status when an adapter
/// is configured, and its `--json` counterpart.
#[must_use]
pub fn status(adapted: &Adapted) -> (String, serde_json::Value) {
    let rows = adapted.rows();
    let mut md = String::from("\n## Read in place\n\n");
    if rows.is_empty() {
        md.push_str("No Warrant could be read from the configured folders.\n");
    }
    for r in &rows {
        md.push_str(&format!(
            "- {} — {} · {} · {}/{} tasks done · `{}`\n",
            r.id,
            r.title,
            r.state.replace('_', " "),
            r.done.unwrap_or(0),
            r.total.unwrap_or(0),
            r.dir
        ));
    }
    let faults: Vec<&Fault> = adapted.faults().collect();
    if !faults.is_empty() {
        md.push_str("\nNot read:\n\n");
        for f in &faults {
            md.push_str(&format!("- {} ({}): {}\n", f.place(), f.rule, f.message));
        }
    }
    (
        md,
        serde_json::json!({
            "warrants": rows,
            "records": adapted.trees.iter().map(|t| t.records.len()).sum::<usize>(),
            "faults": faults.iter().map(|f| serde_json::json!({
                "rule": f.rule, "file": f.place(), "message": f.message,
            })).collect::<Vec<_>>(),
        }),
    )
}

/// The folders `[[adapters]]` names, for a watcher's fingerprint.
#[must_use]
pub fn watched(repo: &Repository) -> Vec<Utf8PathBuf> {
    entries(repo)
        .unwrap_or_default()
        .iter()
        .map(|e| super::resolve(repo, camino::Utf8Path::new(&e.path)))
        .collect()
}
