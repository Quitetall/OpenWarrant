// SPDX-License-Identifier: Apache-2.0
//! A Spec Kit `specs/` folder, read (OW-WAR-0148 M10).
//!
//! The format is Spec Kit's own (github.com/github/spec-kit, read at
//! 62b6fcf4, 2026-10-06: `templates/spec-template.md`,
//! `templates/plan-template.md`, `templates/tasks-template.md`):
//!
//! ```text
//! specs/<feature>/spec.md    # Feature Specification: <name>
//!                            **Created**: <date>; **Input**: User description: "..."
//!                            ### User Story 1 - <title> (Priority: P1)
//!                            - **FR-001**: System MUST ...
//!                            - **SC-001**: <measurable outcome>
//! specs/<feature>/plan.md    # Implementation Plan: <name>; ## Summary
//! specs/<feature>/tasks.md   - [ ] T001 [P] [US1] <description> (depends on T000)
//! .specify/                  templates, scripts and the constitution: not read
//! ```
//!
//! Read as: each feature a Warrant (`speckit:<feature>`); each task an item
//! (`speckit:<feature>/T001`, part of the feature, waiting on the tasks its
//! text says it depends on, implementing the user story its `[US1]` names
//! and any requirement it names by id); each user story, functional
//! requirement and success criterion a record (`.../US1` a `story`,
//! `.../FR-001` a `requirement`, `.../SC-001` an `outcome`), part of the
//! feature.
//!
//! Refused by rule, each naming its file and line: a task line that is not
//! one, or does not open with its id (`speckit.tasks-malformed`, also an id
//! used twice), a record id declared twice (`speckit.record-duplicate`),
//! and a folder with no feature in it (`speckit.missing`).

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};

use super::{Fault, Foreign, ForeignRecord, Task, Tree, heading, shown, unfenced};
use crate::repo::Repository;

pub const KIND: &str = "speckit";
const TASKS: &str = "speckit.tasks-malformed";
const FILES: [&str; 3] = ["spec.md", "plan.md", "tasks.md"];

fn is_feature(dir: &Utf8Path) -> bool {
    FILES.iter().any(|f| crate::vfs::is_file(dir.join(f)))
}

/// The folder of features under `path`: `path/specs` when that is a folder,
/// else `path` itself when a feature is in it.
#[must_use]
pub fn locate(path: &Utf8Path) -> Option<Utf8PathBuf> {
    let specs = path.join("specs");
    if crate::vfs::is_dir(&specs) {
        Some(specs)
    } else if super::subdirs(path).iter().any(|d| is_feature(d)) {
        Some(path.to_owned())
    } else {
        None
    }
}

/// `T001` at the start of a task's text.
fn task_id(text: &str) -> Option<&str> {
    let end = text
        .char_indices()
        .find(|(_, c)| !c.is_ascii_alphanumeric())
        .map_or(text.len(), |(i, _)| i);
    let id = &text[..end];
    (id.len() > 1 && id.starts_with('T') && id[1..].bytes().all(|b| b.is_ascii_digit()))
        .then_some(id)
}

/// Every `T<digits>` a task's text says it depends on: `depends on T012,
/// T013`, `depends on T012 and T013`.
fn depends_on(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find("depends on") {
        let start = from + at + "depends on".len();
        for word in text[start..].split([',', ' ', ')', ';']) {
            let w = word.trim().trim_end_matches('.');
            if w.is_empty() || w.eq_ignore_ascii_case("and") {
                continue;
            }
            if task_id(w) == Some(w) {
                out.push(w.to_owned());
            } else {
                break;
            }
        }
        from = start;
    }
    out
}

/// Every bracketed marker of a task: `[P]`, `[US1]`.
fn markers(text: &str) -> Vec<&str> {
    text.split('[')
        .skip(1)
        .filter_map(|s| s.split_once(']').map(|(m, _)| m.trim()))
        .collect()
}

