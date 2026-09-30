// SPDX-License-Identifier: Apache-2.0
//! Opt-in SAS acceptance bootstrap, governed by 0074 AM-001's adoption decision.
//! File capture and publication are descriptor-relative; this is not a sandbox.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Seek, Write};

use openwarrant_compiler::{sha256_hex, to_canonical_bytes};
use openwarrant_core::sas::{
    SAS_SOURCE_SET_REVISION_SCHEMA, SAS_SOURCE_SET_SUBJECT_SCHEMA, SasPredecessorIdentity,
    SasRevision, SasSourceIdentity, SasSourceSetSubject, Section106Diff, section_106,
};
use rustix::fs::{Mode, OFlags, openat};
use serde::Deserialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

const MAX_FILE: u64 = 8 * 1024 * 1024;
const MAX_TOTAL: u64 = 32 * 1024 * 1024;
const MAX_MANIFEST: u64 = 2 * 1024 * 1024;
const MAX_MEMBERS: usize = 1024;
const MANIFEST_SCHEMA: &str = "oh.war/spec-source-set/1.0.0-rc.2";

fn err(message: impl std::fmt::Display) -> RepoError {
    RepoError::Message(format!("sas.source-set: {message}"))
}

fn path_parts(path: &str) -> Result<Vec<&str>, RepoError> {
    let parts: Vec<_> = path.split('/').collect();
    if path.len() > 4096
        || path.contains(['\\', '\0', ':'])
        || parts
            .iter()
            .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return Err(err(format!("unsafe path {path:?}")));
    }
    Ok(parts)
}

fn version_path(version: &str) -> Result<(), RepoError> {
    path_parts(version)?;
    if version.contains('/')
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
    {
        return Err(err("unsafe edition"));
    }
    Ok(())
}

/// Every component is opened with NOFOLLOW, relative to an already opened dir.
struct Store(File);

impl Store {
    fn new(repo: &Repository) -> Result<Self, RepoError> {
        File::open(&repo.root).map(Self).map_err(err)
    }

