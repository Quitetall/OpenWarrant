// SPDX-License-Identifier: Apache-2.0
//! Operator-owned reference store. The execution account must not own this tree.
use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema: String,
    agent_uid: Option<u32>,
    unprotected_test_store: bool,
    /// Public authority metadata may be read by the execution account. Never
    /// gives that account write access or changes who may activate authority.
    #[serde(default, skip_serializing_if = "is_false")]
    execution_readable: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    collector_enrollments: BTreeMap<String, CollectorActivation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    active_collectors: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    collector_activation_events: Vec<CollectorActivationEvent>,
    genesis: Revision,
    legacy: BTreeMap<String, Vec<u8>>,
    transitions: Vec<Signed>,
    // Absent on older snapshots. Never invent observation times for old acts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    activation_receipts: BTreeMap<u64, ActivationReceipt>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectorActivation {
    signed: openwarrant_core::runtime_collector::Signed,
    operator_uid: Option<u32>,
    observed_at_unix_seconds: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectorActivationEvent {
    collector: String,
    digest: String,
    authority_head: String,
    operator_uid: Option<u32>,
    observed_at_unix_seconds: u64,
}
#[derive(Clone)]
pub(crate) struct ActiveCollector {
    pub digest: String,
    pub signed: openwarrant_core::runtime_collector::Signed,
}
fn collector_digest(signed: &openwarrant_core::runtime_collector::Signed) -> Result<String> {
    Ok(format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(&signed.encode().map_err(err)?)
    ))
}
fn is_false(value: &bool) -> bool {
    !value
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationReceipt {
    transition_digest: String,
    previous_head: String,
    new_head: String,
    authenticated_signers: Vec<String>,
    operator_uid: Option<u32>,
    observed_at_unix_seconds: u64,
}
#[cfg(unix)]
fn operator_uid() -> Option<u32> {
    Some(rustix::process::geteuid().as_raw())
}
#[cfg(not(unix))]
fn operator_uid() -> Option<u32> {
    None
}

impl State {
    fn current(&self) -> &Revision {
        self.transitions
            .last()
            .map_or(&self.genesis, |t| &t.proposal.next)
    }
    fn validate(&self) -> Result<()> {
        if self.schema != "oh.war/authority-store/1" || self.transitions.len() > 4096 {
            return Err(err("authority-store-format"));
        }
        if self.collector_enrollments.len() > 256
            || self.active_collectors.len() > 64
            || self.collector_activation_events.len() > 1024
        {
            return Err(err("collector-activation-history-limit"));
        }
        for (digest, record) in &self.collector_enrollments {
            if collector_digest(&record.signed)? != *digest {
                return Err(err("collector-activation-content-digest"));
            }
        }
        for (collector, digest) in &self.active_collectors {
            if self
                .collector_enrollments
                .get(digest)
                .is_none_or(|record| record.signed.enrollment.collector != *collector)
            {
                return Err(err("collector-activation-selection"));
            }
        }
        let mut selected = BTreeMap::new();
        for event in &self.collector_activation_events {
            if self
                .collector_enrollments
                .get(&event.digest)
                .is_none_or(|record| {
                    record.signed.enrollment.collector != event.collector
                        || record.signed.enrollment.authority_digest != event.authority_head
                })
            {
                return Err(err("collector-activation-event-binding"));
            }
            selected.insert(&event.collector, &event.digest);
        }
        for (collector, digest) in selected {
            if self.active_collectors.get(collector) != Some(digest) {
                return Err(err("collector-activation-event-selection"));
            }
        }
        self.genesis.validate().map_err(err)?;
        if self.genesis.sequence != 0 {
            return Err(err("authority-bootstrap-sequence"));
        }
        let mut previous = &self.genesis;
        for t in &self.transitions {
            signing::verify(previous, t)?;
            previous = &t.proposal.next;
        }
        for (sequence, receipt) in &self.activation_receipts {
            let index = sequence
                .checked_sub(1)
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| err("authority-receipt-sequence"))?;
            let transition = self
                .transitions
                .get(index)
                .ok_or_else(|| err("authority-receipt-sequence"))?;
            if receipt.transition_digest != transition.proposal.digest().map_err(err)?
                || receipt.previous_head != transition.proposal.previous_digest
                || receipt.new_head != transition.proposal.next.digest().map_err(err)?
                || receipt.authenticated_signers
                    != transition.signatures.keys().cloned().collect::<Vec<_>>()
            {
                return Err(err("authority-receipt-subject"));
            }
        }
        Ok(())
    }
    fn view(&self) -> Result<serde_json::Value> {
        Ok(
            serde_json::json!({"current":self.current(),"head":self.current().digest().map_err(err)?,"transitions":self.transitions.len(),"legacy_files":self.legacy.keys().collect::<Vec<_>>(),"isolation_enforced":false,"storage_boundary":if self.unprotected_test_store{"unprotected-test"}else{"separate-account-required"},"configured_agent_uid":self.agent_uid,"execution_readable":self.execution_readable,"configured_collectors":self.active_collectors,"collector_activation_history":self.collector_enrollments.len(),"collector_activation_events":self.collector_activation_events.len(),"human_review_established":false,"activation_receipts":self.activation_receipts,"missing_activation_receipts":self.transitions.len()-self.activation_receipts.len(),"activation_time_authenticated":false}),
        )
    }
}
#[cfg(unix)]
fn guard(root: &Path, agent: Option<u32>, test: bool, execution_readable: bool) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let uid = rustix::process::geteuid().as_raw();
    if !root.is_absolute()
        || root
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(err("authority-store-absolute-path-required"));
    }
    if !test && (agent.is_none() || agent == Some(uid)) {
        return Err(err("authority-store-separate-account-required"));
    }
    for path in root.ancestors() {
        let m = fs::symlink_metadata(path).map_err(err)?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(err("authority-store-directory-required"));
        }
        if !test
            && ((m.mode() & 0o022) != 0
                || (m.uid() != 0 && m.uid() != uid)
                || agent == Some(m.uid()))
        {
            return Err(err("authority-store-unsafe-owner-or-mode"));
        }
    }
    let m = fs::symlink_metadata(root).map_err(err)?;
    if !test && (m.uid() != uid || (!execution_readable && (m.mode() & 0o077) != 0)) {
        return Err(err("authority-store-private-owner-required"));
    }
    Ok(())
}
#[cfg(not(unix))]
fn guard(_: &Path, _: Option<u32>, _: bool, _: bool) -> Result<()> {
    Err(err("authority-store-platform-unsupported"))
}
#[cfg(unix)]
struct Lock(fs::File);
#[cfg(unix)]
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = rustix::fs::flock(&self.0, rustix::fs::FlockOperation::Unlock);
    }
}
#[cfg(unix)]
fn lock(root: &Path) -> Result<Lock> {
    use std::os::unix::fs::OpenOptionsExt;
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(root.join("lock"))
        .map_err(err)?;
    if !f.metadata().map_err(err)?.is_file() {
        return Err(err("authority-lock-not-regular"));
    }
    rustix::fs::flock(&f, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| err("authority-store-busy"))?;
    Ok(Lock(f))
}
#[cfg(not(unix))]
fn lock(_: &Path) -> Result<()> {
    Err(err("authority-store-platform-unsupported"))
}
fn load(root: &Path, test: bool) -> Result<State> {
    #[cfg(unix)]
    if !test {
        use std::os::unix::fs::MetadataExt;
        let m = fs::symlink_metadata(root.join("state.json")).map_err(err)?;
        if !m.is_file()
            || m.file_type().is_symlink()
            || m.uid() != rustix::process::geteuid().as_raw()
            || m.mode() & 0o022 != 0
        {
            return Err(err("authority-store-state-unsafe-owner-or-mode"));
        }
    }
    let bytes = read(&root.join("state.json"))?;
    let state: State = serde_json::from_slice(&bytes).map_err(err)?;
    if serde_jcs::to_vec(&state).map_err(err)? != bytes {
        return Err(err("authority-store-noncanonical"));
    }
    if state.unprotected_test_store != test {
        return Err(err("authority-store-mode-mismatch"));
    }
    guard(root, state.agent_uid, test, state.execution_readable)?;
    state.validate()?;
    Ok(state)
}
fn persist(root: &Path, state: &State) -> Result<()> {
    let bytes = serde_jcs::to_vec(state).map_err(err)?;
    if bytes.len() > LIMIT {
        return Err(err("authority-store-full"));
    }
    let temp = root.join(format!(
        "pending-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(err)?
            .as_nanos()
    ));
    write_new(&temp, &bytes)?;
    #[cfg(unix)]
    if state.execution_readable {
        use std::os::unix::fs::PermissionsExt;
        // Only public authority metadata is shared. The lock and signing keys
        // are not published. Readers still cannot write the directory or file.
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o644)).map_err(err)?;
        fs::File::open(&temp)
            .map_err(err)?
            .sync_all()
            .map_err(err)?;
    }
    if let Err(e) = fs::rename(&temp, root.join("state.json")) {
        let _ = fs::remove_file(&temp);
        return Err(err(e));
    }
    // One snapshot contains both accepted history and current revision. After a
    // crash, old or new complete snapshot survives, never a split head/history.
    fs::File::open(root).map_err(err)?.sync_all().map_err(err)
}
pub(super) fn bootstrap(
    root: &Path,
    genesis: Revision,
    expected: &str,
    agent: Option<u32>,
    legacy: Option<&Path>,
    test: bool,
    execution_readable: bool,
) -> Result<serde_json::Value> {
    if execution_readable && test {
        return Err(err("authority-read-sharing-requires-protected-mode"));
    }
    guard(root, agent, test, execution_readable)?;
    let _lock = lock(root)?;
    if root.join("state.json").symlink_metadata().is_ok() {
        return Err(err("authority-already-bootstrapped"));
    }
    genesis.validate().map_err(err)?;
    if genesis.sequence != 0 || genesis.digest().map_err(err)? != expected {
        return Err(err("authority-bootstrap-digest-or-sequence"));
    }
    let mut retained = BTreeMap::new();
    if let Some(path) = legacy {
        for name in ["roles.toml", "allowed_signers"] {
            retained.insert(name.into(), read(&path.join(name))?);
        }
    }
    let state = State {
        schema: "oh.war/authority-store/1".into(),
        agent_uid: agent,
        unprotected_test_store: test,
        execution_readable,
        collector_enrollments: BTreeMap::new(),
        active_collectors: BTreeMap::new(),
        collector_activation_events: Vec::new(),
        genesis,
        legacy: retained,
        transitions: Vec::new(),
        activation_receipts: BTreeMap::new(),
    };
    persist(root, &state)?;
    #[cfg(unix)]
    if execution_readable {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o755)).map_err(err)?;
        fs::File::open(root).map_err(err)?.sync_all().map_err(err)?;
    }
    state.view()
}
pub(super) fn activate(root: &Path, record: Signed, test: bool) -> Result<serde_json::Value> {
    // Load and guard before opening the lock, then reload under the lock.
    let _ = load(root, test)?;
    let _lock = lock(root)?;
    let mut state = load(root, test)?;
    signing::verify_activation(state.current(), &record)?;
    if state.transitions.len() >= 4096 {
        return Err(err("authority-store-full"));
    }
    let receipt = ActivationReceipt {
        transition_digest: record.proposal.digest().map_err(err)?,
        previous_head: record.proposal.previous_digest.clone(),
        new_head: record.proposal.next.digest().map_err(err)?,
        authenticated_signers: record.signatures.keys().cloned().collect(),
        operator_uid: operator_uid(),
        observed_at_unix_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(err)?
            .as_secs(),
    };
    state
        .activation_receipts
        .insert(record.proposal.next.sequence, receipt);
    state.transitions.push(record);
    persist(root, &state)?;
    state.view()
}
/// Activation is an operator-owned configuration action, never a human review record.
pub(super) fn activate_collector(
    root: &Path,
    signed: openwarrant_core::runtime_collector::Signed,
) -> Result<serde_json::Value> {
    let _ = load(root, false)?;
    let _lock = lock(root)?;
    let mut state = load(root, false)?;
    let verifier = crate::runtime_capture::collector_signature::OpenSshSignatureCheck::new(
        root.to_owned(),
        std::time::Duration::from_secs(5),
    )
    .map_err(err)?;
    signed
        .authenticate(state.current(), &state.current().repository, &verifier)
        .map_err(|e| match e {
            openwarrant_core::runtime_collector::Fault::Unavailable(_) => {
                RepoError::ObservationUnavailable {
                    rule: "runtime.collector-activation-unavailable",
                    message: e.to_string(),
                }
            }
            _ => err(e),
        })?;
    let digest = collector_digest(&signed)?;
    let collector = signed.enrollment.collector.clone();
    let replay = state.active_collectors.get(&collector) == Some(&digest);
    if !replay {
        state
            .collector_enrollments
            .entry(digest.clone())
            .or_insert(CollectorActivation {
                signed,
                operator_uid: operator_uid(),
                observed_at_unix_seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(err)?
                    .as_secs(),
            });
        state
            .active_collectors
            .insert(collector.clone(), digest.clone());
        state
            .collector_activation_events
            .push(CollectorActivationEvent {
                collector: collector.clone(),
                digest: digest.clone(),
                authority_head: state.current().digest().map_err(err)?,
                operator_uid: operator_uid(),
                observed_at_unix_seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(err)?
                    .as_secs(),
            });
        state.validate()?;
        persist(root, &state)?;
    }
    Ok(
        serde_json::json!({"collector":collector,"activation_digest":digest,"replay":replay,
        "configured":true,"human_review_established":false,"caller_authenticated":false,
        "launch_fenced":false,"activation_time_authenticated":false}),
    )
}

