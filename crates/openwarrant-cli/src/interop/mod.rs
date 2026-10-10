// SPDX-License-Identifier: Apache-2.0
//! Bring your work with you (OW-WAR-0148 M10): import and export for
//! Beads, OpenSpec and Spec Kit, and adapters that read OpenSpec and Spec
//! Kit folders in place. docs/TYPES.md, "One Warrant, three encodings", is
//! the guide.
//!
//! - [`beads`] — Beads' issue JSONL (`bd export`), in (`war admin import beads`)
//!   and out (`war admin export beads`).
//! - [`openspec`] — an `openspec/` folder: each change a Warrant, its
//!   tasks the items, its spec deltas the requirements it implements.
//! - [`speckit`] — a Spec Kit `specs/` folder: each feature a Warrant, its
//!   tasks the items, its requirements, success criteria and user stories
//!   records.
//! - [`adapters`] — `[[adapters]]` in `openwarrant.toml`: the same readers
//!   over a folder that stays where it is and is never written.
//!
//! Every import is deterministic (the same input gives the same Warrants,
//! byte for byte, apart from the journal line that records when the import
//! ran) and idempotent (a Warrant already brought in is named and left
//! alone). Input it cannot map is refused by rule, and a refusal writes
//! nothing.

pub mod adapters;
pub mod beads;
pub mod openspec;
pub mod speckit;
pub mod write;

use camino::{Utf8Path, Utf8PathBuf};

use crate::diagnostic::Diagnostic;
use crate::repo::Repository;

/// One finding about foreign input, by rule, anchored to `file:line`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fault {
    pub rule: &'static str,
    /// Repository-relative where the file is inside the repository,
    /// otherwise as given.
    pub file: String,
    /// 1-based; 0 when the fault is about the file or folder as a whole.
    pub line: usize,
    pub message: String,
}

impl Fault {
    #[must_use]
    pub fn new(rule: &'static str, file: &str, line: usize, message: impl Into<String>) -> Self {
        Self {
            rule,
            file: file.to_owned(),
            line,
            message: message.into(),
        }
    }

    /// The fault as an error diagnostic, `file:line` when there is a line.
    #[must_use]
    pub fn error(&self) -> Diagnostic {
        Diagnostic::error(self.rule, self.place(), self.message.clone())
    }

    /// The fault as a warning diagnostic.
    #[must_use]
    pub fn warn(&self) -> Diagnostic {
        Diagnostic::warn(self.rule, self.place(), self.message.clone())
    }

    #[must_use]
    pub fn place(&self) -> String {
        if self.line == 0 {
            self.file.clone()
        } else {
            format!("{}:{}", self.file, self.line)
        }
    }
}

/// `path` as a message shows it: relative to the repository root when it is
/// inside it.
#[must_use]
pub fn shown(repo: &Repository, path: &Utf8Path) -> String {
    path.strip_prefix(&repo.root)
        .map_or_else(|_| path.to_string(), ToString::to_string)
}

/// `path` resolved against the repository root unless it is absolute.
#[must_use]
pub fn resolve(repo: &Repository, path: &Utf8Path) -> Utf8PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        repo.root.join(path)
    }
}

/// `sha256:<hex>` of `bytes`.
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes))
}

/// A heading's words as an id part: lowercase letters and digits, every
/// other run one `-`, no `-` at either end.
#[must_use]
pub fn slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// The lines of `text` outside fenced code blocks, each with its 1-based
/// number and without its terminator. An unclosed fence hides the rest of
/// the file, as a Markdown renderer would.
#[must_use]
pub fn unfenced(text: &str) -> Vec<(usize, &str)> {
    let mut fence: Option<&str> = None;
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let t = raw.trim_start();
        let shallow = raw.len() - t.len() <= 3;
        let marker = ["```", "~~~"]
            .into_iter()
            .find(|m| shallow && t.starts_with(m));
        match (fence, marker) {
            (None, Some(m)) => {
                fence = Some(m);
                continue;
            }
            (Some(open), Some(m)) if open == m => {
                fence = None;
                continue;
            }
            (Some(_), _) => continue,
            (None, None) => out.push((n + 1, raw)),
        }
    }
    out
}

/// An ATX heading: its level and its text, closing `#`s trimmed.
#[must_use]
pub fn heading(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_end();
    let level = t.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 || line.starts_with(' ') {
        return None;
    }
    let rest = &t[level..];
    if !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    Some((level, rest.trim().trim_end_matches('#').trim_end()))
}

