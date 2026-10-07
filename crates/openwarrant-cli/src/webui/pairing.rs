// SPDX-License-Identifier: Apache-2.0
//! Devices for `war view ui --lan` (OW-WAR-0139): pairing, credentials, nonces.
//!
//! A device credential is a session in OW-WAR-0138's sense. It is never an
//! authority to sign: a paired device reads the program, runs `auto`
//! remedies and can ask for a signature at the host (U-001 option A).
//!
//! - **Pairing.** The host prints a link carrying a single-use code (and the
//!   certificate's fingerprint, for the human to compare). A code works once
//!   and for five minutes. Presenting it asks the human at the host's
//!   terminal — naming the device's address and user agent — and only a `y`
//!   typed there issues a credential. No answer, `n`, or no terminal at all
//!   issues none, each refused by name.
//! - **Credential.** 256 bits from the OS, sent once as a `Secure; HttpOnly;
//!   SameSite=Strict` cookie. The server keeps only its SHA-256, with the
//!   device's address, user agent and expiry, in the per-user state
//!   directory (`$XDG_STATE_HOME/openwarrant/ui-devices.json`, mode 0600) —
//!   never under a repository. It is read on every request, so `war view ui
//!   devices --revoke <id>` in another terminal takes effect at once.
//! - **Nonces.** Every act a device starts carries a nonce the server issued
//!   to that device; a nonce is spent on first use, and a second use is
//!   refused.
//!
//! Local state, not a record: nothing here is committed, and deleting the
//! file revokes every device.

use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, Write};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

/// The cookie that carries a device credential. `__Host-` makes the browser
/// refuse it unless it is `Secure`, has `Path=/` and names no domain.
pub const COOKIE: &str = "__Host-war_device";

/// How long a pairing code lives.
pub const CODE_TTL: Duration = Duration::from_secs(300);
/// How long the host's human has to answer.
pub const ANSWER_TTL: Duration = Duration::from_secs(60);
/// How long a credential lives.
pub const DEVICE_TTL: Duration = Duration::from_secs(30 * 86_400);
/// How long an issued nonce stays spendable, and how many a device may hold.
const NONCE_TTL: Duration = Duration::from_secs(300);
const NONCES_PER_DEVICE: usize = 16;

fn sha(s: &str) -> String {
    openwarrant_compiler::sha256_hex(s.as_bytes())
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn rfc3339(unix: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(unix, 0)
        .map_or_else(String::new, |t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// `$XDG_STATE_HOME/openwarrant`, else `~/.local/state/openwarrant`.
pub fn state_dir() -> Result<Utf8PathBuf, String> {
    let base = std::env::var("XDG_STATE_HOME")
        .ok()
        .filter(|v| v.starts_with('/'))
        .map(Utf8PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| Utf8PathBuf::from(h).join(".local/state"))
        })
        .ok_or("ui.devices-no-state: neither XDG_STATE_HOME nor HOME is set")?;
    Ok(base.join("openwarrant"))
}

/// Write `bytes` to `path` as a private file: created 0600 in a 0700
/// directory, fsynced, renamed into place.
#[cfg(unix)]
pub fn write_private(path: &Utf8Path, bytes: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let dir = path
        .parent()
        .ok_or("ui.devices-path: no parent directory")?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|e| format!("ui.devices-write: {dir}: {e}"))?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or("state"),
        std::process::id()
    ));
    let _ = std::fs::remove_file(&tmp);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| format!("ui.devices-write: {tmp}: {e}"))?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| format!("ui.devices-write: {tmp}: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("ui.devices-write: {path}: {e}"))
}

#[cfg(not(unix))]
pub fn write_private(_path: &Utf8Path, _bytes: &[u8]) -> Result<(), String> {
    Err("ui.lan-unsupported: device state needs a Unix file mode".to_owned())
}

/// One paired device. `credential_sha256` is all that is kept of the
/// credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    /// The repository root the server was serving.
    pub root: String,
    pub credential_sha256: String,
    pub address: String,
    pub user_agent: String,
    pub paired_at: i64,
    pub expires_at: i64,
    #[serde(default)]
    pub revoked_at: Option<i64>,
}