pub(super) fn status(root: &Path, test: bool) -> Result<serde_json::Value> {
    load(root, test)?.view()
}

/// The store as the performer's account may read it (OW-WAR-0138): its
/// current revision and head.
pub(crate) struct Current {
    pub revision: Revision,
    pub head: String,
    pub test_mode: bool,
    /// Store configuration, not a proof of the caller's operating-system UID.
    pub agent_uid: Option<u32>,
    pub active_collectors: BTreeMap<String, ActiveCollector>,
}

/// Read a store without writing to it, from any account, for `war check` and
/// `war sign` (OW-WAR-0138 D-002).
///
/// [`load`]'s guard asks "does the operator running this own the store
/// privately", which the execution account never does. This asks the
/// opposite question: can the execution account write any of it. Outside test
/// mode the store must name that account (`agent_uid`), and the store, every
/// ancestor and `state.json` must be owned by someone else and writable by no
/// group or other. In test mode the account that reads it also owns it, so
/// what is checked is only that this process cannot write the store or
/// `state.json` now; that is a simulation of the boundary, labeled as such
/// everywhere it is reported. Every transition is verified as on any load,
/// and no lock is taken: a snapshot is replaced by rename, never in place.
pub(crate) fn read_current(root: &Path, test: bool) -> Result<Current> {
    if !root.is_absolute()
        || root
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(err("authority-store-absolute-path-required"));
    }
    let state_path = root.join("state.json");
    let bytes = read(&state_path)?;
    let state: State = serde_json::from_slice(&bytes).map_err(err)?;
    if serde_jcs::to_vec(&state).map_err(err)? != bytes {
        return Err(err("authority-store-noncanonical"));
    }
    if state.unprotected_test_store != test {
        return Err(err("authority-store-mode-mismatch"));
    }
    reader_guard(root, &state_path, state.agent_uid, test)?;
    // Verifying every transition asks ssh-keygen once per signature; one
    // process asks once per distinct snapshot.
    let key = format!(
        "{}\0{test}\0{}",
        root.display(),
        openwarrant_compiler::sha256_hex(&bytes)
    );
    let cache = VALIDATED.get_or_init(Default::default);
    let known = cache.lock().ok().is_some_and(|c| c.contains(&key));
    if !known {
        state.validate()?;
        if let Ok(mut c) = cache.lock() {
            c.insert(key);
        }
    }
    let revision = state.current().clone();
    Ok(Current {
        head: revision.digest().map_err(err)?,
        revision,
        test_mode: test,
        agent_uid: state.agent_uid,
        active_collectors: state
            .active_collectors
            .iter()
            .map(|(name, digest)| {
                let signed = state.collector_enrollments[digest].signed.clone();
                (
                    name.clone(),
                    ActiveCollector {
                        digest: digest.clone(),
                        signed,
                    },
                )
            })
            .collect(),
    })
}

