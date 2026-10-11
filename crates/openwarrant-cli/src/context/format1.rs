// SPDX-License-Identifier: Apache-2.0
//! The frontmatter of Format 1.0 work documents, read only as far as the
//! summary tier needs it (Format 1.0 sections 3.2, 3.3, 4.1, 10.1).
//!
//! A work document is `docs/warrants/<alias>.md`,
//! `docs/warrants/<alias>/warrant.md`, `docs/tickets/<alias>.md` or
//! `docs/tickets/<alias>/ticket.md` whose frontmatter declares `format:
//! openwarrant/1.<minor>` and an `alias`. This is no Format 1.0 reader: the
//! model's reader owns the grammar and its refusals. It reads the keys the
//! context compiler and `war check` need (`alias`, `summary`, `paths`) and
//! nothing else. ADAPTER(model §8): once the model carries a work
//! document's `fields`, the compiler reads them there.

use camino::Utf8PathBuf;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::Repository;

/// The longest `summary` section 3.3 allows, in bytes.
pub const SUMMARY_MAX: usize = 200;

/// One Format 1.0 work document's frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    /// Repository-relative path.
    pub path: String,
    pub alias: Option<String>,
    /// `(key, value, line)` scalars, in order.
    pub scalars: Vec<(String, String, usize)>,
    /// `(key, items)` flat lists.
    pub lists: Vec<(String, Vec<String>)>,
}

impl Document {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.scalars
            .iter()
            .find(|(k, _, _)| k == key)
            .map(|(_, v, _)| v.as_str())
    }

    #[must_use]
    pub fn list(&self, key: &str) -> &[String] {
        self.lists
            .iter()
            .find(|(k, _)| k == key)
            .map_or(&[], |(_, v)| v.as_slice())
    }
}

fn scalar(v: &str) -> String {
    let v = v.trim();
    if let Some(x) = v.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        return x.replace("\\\"", "\"").replace("\\\\", "\\");
    }
    if let Some(x) = v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
        return x.replace("''", "'");
    }
    v.to_owned()
}

/// The frontmatter of `text`, when it opens with one and declares a 1.x
/// `format`; `None` otherwise (a legacy or foreign file).
#[must_use]
pub fn parse(path: &str, text: &str) -> Option<Document> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut doc = Document {
        path: path.to_owned(),
        ..Document::default()
    };
    let mut list: Option<(String, Vec<String>)> = None;
    for (i, l) in lines.enumerate() {
        let line = i + 2;
        if l.trim_end() == "---" {
            break;
        }
        if let Some(item) = l.strip_prefix("  - ") {
            if let Some((_, items)) = list.as_mut() {
                items.push(scalar(item));
            }
            continue;
        }
        if let Some(done) = list.take() {
            doc.lists.push(done);
        }
        if l.trim_start().starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let Some((k, v)) = l.split_once(':') else {
            continue;
        };
        if v.trim().is_empty() {
            list = Some((k.trim().to_owned(), Vec::new()));
        } else {
            doc.scalars.push((k.trim().to_owned(), scalar(v), line));
        }
    }
    if let Some(done) = list.take() {
        doc.lists.push(done);
    }
    doc.get("format")?.strip_prefix("openwarrant/1.")?;
    doc.alias = doc.get("alias").map(str::to_owned);
    Some(doc)
}

/// Every Format 1.0 work document of `repo`, sorted by path.
#[must_use]
pub fn documents(repo: &Repository) -> Vec<Document> {
    let mut paths: Vec<Utf8PathBuf> = Vec::new();
    for (dir, inner) in [
        ("docs/warrants", "warrant.md"),
        ("docs/tickets", "ticket.md"),
    ] {
        let Ok(entries) = crate::vfs::read_dir_utf8(&repo.root.join(dir)) else {
            continue;
        };
        for p in entries {
            if p.extension() == Some("md") && crate::vfs::is_file(&p) {
                paths.push(p);
            } else if crate::vfs::is_dir(&p) && crate::vfs::is_file(p.join(inner)) {
                paths.push(p.join(inner));
            }
        }
    }
    paths.sort();
    paths
        .iter()
        .filter_map(|p| {
            let text = crate::vfs::read_to_string(p).ok()?;
            parse(&repo.relative(p), &text)
        })
        .filter(|d| d.alias.is_some())
        .collect()
}

/// `war check`: a Format 1.0 work document carries its one-line `summary`
/// (sections 3.3 and 10.1 make it required of a work document; a missing
/// one is `format.field-missing`). An over-long or multi-line one is a
/// warning: the format bounds it and names no refusal for it.
pub fn check(repo: &Repository, report: &mut Report) {
    for d in documents(repo) {
        let alias = d.alias.as_deref().unwrap_or("?");
        match d.scalars.iter().find(|(k, _, _)| k == "summary") {
            None => report.push(Diagnostic::error(
                "format.field-missing",
                format!("{}:1", d.path),
                format!(
                    "{alias}: a work document needs `summary:` in its frontmatter, one line of at most {SUMMARY_MAX} bytes saying what the work is for; an agent reads it before deciding what to read in full"
                ),
            )),
            Some((_, v, line)) if v.trim().is_empty() || v.len() > SUMMARY_MAX => {
                report.push(Diagnostic::warn(
                    "context.summary-shape",
                    format!("{}:{line}", d.path),
                    format!(
                        "{alias}: `summary` is {} bytes; one line of 1 to {SUMMARY_MAX} bytes is what the index shows",
                        v.len()
                    ),
                ));
            }
            Some(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_1_frontmatter_is_read_and_a_legacy_atom_is_not() {
        let d = parse(
            "docs/warrants/OW-WAR-0146.md",
            "---\nformat: openwarrant/1.0\nprofile: delivery\nalias: OW-WAR-0146\nsummary: \"A bundle: carries output\"\npaths:\n  - crates/**/bundle.rs\n  - 'conformance/'\n---\n# T\n",
        )
        .unwrap();
        assert_eq!(d.alias.as_deref(), Some("OW-WAR-0146"));
        assert_eq!(d.get("summary"), Some("A bundle: carries output"));
        assert_eq!(d.list("paths"), ["crates/**/bundle.rs", "conformance/"]);
        assert!(parse("a.md", "---\nschema: oh.war/atom/v1\n---\n").is_none());
        assert!(parse("a.md", "---\nformat: openwarrant/2.0\n---\n").is_none());
        assert!(parse("a.md", "# no frontmatter\n").is_none());
    }
}