    fn directory(&self, parts: &[&str], create: bool) -> Result<File, RepoError> {
        let mut dir = self.0.try_clone().map_err(err)?;
        for part in parts {
            if create {
                match rustix::fs::mkdirat(&dir, *part, Mode::RWXU) {
                    Ok(()) => dir.sync_all().map_err(err)?,
                    Err(rustix::io::Errno::EXIST) => {}
                    Err(e) => return Err(err(e)),
                }
            }
            dir = openat(
                &dir,
                *part,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map(File::from)
            .map_err(|e| err(format!("unsafe or missing directory {part:?}: {e}")))?;
        }
        Ok(dir)
    }

    fn parent(&self, path: &str, create: bool) -> Result<(File, String), RepoError> {
        let parts = path_parts(path)?;
        Ok((
            self.directory(&parts[..parts.len() - 1], create)?,
            parts[parts.len() - 1].to_owned(),
        ))
    }

    fn read(&self, path: &str, limit: u64) -> Result<Vec<u8>, RepoError> {
        let (dir, name) = self.parent(path, false)?;
        let mut file = openat(
            &dir,
            name.as_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map(File::from)
        .map_err(|e| err(format!("unsafe or missing file {path:?}: {e}")))?;
        let before = file.metadata().map_err(err)?;
        if !before.is_file() || before.len() > limit {
            return Err(err(format!("non-regular or oversized file {path:?}")));
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        file.rewind().map_err(err)?;
        let mut again = Vec::new();
        (&mut file)
            .take(limit + 1)
            .read_to_end(&mut again)
            .map_err(err)?;
        let after = file.metadata().map_err(err)?;
        if bytes.len() as u64 != before.len()
            || bytes != again
            || before.len() != after.len()
            || before.modified().map_err(err)? != after.modified().map_err(err)?
        {
            return Err(err(format!("inconsistent read {path:?}")));
        }
        Ok(bytes)
    }

    fn new_file(&self, path: &str, bytes: &[u8]) -> Result<(), RepoError> {
        let (dir, name) = self.parent(path, true)?;
        let mut file = openat(
            &dir,
            name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map(File::from)
        .map_err(|e| {
            err(format!(
                "destination {path:?} already exists or cannot be created: {e}"
            ))
        })?;
        file.write_all(bytes).map_err(err)?;
        file.sync_all().map_err(err)?;
        dir.sync_all().map_err(err)
    }

    fn new_directory(&self, path: &str) -> Result<(), RepoError> {
        let (dir, name) = self.parent(path, true)?;
        rustix::fs::mkdirat(&dir, name.as_str(), Mode::RWXU).map_err(|e| {
            err(format!(
                "snapshot destination {path:?} already exists or cannot be created: {e}"
            ))
        })?;
        dir.sync_all().map_err(err)
    }

    fn publish_new(&self, path: &str, bytes: &[u8]) -> Result<(), RepoError> {
        let pending = format!("{path}.pending");
        self.new_file(&pending, bytes)?;
        let (dir, name) = self.parent(path, false)?;
        let (_, pending_name) = self.parent(&pending, false)?;
        // A hard link publishes the complete, synced file without replacing a
        // destination created concurrently. Both names are in this directory.
        rustix::fs::linkat(
            &dir,
            pending_name.as_str(),
            &dir,
            name.as_str(),
            rustix::fs::AtFlags::empty(),
        )
        .map_err(|e| {
            err(format!(
                "revision destination already exists or cannot be published: {e}"
            ))
        })?;
        dir.sync_all()
            .map_err(|e| err(format!("proposal published; directory sync failed: {e}")))?;
        rustix::fs::unlinkat(&dir, pending_name.as_str(), rustix::fs::AtFlags::empty()).map_err(
            |e| {
                err(format!(
                    "proposal published; pending-name cleanup failed: {e}"
                ))
            },
        )?;
        dir.sync_all()
            .map_err(|e| err(format!("proposal published; cleanup sync failed: {e}")))
    }
}

pub struct RevisionLock {
    dir: File,
    name: String,
    _file: File,
}

impl Drop for RevisionLock {
    fn drop(&mut self) {
        let _ = rustix::fs::unlinkat(&self.dir, self.name.as_str(), rustix::fs::AtFlags::empty());
    }
}

pub fn lock(repo: &Repository, version: &str) -> Result<RevisionLock, RepoError> {
    version_path(version)?;
    let store = Store::new(repo)?;
    let (dir, name) = store.parent(
        &format!(
            "{}/.{version}.lock",
            repo.relative(&repo.sas_revisions_dir())
        ),
        true,
    )?;
    let file = openat(
        &dir,
        name.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map(File::from)
    .map_err(|e| err(format!("revision busy or retained lock {name:?}: {e}")))?;
    Ok(RevisionLock {
        dir,
        name,
        _file: file,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    edition: String,
    status: String,
    files: Vec<Member>,
    #[serde(default)]
    historical_baseline: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    path: String,
    bytes: u64,
    sha256: String,
    role: String,
}

struct Capture {
    manifest: Vec<u8>,
    decision: Vec<u8>,
    members: BTreeMap<String, Vec<u8>>,
    main: Vec<u8>,
}

fn capture(
    store: &Store,
    subject: &SasSourceSetSubject,
    snapshot: Option<&str>,
) -> Result<Capture, RepoError> {
    let manifest_path = snapshot
        .map(|s| format!("{s}/manifest.json"))
        .unwrap_or_else(|| subject.manifest.path.clone());
    let manifest_bytes = store.read(&manifest_path, MAX_MANIFEST)?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| err(format!("invalid manifest: {e}")))?;
    if manifest.schema != MANIFEST_SCHEMA
        || manifest.edition != subject.edition
        || manifest.status != "candidate-unaccepted"
    {
        return Err(err("manifest schema, edition or status mismatch"));
    }
    // Retained in the exact manifest bytes; this label grants no authority.
    let _ = manifest.historical_baseline;
    if manifest.files.is_empty() || manifest.files.len() > MAX_MEMBERS {
        return Err(err("manifest member count exceeds bounds"));
    }
    let base = subject
        .manifest
        .path
        .rsplit_once('/')
        .map(|(p, _)| format!("{p}/"))
        .unwrap_or_default();
    let mut paths = BTreeSet::new();
    let mut members = BTreeMap::new();
    let mut main = None;
    let mut total = manifest_bytes.len() as u64;
    for member in manifest.files {
        path_parts(&member.path)?;
        if !paths.insert(member.path.clone()) {
            return Err(err(format!("duplicate member path {:?}", member.path)));
        }
        if member.role != "normative" && member.role != "reference" {
            return Err(err("unknown member role"));
        }
        if member.bytes > MAX_FILE {
            return Err(err("oversized member"));
        }
        total = total
            .checked_add(member.bytes)
            .ok_or_else(|| err("capture size overflow"))?;
        if total > MAX_TOTAL {
            return Err(err("capture exceeds total size bound"));
        }
        let live = format!("{base}{}", member.path);
        let path = snapshot
            .map(|s| format!("{s}/members/{}", member.path))
            .unwrap_or_else(|| live.clone());
        let bytes = store.read(&path, member.bytes)?;
        if bytes.len() as u64 != member.bytes
            || member.sha256 != format!("sha256:{}", sha256_hex(&bytes))
        {
            return Err(err(format!(
                "member length/digest mismatch: {}",
                member.path
            )));
        }
        if live == subject.main.path {
            if member.role != "normative" {
                return Err(err("main source is not normative"));
            }
            main = Some(bytes.clone());
        }
        members.insert(member.path, bytes);
    }
    let main = main.ok_or_else(|| err("main source is not a manifest member"))?;
    let decision_path = snapshot
        .map(|s| format!("{s}/decision.md"))
        .unwrap_or_else(|| subject.decision.path.clone());
    let decision = store.read(&decision_path, MAX_FILE)?;
    if total + decision.len() as u64 > MAX_TOTAL {
        return Err(err("capture exceeds total size bound"));
    }
    Ok(Capture {
        manifest: manifest_bytes,
        decision,
        members,
        main,
    })
}

fn identity(path: &str, bytes: &[u8]) -> SasSourceIdentity {
    SasSourceIdentity {
        path: path.to_owned(),
        sha256: sha256_hex(bytes),
    }
}

fn subject_digest(subject: &SasSourceSetSubject) -> Result<String, RepoError> {
    let mut bytes = SAS_SOURCE_SET_SUBJECT_SCHEMA.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend(to_canonical_bytes(subject).map_err(err)?);
    Ok(sha256_hex(&bytes))
}

fn snapshot_path(repo: &Repository, version: &str) -> String {
    format!(
        "{}/source-sets/{version}",
        repo.relative(&repo.sas_revisions_dir())
    )
}

pub fn propose(
    repo: &Repository,
    version: &str,
    main: &str,
    manifest: &str,
    decision: &str,
) -> Result<Report, RepoError> {
    let _lock = lock(repo, version)?;
    for path in [main, manifest, decision] {
        path_parts(path)?;
    }
    let existing = repo.load_sas_revisions()?;
    if existing.iter().any(|r| r.version == version) {
        return Err(err("revision already exists"));
    }
    let predecessor = crate::sas::pin_of(&existing);
    let mut subject = SasSourceSetSubject {
        schema: SAS_SOURCE_SET_SUBJECT_SCHEMA.to_owned(),
        edition: version.to_owned(),
        main: identity(main, &[]),
        manifest: identity(manifest, &[]),
        decision: identity(decision, &[]),
        predecessor: predecessor.map(|p| SasPredecessorIdentity {
            version: p.version.clone(),
            sha256: p.sha256.clone(),
        }),
    };
    let store = Store::new(repo)?;
    let captured = capture(&store, &subject, None)?;
    subject.main = identity(main, &captured.main);
    subject.manifest = identity(manifest, &captured.manifest);
    subject.decision = identity(decision, &captured.decision);
    let requirements = section_106(std::str::from_utf8(&captured.main).map_err(err)?);
    if requirements.is_empty() {
        return Err(err("main source has no §106 requirement index"));
    }
    let diff = predecessor
        .map(|p| Section106Diff::between(&p.requirements, &requirements))
        .unwrap_or_default();
    diff.check_stability().map_err(err)?;
    let mut record = SasRevision::proposed(
        version,
        main,
        subject_digest(&subject)?,
        predecessor.map(|p| p.version.clone()),
        requirements,
        diff.is_architecture_changing(),
    );
    record.schema = SAS_SOURCE_SET_REVISION_SCHEMA.to_owned();
    record.source_set = Some(subject);
    record.validate().map_err(err)?;
    let snapshot = snapshot_path(repo, version);
    store.new_directory(&snapshot)?;
    store.new_file(&format!("{snapshot}/manifest.json"), &captured.manifest)?;
    store.new_file(&format!("{snapshot}/decision.md"), &captured.decision)?;
    for (path, bytes) in captured.members {
        store.new_file(&format!("{snapshot}/members/{path}"), &bytes)?;
    }
    // Capture must be complete and current before the revision becomes visible.
    validate(repo, &record, true)?;
    store.publish_new(
        &repo.relative(&repo.sas_revision_path(version)),
        toml::to_string_pretty(&record).map_err(err)?.as_bytes(),
    )?;
    let mut report = Report::default();
    report.push(Diagnostic::pass(
        "sas.proposed",
        format!(
            "{version} proposed; complete subject sha256:{}; snapshot {snapshot}",
            record.sha256
        ),
    ));
    report.note(
        "Proposed, not accepted. The complete source-set subject requires human acceptance."
            .to_owned(),
    );
    Ok(report)
}

/// Checks retained bytes separately from the current input agreement.
pub fn validate(repo: &Repository, record: &SasRevision, current: bool) -> Result<(), RepoError> {
    record.validate().map_err(err)?;
    version_path(&record.version)?;
    let subject = record
        .source_set
        .as_ref()
        .ok_or_else(|| err("missing source-set subject"))?;
    for i in [&subject.main, &subject.manifest, &subject.decision] {
        path_parts(&i.path)?;
    }
    if subject_digest(subject)? != record.sha256 {
        return Err(err("subject digest mismatch"));
    }
    let store = Store::new(repo)?;
    let snapshot = snapshot_path(repo, &record.version);
    let captured = capture(&store, subject, Some(&snapshot))?;
    check_identity(subject, &captured)?;
    if section_106(std::str::from_utf8(&captured.main).map_err(err)?) != record.requirements {
        return Err(err("captured requirement index differs from revision"));
    }
    let all = repo.load_sas_revisions()?;
    if let Some(p) = &subject.predecessor {
        let prior = all
            .iter()
            .find(|r| r.version == p.version && r.sha256 == p.sha256)
            .ok_or_else(|| err("predecessor identity mismatch"))?;
        let diff = Section106Diff::between(&prior.requirements, &record.requirements);
        diff.check_stability().map_err(err)?;
        if diff.is_architecture_changing() != record.architecture_changing {
            return Err(err("architecture diff mismatch"));
        }
    }
    if current {
        let live = capture(&store, subject, None)?;
        check_identity(subject, &live)?;
        if !record.is_accepted() {
            let others: Vec<_> = all
                .into_iter()
                .filter(|r| r.version != record.version)
                .collect();
            let predecessor = crate::sas::pin_of(&others).map(|p| SasPredecessorIdentity {
                version: p.version.clone(),
                sha256: p.sha256.clone(),
            });
            if predecessor != subject.predecessor {
                return Err(err("predecessor changed since proposal"));
            }
        }
    }
    Ok(())
}

fn check_identity(subject: &SasSourceSetSubject, captured: &Capture) -> Result<(), RepoError> {
    for (id, bytes) in [
        (&subject.main, &captured.main),
        (&subject.manifest, &captured.manifest),
        (&subject.decision, &captured.decision),
    ] {
        if id.sha256 != sha256_hex(bytes) {
            return Err(err(format!("source identity mismatch: {}", id.path)));
        }
    }
    Ok(())
}

pub fn captured_main(
    repo: &Repository,
    record: &SasRevision,
) -> Result<(camino::Utf8PathBuf, Vec<u8>), RepoError> {
    validate(repo, record, false)?;
    let subject = record
        .source_set
        .as_ref()
        .ok_or_else(|| err("missing source-set subject"))?;
    let base = subject
        .manifest
        .path
        .rsplit_once('/')
        .map(|(p, _)| format!("{p}/"))
        .unwrap_or_default();
    let member = subject
        .main
        .path
        .strip_prefix(&base)
        .ok_or_else(|| err("main source is outside manifest directory"))?;
    let path = format!("{}/members/{member}", snapshot_path(repo, &record.version));
    let bytes = Store::new(repo)?.read(&path, MAX_FILE)?;
    if sha256_hex(&bytes) != subject.main.sha256 {
        return Err(err("captured main changed during read"));
    }
    Ok((repo.root.join(path), bytes))
}

/// Caller holds the revision lock throughout validation and publication.
pub fn publish_acceptance(
    repo: &Repository,
    proposed: &SasRevision,
    accepted: &SasRevision,
) -> Result<(), RepoError> {
    let store = Store::new(repo)?;
    let path = repo.relative(&repo.sas_revision_path(&proposed.version));
    let bytes = store.read(&path, MAX_MANIFEST)?;
    let actual: SasRevision =
        toml::from_str(std::str::from_utf8(&bytes).map_err(err)?).map_err(err)?;
    if &actual != proposed || proposed.is_accepted() {
        return Err(err("proposal changed before acceptance"));
    }
    validate(repo, proposed, true)?;
    let pending = format!("{path}.pending");
    store.new_file(
        &pending,
        toml::to_string_pretty(accepted).map_err(err)?.as_bytes(),
    )?;
    let (dir, name) = store.parent(&path, false)?;
    let (_, pending_name) = store.parent(&pending, false)?;
    rustix::fs::renameat(&dir, pending_name.as_str(), &dir, name.as_str()).map_err(err)?;
    dir.sync_all()
        .map_err(|e| err(format!("acceptance published; directory sync failed: {e}")))
}