impl Device {
    /// `active`, `revoked` or `expired`, now.
    #[must_use]
    pub fn state(&self) -> &'static str {
        if self.revoked_at.is_some() {
            "revoked"
        } else if now_unix() >= self.expires_at {
            "expired"
        } else {
            "active"
        }
    }

    /// The row `war view ui devices` prints and reports.
    #[must_use]
    pub fn row(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id, "state": self.state(), "address": self.address,
            "user_agent": self.user_agent, "paired_at": rfc3339(self.paired_at),
            "expires_at": rfc3339(self.expires_at),
            "revoked_at": self.revoked_at.map(rfc3339),
        })
    }

    /// How a request from this device is named on the host.
    #[must_use]
    pub fn label(&self) -> String {
        format!("device {} ({})", self.id, self.address)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DeviceFile {
    schema: String,
    devices: Vec<Device>,
}

const FILE_SCHEMA: &str = "oh.war/ui-devices/v1 (local state, not a record)";

/// Why a request's credential was not accepted; each is a 401.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthRefusal {
    Missing,
    Unknown,
    Revoked,
    Expired,
    Unreadable,
}

impl AuthRefusal {
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::Missing => {
                "ui.device-required: this device is not paired; open a pairing link from the host"
            }
            Self::Unknown => "ui.device-unknown: no paired device holds this credential",
            Self::Revoked => "ui.device-revoked: this device's pairing was revoked at the host",
            Self::Expired => "ui.device-expired: this device's pairing expired; pair it again",
            Self::Unreadable => "ui.device-state: the host could not read its device file",
        }
    }
}

/// The device file for one repository's server.
pub struct Store {
    pub path: Utf8PathBuf,
    root: String,
}

impl Store {
    /// The per-user device file, refused when it would sit under `root`.
    pub fn open(root: &Utf8Path) -> Result<Self, String> {
        let path = state_dir()?.join("ui-devices.json");
        Self::at(path, root)
    }

    fn at(path: Utf8PathBuf, root: &Utf8Path) -> Result<Self, String> {
        let canon = |p: &Utf8Path| -> Utf8PathBuf {
            // The deepest existing ancestor, canonicalized, so a symlinked
            // state directory into the repository is seen for what it is.
            let mut cur = p.to_path_buf();
            let mut tail = Vec::new();
            loop {
                if let Ok(c) = cur.canonicalize_utf8() {
                    let mut c = c;
                    for t in tail.iter().rev() {
                        c.push(t);
                    }
                    return c;
                }
                match (cur.parent(), cur.file_name()) {
                    (Some(parent), Some(name)) => {
                        tail.push(name.to_owned());
                        cur = parent.to_path_buf();
                    }
                    _ => return p.to_path_buf(),
                }
            }
        };
        let root_c = canon(root);
        if canon(&path).starts_with(&root_c) {
            return Err(format!(
                "ui.devices-in-repository: the device file {path} would be under the repository {root}; \
                 device state is per-user and never recorded — set XDG_STATE_HOME outside it"
            ));
        }
        Ok(Self {
            path,
            root: root_c.to_string(),
        })
    }

