// SPDX-License-Identifier: Apache-2.0
//! `[resources]` (OW-ADR-0037): use the machine well.
//!
//! - `[resources.env]`: variables every `war`-made worktree gets; `{common}`
//!   is the clone's git common directory, so one build cache serves every
//!   worktree. `war start` writes them to the session's Claude settings
//!   `env` and to `.openwarrant/env` (`KEY=VALUE` lines a shell can source).
//! - `seed`: what `war admin seed` runs once in the main checkout, the env
//!   applied; the seeded revision is kept under the common directory, and
//!   `war start` says when it is stale. Nothing here builds on its own.
//! - `[resources.slots]`: how many may hold a named slot (default 1).
//!   `war evidence run <slot> -- CMD…` holds one while CMD runs: a lock file
//!   under the common directory, taken as a claim is (temp file, hard link).
//! - `max_worktrees`: `war start` refuses a managed worktree past it.
//! - `war admin cleanup`: [`crate::worktrees`]'s prune, plus the merged,
//!   unclaimed `war-go/*` branches nothing has checked out. Dry run first.
//!
//! What cannot be read is UNKNOWN: a slot whose holder cannot be read is
//! never taken, and a stale seed is never assumed fresh.

use std::collections::BTreeMap;
use std::io::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::go::git;
use crate::repo::{RepoError, Repository};
use crate::ticket::Outcome;
use crate::worktrees;

/// The env file `war start` writes in a worktree.
pub const ENV_FILE: &str = ".openwarrant/env";
/// `war start` past `max_worktrees`.
pub const LIMIT: &str = "resources.worktree-limit";
/// A slot whose holder cannot be read.
pub const SLOT_UNKNOWN: &str = "resources.slot-unknown";
const SLOT_SCHEMA: &str = "oh.war/slot/v1";
const SEED_SCHEMA: &str = "oh.war/seed/v1";
/// The slot the brief names.
pub const BUILD: &str = "build";

/// A command as configured: a string for `sh -c`, or an argv.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Cmd {
    Shell(String),
    Argv(Vec<String>),
}

impl Cmd {
    #[must_use]
    pub fn argv(&self) -> Vec<String> {
        match self {
            Self::Shell(s) => vec!["sh".into(), "-c".into(), s.clone()],
            Self::Argv(v) => v.clone(),
        }
    }
}

/// `[resources]` as written. Absent: [`Config::default`], not configured.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub seed: Option<Cmd>,
    #[serde(default)]
    pub slots: BTreeMap<String, u32>,
    #[serde(default)]
    pub max_worktrees: Option<u32>,
    /// `Nh` or `Nd` (default 24h; 0 disables): where a process probe is
    /// UNKNOWN, a worktree no file of which changed for this long is idle.
    #[serde(default)]
    pub idle_after: Option<String>,
    #[serde(skip)]
    pub configured: bool,
}

/// `[resources] idle_after` when absent.
pub const IDLE_AFTER: &str = "24h";

/// `Nh` or `Nd` in seconds; `0` (any unit) disables: `None`.
pub fn idle_seconds(text: &str) -> Result<Option<u64>, String> {
    let bad = || format!("[resources] idle_after {text:?}: Nh or Nd, or 0 to disable");
    if text == "0" {
        return Ok(None);
    }
    let (n, unit) = text.split_at(text.len().saturating_sub(1));
    let per = match unit {
        "h" => 3600,
        "d" => 86_400,
        _ => return Err(bad()),
    };
    let n: u64 = n
        .parse()
        .ok()
        .filter(|_| n.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(bad)?;
    Ok((n > 0).then(|| n.saturating_mul(per)))
}

impl Config {
    /// `idle_after` in seconds, or `None` when it is 0.
    pub fn idle_after(&self) -> Result<Option<u64>, String> {
        idle_seconds(self.idle_after.as_deref().unwrap_or(IDLE_AFTER))
    }

    /// How many may hold `slot` at once: configured, else 1.
    #[must_use]
    pub fn capacity(&self, slot: &str) -> u32 {
        self.slots.get(slot).copied().unwrap_or(1).max(1)
    }
}

fn config_text(root: &Utf8Path) -> Option<String> {
    crate::vfs::read_to_string(root.join(crate::init::CONFIG_FILE)).ok()
}

/// `[resources]` of `text`. `Err` names what is malformed.
pub fn parse(text: &str) -> Result<Config, String> {
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        resources: Option<Config>,
    }
    let file: File = toml::from_str(text).map_err(|e| format!("[resources]: {e}"))?;
    let Some(mut c) = file.resources else {
        return Ok(Config::default());
    };
    if let Some(k) = c.env.keys().find(|k| !env_name(k)) {
        return Err(format!("[resources.env] {k}: not a variable name"));
    }
    if let Some(k) = c.slots.keys().find(|k| !slot_name(k)) {
        return Err(format!(
            "[resources.slots] {k}: letters, digits, - and _ only"
        ));
    }
    c.idle_after()?;
    c.configured = true;
    Ok(c)
}