/// `**FR-001**: text` or `**FR-001:** text` in a bullet: (id, text).
fn bold_id(line: &str, prefix: &str) -> Option<(String, String)> {
    let t = line
        .trim_start()
        .strip_prefix(['-', '*', '+'])?
        .trim_start();
    let rest = t.strip_prefix("**")?;
    let (inner, after) = rest.split_once("**")?;
    let id = inner.trim().trim_end_matches(':').trim();
    let n = id.strip_prefix(prefix)?.strip_prefix('-')?;
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then(|| {
        (
            id.to_owned(),
            after.trim_start_matches(':').trim().to_owned(),
        )
    })
}

/// `### User Story 2 - Title (Priority: P2)` → (`US2`, `Title`).
fn user_story(line: &str) -> Option<(String, String)> {
    let (level, h) = heading(line)?;
    if level != 3 {
        return None;
    }
    let rest = h.strip_prefix("User Story")?.trim_start();
    let end = rest
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(rest.len(), |(i, _)| i);
    if end == 0 {
        return None;
    }
    let title = rest[end..]
        .trim_start()
        .trim_start_matches(['-', '—', ':'])
        .trim();
    let title = title
        .rsplit_once("(Priority")
        .map_or(title, |(t, _)| t)
        .trim();
    Some((format!("US{}", &rest[..end]), title.to_owned()))
}

/// The text after `# Feature Specification:` (or the plan's or the tasks'
/// own title prefix).
fn titled(text: &str, prefix: &str) -> Option<String> {
    unfenced(text).into_iter().find_map(|(_, l)| {
        let (level, h) = heading(l)?;
        (level == 1).then(|| {
            h.strip_prefix(prefix)
                .unwrap_or(h)
                .trim()
                .trim_matches(['[', ']'])
                .to_owned()
        })
    })
}

/// `**Name**: value` on a line of its own.
fn field(text: &str, name: &str) -> Option<String> {
    let key = format!("**{name}**:");
    text.lines()
        .find_map(|l| l.trim().strip_prefix(&key).map(|v| v.trim().to_owned()))
        .filter(|v| !v.is_empty())
}

/// The first paragraph under `## Summary` in a plan.
fn summary(plan: &str) -> String {
    let mut inside = false;
    let mut out: Vec<&str> = Vec::new();
    for (_, l) in unfenced(plan) {
        if let Some((level, h)) = heading(l) {
            if inside {
                break;
            }
            inside = level == 2 && h.eq_ignore_ascii_case("summary");
            continue;
        }
        if inside {
            if l.trim().is_empty() && !out.is_empty() {
                break;
            }
            if !l.trim().is_empty() && !l.trim_start().starts_with("<!--") {
                out.push(l.trim());
            }
        }
    }
    out.join(" ")
}

