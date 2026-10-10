// SPDX-License-Identifier: Apache-2.0
//! Read-only execution-account loading, not activation or launch fencing.
//! The trusted host supplies the authority-store path independently of agent
//! input. Same-account stores and unprotected test stores never become eligible.
use super::collector_signature::OpenSshSignatureCheck;
use openwarrant_core::{
    authority_transition::Revision,
    runtime_collector::{AuthenticatedEnrollment, Enrollment, Fault, Signed, Use},
};
use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

pub struct LoadedEnrollment {
    authority_store: PathBuf,
    authenticated: AuthenticatedEnrollment,
    active_digest: Option<String>,
}
/// Operator-owned local host settings, not a document-standard authority act.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HostConfig {
    #[serde(rename = "schema")]
    _schema: String,
    repository: String,
    repository_root: PathBuf,
    collector: String,
    execution_uid: u32,
}

/// Host-observed identities cannot be replaced by a capture's principal labels.
pub struct HostEnrollment {
    repository: String,
    collector: String,
    enrollment: LoadedEnrollment,
    observation: HostObservation,
}
pub(super) struct HostObservation {
    store: PathBuf,
    bytes: Vec<u8>,
}
impl HostObservation {
    pub(super) fn check(&self) -> Result<(), Fault> {
        if host_bytes(&self.store)? != self.bytes {
            return Err(Fault::Rejected("runtime host configuration changed"));
        }
        Ok(())
    }
}
impl HostEnrollment {
    pub fn repository(&self) -> &str {
        &self.repository
    }
    pub fn collector(&self) -> &str {
        &self.collector
    }
    /// Acquires an already-activated enrollment using the protected host's
    /// identities. This records no activation. Native input custody remains
    /// a host responsibility; surrounding checks are not atomic launch fencing.
    pub fn acquire_verifier(
        self,
        warrant: &str,
        provider: &openwarrant_core::document::runtime::ProviderInterface,
        executable: &Path,
    ) -> Result<
        super::activated_verifier::ActivatedVerifier,
        openwarrant_core::document::runtime::ProviderFailure,
    > {
        self.observation
            .check()
            .map_err(super::activated_verifier::failure)?;
        super::activated_verifier::ActivatedVerifier::acquire(
            self.enrollment,
            &self.repository,
            &self.collector,
            warrant,
            provider,
            executable,
        )?
        .with_host(self.observation)
    }
}
fn host_bytes(store: &Path) -> Result<Vec<u8>, Fault> {
    snapshot_for_execution(store)?;
    #[cfg(not(unix))]
    {
        Err(Fault::Unavailable(
            "protected runtime host unsupported on this platform",
        ))
    }
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags};
        use std::os::unix::fs::MetadataExt;
        let path = store.join("runtime-host.json");
        let fd = rustix::fs::open(
            &path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|e| {
            if e == rustix::io::Errno::LOOP {
                Fault::Rejected("runtime host source is a symlink")
            } else {
                Fault::Unavailable("runtime host configuration unavailable")
            }
        })?;
        let file = fs::File::from(fd);
        let metadata = file
            .metadata()
            .map_err(|_| Fault::Unavailable("runtime host metadata unavailable"))?;
        let owner = fs::symlink_metadata(store)
            .map_err(|_| Fault::Unavailable("runtime host owner unavailable"))?
            .uid();
        if !metadata.is_file() || metadata.uid() != owner || metadata.mode() & 0o022 != 0 {
            return Err(Fault::Rejected(
                "regular operator-owned runtime host configuration required",
            ));
        }
        if crate::authority_cmd::store::effective_write_access(&path)
            .map_err(|_| Fault::Unavailable("runtime host effective access unavailable"))?
        {
            return Err(Fault::Rejected(
                "runtime host configuration is writable by the executor",
            ));
        }
        if metadata.len() > 65_536 {
            return Err(Fault::Rejected("runtime host configuration budget"));
        }
        let mut bytes = Vec::new();
        file.take(65_537)
            .read_to_end(&mut bytes)
            .map_err(|_| Fault::Unavailable("runtime host configuration read unavailable"))?;
        if bytes.len() > 65_536 {
            return Err(Fault::Rejected("runtime host configuration budget"));
        }
        snapshot_for_execution(store)?;
        Ok(bytes)
    }
}
fn absolute(path: &Path) -> Result<(), Fault> {
    if !path.is_absolute() || path.components().any(|p| matches!(p, Component::ParentDir)) {
        return Err(Fault::Rejected(
            "absolute path without parent traversal required",
        ));
    }
    Ok(())
}
fn enrollment_bytes(path: &Path) -> Result<Vec<u8>, Fault> {
    absolute(path)?;
    #[cfg(not(unix))]
    {
        return Err(Fault::Unavailable(
            "protected collector loading unsupported on this platform",
        ));
    }
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags};
        // NONBLOCK prevents a FIFO from hanging before the regular-file check.
        // NOFOLLOW rejects a symlink; validation uses the opened descriptor.
        let fd = rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|e| {
            if e == rustix::io::Errno::LOOP {
                Fault::Rejected("enrollment source is a symlink")
            } else {
                Fault::Unavailable("enrollment source unavailable")
            }
        })?;
        let file = fs::File::from(fd);
        let metadata = file
            .metadata()
            .map_err(|_| Fault::Unavailable("enrollment metadata unavailable"))?;
        if !metadata.is_file() {
            return Err(Fault::Rejected("regular enrollment file required"));
        }
        if metadata.len() > 65_536 {
            return Err(Fault::Rejected("enrollment wire budget"));
        }
        let mut bytes = Vec::new();
        file.take(65_537)
            .read_to_end(&mut bytes)
            .map_err(|_| Fault::Unavailable("enrollment read unavailable"))?;
        if bytes.len() > 65_536 {
            return Err(Fault::Rejected("enrollment wire budget"));
        }
        Ok(bytes)
    }
}
fn snapshot_for_execution(root: &Path) -> Result<crate::authority_cmd::store::Current, Fault> {
    absolute(root)?;
    #[cfg(not(unix))]
    {
        return Err(Fault::Unavailable(
            "protected collector loading unsupported on this platform",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = rustix::process::geteuid().as_raw();
        let state = root.join("state.json");
        // Check the actual account BEFORE reading unsigned store configuration.
        // Its claimed agent_uid cannot make a self-writable store a trust root.
        for path in root.ancestors().chain(std::iter::once(state.as_path())) {
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| Fault::Unavailable("authority store metadata unavailable"))?;
            if metadata.file_type().is_symlink()
                || (path == state && !metadata.is_file())
                || (path != state && !metadata.is_dir())
            {
                return Err(Fault::Rejected("regular protected authority path required"));
            }
            if metadata.uid() == uid
                || metadata.mode() & 0o022 != 0
                || crate::authority_cmd::store::effective_write_access(path).map_err(|_| {
                    Fault::Unavailable("effective authority write access unavailable")
                })?
            {
                return Err(Fault::Rejected(
                    "authority store is writable by the executor",
                ));
            }
        }
        crate::authority_cmd::store::unprivileged_reader().map_err(|_| {
            Fault::Unavailable("execution privileges cannot establish protected authority")
        })?;
        let current = crate::authority_cmd::store::read_current(root, false)
            .map_err(|_| Fault::Unavailable("authenticated authority snapshot unavailable"))?;
        if current.test_mode || current.agent_uid != Some(uid) {
            return Err(Fault::Rejected(
                "authority store execution account mismatch",
            ));
        }
        Ok(current)
    }
}
fn current_for_execution(root: &Path) -> Result<Revision, Fault> {
    Ok(snapshot_for_execution(root)?.revision)
}
impl LoadedEnrollment {
    /// Read the operator's protected UID-to-collector/repository binding. The
    /// caller supplies the actual workspace path, never principal labels.
    /// Missing settings remain unavailable; there is no legacy-name fallback.
    pub fn load_host(
        authority_store: &Path,
        repository_root: &Path,
        verifier: &OpenSshSignatureCheck,
    ) -> Result<HostEnrollment, Fault> {
        absolute(repository_root)?;
        let bytes = host_bytes(authority_store)?;
        let value = crate::sdk::wire::decode_value(&bytes)
            .map_err(|_| Fault::Rejected("runtime host configuration syntax"))?;
        if value["schema"] != "oh.war/runtime-host-config/v1-draft.1" {
            return Err(Fault::Unavailable(
                "runtime host configuration schema unsupported",
            ));
        }
        let config: HostConfig = serde_json::from_value(value)
            .map_err(|_| Fault::Rejected("runtime host configuration fields"))?;
        absolute(&config.repository_root)?;
        let actual = repository_root
            .canonicalize()
            .map_err(|_| Fault::Unavailable("runtime host repository unavailable"))?;
        if actual != config.repository_root {
            return Err(Fault::Rejected("runtime host repository path mismatch"));
        }
        #[cfg(unix)]
        if config.execution_uid != rustix::process::geteuid().as_raw() {
            return Err(Fault::Rejected("runtime host execution account mismatch"));
        }
        let enrollment = Self::load_active(
            authority_store,
            &config.repository,
            &config.collector,
            verifier,
        )?;
        let observation = HostObservation {
            store: authority_store.to_owned(),
            bytes,
        };
        observation.check()?;
        Ok(HostEnrollment {
            repository: config.repository,
            collector: config.collector,
            enrollment,
            observation,
        })
    }
    /// Load the operator-selected configuration. The trusted host still supplies
    /// the store and authenticated collector identity; this method does not
    /// turn an arbitrary caller-provided principal name into authentication.
    pub fn load_active(
        authority_store: &Path,
        repository: &str,
        collector: &str,
        verifier: &OpenSshSignatureCheck,
    ) -> Result<Self, Fault> {
        let snapshot = snapshot_for_execution(authority_store)?;
        let active = snapshot
            .active_collectors
            .get(collector)
            .ok_or(Fault::Unavailable("collector activation unavailable"))?;
        let authenticated = active
            .signed
            .authenticate(&snapshot.revision, repository, verifier)?;
        let latest = snapshot_for_execution(authority_store)?;
        if latest.head != snapshot.head
            || latest
                .active_collectors
                .get(collector)
                .is_none_or(|record| record.digest != active.digest)
        {
            return Err(Fault::Rejected(
                "collector activation changed during verification",
            ));
        }
        Ok(Self {
            authority_store: authority_store.to_owned(),
            authenticated,
            active_digest: Some(active.digest.clone()),
        })
    }

    /// Authenticate supplied bytes without asserting operator activation.
    pub fn load(
        authority_store: &Path,
        repository: &str,
        enrollment_path: &Path,
        verifier: &OpenSshSignatureCheck,
    ) -> Result<Self, Fault> {
        let signed = Signed::decode(&enrollment_bytes(enrollment_path)?)?;
        let current = current_for_execution(authority_store)?;
        let authenticated = signed.authenticate(&current, repository, verifier)?;
        // Do not return a configuration revoked while its signature was checked.
        let latest = current_for_execution(authority_store)?;
        if latest
            .digest()
            .map_err(|_| Fault::Rejected("authority digest"))?
            != current
                .digest()
                .map_err(|_| Fault::Rejected("authority digest"))?
        {
            return Err(Fault::Rejected(
                "authority changed during enrollment verification",
            ));
        }
        Ok(Self {
            authority_store: authority_store.to_owned(),
            authenticated,
            active_digest: None,
        })
    }
    pub fn enrollment(&self) -> &Enrollment {
        self.authenticated.enrollment()
    }
    /// Require an observed operator activation, then check fresh authority and
    /// current selection. Authentication-only loading cannot satisfy this path.
    pub fn allows_active(&self, input: Use<'_>) -> Result<(), Fault> {
        if self.active_digest.is_none() {
            return Err(Fault::Unavailable("collector activation not observed"));
        }
        self.allows(input)
    }

    /// Fresh authority is required at every use. This is not atomic launch fencing.
    pub fn allows(&self, input: Use<'_>) -> Result<(), Fault> {
        let snapshot = snapshot_for_execution(&self.authority_store)?;
        if let Some(digest) = &self.active_digest
            && snapshot
                .active_collectors
                .get(&self.enrollment().collector)
                .is_none_or(|active| active.digest != *digest)
        {
            return Err(Fault::Rejected("collector activation changed"));
        }
        self.authenticated.allows(&snapshot.revision, &input)
    }
}