/// `[resources]` of this repository; a repository without the file or the
/// table has none.
pub fn load(root: &Utf8Path) -> Result<Config, String> {
    config_text(root).map_or_else(|| Ok(Config::default()), |t| parse(&t))
}

/// `[lanes] cleanup` (default true): `war plan lane new` ensures the cleanup
/// lane.
#[must_use]
pub fn cleanup_lane(root: &Utf8Path) -> bool {
    #[derive(Deserialize)]
    struct Lanes {
        #[serde(default)]
        cleanup: Option<bool>,
    }
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        lanes: Option<Lanes>,
    }
    config_text(root)
        .and_then(|t| toml::from_str::<File>(&t).ok())
        .and_then(|f| f.lanes)
        .and_then(|l| l.cleanup)
        .unwrap_or(true)
}

fn env_name(k: &str) -> bool {
    let mut c = k.chars();
    c.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

fn slot_name(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 64
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `env` with `{common}` replaced.
#[must_use]
pub fn expand(env: &BTreeMap<String, String>, common: &str) -> Vec<(String, String)> {
    env.iter()
        .map(|(k, v)| (k.clone(), v.replace("{common}", common)))
        .collect()
}

/// The resources env of this repository, expanded. `Err`: the config is
/// malformed, or `{common}` is used and the common directory is UNKNOWN.
pub fn vars(root: &Utf8Path) -> Result<Vec<(String, String)>, String> {
    let c = load(root)?;
    if c.env.is_empty() {
        return Ok(Vec::new());
    }
    let common = worktrees::common_dir(root);
    match common {
        Some(common) => Ok(expand(&c.env, common.as_str())),
        None if c.env.values().any(|v| v.contains("{common}")) => {
            Err("the clone's git common directory is UNKNOWN; {common} cannot expand".into())
        }
        None => Ok(expand(&c.env, "")),
    }
}

fn quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:@%+,".contains(c))
    {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// `.openwarrant/env`: one `KEY=VALUE` line each, quoted for a shell.
#[must_use]
pub fn env_text(vars: &[(String, String)]) -> String {
    vars.iter()
        .map(|(k, v)| format!("{k}={}\n", quote(v)))
        .collect()
}

// ---- seed --------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedRecord {
    schema: String,
    revision: String,
    command: Vec<String>,
    at: String,
}

fn seed_path(common: &Utf8Path) -> Utf8PathBuf {
    common.join("openwarrant/seed.json")
}

/// What the seed is to `base`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seed {
    Fresh,
    Missing,
    Stale(String),
    Unknown(String),
}

/// `None` when no seed is configured.
#[must_use]
pub fn seed_state(root: &Utf8Path, base: &str) -> Option<Seed> {
    let c = load(root).ok()?;
    c.seed.as_ref()?;
    let Some(common) = worktrees::common_dir(root) else {
        return Some(Seed::Unknown("git common directory unreadable".into()));
    };
    let path = seed_path(&common);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Some(Seed::Missing),
        Err(e) => return Some(Seed::Unknown(format!("{path}: {e}"))),
    };
    let Ok(record) = serde_json::from_slice::<SeedRecord>(&bytes) else {
        return Some(Seed::Unknown(format!("{path} does not parse")));
    };
    Some(if git::is_ancestor(root, &record.revision, base) {
        Seed::Fresh
    } else {
        Seed::Stale(record.revision)
    })
}

/// The brief's line for a seed that is not fresh.
#[must_use]
pub fn seed_line(state: &Seed) -> Option<String> {
    match state {
        Seed::Fresh => None,
        Seed::Missing => {
            Some("seed: none yet; `war admin seed` builds the shared cache once".into())
        }
        Seed::Stale(rev) => Some(format!(
            "seed: stale ({} is not in this base); `war admin seed` refreshes it",
            &rev[..rev.len().min(12)]
        )),
        Seed::Unknown(why) => Some(format!("seed: UNKNOWN ({why})")),
    }
}