    fn read(&self) -> Result<DeviceFile, String> {
        match std::fs::read(&self.path) {
            Ok(bytes) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = std::fs::metadata(&self.path)
                        .map_err(|e| e.to_string())?
                        .permissions()
                        .mode();
                    if mode & 0o077 != 0 {
                        return Err(format!(
                            "ui.devices-mode: {} is mode {:o}; it must be 0600",
                            self.path,
                            mode & 0o777
                        ));
                    }
                }
                serde_json::from_slice(&bytes)
                    .map_err(|e| format!("ui.devices-unreadable: {}: {e}", self.path))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DeviceFile::default()),
            Err(e) => Err(format!("ui.devices-unreadable: {}: {e}", self.path)),
        }
    }

    /// Read, change and write the file under an exclusive lock, so a
    /// revocation in another process is never lost to a pairing here.
    #[cfg(unix)]
    fn update<R>(&self, f: impl FnOnce(&mut DeviceFile) -> Result<R, String>) -> Result<R, String> {
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        let dir = self.path.parent().ok_or("ui.devices-path: no parent")?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| format!("ui.devices-write: {dir}: {e}"))?;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(self.path.with_extension("lock"))
            .map_err(|e| format!("ui.devices-lock: {e}"))?;
        rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockExclusive)
            .map_err(|e| format!("ui.devices-lock: {e}"))?;
        let mut file = self.read()?;
        let r = f(&mut file)?;
        file.schema = FILE_SCHEMA.to_owned();
        let bytes = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
        write_private(&self.path, &bytes)?;
        Ok(r)
    }

    #[cfg(not(unix))]
    fn update<R>(
        &self,
        _f: impl FnOnce(&mut DeviceFile) -> Result<R, String>,
    ) -> Result<R, String> {
        Err("ui.lan-unsupported: device state needs a Unix file mode".to_owned())
    }

    /// Record a new device and return it with its credential — the only
    /// time the credential exists outside the device.
    pub fn issue(
        &self,
        address: &str,
        user_agent: &str,
        ttl: Duration,
    ) -> Result<(Device, String), String> {
        let credential = super::token().map_err(|e| e.to_string())?;
        let id = super::token().map_err(|e| e.to_string())?[..8].to_owned();
        let now = now_unix();
        let device = Device {
            id,
            root: self.root.clone(),
            credential_sha256: sha(&credential),
            address: address.to_owned(),
            user_agent: user_agent.to_owned(),
            paired_at: now,
            expires_at: now + ttl.as_secs() as i64,
            revoked_at: None,
        };
        let d = device.clone();
        self.update(move |f| {
            f.devices.push(d);
            Ok(())
        })?;
        Ok((device, credential))
    }

    /// The device holding `credential`, if it is active for this repository.
    pub fn authenticate(&self, credential: Option<&str>) -> Result<Device, AuthRefusal> {
        let Some(c) = credential.filter(|c| !c.is_empty()) else {
            return Err(AuthRefusal::Missing);
        };
        let h = sha(c);
        let file = self.read().map_err(|_| AuthRefusal::Unreadable)?;
        let d = file
            .devices
            .into_iter()
            .find(|d| d.root == self.root && super::same(&d.credential_sha256, &h))
            .ok_or(AuthRefusal::Unknown)?;
        match d.state() {
            "revoked" => Err(AuthRefusal::Revoked),
            "expired" => Err(AuthRefusal::Expired),
            _ => Ok(d),
        }
    }

    /// Every device paired for this repository, oldest first.
    pub fn list(&self) -> Result<Vec<Device>, String> {
        Ok(self
            .read()?
            .devices
            .into_iter()
            .filter(|d| d.root == self.root)
            .collect())
    }

    /// Revoke one device by id; revoking twice is refused by name.
    pub fn revoke(&self, id: &str) -> Result<Device, String> {
        let root = self.root.clone();
        self.update(|f| {
            let d = f
                .devices
                .iter_mut()
                .find(|d| d.root == root && d.id == id)
                .ok_or_else(|| {
                    format!("ui.device-unknown: no device {id} is paired for this repository")
                })?;
            if d.revoked_at.is_some() {
                return Err(format!("ui.device-revoked: device {id} is already revoked"));
            }
            d.revoked_at = Some(now_unix());
            Ok(d.clone())
        })
    }
}

/// Why a pairing code was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeRefusal {
    Unknown,
    Used,
    Expired,
}

impl CodeRefusal {
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::Unknown => "pair.code-unknown: this is not the host's pairing code",
            Self::Used => {
                "pair.code-used: this pairing code was already used; the host's terminal shows a new link"
            }
            Self::Expired => {
                "pair.code-expired: this pairing code expired; the host's terminal shows a new link"
            }
        }
    }
}

/// The outstanding code's hash and deadline, and every spent code's hash.
type CodeState = (Option<(String, Instant)>, HashSet<String>);

/// The single outstanding pairing code, and every code already spent.
pub struct Codes {
    inner: Mutex<CodeState>,
    ttl: Duration,
}