/// Read the Spec Kit folder of features at `root` (already located).
#[must_use]
pub fn read(repo: &Repository, root: &Utf8Path) -> Tree {
    let mut tree = Tree {
        kind: KIND,
        root: root.to_owned(),
        ..Tree::default()
    };
    let features: Vec<Utf8PathBuf> = super::subdirs(root)
        .into_iter()
        .filter(|d| is_feature(d))
        .collect();
    if features.is_empty() {
        tree.faults.push(Fault::new(
            "speckit.missing",
            &shown(repo, root),
            0,
            "no feature folder (one holding spec.md, plan.md or tasks.md) is here",
        ));
        return tree;
    }
    for dir in features {
        let name = dir.file_name().unwrap_or_default().to_owned();
        let id = format!("{KIND}:{name}");
        let mut read = |file: &str| -> Option<String> {
            let path = dir.join(file);
            match super::read_optional(&path) {
                Ok(t) => t,
                Err(e) => {
                    tree.faults
                        .push(Fault::new("speckit.unreadable", &shown(repo, &path), 0, e));
                    None
                }
            }
        };
        let spec = read("spec.md");
        let plan = read("plan.md");
        let tasks = read("tasks.md");
        let title = spec
            .as_deref()
            .and_then(|s| titled(s, "Feature Specification:"))
            .or_else(|| {
                plan.as_deref()
                    .and_then(|p| titled(p, "Implementation Plan:"))
            })
            .or_else(|| tasks.as_deref().and_then(|t| titled(t, "Tasks:")))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| name.replace('-', " "));
        let description = spec
            .as_deref()
            .and_then(|s| field(s, "Input"))
            .map(|i| {
                i.strip_prefix("User description:")
                    .unwrap_or(&i)
                    .trim()
                    .trim_matches('"')
                    .to_owned()
            })
            .or_else(|| plan.as_deref().map(summary))
            .unwrap_or_default();
        let main = if spec.is_some() {
            "spec.md"
        } else if plan.is_some() {
            "plan.md"
        } else {
            "tasks.md"
        };
        let main_text = match main {
            "spec.md" => spec.as_deref(),
            "plan.md" => plan.as_deref(),
            _ => tasks.as_deref(),
        };
        let mut w = Foreign {
            id: id.clone(),
            name: name.clone(),
            title,
            description,
            source: shown(repo, &dir.join(main)),
            revision: super::digest(main_text.unwrap_or_default().as_bytes()),
            created: spec
                .as_deref()
                .and_then(|s| field(s, "Created"))
                .and_then(|d| super::timestamp(&d)),
            dir: dir.clone(),
            ..Foreign::default()
        };
        // Records of the spec.
        let mut declared: BTreeMap<String, usize> = BTreeMap::new();
        if let Some(text) = spec.as_deref() {
            let file = shown(repo, &dir.join("spec.md"));
            let lines: Vec<(usize, &str)> = unfenced(text);
            for (i, (n, l)) in lines.iter().enumerate() {
                let found = if let Some((us, title)) = user_story(l) {
                    // A story runs to the next heading of level 3 or less.
                    let body: String = lines[i..]
                        .iter()
                        .enumerate()
                        .take_while(|(k, (_, x))| {
                            *k == 0 || !heading(x).is_some_and(|(lv, _)| lv <= 3)
                        })
                        .map(|(_, (_, x))| format!("{x}\n"))
                        .collect();
                    Some((us, "story", title, body))
                } else if let Some((fr, text)) = bold_id(l, "FR") {
                    Some((fr, "requirement", text, format!("{l}\n")))
                } else if let Some((sc, text)) = bold_id(l, "SC") {
                    Some((sc, "outcome", text, format!("{l}\n")))
                } else {
                    None
                };
                let Some((key, kind, title, body)) = found else {
                    continue;
                };
                if let Some(first) = declared.insert(key.clone(), *n) {
                    tree.faults.push(Fault::new(
                        "speckit.record-duplicate",
                        &file,
                        *n,
                        format!("{key} is also on line {first}; an id names one record"),
                    ));
                    continue;
                }
                let rid = format!("{id}/{key}");
                tree.relations
                    .push((rid.clone(), "part_of".to_owned(), id.clone()));
                tree.records.push(ForeignRecord {
                    id: rid,
                    kind,
                    title,
                    source: file.clone(),
                    line: *n,
                    revision: super::digest(body.as_bytes()),
                });
            }
        }
        // Tasks.
        if let Some(text) = tasks.as_deref() {
            let file = shown(repo, &dir.join("tasks.md"));
            let (lines, faults) = super::task_lines(text, &file, TASKS);
            tree.faults.extend(faults);
            let mut first: BTreeMap<String, usize> = BTreeMap::new();
            for t in lines {
                let Some(key) = task_id(&t.text).map(str::to_owned) else {
                    tree.faults.push(Fault::new(
                        TASKS,
                        &file,
                        t.line,
                        "a Spec Kit task opens with its id: `- [ ] T001 [P] [US1] Description`",
                    ));
                    continue;
                };
                if let Some(at) = first.insert(key.clone(), t.line) {
                    tree.faults.push(Fault::new(
                        TASKS,
                        &file,
                        t.line,
                        format!("task {key} is also on line {at}; an id names one task"),
                    ));
                    continue;
                }
                let tid = format!("{id}/{key}");
                tree.relations
                    .push((tid.clone(), "part_of".to_owned(), id.clone()));
                for m in markers(&t.text) {
                    if m.starts_with("US")
                        && m[2..].bytes().all(|b| b.is_ascii_digit())
                        && m.len() > 2
                    {
                        tree.relations.push((
                            tid.clone(),
                            "implements".to_owned(),
                            format!("{id}/{m}"),
                        ));
                    }
                }
                for word in t
                    .text
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                {
                    if bold_id(&format!("- **{word}**:"), "FR").is_some() {
                        tree.relations.push((
                            tid.clone(),
                            "implements".to_owned(),
                            format!("{id}/{word}"),
                        ));
                    }
                }
                let after = depends_on(&t.text);
                for a in &after {
                    tree.relations.push((
                        tid.clone(),
                        "depends_on".to_owned(),
                        format!("{id}/{a}"),
                    ));
                }
                w.tasks.push(Task {
                    id: tid,
                    key,
                    text: t.text,
                    done: t.done,
                    line: t.line,
                    revision: t.revision,
                    after,
                });
            }
            w.tasks_file = Some(file);
        }
        tree.warrants.push(w);
    }
    tree
}