/// The main checkout: the first worktree git lists.
fn main_checkout(root: &Utf8Path) -> Option<Utf8PathBuf> {
    let r = git::git(root, &["worktree", "list", "--porcelain"]);
    r.ok.then(|| {
        r.stdout
            .lines()
            .find_map(|l| l.strip_prefix("worktree "))
            .map(Utf8PathBuf::from)
    })
    .flatten()
}

/// `war admin seed`: run `[resources] seed` once in the main checkout with
/// the env applied; on success record the revision it built. The exit code
/// is the command's.
pub fn seed(root: &Utf8Path, json: bool) -> u8 {
    let c = match load(root) {
        Ok(c) => c,
        Err(why) => return say_unknown("resources.config", &why),
    };
    let Some(cmd) = c.seed.as_ref() else {
        eprintln!(
            "refused (resources.no-seed): no `[resources] seed` in openwarrant.toml; \
             e.g. seed = \"cargo build --workspace --all-targets\""
        );
        return crate::EXIT_NOT_READY;
    };
    let (Some(main), Some(common)) = (main_checkout(root), worktrees::common_dir(root)) else {
        return say_unknown(
            "resources.seed",
            "the main checkout or git common directory is UNKNOWN",
        );
    };
    let Some(revision) = git::rev(&main, "HEAD") else {
        return say_unknown("resources.seed", "the main checkout has no commit");
    };
    let vars = match vars(root) {
        Ok(v) => v,
        Err(why) => return say_unknown("resources.config", &why),
    };
    let argv = cmd.argv();
    eprintln!("seed {} in {main}", &revision[..revision.len().min(12)]);
    let code = match spawn_wait(&argv, &main, &vars, json) {
        Ok(code) => code,
        Err(why) => return say_unknown("resources.seed", &why),
    };
    if code == 0 {
        let record = SeedRecord {
            schema: SEED_SCHEMA.into(),
            revision: revision.clone(),
            command: argv,
            at: crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs()),
        };
        let path = seed_path(&common);
        let written = serde_json::to_vec_pretty(&record)
            .map_err(std::io::Error::other)
            .and_then(|b| write_atomic(&path, &b));
        match written {
            Ok(()) => eprintln!("seeded {}", &revision[..revision.len().min(12)]),
            Err(e) => {
                eprintln!("UNKNOWN (resources.seed): built, but {path} was not written: {e}");
            }
        }
    }
    code
}

fn write_atomic(path: &Utf8Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path)
}

fn say_unknown(rule: &str, why: &str) -> u8 {
    eprintln!("UNKNOWN ({rule}): {why}; nothing ran");
    crate::EXIT_NOT_READY
}

/// Run `argv` in `dir` with `vars`, SIGINT and SIGTERM passed to it rather
/// than killing `war`; its exit code (128 + the signal for a signal). With
/// `json`, its stdout goes to stderr, so stdout holds one envelope.
fn spawn_wait(
    argv: &[String],
    dir: &Utf8Path,
    vars: &[(String, String)],
    json: bool,
) -> Result<u8, String> {
    let Some((program, args)) = argv.split_first() else {
        return Err("no command".into());
    };
    let _armed = crate::perform::cancel::arm()?;
    let mut command = std::process::Command::new(program);
    command.args(args).current_dir(dir);
    for (k, v) in vars {
        command.env(k, v);
    }
    #[cfg(unix)]
    if json {
        use std::os::fd::AsFd as _;
        if let Ok(fd) = std::io::stderr().as_fd().try_clone_to_owned() {
            command.stdout(std::process::Stdio::from(fd));
        }
    }
    #[cfg(not(unix))]
    let _ = json;
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    let mut forwarded = false;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(exit_of(status)),
            Ok(None) => {}
            Err(e) => return Err(format!("waiting on {program}: {e}")),
        }
        if !forwarded && let Some(sig) = crate::perform::cancel::requested() {
            forwarded = true;
            #[cfg(unix)]
            if let Some(pid) = i32::try_from(child.id())
                .ok()
                .and_then(rustix::process::Pid::from_raw)
            {
                let s = if sig == "SIGINT" {
                    rustix::process::Signal::INT
                } else {
                    rustix::process::Signal::TERM
                };
                let _ = rustix::process::kill_process(pid, s);
            }
            #[cfg(not(unix))]
            let _ = (sig, child.kill());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn exit_of(status: std::process::ExitStatus) -> u8 {
    if let Some(code) = status.code() {
        return u8::try_from(code & 0xff).unwrap_or(1);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt as _;
        if let Some(sig) = status.signal() {
            return u8::try_from((128 + sig) & 0xff).unwrap_or(1);
        }
    }
    1
}

// ---- slots -------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Holder {
    schema: String,
    slot: String,
    actor: String,
    pid: u32,
    since: u64,
    /// The process's start time (Linux), so a reused pid is not a holder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    started: Option<u64>,
    /// The holder's pid namespace (Linux): a pid from another one cannot be
    /// judged, so it is never taken over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pidns: Option<String>,
    token: String,
}