/// One Markdown task line (`- [ ] text`, `- [x] text`) as read from a
/// foreign task list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLine {
    /// 1-based.
    pub line: usize,
    pub done: bool,
    pub text: String,
    /// `sha256:` of the line's own bytes.
    pub revision: String,
}

/// Every task line of a task list, outside fences, and a fault for each
/// line that opens like a task and is not one: a box other than `[ ]` or
/// `[x]`, an empty box, or a task with no text. `rule` names the format's
/// rule (`openspec.tasks-malformed`).
#[must_use]
pub fn task_lines(text: &str, file: &str, rule: &'static str) -> (Vec<TaskLine>, Vec<Fault>) {
    let mut tasks = Vec::new();
    let mut faults = Vec::new();
    for (n, raw) in unfenced(text) {
        let t = raw.trim_start();
        let Some(rest) = t.strip_prefix(['-', '*', '+']) else {
            continue;
        };
        let rest_trimmed = rest.trim_start_matches(' ');
        if rest_trimmed.len() == rest.len() || !rest_trimmed.starts_with('[') {
            continue;
        }
        let inner = &rest_trimmed[1..];
        let mut chars = inner.chars();
        let (mark, close) = (chars.next(), chars.next());
        let after = chars.as_str();
        if mark == Some(']') {
            faults.push(Fault::new(
                rule,
                file,
                n,
                "an empty box `[]`; a task is `- [ ] ...` or `- [x] ...`",
            ));
            continue;
        }
        if close != Some(']') {
            continue;
        }
        if !(after.is_empty() || after.starts_with(' ')) {
            // `- [a](link)`: a link, not a task.
            continue;
        }
        let done = match mark {
            Some(' ') => false,
            Some('x' | 'X') => true,
            Some(other) => {
                faults.push(Fault::new(
                    rule,
                    file,
                    n,
                    format!("box `[{other}]`; a task is `- [ ] ...` (open) or `- [x] ...` (done)"),
                ));
                continue;
            }
            None => continue,
        };
        let text = after.trim();
        if text.is_empty() {
            faults.push(Fault::new(rule, file, n, "a task with no text"));
            continue;
        }
        tasks.push(TaskLine {
            line: n,
            done,
            text: text.to_owned(),
            revision: digest(raw.as_bytes()),
        });
    }
    (tasks, faults)
}

/// A folder's files are read with these, so a missing one is `None` and an
/// unreadable one is a fault rather than an empty file.
pub(crate) fn read_optional(path: &Utf8Path) -> Result<Option<String>, String> {
    match crate::vfs::read(path) {
        Ok(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|e| format!("not UTF-8 ({e})")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// The sub-directories of `dir`, sorted by name; hidden ones are skipped.
pub(crate) fn subdirs(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let Ok(rd) = crate::vfs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Utf8PathBuf> = rd
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| crate::vfs::is_dir(p))
        .filter(|p| !p.file_name().unwrap_or_default().starts_with('.'))
        .collect();
    out.sort();
    out
}

/// A date (`2026-09-28`) or a timestamp, as an RFC 3339 UTC timestamp
/// (`2026-09-28T00:00:00Z`), or `None` when it is neither.
#[must_use]
pub fn timestamp(text: &str) -> Option<String> {
    let t = text.trim();
    if let Ok(d) = chrono::DateTime::parse_from_rfc3339(t) {
        return Some(
            d.with_timezone(&chrono::Utc)
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
        );
    }
    chrono::NaiveDate::parse_from_str(t.get(..10)?, "%Y-%m-%d")
        .ok()
        .filter(|_| t.len() == 10)
        .map(|d| format!("{}T00:00:00Z", d.format("%Y-%m-%d")))
}

/// One folder of another tool's work, read (OW-WAR-0148 M10): the
/// Warrants in it, the records they hold or change, the relations between
/// them, and what could not be read, by rule. The same reading serves
/// `war admin import` and the read-in-place adapters.
#[derive(Debug, Clone, Default)]
pub struct Tree {
    /// `openspec` or `speckit`.
    pub kind: &'static str,
    /// The folder read.
    pub root: Utf8PathBuf,
    pub warrants: Vec<Foreign>,
    /// Records that are not a Warrant's task: requirements, success
    /// criteria, user stories.
    pub records: Vec<ForeignRecord>,
    /// `{from, kind, to}`; every id global.
    pub relations: Vec<(String, String, String)>,
    pub faults: Vec<Fault>,
}

/// One Warrant of a folder: an OpenSpec change or a Spec Kit feature.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Foreign {
    /// `openspec:<change>` or `speckit:<feature>`.
    pub id: String,
    /// The folder's own name for it.
    pub name: String,
    pub title: String,
    /// What it is for, in its own words; may be empty.
    pub description: String,
    pub dir: Utf8PathBuf,
    /// The file that names it (`proposal.md`, `spec.md`), shown.
    pub source: String,
    /// `sha256:` of that file, or of the folder's tasks when it has none.
    pub revision: String,
    /// RFC 3339, when the folder dates it.
    pub created: Option<String>,
    /// Its tasks file, shown, when it has one.
    pub tasks_file: Option<String>,
    pub tasks: Vec<Task>,
    /// Lines `war show` prints under "Changes" (OpenSpec's deltas).
    pub changes: Vec<String>,
}

impl Foreign {
    /// `(done, total)` over its tasks.
    #[must_use]
    pub fn progress(&self) -> (usize, usize) {
        (
            self.tasks.iter().filter(|t| t.done).count(),
            self.tasks.len(),
        )
    }

    /// `open`, `in_progress` or `done`, from its tasks alone: done when
    /// every task is and there is one.
    #[must_use]
    pub fn state(&self) -> &'static str {
        match self.progress() {
            (d, n) if n > 0 && d == n => "done",
            (0, _) => "open",
            _ => "in_progress",
        }
    }
}

