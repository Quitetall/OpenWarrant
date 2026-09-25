// SPDX-License-Identifier: Apache-2.0
//! Who governs a delivered path (OW-ADR-0021, SAS §37.5).
//!
//! A pin used to be forever: a resolved Warrant's `deliverables.toml` bound
//! each file's digest, and every later edit — however legitimate, under
//! however new a Warrant — needed a human-signed correction from every
//! Warrant that once delivered it. Under this module a path is governed by
//! the most recently AUTHORIZED Warrant whose recorded declaration set names
//! it. Earlier pins are historical: still recorded, still verifiable at their
//! own resolution, and not drift.
//!
//! Two facts shape the record this reads. Deliverables are not inside the
//! contract digest, so `deliverables.toml` can move after signing — which is
//! why the set is copied into `authorization.toml` at ingest, under the
//! attestation, and read from there and never from the manifest. And
//! `war pins --refresh` rewrites content digests during ordinary work, so the
//! thing an authorizer grants is the set of `(id, target_ref)` PAIRS, not
//! bytes. [`set_digest`] is the digest of exactly that.
//!
//! An authorization recorded before OW-ADR-0021 carries no set and owns
//! nothing. That is fail-closed and deliberate: nobody gains ownership without
//! a fresh human signature over a request that lists the paths.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::repo::{RepoError, Repository};

/// One `(id, target_ref)` pair as the authorizer saw it. Lives here rather
/// than in `authorize.rs` so `pins`, `correct` and `check` can name it without
/// importing the whole authorization surface.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OwnedDeliverable {
    pub id: String,
    pub target_ref: String,
}

/// The declared set of a Warrant, read from its `deliverables.toml`, sorted
/// by id so two readers agree on the digest.
pub fn declared_set(
    repo: &Repository,
    dir: &camino::Utf8Path,
) -> Result<Vec<OwnedDeliverable>, RepoError> {
    let mut set: Vec<OwnedDeliverable> = repo
        .load_deliverables(dir)?
        .records
        .iter()
        .map(|d| OwnedDeliverable {
            id: d.id.clone(),
            target_ref: d.target_ref.clone(),
        })
        .collect();
    set.sort();
    Ok(set)
}

/// sha256 over the RFC 8785 rendering of `[[id, target_ref], …]`, sorted by
/// id. Documented here rather than registered as a digest domain: it is a
/// fingerprint of a list, not a canonical artifact, and the compiler's domain
/// table is for things a foreign verifier recomputes.
#[must_use]
pub fn set_digest(set: &[OwnedDeliverable]) -> String {
    let mut sorted: Vec<&OwnedDeliverable> = set.iter().collect();
    sorted.sort();
    let pairs: Vec<[&str; 2]> = sorted
        .iter()
        .map(|d| [d.id.as_str(), d.target_ref.as_str()])
        .collect();
    let jcs = serde_jcs::to_string(&pairs).unwrap_or_default();
    format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(jcs.as_bytes())
    )
}

/// One Warrant's claim on one path, as its authorization recorded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Owner {
    pub alias: String,
    pub deliverable_id: String,
    /// The authorization's effective time — the ordering key. §67.2 calls a
    /// locally stamped time provisional; `authorize` refuses a time that
    /// precedes an existing owner of the same path so the order seen is the
    /// order recorded.
    pub authorized_at: String,
    pub resolved: bool,
}

/// Every governed path in the corpus, newest owner first.
#[derive(Debug, Default, Clone)]
pub struct Ownership {
    by_path: BTreeMap<String, Vec<Owner>>,
}

/// Indexes this process has built, by the [`fingerprint`] of their inputs.
static INDEXES: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, Ownership>>,
> = std::sync::OnceLock::new();

/// Every file the index reads, as (path, length, modification time in
/// nanoseconds): each Warrant's top-level records, `atoms/` and
/// `amendments/`; the authority register, allowed signers, responses and
/// batches; the SAS revisions; `openwarrant.toml`. `None` when any of it
/// cannot be read — then nothing is cached and the index is built fresh.
fn fingerprint(repo: &Repository) -> Option<String> {
    fn add(out: &mut Vec<u8>, path: &std::path::Path) -> Option<()> {
        let meta = std::fs::metadata(path).ok()?;
        if meta.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(path)
                .ok()?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect();
            entries.sort();
            for e in entries {
                add(out, &e)?;
            }
            return Some(());
        }
        let mtime = meta
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos();
        out.extend_from_slice(path.as_os_str().as_encoded_bytes());
        out.extend_from_slice(&meta.len().to_le_bytes());
        out.extend_from_slice(&mtime.to_le_bytes());
        out.push(0);
        Some(())
    }
    let root = repo.root.as_std_path();
    let mut out = Vec::new();
    add(&mut out, &root.join("openwarrant.toml"))?;
    // `docs/authority/standing` (OW-ADR-0029): a covered authorization is
    // believed only while its class file and the class's signed responses
    // are what they were, so the index is keyed by them too.
    for d in [
        "docs/authority",
        "docs/authority/responses",
        "docs/authority/batches",
        "docs/authority/standing",
    ] {
        let p = root.join(d);
        if p.is_dir() {
            add(&mut out, &p)?;
        }
    }
    let sas = repo.sas_revisions_dir();
    if sas.is_dir() {
        add(&mut out, sas.as_std_path())?;
    }
    let mut dirs = repo.warrant_dirs().ok()?;
    dirs.sort();
    for dir in dirs {
        let dir = dir.as_std_path();
        add(&mut out, dir)?;
        for sub in ["atoms", "amendments"] {
            let p = dir.join(sub);
            if p.is_dir() {
                add(&mut out, &p)?;
            }
        }
    }
    Some(openwarrant_compiler::sha256_hex(&out))
}