#[derive(Debug)]
enum Probe {
    Free,
    Held(Holder, Vec<u8>),
    Unreadable(String),
}

fn probe(path: &Utf8Path) -> Probe {
    match std::fs::read(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Probe::Free,
        Err(e) => Probe::Unreadable(e.to_string()),
        Ok(bytes) => match serde_json::from_slice::<Holder>(&bytes) {
            Ok(h) if h.schema == SLOT_SCHEMA => Probe::Held(h, bytes),
            _ => Probe::Unreadable("holder record does not parse".into()),
        },
    }
}

fn take(path: &Utf8Path, bytes: &[u8]) -> std::io::Result<bool> {
    let dir = path.parent().unwrap_or(Utf8Path::new("."));
    std::fs::create_dir_all(dir)?;
    let temp = dir.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name().unwrap_or("slot"),
        std::process::id(),
        openwarrant_core::WarUuid::mint()
    ));
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    let linked = std::fs::hard_link(&temp, path);
    let _ = std::fs::remove_file(&temp);
    match linked {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(target_os = "linux")]
fn started(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Field 22, counted after the parenthesised name (which may hold spaces).
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}
#[cfg(not(target_os = "linux"))]
fn started(_pid: u32) -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn pidns() -> Option<String> {
    std::fs::read_link("/proc/self/ns/pid")
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}
#[cfg(not(target_os = "linux"))]
fn pidns() -> Option<String> {
    None
}

