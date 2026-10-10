// SPDX-License-Identifier: Apache-2.0
//! What every `war evidence go` of one clone shares (OW-WAR-0148 M15),
//! under git's common directory beside M11's claims
//! (`<common>/openwarrant/go/`):
//!
//! - **landed/** — one marker per node that landed: its commit, the run and
//!   the actor. A tick is written to the checkout its run started from, and
//!   another worktree's checklist does not show it until the branches merge;
//!   the marker is what tells a second run, in another worktree, that the
//!   node is done, so it never runs it again. A marker is written while the
//!   run still holds the node's claim, before the tick releases it, and
//!   removed only if the tick is then refused, still under the claim.
//! - **land.lock** — one landing at a time onto an integration branch: the
//!   merge, the checks and the compare-and-set of the branch run under it,
//!   so two runs never both pass a check against a tip that one of them is
//!   about to move.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

/// One landed node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Landed {
    pub node: String,
    pub run: String,
    pub actor: String,
    /// The commit the integration branch moved to, or the pull request's
    /// branch tip.
    pub commit: String,
    pub integration: String,
    /// `claimed` or `observed`.
    pub level: String,
    pub at: String,
}

/// The shared directory, under the clone's common directory.
#[must_use]
pub fn dir(common: &Utf8Path) -> Utf8PathBuf {
    common.join("openwarrant").join("go")
}

fn marker(common: &Utf8Path, node: &str) -> Utf8PathBuf {
    dir(common)
        .join("landed")
        .join(format!("{}.json", crate::go::git::slug(node)))
}

/// The marker for `node`, when it landed.
#[must_use]
pub fn landed(common: &Utf8Path, node: &str) -> Option<Landed> {
    std::fs::read(marker(common, node))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

/// Every node with a marker.
#[must_use]
pub fn all(common: &Utf8Path) -> Vec<Landed> {
    let Ok(rd) = std::fs::read_dir(dir(common).join("landed")) else {
        return Vec::new();
    };
    let mut out: Vec<Landed> = rd
        .filter_map(Result::ok)
        .filter_map(|e| std::fs::read(e.path()).ok())
        .filter_map(|b| serde_json::from_slice(&b).ok())
        .collect();
    out.sort_by(|a, b| a.node.cmp(&b.node));
    out
}

/// Write the marker.
pub fn mark(common: &Utf8Path, l: &Landed) -> Result<(), String> {
    let path = marker(common, &l.node);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("could not create {parent}: {e}"))?;
    }
    let bytes = serde_json::to_vec_pretty(l).map_err(|e| e.to_string())?;
    crate::compile::atomic::write(&path, bytes).map_err(|e| format!("{path}: {e}"))
}

/// Remove the marker (the tick after it was refused).
pub fn unmark(common: &Utf8Path, node: &str) {
    let _ = std::fs::remove_file(marker(common, node));
}

/// Hold the landing lock until the returned file is dropped.
pub fn land_lock(common: &Utf8Path) -> Result<std::fs::File, String> {
    let d = dir(common);
    std::fs::create_dir_all(&d).map_err(|e| format!("could not create {d}: {e}"))?;
    let path = d.join("land.lock");
    let f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| format!("could not open {path}: {e}"))?;
    f.lock()
        .map_err(|e| format!("could not lock {path}: {e}"))?;
    Ok(f)
}