impl Ownership {
    /// Read every authorized Warrant with a recorded, attested set.
    ///
    /// Skipped, and therefore never owners: records with no `owned` set
    /// (legacy), records whose revision is not authorized, records whose
    /// authorize signature does not verify against the register, Warrants
    /// whose DERIVED currency is `superseded` (OW-ADR-0022: an authorized
    /// successor's `supersedes`, never a field), and Warrants whose resolution
    /// standing is annulled.
    ///
    /// Memoized for the process by [`fingerprint`]: `war compile` built this
    /// index once per pending act it judged — the whole corpus loaded dozens
    /// of times over. Any write to what the index reads moves the
    /// fingerprint, and the next call rebuilds.
    pub fn index(repo: &Repository) -> Result<Self, RepoError> {
        let key = fingerprint(repo);
        if let Some(key) = &key
            && let Some(hit) = INDEXES
                .get_or_init(Default::default)
                .lock()
                .ok()
                .and_then(|m| m.get(key).cloned())
        {
            return Ok(hit);
        }
        let corpus: Vec<crate::repo::Loaded> = repo
            .warrant_dirs()?
            .iter()
            .filter_map(|d| repo.load_warrant(d).ok())
            .collect();
        let built = Self::index_with(repo, &crate::relations::currencies(&corpus))?;
        if let Some(key) = key
            && let Ok(mut m) = INDEXES.get_or_init(Default::default).lock()
        {
            m.insert(key, built.clone());
        }
        Ok(built)
    }

    /// [`Self::index`] over a currency derivation the caller already holds,
    /// so `war check` derives it once for the corpus.
    pub fn index_with(
        repo: &Repository,
        currencies: &crate::relations::Currencies,
    ) -> Result<Self, RepoError> {
        let mut by_path: BTreeMap<String, Vec<Owner>> = BTreeMap::new();
        for dir in repo.warrant_dirs()? {
            let Some(alias) = dir.file_name().map(str::to_owned) else {
                continue;
            };
            let Some(record) = repo.load_authorization(&dir)? else {
                continue;
            };
            if record.owned.is_empty() {
                continue;
            }
            if record.revision.state != openwarrant_core::RevisionState::Authorized {
                continue;
            }
            let Some(auth) = &record.revision.authorization else {
                continue;
            };
            if !crate::authority_check::verify(
                repo,
                crate::authority_check::Act::Authorize,
                &alias,
                &auth.authorizer,
                Some(&record.revision.contract_digest),
            )
            .is_signed()
            {
                continue;
            }
            if matches!(
                currencies.of(&alias),
                crate::relations::Derived::Superseded { .. }
            ) {
                continue;
            }
            let resolution = repo.load_resolution(&dir)?;
            if resolution.as_ref().is_some_and(|r| {
                r.resolution.standing == openwarrant_core::ResolutionStanding::Annulled
            }) {
                continue;
            }
            let resolved = resolution.is_some();
            for d in &record.owned {
                by_path
                    .entry(d.target_ref.clone())
                    .or_default()
                    .push(Owner {
                        alias: alias.clone(),
                        deliverable_id: d.id.clone(),
                        authorized_at: auth.effective_time.clone(),
                        resolved,
                    });
            }
        }
        for owners in by_path.values_mut() {
            owners.sort_by(|a, b| {
                b.authorized_at
                    .cmp(&a.authorized_at)
                    .then_with(|| b.alias.cmp(&a.alias))
            });
        }
        Ok(Self { by_path })
    }

    /// The Warrant that governs `path` now, if any.
    #[must_use]
    pub fn current(&self, path: &str) -> Option<&Owner> {
        self.by_path.get(path).and_then(|v| v.first())
    }