/// The id prefix a feature's imported records take: `SK001` for
/// `001-photo-albums`, `SK-photo-albums` for an unnumbered one, so that
/// `FR-001` of two features are two records.
#[must_use]
pub fn record_prefix(feature: &str) -> String {
    let digits: String = feature.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        format!("SK-{}", super::slug(feature))
    } else {
        format!("SK{digits}")
    }
}

/// `war import speckit <dir>`: each feature a Warrant in the light
/// encoding, its tasks the items. Its functional requirements and success
/// criteria become records in `docs/records/<feature>/10-speckit.md` where
/// the program's record format admits them (the `delivery` profile
/// declares `requirement` and `outcome`, and the atom passes every rule
/// `war check` applies); otherwise they stay in the Warrant's description,
/// and the import says why.
pub fn import(
    repo: &Repository,
    store: &crate::ticket::Store,
    path: &Utf8Path,
) -> Result<super::write::Import, Vec<Fault>> {
    let Some(root) = locate(path) else {
        return Err(vec![Fault::new(
            "speckit.missing",
            &shown(repo, path),
            0,
            "no specs/ folder, nor a feature folder (spec.md, plan.md or tasks.md), is here",
        )]);
    };
    let tree = read(repo, &root);
    if !tree.faults.is_empty() {
        return Err(tree.faults);
    }
    let now = crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs());
    let corpus = crate::corpus::Corpus::new(repo);
    let mut incoming = Vec::new();
    let mut records = Vec::new();
    let mut warnings = Vec::new();
    for w in &tree.warrants {
        let own: Vec<&ForeignRecord> = tree
            .records
            .iter()
            .filter(|r| r.id.starts_with(&format!("{}/", w.id)))
            .collect();
        let key = |r: &ForeignRecord| r.id.rsplit('/').next().unwrap_or_default().to_owned();
        // Records where the format admits them.
        let prefix = record_prefix(&w.name);
        let typed: Vec<&&ForeignRecord> = own
            .iter()
            .filter(|r| matches!(r.kind, "requirement" | "outcome"))
            .collect();
        let mut written = None;
        if !typed.is_empty() {
            let rel = format!("{}/{}/10-speckit.md", crate::records::DIR, w.name);
            let mut atom = format!(
                "---\nschema: oh.war/records/v1\nprofile: delivery\n---\n# {}\n\nImported from the \
                 Spec Kit feature `{}/spec.md` (`{}`): its functional requirements and \
                 success criteria.\n",
                w.title, w.name, w.id
            );
            for r in &typed {
                atom.push_str(&format!(
                    "\n## {prefix}-{} · {}\n\n{}\n",
                    key(r),
                    r.kind,
                    openwarrant_core::ticket::one_line(&r.title)
                ));
            }
            let checked = crate::records::load_with(&corpus, &[(rel.clone(), atom.clone())]);
            match checked.faults.iter().find(|f| f.file == rel) {
                None => {
                    records.push((rel.clone(), atom));
                    written = Some(rel);
                }
                Some(f) => warnings.push(crate::diagnostic::Diagnostic::warn(
                    "speckit.records-not-admitted",
                    rel,
                    format!(
                        "{}: its requirements and success criteria stay in its description: \
                         this program's record format does not admit them as written ({}: {})",
                        w.id, f.rule, f.message
                    ),
                )),
            }
        }
        let mut body = w.description.clone();
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str(&format!(
            "## From Spec Kit\n\nImported from the Spec Kit feature `{}/`.",
            w.name
        ));
        if let Some(rel) = &written {
            body.push_str(&format!(
                " Its requirements and success criteria are records in `{rel}` \
                 (`{prefix}-FR-001`, ...)."
            ));
        }
        if !own.is_empty() {
            body.push('\n');
            for r in &own {
                body.push_str(&format!(
                    "\n- **{}** ({}): {}",
                    key(r),
                    r.kind,
                    openwarrant_core::ticket::one_line(&r.title)
                ));
            }
        }
        incoming.push(super::write::Incoming {
            source: w.id.clone(),
            title: w.title.clone(),
            body,
            notes: Vec::new(),
            priority: openwarrant_core::ticket::DEFAULT_PRIORITY,
            kind: None,
            labels: Vec::new(),
            part_of: None,
            created_at: w.created.clone().unwrap_or_else(|| now.clone()),
            created_by: store.actor.clone(),
            items: w
                .tasks
                .iter()
                .map(|t| super::write::IncomingItem {
                    key: t.key.clone(),
                    text: t.text.clone(),
                    done: t.done,
                    done_by: t.done.then(|| "unknown".to_owned()),
                    done_on: None,
                    note: None,
                    after: t
                        .after
                        .iter()
                        .map(|a| super::write::Ref::Item(a.clone()))
                        .collect(),
                })
                .collect(),
        });
    }
    Ok(super::write::Import {
        format: KIND,
        origin: shown(repo, &root),
        incoming,
        records,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_kit_lines_read_as_their_template_writes_them() {
        assert_eq!(task_id("T001 [P] Create"), Some("T001"));
        assert_eq!(task_id("Create T001"), None);
        assert_eq!(task_id("Tests"), None);
        assert_eq!(
            depends_on("Implement service in src/s.py (depends on T012, T013)"),
            vec!["T012".to_owned(), "T013".to_owned()]
        );
        assert_eq!(depends_on("Depends on T2 and T3."), vec!["T2", "T3"]);
        assert_eq!(markers("T010 [P] [US1] Contract test"), vec!["P", "US1"]);
        assert_eq!(
            bold_id("- **FR-001**: System MUST allow albums", "FR"),
            Some(("FR-001".to_owned(), "System MUST allow albums".to_owned()))
        );
        assert_eq!(
            bold_id("- **SC-002:** Users finish in 2 minutes", "SC"),
            Some(("SC-002".to_owned(), "Users finish in 2 minutes".to_owned()))
        );
        assert_eq!(bold_id("- **Key Entities**: x", "FR"), None);
        assert_eq!(
            user_story("### User Story 2 - Share an album (Priority: P2)"),
            Some(("US2".to_owned(), "Share an album".to_owned()))
        );
    }
}
