// SPDX-License-Identifier: Apache-2.0
//! Where a test finds a ticket's files, whichever layout holds it.
//!
//! `war create` writes a master, `docs/work/<id>/<id>-<slug>.md`, with its
//! state beside it; `war create --legacy` and older fixtures keep
//! `docs/tickets/<id>.md` or `docs/tickets/<id>/`. A test names the older
//! path and [`at`] answers with the file as it is on disk.

use std::path::{Path, PathBuf};

/// `path` (`…/docs/tickets/<id>.md` or `…/docs/tickets/<id>/<rest>`) where
/// the ticket keeps it: unchanged when it exists there or the ticket is no
/// master, else under `…/docs/work/<id>/`.
pub fn at(path: PathBuf) -> PathBuf {
    if path.exists() {
        return path;
    }
    let text = path.to_string_lossy().into_owned();
    let Some((base, rest)) = text.rsplit_once("docs/tickets/") else {
        return path;
    };
    let (id, inner) = match rest.split_once('/') {
        Some((id, inner)) => (id, Some(inner)),
        None => (rest.strip_suffix(".md").unwrap_or(rest), None),
    };
    let work = Path::new(base).join("docs/work").join(id);
    if !work.is_dir() {
        return path;
    }
    match inner {
        Some(inner) => work.join(inner),
        None => std::fs::read_dir(&work)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.extension().is_some_and(|x| x == "md")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(id))
            })
            .unwrap_or(path),
    }
}