    /// A governing Warrant other than `alias`, authorized later than `alias`
    /// was — the fact that makes `alias`'s pin on `path` historical. A Warrant
    /// with no recorded authorization time (legacy) is treated as authorized at
    /// the beginning of time, so any recorded owner is newer.
    #[must_use]
    pub fn newer_than(
        &self,
        path: &str,
        alias: &str,
        alias_authorized_at: Option<&str>,
    ) -> Option<&Owner> {
        let cur = self.current(path)?;
        if cur.alias == alias {
            return None;
        }
        match alias_authorized_at {
            Some(t) if cur.authorized_at.as_str() <= t => None,
            _ => Some(cur),
        }
    }

    /// OW-ADR-0029 with Q-003 (refuse, as recommended): the owner a Warrant
    /// covered by a standing authorization may NOT take `path` from — an
    /// authorized Warrant other than `alias` that has not resolved. Routine
    /// work never silently takes a file from work in flight; the in-flight
    /// Warrant resolves first, or the routine change is signed on its own.
    /// A resolved owner is not in flight: its pin becomes historical under
    /// OW-ADR-0021 exactly as for a signed Warrant.
    #[must_use]
    pub fn in_flight_owner(&self, path: &str, alias: &str) -> Option<&Owner> {
        self.current(path)
            .filter(|owner| owner.alias != alias && !owner.resolved)
    }

    /// Every owner of `path`, oldest first — the lineage `war pins --history`
    /// renders.
    #[must_use]
    pub fn lineage(&self, path: &str) -> Vec<&Owner> {
        let mut v: Vec<&Owner> = self
            .by_path
            .get(path)
            .map(|o| o.iter().collect())
            .unwrap_or_default();
        v.reverse();
        v
    }

    /// Paths governed by anyone, for tests and the hook.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.by_path.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_set_digest_ignores_declaration_order() {
        let a = vec![
            OwnedDeliverable {
                id: "D-002".into(),
                target_ref: "b.rs".into(),
            },
            OwnedDeliverable {
                id: "D-001".into(),
                target_ref: "a.rs".into(),
            },
        ];
        let b = vec![a[1].clone(), a[0].clone()];
        assert_eq!(set_digest(&a), set_digest(&b));
        assert!(set_digest(&a).starts_with("sha256:"));
        assert_eq!(set_digest(&a).len(), 7 + 64);
    }

    #[test]
    fn the_set_digest_moves_when_a_path_is_added() {
        let mut a = vec![OwnedDeliverable {
            id: "D-001".into(),
            target_ref: "a.rs".into(),
        }];
        let before = set_digest(&a);
        a.push(OwnedDeliverable {
            id: "D-002".into(),
            target_ref: "b.rs".into(),
        });
        assert_ne!(before, set_digest(&a));
    }

    fn owner(alias: &str, at: &str) -> Owner {
        Owner {
            alias: alias.into(),
            deliverable_id: "D-001".into(),
            authorized_at: at.into(),
            resolved: false,
        }
    }

    #[test]
    fn the_latest_authorization_governs_and_the_earlier_pin_is_historical() {
        let mut o = Ownership::default();
        o.by_path.insert(
            "x.rs".into(),
            vec![
                owner("OW-WAR-0112", "2026-09-22T10:00:00Z"),
                owner("OW-WAR-0005", "2026-09-01T10:00:00Z"),
            ],
        );
        assert_eq!(o.current("x.rs").unwrap().alias, "OW-WAR-0112");
        assert_eq!(
            o.newer_than("x.rs", "OW-WAR-0005", Some("2026-09-01T10:00:00Z"))
                .unwrap()
                .alias,
            "OW-WAR-0112"
        );
        assert!(
            o.newer_than("x.rs", "OW-WAR-0112", Some("2026-09-22T10:00:00Z"))
                .is_none()
        );
        // A legacy Warrant with no recorded time is older than any owner.
        assert!(o.newer_than("x.rs", "OW-WAR-0001", None).is_some());
        assert_eq!(
            o.lineage("x.rs")
                .iter()
                .map(|w| w.alias.as_str())
                .collect::<Vec<_>>(),
            ["OW-WAR-0005", "OW-WAR-0112"]
        );
    }

    #[test]
    fn a_covered_warrant_may_not_take_a_path_from_work_in_flight() {
        let mut o = Ownership::default();
        o.by_path.insert(
            "x.rs".into(),
            vec![owner("OW-WAR-0200", "2026-09-22T10:00:00Z")],
        );
        assert_eq!(
            o.in_flight_owner("x.rs", "OW-WAR-0201")
                .map(|w| w.alias.as_str()),
            Some("OW-WAR-0200")
        );
        assert!(o.in_flight_owner("x.rs", "OW-WAR-0200").is_none());
        o.by_path.get_mut("x.rs").expect("path")[0].resolved = true;
        assert!(o.in_flight_owner("x.rs", "OW-WAR-0201").is_none());
    }

    #[test]
    fn an_ungoverned_path_has_no_owner() {
        let o = Ownership::default();
        assert!(o.current("nothing").is_none());
        assert!(o.newer_than("nothing", "OW-WAR-0001", None).is_none());
    }
}