/// Snapshots [`read_current`] has validated in this process, by path, mode
/// and the digest of every byte.
static VALIDATED: std::sync::OnceLock<std::sync::Mutex<std::collections::BTreeSet<String>>> =
    std::sync::OnceLock::new();

/// Neither owned by the execution account nor writable by any group or other.
#[cfg_attr(not(unix), allow(dead_code))]
const fn reader_safe(owner: u32, mode: u32, agent: u32) -> bool {
    owner != agent && (mode & 0o022) == 0
}

#[cfg(unix)]
pub(crate) fn effective_write_access(path: &Path) -> Result<bool> {
    match rustix::fs::accessat(
        rustix::fs::CWD,
        path,
        rustix::fs::Access::WRITE_OK,
        rustix::fs::AtFlags::EACCESS,
    ) {
        Ok(()) => Ok(true),
        Err(rustix::io::Errno::ACCESS | rustix::io::Errno::ROFS) => Ok(false),
        Err(error) => Err(RepoError::ObservationUnavailable {
            rule: "authority-store-effective-write-access-unavailable",
            message: format!("effective authority write access unavailable: {error}"),
        }),
    }
}

/// Mode and EACCESS checks cannot establish protection against a privileged
/// reader that can change permissions or identity. Normal Linux execution
/// readers currently require empty effective/permitted capability sets.
/// Operator-owner inspection is separate; unavailable observations fail closed.
pub(crate) fn unprivileged_reader() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let caps = rustix::thread::capabilities(None)
            .map_err(|_| err("authority-store-reader-privileges-unavailable"))?;
        if !caps.effective.is_empty() || !caps.permitted.is_empty() {
            return Err(err("authority-store-reader-privileges-unqualified"));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn reader_guard(root: &Path, state: &Path, agent: Option<u32>, test: bool) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    if test {
        for path in [root, state] {
            if effective_write_access(path)? {
                return Err(err(
                    "authority-store-writable-by-reader: this process can write the test store",
                ));
            }
        }
        return Ok(());
    }
    let Some(agent) = agent else {
        return Err(err("authority-store-separate-account-required"));
    };
    // An owner may inspect their operator store. For every other reader,
    // actual effective permissions must be checked independently of unsigned
    // agent_uid: a capable process must not reassign it to bypass this guard.
    let reader_owns_root =
        fs::symlink_metadata(root).map_err(err)?.uid() == rustix::process::geteuid().as_raw();
    for path in root.ancestors().chain(std::iter::once(state)) {
        let m = fs::symlink_metadata(path).map_err(err)?;
        if m.file_type().is_symlink() {
            return Err(err("authority-store-directory-required"));
        }
        if !reader_safe(m.uid(), m.mode(), agent) {
            return Err(err(
                "authority-store-writable-by-agent: the execution account owns, or a group or \
                 other can write, the store or an ancestor",
            ));
        }
        if !reader_owns_root && effective_write_access(path)? {
            return Err(err("authority-store-writable-by-reader"));
        }
    }
    if !reader_owns_root {
        unprivileged_reader()?;
    }
    Ok(())
}
#[cfg(not(unix))]
fn reader_guard(_: &Path, _: &Path, _: Option<u32>, _: bool) -> Result<()> {
    Err(err("authority-store-platform-unsupported"))
}