/// One task of a Warrant read from a task list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Task {
    /// Its own key: `1.1`, `T001`, or `task-<n>` for an unnumbered one.
    pub key: String,
    /// Global: `<warrant id>/<key>`.
    pub id: String,
    pub text: String,
    pub done: bool,
    /// 1-based line in the tasks file.
    pub line: usize,
    /// `sha256:` of its line.
    pub revision: String,
    /// Keys of the tasks it waits on.
    pub after: Vec<String>,
}

/// One record a folder declares beside its Warrants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignRecord {
    pub id: String,
    /// `requirement`, `outcome` or `story`.
    pub kind: &'static str,
    pub title: String,
    /// The file that declares it, shown.
    pub source: String,
    pub line: usize,
    /// `sha256:` of its own bytes.
    pub revision: String,
}

impl Tree {
    /// Every task of every Warrant.
    pub fn tasks(&self) -> impl Iterator<Item = (&Foreign, &Task)> {
        self.warrants
            .iter()
            .flat_map(|w| w.tasks.iter().map(move |t| (w, t)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lines_read_boxes_and_refuse_the_malformed() {
        let text = "# Tasks\n\n- [ ] 1.1 Open one\n- [x] 1.2 Done one\n```\n- [y] in a fence\n```\n\
                    - [y] 1.3 Bad box\n- [] 1.4 Empty box\n* [X] 1.5 Star\n- [a](link) not a task\n";
        let (tasks, faults) = task_lines(text, "tasks.md", "openspec.tasks-malformed");
        let got: Vec<(usize, bool, &str)> = tasks
            .iter()
            .map(|t| (t.line, t.done, t.text.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (3, false, "1.1 Open one"),
                (4, true, "1.2 Done one"),
                (10, true, "1.5 Star")
            ]
        );
        let lines: Vec<usize> = faults.iter().map(|f| f.line).collect();
        assert_eq!(lines, vec![8, 9]);
        assert!(faults.iter().all(|f| f.rule == "openspec.tasks-malformed"));
    }

    #[test]
    fn slugs_headings_and_timestamps() {
        assert_eq!(
            slug("Report the installed version"),
            "report-the-installed-version"
        );
        assert_eq!(slug("  `--check`: on request!  "), "check-on-request");
        assert_eq!(
            heading("### Requirement: Foo ###"),
            Some((3, "Requirement: Foo"))
        );
        assert_eq!(heading("#hashtag"), None);
        assert_eq!(
            timestamp("2026-09-28").as_deref(),
            Some("2026-09-28T00:00:00Z")
        );
        assert_eq!(
            timestamp("2026-04-18T16:19:12.5+02:00").as_deref(),
            Some("2026-04-18T14:19:12Z")
        );
        assert_eq!(timestamp("yesterday"), None);
    }
}
