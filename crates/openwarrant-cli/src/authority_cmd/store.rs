// SPDX-License-Identifier: Apache-2.0
//! Operator-owned reference store. The execution account must not own this tree.
use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema: String,
    agent_uid: Option<u32>,
    unprotected_test_store: bool,
    genesis: Revision,
    legacy: BTreeMap<String, Vec<u8>>,
    transitions: Vec<Signed>,
    // Absent on older snapshots. Never invent observation times for old acts.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    activation_receipts: BTreeMap<u64, ActivationReceipt>,
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
            serde_json::json!({"current":self.current(),"head":self.current().digest().map_err(err)?,"transitions":self.transitions.len(),"legacy_files":self.legacy.keys().collect::<Vec<_>>(),"isolation_enforced":false,"storage_boundary":if self.unprotected_test_store{"unprotected-test"}else{"separate-account-required"},"configured_agent_uid":self.agent_uid,"human_review_established":false,"activation_receipts":self.activation_receipts,"missing_activation_receipts":self.transitions.len()-self.activation_receipts.len(),"activation_time_authenticated":false}),
        )
    }
}
#[cfg(unix)]
fn guard(root: &Path, agent: Option<u32>, test: bool) -> Result<()> {
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
    if !test && (m.uid() != uid || (m.mode() & 0o077) != 0) {
        return Err(err("authority-store-private-owner-required"));
    }
    Ok(())
}
#[cfg(not(unix))]
fn guard(_: &Path, _: Option<u32>, _: bool) -> Result<()> {
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
    let bytes = read(&root.join("state.json"))?;
    let state: State = serde_json::from_slice(&bytes).map_err(err)?;
    if serde_jcs::to_vec(&state).map_err(err)? != bytes {
        return Err(err("authority-store-noncanonical"));
    }
    if state.unprotected_test_store != test {
        return Err(err("authority-store-mode-mismatch"));
    }
    guard(root, state.agent_uid, test)?;
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
) -> Result<serde_json::Value> {
    guard(root, agent, test)?;
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
        genesis,
        legacy: retained,
        transitions: Vec::new(),
        activation_receipts: BTreeMap::new(),
    };
    persist(root, &state)?;
    state.view()
}
pub(super) fn activate(root: &Path, record: Signed, test: bool) -> Result<serde_json::Value> {
    // Load and guard before opening the lock, then reload under the lock.
    let _ = load(root, test)?;
    let _lock = lock(root)?;
    let mut state = load(root, test)?;
    signing::verify(state.current(), &record)?;
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
pub(super) fn status(root: &Path, test: bool) -> Result<serde_json::Value> {
    load(root, test)?.view()
}

pub(super) fn history(root: &Path, emit: &Path, test: bool) -> Result<serde_json::Value> {
    let state = load(root, test)?;
    let bytes = serde_jcs::to_vec(&state).map_err(err)?;
    write_new(emit, &bytes)?;
    Ok(
        serde_json::json!({"path":emit,"head":state.current().digest().map_err(err)?,"effective":false,"trust_transferred":false}),
    )
}