pub(super) fn history(root: &Path, emit: &Path, test: bool) -> Result<serde_json::Value> {
    let state = load(root, test)?;
    let bytes = serde_jcs::to_vec(&state).map_err(err)?;
    write_new(emit, &bytes)?;
    Ok(
        serde_json::json!({"path":emit,"head":state.current().digest().map_err(err)?,"effective":false,"trust_transferred":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::reader_safe;

    /// Unit coverage for owner/mode rules; the collector namespace fixture
    /// separately exercises actual account and capability boundaries.
    #[test]
    fn a_store_the_agent_owns_or_a_group_can_write_is_refused() {
        let (operator, agent) = (1001, 1000);
        assert!(reader_safe(operator, 0o40700, agent));
        assert!(reader_safe(0, 0o40755, agent));
        assert!(!reader_safe(agent, 0o40700, agent), "owned by the agent");
        assert!(!reader_safe(operator, 0o40770, agent), "group-writable");
        assert!(!reader_safe(operator, 0o40702, agent), "other-writable");
        assert!(!reader_safe(0, 0o41777, agent), "/tmp-like");
    }

    #[cfg(unix)]
    #[test]
    fn missing_effective_access_observations_are_unknown() {
        use super::*;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".authority-access-unknown-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        assert!(effective_write_access(&root).unwrap());
        assert!(matches!(
            effective_write_access(&root.join("missing")),
            Err(RepoError::ObservationUnavailable { .. })
        ));
        fs::remove_dir(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sharing_is_explicit_and_never_allows_other_account_writes() {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        // A normal authority store must not have a world-writable /tmp ancestor.
        // This disposable test stays in the isolated checkout and is removed.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".authority-sharing-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let genesis = Revision {
            schema: sdk::REVISION_SCHEMA.into(), repository: "fixture".into(), sequence: 0,
            principals: BTreeMap::from([("owner".into(), sdk::Principal {
                public_key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
                roles: std::collections::BTreeSet::from(["authority-admin".into()]), actor: None, kind: None,
            })]), policy: None,
        };
        let agent = rustix::process::geteuid().as_raw().checked_add(1).unwrap();
        assert!(
            bootstrap(
                &root,
                genesis.clone(),
                &genesis.digest().unwrap(),
                Some(agent),
                None,
                true,
                true
            )
            .unwrap_err()
            .to_string()
            .contains("requires-protected-mode")
        );
        let view = bootstrap(
            &root,
            genesis.clone(),
            &genesis.digest().unwrap(),
            Some(agent),
            None,
            false,
            true,
        )
        .unwrap();
        assert_eq!(view["execution_readable"], true);
        assert_eq!(view["human_review_established"], false);
        let state = root.join("state.json");
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(&state).unwrap().permissions().mode() & 0o777,
            0o644
        );
        status(&root, false).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(
            status(&root, false)
                .unwrap_err()
                .to_string()
                .contains("state-unsafe-owner-or-mode")
        );
        fs::set_permissions(&state, fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(
            status(&root, false)
                .unwrap_err()
                .to_string()
                .contains("unsafe-owner-or-mode")
        );
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_file(&state).unwrap();
        bootstrap(
            &root,
            genesis.clone(),
            &genesis.digest().unwrap(),
            Some(agent),
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&state).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(
            !fs::read_to_string(&state)
                .unwrap()
                .contains("execution_readable")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