impl Codes {
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new((None, HashSet::new())),
            ttl,
        }
    }

    #[must_use]
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// A fresh code, replacing any outstanding one (which is then spent).
    pub fn fresh(&self) -> Result<String, String> {
        let code = super::token().map_err(|e| e.to_string())?[..32].to_owned();
        let mut g = self.inner.lock().map_err(|_| "pairing state poisoned")?;
        if let Some((old, _)) = g.0.take() {
            g.1.insert(old);
        }
        g.0 = Some((sha(&code), Instant::now() + self.ttl));
        Ok(code)
    }

    /// Spend `code`. Only the outstanding, unexpired code is accepted, once.
    pub fn take(&self, code: &str) -> Result<(), CodeRefusal> {
        let h = sha(code);
        let mut g = self.inner.lock().map_err(|_| CodeRefusal::Unknown)?;
        if g.1.contains(&h) {
            return Err(CodeRefusal::Used);
        }
        match g.0.take() {
            Some((cur, until)) if super::same(&cur, &h) => {
                g.1.insert(cur);
                if Instant::now() >= until {
                    Err(CodeRefusal::Expired)
                } else {
                    Ok(())
                }
            }
            other => {
                g.0 = other;
                Err(CodeRefusal::Unknown)
            }
        }
    }
}

/// What the human at the host said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Yes,
    No,
    TimedOut,
    NoTty,
}

/// The host's terminal (`/dev/tty`), where every pairing is confirmed. One
/// question at a time; lines typed before a question are discarded, so a
/// stale `y` never answers a later pairing.
#[derive(Default)]
pub struct HostTty {
    io: Mutex<Option<(std::fs::File, Receiver<String>)>>,
}

impl HostTty {
    fn open() -> Option<(std::fs::File, Receiver<String>)> {
        let tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .ok()?;
        let reader = tty.try_clone().ok()?;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(reader).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Some((tty, rx))
    }

    /// Ask `question` at the host terminal and wait up to `timeout`.
    pub fn ask(&self, question: &str, timeout: Duration) -> Answer {
        let Ok(mut g) = self.io.lock() else {
            return Answer::NoTty;
        };
        if g.is_none() {
            *g = Self::open();
        }
        let Some((tty, rx)) = g.as_mut() else {
            return Answer::NoTty;
        };
        while rx.try_recv().is_ok() {}
        if tty
            .write_all(question.as_bytes())
            .and_then(|()| tty.flush())
            .is_err()
        {
            *g = None;
            return Answer::NoTty;
        }
        match rx.recv_timeout(timeout) {
            Ok(line) => {
                let a = line.trim().to_ascii_lowercase();
                if a == "y" || a == "yes" {
                    Answer::Yes
                } else {
                    Answer::No
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = tty.write_all(b"\n  (no answer; not paired)\n");
                Answer::TimedOut
            }
            Err(RecvTimeoutError::Disconnected) => {
                *g = None;
                Answer::NoTty
            }
        }
    }
}

/// Why a nonce was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceRefusal {
    Used,
    NotIssued,
    Expired,
}

impl NonceRefusal {
    #[must_use]
    pub fn status(self) -> &'static str {
        match self {
            Self::Used => "409 Conflict",
            Self::NotIssued | Self::Expired => "403 Forbidden",
        }
    }

    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::Used => "act.nonce-used: this nonce was already spent; nothing ran",
            Self::NotIssued => {
                "act.nonce-unknown: no such nonce was issued to this device; nothing ran"
            }
            Self::Expired => "act.nonce-expired: this nonce expired; ask for a new one",
        }
    }
}

/// Issued nonces by hash → (device, when), and every spent nonce's hash.
type NonceState = (BTreeMap<String, (String, Instant)>, HashSet<String>);

/// Single-use act nonces, per device.
#[derive(Default)]
pub struct Nonces {
    inner: Mutex<NonceState>,
}

impl Nonces {
    /// A fresh nonce for `device`; its oldest is dropped past the bound.
    pub fn issue(&self, device: &str) -> Result<String, String> {
        let n = super::token().map_err(|e| e.to_string())?;
        let mut g = self.inner.lock().map_err(|_| "nonce state poisoned")?;
        let mut mine: Vec<(String, Instant)> =
            g.0.iter()
                .filter(|(_, (d, _))| d == device)
                .map(|(k, (_, t))| (k.clone(), *t))
                .collect();
        if mine.len() >= NONCES_PER_DEVICE {
            mine.sort_by_key(|(_, t)| *t);
            for (k, _) in mine.iter().take(mine.len() + 1 - NONCES_PER_DEVICE) {
                g.0.remove(k);
            }
        }
        g.0.insert(sha(&n), (device.to_owned(), Instant::now()));
        Ok(n)
    }