/// Whether the holder is alive: `Some(false)` only when that is established.
fn alive(h: &Holder) -> Option<bool> {
    if h.pidns.is_some() && h.pidns != pidns() {
        return None;
    }
    #[cfg(unix)]
    {
        let pid = i32::try_from(h.pid)
            .ok()
            .and_then(rustix::process::Pid::from_raw)?;
        match rustix::process::test_kill_process(pid) {
            Ok(()) | Err(rustix::io::Errno::PERM) => {}
            Err(rustix::io::Errno::SRCH) => return Some(false),
            Err(_) => return None,
        }
        match (h.started, started(h.pid)) {
            (Some(a), Some(b)) if a != b => Some(false),
            _ => Some(true),
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Remove a dead holder's lock, only while it is still the bytes judged.
fn take_over(path: &Utf8Path, seen: &[u8]) -> bool {
    let aside = path.with_extension(format!("stale-{}", openwarrant_core::WarUuid::mint()));
    if std::fs::rename(path, &aside).is_err() {
        return false;
    }
    let same = std::fs::read(&aside).is_ok_and(|b| b == seen);
    if !same {
        // Someone took it between the read and the rename: give it back.
        let _ = std::fs::hard_link(&aside, path);
    }
    let _ = std::fs::remove_file(&aside);
    same
}

struct Held {
    path: Utf8PathBuf,
    token: String,
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Probe::Held(h, _) = probe(&self.path)
            && h.token == self.token
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn when(secs: u64) -> String {
    crate::gate_cmd::receipt::rfc3339_from_secs(secs)
}

/// `war evidence run <slot> -- CMD…`: CMD with the resources env, holding
/// one of the slot's places. Waits, saying once who holds it; takes over a
/// holder whose process is gone, and says so. The exit code is CMD's; a
/// holder that cannot be read is UNKNOWN, CMD not run, exit 2.
pub fn run(root: &Utf8Path, slot: &str, argv: &[String], actor: &str, json: bool) -> u8 {
    if !slot_name(slot) {
        eprintln!("refused (resources.slot-name): `{slot}`: letters, digits, - and _ only");
        return crate::EXIT_NOT_READY;
    }
    if argv.is_empty() {
        eprintln!("refused (resources.no-command): `war evidence run {slot} -- <command>`");
        return crate::EXIT_NOT_READY;
    }
    let c = match load(root) {
        Ok(c) => c,
        Err(why) => return say_unknown("resources.config", &why),
    };
    let vars = match vars(root) {
        Ok(v) => v,
        Err(why) => return say_unknown("resources.config", &why),
    };
    let Some(common) = worktrees::common_dir(root) else {
        return say_unknown(SLOT_UNKNOWN, "the git common directory is UNKNOWN");
    };
    let dir = common.join("openwarrant/slots");
    let capacity = c.capacity(slot);
    let pid = std::process::id();
    let me = Holder {
        schema: SLOT_SCHEMA.into(),
        slot: slot.into(),
        actor: actor.into(),
        pid,
        since: crate::ticket::now_secs(),
        started: started(pid),
        pidns: pidns(),
        token: openwarrant_core::WarUuid::mint().to_string(),
    };
    let bytes = match serde_json::to_vec_pretty(&me) {
        Ok(b) => b,
        Err(e) => return say_unknown(SLOT_UNKNOWN, &e.to_string()),
    };
    let armed = match crate::perform::cancel::arm() {
        Ok(a) => a,
        Err(why) => return say_unknown(SLOT_UNKNOWN, &why),
    };
    let mut said = false;
    let held = 'wait: loop {
        if let Some(sig) = crate::perform::cancel::requested() {
            eprintln!("{sig} while waiting for slot {slot}; nothing ran");
            return 130;
        }
        let mut holders = Vec::new();
        for n in 0..capacity {
            let path = dir.join(format!("{slot}.{n}.json"));
            match take(&path, &bytes) {
                Ok(true) => break 'wait path,
                Ok(false) => {}
                Err(e) => return say_unknown(SLOT_UNKNOWN, &format!("{path}: {e}")),
            }
            match probe(&path) {
                Probe::Free => {}
                Probe::Unreadable(why) => {
                    return say_unknown(SLOT_UNKNOWN, &format!("slot {slot}: {path}: {why}"));
                }
                Probe::Held(h, seen) => {
                    if alive(&h) == Some(false) && take_over(&path, &seen) {
                        eprintln!(
                            "slot {slot}: holder {} (pid {}) is gone; taken over",
                            h.actor, h.pid
                        );
                        match take(&path, &bytes) {
                            Ok(true) => break 'wait path,
                            Ok(false) => {}
                            Err(e) => {
                                return say_unknown(SLOT_UNKNOWN, &format!("{path}: {e}"));
                            }
                        }
                    } else {
                        holders.push(h);
                    }
                }
            }
        }
        if !said && !holders.is_empty() {
            said = true;
            eprintln!(
                "slot {slot}: waiting on {}",
                holders
                    .iter()
                    .map(|h| format!("{} (pid {}, since {})", h.actor, h.pid, when(h.since)))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    drop(armed);
    let _held = Held {
        path: held,
        token: me.token.clone(),
    };
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
        .unwrap_or_else(|| root.to_owned());
    match spawn_wait(argv, &cwd, &vars, json) {
        Ok(code) => code,
        Err(why) => say_unknown("resources.run", &why),
    }
}

// ---- briefs ------------------------------------------------------------------

/// The resources lines of a `war start` brief: the env names, the build
/// slot, a stale seed. `in_worktree`: the env was written there. Nothing
/// without `[resources]`.
#[must_use]
pub fn brief(root: &Utf8Path, base: &str, in_worktree: bool) -> Vec<String> {
    let c = match load(root) {
        Ok(c) => c,
        Err(why) => return vec![format!("resources: UNKNOWN ({why})")],
    };
    if !c.configured {
        return Vec::new();
    }
    let mut out = Vec::new();
    if !c.env.is_empty() {
        let names = c.env.keys().cloned().collect::<Vec<_>>().join(" ");
        out.push(if in_worktree {
            format!("env: {names}, shared by every war worktree ({ENV_FILE})")
        } else {
            format!("env: {names} (set by `war evidence run`)")
        });
    }
    out.push(format!(
        "build through `war evidence run {BUILD} -- <command>` ({} at a time)",
        c.capacity(BUILD)
    ));
    if let Some(line) = seed_state(root, base).as_ref().and_then(seed_line) {
        out.push(line);
    }
    out
}

/// After an in-place `war start`: its resources lines.
pub fn after_start(root: &Utf8Path, outcome: &mut Outcome) {
    // Nothing handed over (a done or empty lane): nothing to build.
    if outcome.is_refused()
        || outcome
            .result
            .get("ticket")
            .is_some_and(serde_json::Value::is_null)
    {
        return;
    }
    let base = git::rev(root, "HEAD").unwrap_or_default();
    let lines = brief(root, &base, false);
    if lines.is_empty() {
        return;
    }
    outcome.human = format!("{}\n{}", outcome.human.trim_end(), lines.join("\n"));
    if let Some(o) = outcome.result.as_object_mut() {
        o.insert("resources".into(), serde_json::json!(lines));
    }
}

// ---- worktree limit --------------------------------------------------------

/// `war start` about to make a worktree: refused past `max_worktrees`,
/// naming the idle managed ones. UNKNOWN when they cannot be counted.
#[must_use]
pub fn worktree_limit(
    repo: &Repository,
    integration: &str,
) -> Option<crate::diagnostic::Diagnostic> {
    use crate::diagnostic::Diagnostic;
    let max = match load(&repo.root) {
        Ok(c) => c.max_worktrees?,
        Err(why) => {
            return Some(Diagnostic::unknown(
                "resources.config",
                crate::init::CONFIG_FILE,
                why,
            ));
        }
    };
    let count = |probe: bool| worktrees::inventory(repo, probe, integration);
    let rows = match count(false) {
        Ok(r) => r,
        Err(e) => {
            return Some(Diagnostic::unknown(
                LIMIT,
                crate::init::CONFIG_FILE,
                format!("managed worktrees could not be counted: {e}"),
            ));
        }
    };
    let n = rows.iter().filter(|r| r.managed).count();
    if n < usize::try_from(max).unwrap_or(usize::MAX) {
        return None;
    }
    let rows = count(true).unwrap_or(rows);
    let idle: Vec<String> = rows
        .iter()
        .filter(|r| r.managed && r.claims.is_empty())
        .map(|r| {
            format!(
                "{}{}",
                repo.relative(Utf8Path::new(&r.path)),
                if r.prunable { " (prunable)" } else { "" }
            )
        })
        .collect();
    Some(Diagnostic::error(
        LIMIT,
        crate::init::CONFIG_FILE,
        format!(
            "{n} managed worktrees, max {max} ([resources] max_worktrees); {}",
            if idle.is_empty() {
                "none is idle: finish one first".to_owned()
            } else {
                format!(
                    "idle: {}. Reuse one, or `war admin cleanup --apply`",
                    idle.join(", ")
                )
            }
        ),
    ))
}

// ---- cleanup -----------------------------------------------------------------

/// One local `war-go/*` branch.
#[derive(Debug, Clone, Serialize)]
pub struct Branch {
    pub branch: String,
    pub tip: String,
    pub deletable: bool,
    pub reasons: Vec<String>,
}

/// The local `war-go/*` branches: deletable when merged into `integration`,
/// checked out nowhere (`gone` names worktrees as if already removed), and
/// claimed by nobody.
fn branches(
    repo: &Repository,
    integration: &str,
    rows: &[worktrees::Row],
    gone: &[String],
) -> Vec<Branch> {
    let listed = git::git(
        &repo.root,
        &[
            "for-each-ref",
            "--format=%(refname:short) %(objectname)",
            "refs/heads/war-go/",
        ],
    );
    if !listed.ok {
        return Vec::new();
    }
    let skip = [
        integration.trim_start_matches("refs/heads/").to_owned(),
        crate::go::policy::load(&repo.root)
            .map_or_else(|_| "war-go/integration".into(), |p| p.integration),
    ];
    let claims = crate::ticket::Store::open(repo, None).and_then(|s| s.claims());
    listed
        .stdout
        .lines()
        .filter_map(|l| l.split_once(' '))
        .filter(|(b, _)| !skip.iter().any(|s| s == b))
        .map(|(b, tip)| {
            let mut reasons = Vec::new();
            if let Some(r) = rows
                .iter()
                .find(|r| r.branch.as_deref() == Some(b) && !gone.contains(&r.path))
            {
                reasons.push(format!(
                    "checked out at {}",
                    repo.relative(Utf8Path::new(&r.path))
                ));
            }
            if !git::is_ancestor(&repo.root, tip, integration) {
                reasons.push(format!("not merged into {integration}"));
            }
            match &claims {
                Ok(claims) => {
                    if claims.values().any(Option::is_none) {
                        reasons.push("unreadable claim retained".into());
                    }
                    for c in claims.values().flatten() {
                        if c.coordination.as_ref().and_then(|c| c.branch.as_deref()) == Some(b) {
                            reasons.push(format!("claimed by {}", c.actor));
                        }
                    }
                }
                Err(_) => reasons.push("claim inventory UNKNOWN".into()),
            }
            Branch {
                branch: b.to_owned(),
                tip: tip.to_owned(),
                deletable: reasons.is_empty(),
                reasons,
            }
        })
        .collect()
}

/// `war admin cleanup [--apply]`: the finished managed worktrees and the
/// merged `war-go/*` branches. A dry run lists them; `--apply` removes them,
/// rechecking each first.
pub fn cleanup(repo: &Repository, apply: bool, integration: &str) -> Result<Outcome, RepoError> {
    let rows = worktrees::inventory(repo, true, integration)?;
    let mut removed = Vec::new();
    if apply {
        for r in rows.iter().filter(|r| r.prunable) {
            if worktrees::remove_if_safe(&repo.root, Utf8Path::new(&r.path), integration) {
                removed.push(r.path.clone());
            }
        }
    }
    let gone: Vec<String> = if apply {
        removed.clone()
    } else {
        rows.iter()
            .filter(|r| r.prunable)
            .map(|r| r.path.clone())
            .collect()
    };
    let mut branches = branches(repo, integration, &rows, &gone);
    let mut deleted = Vec::new();
    if apply {
        for b in branches.iter_mut().filter(|b| b.deletable) {
            let r = git::git(
                &repo.root,
                &[
                    "update-ref",
                    "-d",
                    &format!("refs/heads/{}", b.branch),
                    &b.tip,
                ],
            );
            if r.ok {
                deleted.push(b.branch.clone());
            } else {
                b.deletable = false;
                b.reasons.push(format!("not deleted: {}", r.why()));
            }
        }
    }
    let mut lines = Vec::new();
    for r in rows
        .iter()
        .filter(|r| !r.reasons.iter().any(|x| x == "primary checkout"))
    {
        let path = repo.relative(Utf8Path::new(&r.path));
        let signal = r
            .activity
            .as_ref()
            .map_or_else(String::new, |a| format!(": {a}"));
        if removed.contains(&r.path) {
            lines.push(format!("removed {path}{signal}"));
        } else if r.prunable && !apply {
            lines.push(format!("remove  {path}{signal}"));
        } else {
            let why = if r.prunable {
                "changed on recheck".to_owned()
            } else {
                r.reasons.join("; ")
            };
            lines.push(format!("keep    {path}: {why}"));
        }
    }
    for b in &branches {
        if deleted.contains(&b.branch) {
            lines.push(format!("deleted {}", b.branch));
        } else if b.deletable {
            lines.push(format!("delete  {}", b.branch));
        } else {
            lines.push(format!("keep    {}: {}", b.branch, b.reasons.join("; ")));
        }
    }
    let would = gone.len() + branches.iter().filter(|b| b.deletable).count();
    if lines.is_empty() {
        lines.push("nothing to clean".into());
    } else if !apply && would > 0 {
        lines.push("dry run: `war admin cleanup --apply` removes them".into());
    }
    Ok(Outcome::ok(
        lines.join("\n"),
        serde_json::json!({
            "schema": "oh.war/cleanup/v1",
            "dry_run": !apply,
            "integration": integration,
            "worktrees": rows,
            "branches": branches,
            "removed": removed,
            "deleted": deleted,
        }),
    ))
}

/// At most one line about what is left to clean: finished managed
/// worktrees and merged `war-go/*` branches. `None` when nothing is, or it
/// cannot be read (a hint never blocks and never guesses).
#[must_use]
pub fn leftovers(repo: &Repository) -> Option<String> {
    let rows = worktrees::inventory(repo, false, "HEAD").ok()?;
    let finished = rows
        .iter()
        .filter(|r| {
            r.managed
                && r.reasons
                    .iter()
                    .all(|x| x.starts_with(worktrees::ACTIVITY_UNKNOWN))
        })
        .count();
    let merged = branches(repo, "HEAD", &rows, &[])
        .iter()
        .filter(|b| b.deletable)
        .count();
    if finished + merged == 0 {
        return None;
    }
    Some(format!(
        "leftovers: {finished} finished worktree(s), {merged} merged war-go branch(es); `war admin cleanup` lists them"
    ))
}

/// After `war done`: when it finished a lane's last item, one line about
/// leftovers. Never a refusal.
pub fn after_done(store: &crate::ticket::Store, target: &str, outcome: &mut Outcome) {
    if outcome.is_refused() {
        return;
    }
    let (ticket, _) = openwarrant_core::ticket::split_record_id(target);
    let Ok((tickets, _)) = store.load_all() else {
        return;
    };
    if !crate::ticket::lanes::finished_lane_of(&tickets, ticket) {
        return;
    }
    let Ok(repo) = Repository::discover(Some(store.root.clone())) else {
        return;
    };
    if let Some(line) = leftovers(&repo) {
        outcome.human = format!("{}\n{line}", outcome.human.trim_end());
        if let Some(o) = outcome.result.as_object_mut() {
            o.insert("leftovers".into(), serde_json::json!(line));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_expands_in_every_value() {
        let mut env = BTreeMap::new();
        env.insert(
            "CARGO_BUILD_BUILD_DIR".to_owned(),
            "{common}/openwarrant/build".to_owned(),
        );
        env.insert("PLAIN".to_owned(), "x".to_owned());
        assert_eq!(
            expand(&env, "/r/.git"),
            [
                (
                    "CARGO_BUILD_BUILD_DIR".to_owned(),
                    "/r/.git/openwarrant/build".to_owned()
                ),
                ("PLAIN".to_owned(), "x".to_owned())
            ]
        );
    }

    #[test]
    fn the_env_file_is_key_value_quoted_for_a_shell() {
        let text = env_text(&[
            ("A".into(), "/x/y".into()),
            ("B".into(), "a b".into()),
            ("C".into(), "it's".into()),
        ]);
        assert_eq!(text, "A=/x/y\nB='a b'\nC='it'\\''s'\n");
    }

    #[test]
    fn the_table_parses_and_refuses_a_bad_name() {
        let c = parse(
            "[resources]\nseed = \"cargo build\"\nmax_worktrees = 3\n[resources.env]\nX = \"{common}/b\"\n[resources.slots]\nbuild = 2\n",
        )
        .unwrap();
        assert!(c.configured);
        assert_eq!(c.capacity("build"), 2);
        assert_eq!(c.capacity("test"), 1, "an unconfigured slot holds one");
        assert_eq!(c.max_worktrees, Some(3));
        assert_eq!(c.idle_after(), Ok(Some(86_400)), "24h by default");
        let idle = |t: &str| parse(&format!("[resources]\nidle_after = \"{t}\"\n"));
        assert_eq!(idle("3d").unwrap().idle_after(), Ok(Some(3 * 86_400)));
        assert_eq!(idle("2h").unwrap().idle_after(), Ok(Some(7200)));
        assert_eq!(idle("0").unwrap().idle_after(), Ok(None));
        assert_eq!(idle("0h").unwrap().idle_after(), Ok(None));
        for bad in ["", "h", "24", "1w", "-1d", "+1d", "1.5d"] {
            assert!(idle(bad).is_err(), "{bad:?} refused");
        }
        assert_eq!(c.seed.unwrap().argv(), ["sh", "-c", "cargo build"]);
        assert!(!parse("").unwrap().configured);
        assert!(parse("[resources.env]\n\"1X\" = \"y\"\n").is_err());
        assert!(parse("[resources.slots]\n\"a/b\" = 1\n").is_err());
        assert!(
            parse("[resources]\nmax_worktree = 1\n").is_err(),
            "a misspelt key"
        );
        assert_eq!(
            parse("[resources]\nseed = [\"make\", \"-j4\"]\n")
                .unwrap()
                .seed
                .unwrap()
                .argv(),
            ["make", "-j4"]
        );
    }

    #[test]
    fn a_seed_line_is_said_unless_fresh() {
        assert_eq!(seed_line(&Seed::Fresh), None);
        assert!(
            seed_line(&Seed::Missing)
                .unwrap()
                .contains("war admin seed")
        );
        assert!(
            seed_line(&Seed::Stale("0123456789abcdef".into()))
                .unwrap()
                .contains("0123456789ab is not in this base")
        );
        assert!(
            seed_line(&Seed::Unknown("x".into()))
                .unwrap()
                .contains("UNKNOWN")
        );
    }

    #[test]
    fn a_holder_that_cannot_be_read_is_unknown_and_none_is_free() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        let p = root.join("build.0.json");
        assert!(matches!(probe(&p), Probe::Free));
        std::fs::write(&p, b"garbage").unwrap();
        assert!(matches!(probe(&p), Probe::Unreadable(_)));
        std::fs::remove_file(&p).unwrap();
        assert!(take(&p, b"{}").unwrap());
        assert!(!take(&p, b"{}").unwrap(), "a held slot is not taken twice");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_live_holder_is_alive_and_a_reused_pid_is_not() {
        let pid = std::process::id();
        let mut h = Holder {
            schema: SLOT_SCHEMA.into(),
            slot: "build".into(),
            actor: "a".into(),
            pid,
            since: 0,
            started: started(pid),
            pidns: pidns(),
            token: "t".into(),
        };
        assert_eq!(alive(&h), Some(true));
        h.started = h.started.map(|s| s + 1);
        assert_eq!(alive(&h), Some(false), "same pid, another process");
        h.started = None;
        h.pidns = Some("pid:[0]".into());
        assert_eq!(alive(&h), None, "another namespace cannot be judged");
    }
}