    /// Spend `nonce` for `device`: once, only if issued to it, only fresh.
    pub fn spend(&self, device: &str, nonce: &str) -> Result<(), NonceRefusal> {
        let h = sha(nonce);
        let mut g = self.inner.lock().map_err(|_| NonceRefusal::NotIssued)?;
        if g.1.contains(&h) {
            return Err(NonceRefusal::Used);
        }
        match g.0.get(&h) {
            Some((d, _)) if d == device => {}
            _ => return Err(NonceRefusal::NotIssued),
        }
        let (_, at) = g.0.remove(&h).ok_or(NonceRefusal::NotIssued)?;
        g.1.insert(h);
        if at.elapsed() > NONCE_TTL {
            return Err(NonceRefusal::Expired);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> (Utf8PathBuf, Utf8PathBuf) {
        let base = Utf8PathBuf::from(std::env::temp_dir().to_string_lossy().into_owned())
            .join(format!("war-pairing-{}", super::super::token().unwrap()));
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        (base, repo)
    }

    #[test]
    fn a_credential_is_stored_only_as_its_hash_in_a_private_file() {
        let (base, repo) = scratch();
        let store = Store::at(base.join("state/ui-devices.json"), &repo).unwrap();
        let (d, cred) = store.issue("192.0.2.7", "phone", DEVICE_TTL).unwrap();
        let text = std::fs::read_to_string(&store.path).unwrap();
        assert!(!text.contains(&cred));
        assert!(text.contains(&d.credential_sha256));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&store.path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(store.authenticate(Some(&cred)).unwrap().id, d.id);
        assert_eq!(store.authenticate(None), Err(AuthRefusal::Missing));
        assert_eq!(store.authenticate(Some("00")), Err(AuthRefusal::Unknown));
        store.revoke(&d.id).unwrap();
        assert_eq!(store.authenticate(Some(&cred)), Err(AuthRefusal::Revoked));
        assert!(store.revoke(&d.id).is_err());
        let (_, old) = store.issue("192.0.2.8", "old", Duration::ZERO).unwrap();
        assert_eq!(store.authenticate(Some(&old)), Err(AuthRefusal::Expired));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_device_file_under_the_repository_is_refused() {
        let (base, repo) = scratch();
        let e = Store::at(repo.join(".state/ui-devices.json"), &repo)
            .err()
            .unwrap();
        assert!(e.starts_with("ui.devices-in-repository"), "{e}");
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_pairing_code_works_once_and_not_after_expiry() {
        let codes = Codes::new(Duration::from_secs(60));
        let c = codes.fresh().unwrap();
        assert_eq!(codes.take("nope"), Err(CodeRefusal::Unknown));
        assert_eq!(codes.take(&c), Ok(()));
        assert_eq!(codes.take(&c), Err(CodeRefusal::Used));
        let short = Codes::new(Duration::ZERO);
        let c = short.fresh().unwrap();
        assert_eq!(short.take(&c), Err(CodeRefusal::Expired));
        assert_eq!(short.take(&c), Err(CodeRefusal::Used));
        // A fresh code retires the outstanding one.
        let codes = Codes::new(Duration::from_secs(60));
        let a = codes.fresh().unwrap();
        let b = codes.fresh().unwrap();
        assert_eq!(codes.take(&a), Err(CodeRefusal::Used));
        assert_eq!(codes.take(&b), Ok(()));
    }

    #[test]
    fn a_nonce_is_spent_once_and_only_by_its_device() {
        let n = Nonces::default();
        let a = n.issue("dev-a").unwrap();
        assert_eq!(n.spend("dev-b", &a), Err(NonceRefusal::NotIssued));
        assert_eq!(n.spend("dev-a", &a), Ok(()));
        assert_eq!(n.spend("dev-a", &a), Err(NonceRefusal::Used));
        assert_eq!(n.spend("dev-a", "never"), Err(NonceRefusal::NotIssued));
        let many: Vec<String> = (0..NONCES_PER_DEVICE + 2)
            .map(|_| n.issue("dev-a").unwrap())
            .collect();
        assert_eq!(n.spend("dev-a", &many[0]), Err(NonceRefusal::NotIssued));
        assert_eq!(n.spend("dev-a", many.last().unwrap()), Ok(()));
    }
}
