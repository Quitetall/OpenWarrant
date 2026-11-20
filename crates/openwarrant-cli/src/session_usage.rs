// SPDX-License-Identifier: Apache-2.0
//! What a work session spent, read from the harness's own local record and
//! journalled as one `work.usage` event when `war done` closes work (or `war
//! start` hands the session to other work). `docs/design/tui-usage.md`.
//!
//! Only usage numbers are read: a transcript's message content never enters
//! the record, and nothing is sent anywhere. A session with no record that can
//! be read is journalled `"unknown"` with the reason; it is never zero. The
//! numbers are the provider's, unverified: they grant no assurance and gate
//! nothing. `[usage] record = false` turns the reading off.
//!
//! Reading is incremental: a per-session cursor beside the claim store holds
//! each file's byte offset, so a call reads only what was appended since the
//! last one. Without a cursor (the first call of a session) the files are read
//! once, from the claim's `since`.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Read, Seek, SeekFrom};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::ticket::{Outcome, Store};

/// The journal event (`docs/design/tui-usage.md`).
pub const EVENT: &str = "work.usage";

/// The most a `--usage` file may hold.
const REPORT_LIMIT: u64 = 64 * 1024;

/// How far back `war start` looks for Codex's running total.
const CODEX_TAIL: u64 = 16 * 1024 * 1024;

/// One model's counts in a window.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<u64>,
    /// This entry's total: input, output and cache tokens together.
    pub tokens: u64,
}

impl Counts {
    fn add(&mut self, input: u64, output: u64, cache_read: u64, cache_write: u64) {
        let bump =
            |slot: &mut Option<u64>, n: u64| *slot = Some(slot.unwrap_or(0).saturating_add(n));
        bump(&mut self.input, input);
        bump(&mut self.output, output);
        bump(&mut self.cache_read, cache_read);
        bump(&mut self.cache_write, cache_write);
        self.tokens = self
            .tokens
            .saturating_add(input)
            .saturating_add(output)
            .saturating_add(cache_read)
            .saturating_add(cache_write);
    }
}

/// A harness this reads, named by the session variable it sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Harness {
    ClaudeCode(String),
    Codex(String),
    /// opencode's shell tool sets `OPENCODE_SESSION_ID` for the commands it runs.
    OpenCode(String),
    /// A session some other harness named: there is no reader for it.
    Other {
        var: &'static str,
        id: String,
    },
}

impl Harness {
    /// The harness this process runs under, from its session variable.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        if let Some(id) = var("CLAUDE_CODE_SESSION_ID") {
            return Some(Self::ClaudeCode(id));
        }
        if let Some(id) = var("CODEX_THREAD_ID") {
            return Some(Self::Codex(id));
        }
        if let Some(id) = var("OPENCODE_SESSION_ID") {
            return Some(Self::OpenCode(id));
        }
        for name in [
            "CODEX_SESSION_ID",
            "OPENWARRANT_SESSION_ID",
            "KATANA_SESSION_ID",
        ] {
            if let Some(id) = var(name) {
                return Some(Self::Other { var: name, id });
            }
        }
        None
    }

    fn source(&self) -> &'static str {
        match self {
            Self::ClaudeCode(_) => "claude-code",
            Self::Codex(_) => "codex",
            Self::OpenCode(_) => "opencode",
            Self::Other { .. } => "harness",
        }
    }

    fn id(&self) -> &str {
        match self {
            Self::ClaudeCode(id)
            | Self::Codex(id)
            | Self::OpenCode(id)
            | Self::Other { id, .. } => id,
        }
    }

    /// `sha256:<hex>` of the session id: the record never holds the id.
    fn digest(&self) -> String {
        format!(
            "sha256:{}",
            openwarrant_compiler::sha256_hex(self.id().as_bytes())
        )
    }
}

/// `[usage] record` in `openwarrant.toml`; on unless set `false`.
#[must_use]
pub fn enabled(root: &Utf8Path) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join(crate::init::CONFIG_FILE)) else {
        return true;
    };
    let Ok(doc) = toml::from_str::<toml::Table>(&text) else {
        return true;
    };
    doc.get("usage")
        .and_then(|u| u.get("record"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true)
}

// ---- the cursor --------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FileMark {
    offset: u64,
    /// The last message counted from this file, so one split across the
    /// offset is not counted twice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Cursor {
    /// The ticket the open window is for, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    /// When the window opened.
    since: String,
    files: BTreeMap<String, FileMark>,
    /// Codex: the running total at the mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    codex_total: Option<[u64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    codex_model: Option<String>,
    /// opencode: per session of the tree, the `seq` of the last message
    /// counted (a message count, not a byte offset: the record is a database).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    opencode: BTreeMap<String, i64>,
}

fn cursor_path(store: &Store, harness: &Harness) -> Utf8PathBuf {
    let dir = store.claims_dir.with_file_name("usage-cursors");
    let tag = &harness.digest()["sha256:".len()..][..24];
    dir.join(format!("{}-{tag}.json", harness.source()))
}

fn load_cursor(path: &Utf8Path) -> Option<Cursor> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn save_cursor(path: &Utf8Path, cursor: &Cursor) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_vec(cursor) {
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

/// Held while a cursor is read, a window recorded and the cursor moved, so
/// two `war` calls of one session never count one window twice.
struct CursorLock(#[allow(dead_code)] Option<std::fs::File>);

fn lock_cursor(path: &Utf8Path) -> CursorLock {
    let Some(dir) = path.parent() else {
        return CursorLock(None);
    };
    let _ = std::fs::create_dir_all(dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.with_extension("lock"))
        .ok();
    #[cfg(unix)]
    if let Some(f) = &file {
        let _ = rustix::fs::flock(f, rustix::fs::FlockOperation::LockExclusive);
    }
    CursorLock(file)
}

// ---- where a harness keeps its record -------------------------------------------

fn home() -> Option<Utf8PathBuf> {
    std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .map(Utf8PathBuf::from)
}

/// Claude Code's transcript and its subagents' transcripts:
/// `<config>/projects/<project-key>/<session>.jsonl` and
/// `<config>/projects/<project-key>/<session>/subagents/*.jsonl`, where
/// `<config>` is `CLAUDE_CONFIG_DIR`, else `~/.claude`.
fn claude_files(session: &str) -> Result<Vec<Utf8PathBuf>, String> {
    if session.contains('/') || session.contains("..") {
        return Err("the session id is not a file name".to_owned());
    }
    let config = std::env::var("CLAUDE_CONFIG_DIR")
        .ok()
        .filter(|d| !d.is_empty())
        .map(Utf8PathBuf::from)
        .or_else(|| home().map(|h| h.join(".claude")))
        .ok_or("neither CLAUDE_CONFIG_DIR nor HOME is set")?;
    let projects = config.join("projects");
    let entries = std::fs::read_dir(&projects).map_err(|e| {
        format!(
            "Claude Code's transcripts could not be listed ({})",
            e.kind()
        )
    })?;
    let name = format!("{session}.jsonl");
    for entry in entries.flatten() {
        let Ok(dir) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        let main = dir.join(&name);
        if !main.is_file() {
            continue;
        }
        let mut files = vec![main];
        if let Ok(subs) = std::fs::read_dir(dir.join(session).join("subagents")) {
            let mut subs: Vec<Utf8PathBuf> = subs
                .flatten()
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|p| p.extension() == Some("jsonl"))
                .collect();
            subs.sort();
            files.extend(subs);
        }
        return Ok(files);
    }
    Err("no Claude Code transcript for this session (CLAUDE_CONFIG_DIR, else ~/.claude)".to_owned())
}

/// Codex's rollout for a thread: `<home>/sessions/YYYY/MM/DD/rollout-*-<thread>.jsonl`,
/// `<home>` being `CODEX_HOME`, else `~/.codex`.
fn codex_file(thread: &str) -> Result<Utf8PathBuf, String> {
    if thread.contains('/') || thread.contains("..") {
        return Err("the thread id is not a file name".to_owned());
    }
    let codex = std::env::var("CODEX_HOME")
        .ok()
        .filter(|d| !d.is_empty())
        .map(Utf8PathBuf::from)
        .or_else(|| home().map(|h| h.join(".codex")))
        .ok_or("neither CODEX_HOME nor HOME is set")?;
    let sessions = codex.join("sessions");
    let suffix = format!("-{thread}.jsonl");
    let mut stack = vec![sessions];
    let mut seen = 0usize;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > 200_000 {
                return Err("Codex's sessions directory is too large to search".to_owned());
            }
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n.ends_with(&suffix)) {
                return Ok(path);
            }
        }
    }
    Err("no Codex rollout for this thread (CODEX_HOME, else ~/.codex)".to_owned())
}

// ---- reading -------------------------------------------------------------------

/// Only the fields read. Everything else on the line, the message's content
/// included, is skipped by the parser and never held.
#[derive(Deserialize)]
struct ClaudeLine {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    message: Option<ClaudeMessage>,
}

#[derive(Deserialize)]
struct ClaudeMessage {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
}

#[derive(Deserialize)]
struct CodexLine {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    payload: Option<CodexPayload>,
}

#[derive(Deserialize)]
struct CodexPayload {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    info: Option<CodexInfo>,
}

#[derive(Deserialize)]
struct CodexInfo {
    total_token_usage: CodexTotal,
}

#[derive(Deserialize)]
struct CodexTotal {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

fn after(timestamp: Option<&str>, since: Option<&chrono::DateTime<chrono::Utc>>) -> bool {
    let Some(since) = since else {
        return true;
    };
    timestamp
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .is_some_and(|t| t.with_timezone(&chrono::Utc) >= *since)
}

/// Each whole line of `path` from `offset`, and the offset after the last
/// whole line. A line still being written is left for the next read.
fn lines_from(path: &Utf8Path, offset: u64, mut each: impl FnMut(&[u8])) -> Result<u64, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("the session's record could not be read ({})", e.kind()))?;
    let len = file
        .metadata()
        .map_err(|e| format!("the session's record could not be read ({})", e.kind()))?
        .len();
    // A file shorter than its mark was replaced: read it whole.
    let start = if offset > len { 0 } else { offset };
    file.seek(SeekFrom::Start(start))
        .map_err(|e| format!("the session's record could not be read ({})", e.kind()))?;
    let mut reader = std::io::BufReader::with_capacity(1 << 20, file);
    let mut at = start;
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = reader
            .read_until(b'\n', &mut line)
            .map_err(|e| format!("the session's record could not be read ({})", e.kind()))?;
        if n == 0 || line.last() != Some(&b'\n') {
            return Ok(at);
        }
        at += n as u64;
        each(&line);
    }
}

/// What one window of a session spent.
#[derive(Debug, Default)]
struct Window {
    by_model: BTreeMap<String, Counts>,
    messages: usize,
    /// opencode's own `cost`, summed: the provider's unverified number, and
    /// known to read 0 for subagents on a paid model.
    cost: Option<f64>,
}

impl Window {
    fn entries(&self) -> Vec<Counts> {
        self.by_model
            .iter()
            .map(|(model, c)| Counts {
                model: (!model.is_empty()).then(|| model.clone()),
                ..c.clone()
            })
            .collect()
    }
}

fn read_claude(
    files: &[Utf8PathBuf],
    cursor: Option<&Cursor>,
    since: Option<&chrono::DateTime<chrono::Utc>>,
    next: &mut Cursor,
) -> Result<Window, String> {
    let mut window = Window::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for path in files {
        let key = path.to_string();
        let mark = cursor.and_then(|c| c.files.get(&key)).cloned();
        // With a cursor, a file it does not know began after the mark.
        let (offset, from_time) = match (&mark, cursor) {
            (Some(m), _) => (m.offset, None),
            (None, Some(_)) => (0, None),
            (None, None) => (0, since),
        };
        let mut last_id = mark.as_ref().and_then(|m| m.last_id.clone());
        let skip = last_id.clone();
        let end = lines_from(path, offset, |line| {
            if memchr::memmem::find(line, b"\"usage\"").is_none() {
                return;
            }
            let Ok(l) = serde_json::from_slice::<ClaudeLine>(line) else {
                return;
            };
            if l.kind.as_deref() != Some("assistant") {
                return;
            }
            let Some(m) = l.message else { return };
            let Some(u) = m.usage else { return };
            if !after(l.timestamp.as_deref(), from_time) {
                return;
            }
            if let Some(id) = &m.id {
                // One API message is written as a line per content block,
                // each carrying the same usage: it is counted once.
                if skip.as_deref() == Some(id.as_str()) || !seen.insert(id.clone()) {
                    return;
                }
                last_id = Some(id.clone());
            }
            let total = u.input_tokens
                + u.output_tokens
                + u.cache_read_input_tokens
                + u.cache_creation_input_tokens;
            if total == 0 {
                return;
            }
            window.messages += 1;
            window
                .by_model
                .entry(m.model.unwrap_or_default())
                .or_default()
                .add(
                    u.input_tokens,
                    u.output_tokens,
                    u.cache_read_input_tokens,
                    u.cache_creation_input_tokens,
                );
        })?;
        next.files.insert(
            key,
            FileMark {
                offset: end,
                last_id,
            },
        );
    }
    Ok(window)
}

// ---- opencode: a SQLite database, read through `sqlite3` ---------------------------

/// How long one `sqlite3` call may run.
const SQLITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// `<XDG_DATA_HOME, else ~/.local/share>/opencode/opencode.db`.
fn opencode_db() -> Result<Utf8PathBuf, String> {
    let data = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|d| !d.is_empty())
        .map(Utf8PathBuf::from)
        .or_else(|| home().map(|h| h.join(".local/share")))
        .ok_or("neither XDG_DATA_HOME nor HOME is set")?;
    let db = data.join("opencode/opencode.db");
    if db.is_file() {
        Ok(db)
    } else {
        Err("opencode's database was not found (XDG_DATA_HOME, else ~/.local/share)".to_owned())
    }
}

/// One assistant message's numbers. The message's content is never selected.
#[derive(Debug, Deserialize)]
struct OcRow {
    session_id: String,
    seq: i64,
    time_created: i64,
    #[serde(default)]
    completed: Option<i64>,
    #[serde(default)]
    model: Option<String>,
    /// `json_type` of `tokens`: absent when the message carries none.
    #[serde(default)]
    tok: Option<String>,
    #[serde(default)]
    input: Option<u64>,
    #[serde(default)]
    output: Option<u64>,
    #[serde(default)]
    reasoning: Option<u64>,
    #[serde(default)]
    cache_read: Option<u64>,
    #[serde(default)]
    cache_write: Option<u64>,
    #[serde(default)]
    cost: Option<f64>,
}

/// The assistant messages of `session` and every session under it, by
/// session and `seq`. Read-only (`mode=ro`); a missing `sqlite3`, a lock that
/// outlasts the timeout, or tables and fields that are not these is an error
/// naming the cause, never an empty answer.
fn opencode_rows(program: &str, db: &Utf8Path, session: &str) -> Result<Vec<OcRow>, String> {
    if session.is_empty()
        || !session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("the opencode session id is not a plain identifier".to_owned());
    }
    let sql = format!(
        "WITH RECURSIVE t(id) AS (SELECT id FROM session_v2 WHERE id = '{session}' \
         UNION ALL SELECT s.id FROM session_v2 s JOIN t ON s.parent_id = t.id) \
         SELECT m.session_id AS session_id, m.seq AS seq, m.time_created AS time_created, \
         json_extract(m.data, '$.time.completed') AS completed, \
         json_extract(m.data, '$.model.id') AS model, \
         json_type(m.data, '$.tokens') AS tok, \
         json_extract(m.data, '$.tokens.input') AS input, \
         json_extract(m.data, '$.tokens.output') AS output, \
         json_extract(m.data, '$.tokens.reasoning') AS reasoning, \
         json_extract(m.data, '$.tokens.cache.read') AS cache_read, \
         json_extract(m.data, '$.tokens.cache.write') AS cache_write, \
         json_extract(m.data, '$.cost') AS cost \
         FROM session_message m WHERE m.type = 'assistant' \
         AND m.session_id IN (SELECT id FROM t) ORDER BY m.session_id, m.seq"
    );
    let mut uri = String::from("file:");
    for c in db.as_str().chars() {
        match c {
            '%' => uri.push_str("%25"),
            '?' => uri.push_str("%3F"),
            '#' => uri.push_str("%23"),
            ' ' => uri.push_str("%20"),
            c => uri.push(c),
        }
    }
    uri.push_str("?mode=ro");
    let mut child = std::process::Command::new(program)
        .args(["-readonly", "-bail", "-json", "-cmd", ".timeout 3000"])
        .arg(&uri)
        .arg(&sql)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "sqlite3 is not installed (opencode's database is read through it)".to_owned()
            } else {
                format!("sqlite3 could not be run ({})", e.kind())
            }
        })?;
    let mut out = child.stdout.take().ok_or("sqlite3 gave no output pipe")?;
    let err = child.stderr.take().ok_or("sqlite3 gave no error pipe")?;
    let out_t = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = out.read_to_end(&mut v);
        v
    });
    let err_t = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = err.take(4096).read_to_end(&mut v);
        v
    });
    let deadline = std::time::Instant::now() + SQLITE_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("sqlite3 timed out reading opencode's database (locked?)".to_owned());
            }
            Err(e) => return Err(format!("sqlite3 could not be waited on ({})", e.kind())),
        }
    };
    let stdout = out_t.join().unwrap_or_default();
    let stderr = String::from_utf8_lossy(&err_t.join().unwrap_or_default()).to_lowercase();
    if !status.success() {
        return Err(if stderr.contains("locked") || stderr.contains("busy") {
            "opencode's database is locked".to_owned()
        } else if stderr.contains("no such table") || stderr.contains("no such column") {
            "opencode's database is not the schema this reads (session_v2, session_message): \
             schema mismatch"
                .to_owned()
        } else if stderr.contains("unable to open") || stderr.contains("not a database") {
            "opencode's database could not be opened read-only".to_owned()
        } else {
            format!("sqlite3 failed reading opencode's database ({status})")
        });
    }
    // `-json` prints nothing at all for no rows.
    if stdout.iter().all(u8::is_ascii_whitespace) {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&stdout).map_err(|_| {
        "opencode's database rows are not the shape this reads: schema mismatch".to_owned()
    })
}

/// Count what `rows` hold past each session's mark. A session with no mark
/// counts from `since_ms`. A message not yet completed, and every later one
/// of its session, waits for the next read, so the mark never passes it.
fn opencode_scan(
    rows: &[OcRow],
    marks: &BTreeMap<String, i64>,
    since_ms: i64,
    next: &mut BTreeMap<String, i64>,
) -> Window {
    let mut window = Window::default();
    let mut waiting: BTreeSet<&str> = BTreeSet::new();
    for r in rows {
        let sid = r.session_id.as_str();
        if waiting.contains(sid) {
            continue;
        }
        match marks.get(sid) {
            Some(m) if r.seq <= *m => continue,
            None if r.time_created < since_ms => continue,
            _ => {}
        }
        if r.completed.is_none() {
            waiting.insert(sid);
            continue;
        }
        next.insert(r.session_id.clone(), r.seq);
        if r.tok.is_none() {
            continue;
        }
        // Reasoning tokens are billed as output.
        let output = r
            .output
            .unwrap_or(0)
            .saturating_add(r.reasoning.unwrap_or(0));
        let (input, cr, cw) = (
            r.input.unwrap_or(0),
            r.cache_read.unwrap_or(0),
            r.cache_write.unwrap_or(0),
        );
        if input + output + cr + cw == 0 {
            continue;
        }
        window.messages += 1;
        window
            .by_model
            .entry(r.model.clone().unwrap_or_default())
            .or_default()
            .add(input, output, cr, cw);
        window.cost = Some(window.cost.unwrap_or(0.0) + r.cost.unwrap_or(0.0));
    }
    window
}

fn read_opencode(
    program: &str,
    db: &Utf8Path,
    session: &str,
    cursor: Option<&Cursor>,
    since: Option<&chrono::DateTime<chrono::Utc>>,
    next: &mut Cursor,
) -> Result<Window, String> {
    let rows = opencode_rows(program, db, session)?;
    let since_ms = cursor
        .and_then(|c| chrono::DateTime::parse_from_rfc3339(&c.since).ok())
        .map(|t| t.timestamp_millis())
        .or_else(|| since.map(chrono::DateTime::timestamp_millis))
        .unwrap_or(0);
    let marks = cursor.map(|c| c.opencode.clone()).unwrap_or_default();
    next.opencode.clone_from(&marks);
    Ok(opencode_scan(&rows, &marks, since_ms, &mut next.opencode))
}

fn codex_line(line: &[u8]) -> Option<CodexLine> {
    if memchr::memmem::find(line, b"token_count").is_none()
        && memchr::memmem::find(line, b"turn_context").is_none()
    {
        return None;
    }
    serde_json::from_slice(line).ok()
}

fn codex_total(l: &CodexLine) -> Option<[u64; 3]> {
    let p = l.payload.as_ref()?;
    if l.kind.as_deref() != Some("event_msg") || p.kind.as_deref() != Some("token_count") {
        return None;
    }
    let t = &p.info.as_ref()?.total_token_usage;
    Some([t.input_tokens, t.cached_input_tokens, t.output_tokens])
}

fn codex_model(l: &CodexLine) -> Option<String> {
    (l.kind.as_deref() == Some("turn_context"))
        .then(|| l.payload.as_ref()?.model.clone())
        .flatten()
}

/// Codex keeps a running total: the window is its last total less the one
/// at the mark. More than one model in the window names none.
fn read_codex(
    path: &Utf8Path,
    cursor: Option<&Cursor>,
    since: Option<&chrono::DateTime<chrono::Utc>>,
    next: &mut Cursor,
) -> Result<Window, String> {
    let key = path.to_string();
    let offset = cursor
        .and_then(|c| c.files.get(&key))
        .map_or(0, |m| m.offset);
    let mut base = cursor.and_then(|c| c.codex_total);
    let mut model = cursor.and_then(|c| c.codex_model.clone());
    let mut models: BTreeSet<String> = BTreeSet::new();
    let mut last: Option<[u64; 3]> = None;
    let end = lines_from(path, offset, |line| {
        let Some(l) = codex_line(line) else { return };
        let inside = cursor.is_some() || after(l.timestamp.as_deref(), since);
        if let Some(m) = codex_model(&l) {
            if inside {
                models.insert(m.clone());
            }
            model = Some(m);
        }
        if let Some(t) = codex_total(&l) {
            if inside {
                last = Some(t);
            } else {
                base = Some(t);
            }
        }
    })?;
    next.files.insert(
        key,
        FileMark {
            offset: end,
            last_id: None,
        },
    );
    next.codex_model = model.clone();
    let mut window = Window::default();
    if let Some(t) = last {
        next.codex_total = Some(t);
        let b = base.unwrap_or([0, 0, 0]);
        let cached = t[1].saturating_sub(b[1]);
        let input = t[0].saturating_sub(b[0]).saturating_sub(cached);
        let output = t[2].saturating_sub(b[2]);
        if input + cached + output > 0 {
            let name = if models.len() > 1 {
                String::new()
            } else {
                models.into_iter().next().or(model).unwrap_or_default()
            };
            window.messages = 1;
            window
                .by_model
                .entry(name)
                .or_default()
                .add(input, output, cached, 0);
        }
    } else {
        next.codex_total = base;
    }
    Ok(window)
}

/// The end of each of the session's files now, for a window that opens here.
fn mark_now(harness: &Harness) -> Option<Cursor> {
    let mut cursor = Cursor {
        since: crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs()),
        ..Cursor::default()
    };
    match harness {
        Harness::ClaudeCode(s) => {
            for path in claude_files(s).ok()? {
                let len = std::fs::metadata(&path).ok()?.len();
                cursor.files.insert(
                    path.to_string(),
                    FileMark {
                        offset: len,
                        last_id: None,
                    },
                );
            }
        }
        Harness::Codex(t) => {
            let path = codex_file(t).ok()?;
            let len = std::fs::metadata(&path).ok()?.len();
            let (total, model) = codex_tail(&path, len);
            cursor.codex_total = total;
            cursor.codex_model = model;
            cursor.files.insert(
                path.to_string(),
                FileMark {
                    offset: len,
                    last_id: None,
                },
            );
        }
        Harness::OpenCode(s) => {
            let rows = opencode_rows("sqlite3", &opencode_db().ok()?, s).ok()?;
            opencode_scan(&rows, &BTreeMap::new(), 0, &mut cursor.opencode);
        }
        Harness::Other { .. } => return None,
    }
    Some(cursor)
}

/// The last running total and model in the `CODEX_TAIL` bytes before `end`.
fn codex_tail(path: &Utf8Path, end: u64) -> (Option<[u64; 3]>, Option<String>) {
    let start = end.saturating_sub(CODEX_TAIL);
    let Ok(mut file) = std::fs::File::open(path) else {
        return (None, None);
    };
    let mut buf = Vec::new();
    if file.seek(SeekFrom::Start(start)).is_err()
        || file.take(end - start).read_to_end(&mut buf).is_err()
    {
        return (None, None);
    }
    let (mut total, mut model) = (None, None);
    for line in buf.split(|b| *b == b'\n') {
        if let Some(l) = codex_line(line) {
            total = codex_total(&l).or(total);
            model = codex_model(&l).or(model);
        }
    }
    (total, model)
}

// ---- recording ---------------------------------------------------------------------

/// A usage report a harness with no local store hands `war done --usage`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
    #[serde(default)]
    pub cache_read: Option<u64>,
    #[serde(default)]
    pub cache_write: Option<u64>,
    #[serde(default)]
    pub tokens: Option<u64>,
    /// Who reports it: a harness's name. Free text, and unverified.
    #[serde(default)]
    pub reporter: Option<String>,
}

/// Read `--usage <file|->`. Refused before anything is ticked when it is not
/// a small JSON object with numbers.
pub fn read_report(arg: &str) -> Result<Report, String> {
    let mut text = String::new();
    let read = if arg == "-" {
        std::io::stdin()
            .take(REPORT_LIMIT + 1)
            .read_to_string(&mut text)
    } else {
        std::fs::File::open(arg).and_then(|f| f.take(REPORT_LIMIT + 1).read_to_string(&mut text))
    };
    read.map_err(|e| format!("usage.report-unreadable: {arg}: {e}"))?;
    if text.len() as u64 > REPORT_LIMIT {
        return Err(format!(
            "usage.report-invalid: {arg} is over {REPORT_LIMIT} bytes"
        ));
    }
    let r: Report = serde_json::from_str(&text).map_err(|e| {
        format!(
            "usage.report-invalid: {arg}: {e}. Expected {{\"model\"?, \"input\"?, \"output\"?, \
             \"cache_read\"?, \"cache_write\"?, \"tokens\"?, \"reporter\"?}}"
        )
    })?;
    if r.tokens.is_none() && r.input.is_none() && r.output.is_none() {
        return Err(format!(
            "usage.report-invalid: {arg} names no count (`tokens`, or `input` and `output`)"
        ));
    }
    let parts = [r.input, r.output, r.cache_read, r.cache_write];
    let sum: u64 = parts.iter().flatten().sum();
    if let Some(t) = r.tokens
        && parts.iter().any(Option::is_some)
        && t != sum
    {
        return Err(format!(
            "usage.report-invalid: {arg}: tokens {t} is not input + output + cache ({sum})"
        ));
    }
    Ok(r)
}

fn report_payload(target: &str, r: &Report) -> serde_json::Value {
    let parts = [r.input, r.output, r.cache_read, r.cache_write];
    let entry = Counts {
        model: r.model.clone(),
        input: r.input,
        output: r.output,
        cache_read: r.cache_read,
        cache_write: r.cache_write,
        tokens: r.tokens.unwrap_or_else(|| parts.iter().flatten().sum()),
    };
    let mut p = serde_json::json!({
        "target": target,
        "source": "reported",
        "at": crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs()),
        "models": [entry],
        "tokens": entry.tokens,
    });
    if let Some(who) = &r.reporter {
        p["reporter"] = serde_json::json!(
            who.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(80)
                .collect::<String>()
        );
    }
    if let Some(h) = Harness::from_env() {
        p["session"] = serde_json::json!(h.digest());
    }
    p
}

fn window_payload(
    harness: &Harness,
    target: &str,
    from: &str,
    read: Result<Window, String>,
) -> serde_json::Value {
    let mut p = serde_json::json!({
        "target": target,
        "source": harness.source(),
        "session": harness.digest(),
        "at": crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs()),
        "from": from,
    });
    match read {
        Ok(w) if w.messages > 0 => {
            let entries = w.entries();
            p["tokens"] = serde_json::json!(entries.iter().map(|e| e.tokens).sum::<u64>());
            p["models"] = serde_json::json!(entries);
            if let Some(c) = w.cost {
                // The provider's own figure, unverified: opencode reads 0 for
                // subagents on a paid model.
                p["cost"] = serde_json::json!(c);
            }
        }
        Ok(_) => {
            p["tokens"] = serde_json::json!("unknown");
            p["reason"] =
                serde_json::json!(format!("the session's record holds no usage since {from}"));
        }
        Err(why) => {
            p["tokens"] = serde_json::json!("unknown");
            p["reason"] = serde_json::json!(why);
        }
    }
    p
}

/// The latest `since` this journal's claims of `target` (or its ticket) give.
fn claimed_since(t: &crate::ticket::Ticket, target: &str) -> Option<String> {
    let journal = crate::journal_cmd::load(&t.dir).ok()?;
    let ticket = target.split('/').next().unwrap_or(target);
    journal
        .events
        .iter()
        .filter(|e| {
            crate::journal_cmd::work_type(&e.event_type) == crate::ticket::event::CLAIMED
                || crate::journal_cmd::work_type(&e.event_type)
                    == crate::ticket::event::CLAIM_RECLAIMED
                || crate::journal_cmd::work_type(&e.event_type)
                    == crate::ticket::event::CLAIM_STOLEN
        })
        .filter_map(|e| serde_json::from_str::<serde_json::Value>(&e.payload).ok())
        .filter(|p| {
            p.get("target")
                .and_then(|v| v.as_str())
                .is_some_and(|x| x == target || x == ticket)
        })
        .filter_map(|p| p.get("since").and_then(|v| v.as_str()).map(str::to_owned))
        .max()
}

/// Read the session's window, from its cursor or from `since`.
fn read_window(
    harness: &Harness,
    cursor: Option<&Cursor>,
    since: Option<&str>,
) -> (Result<Window, String>, Cursor) {
    let mut next = Cursor {
        since: crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs()),
        ..Cursor::default()
    };
    let since_time = since
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&chrono::Utc));
    if cursor.is_none() && since_time.is_none() {
        // Without a mark or a claim time there is no start to count from.
        let read = match harness {
            Harness::ClaudeCode(s) => claude_files(s).map(|_| ()),
            Harness::Codex(t) => codex_file(t).map(|_| ()),
            Harness::OpenCode(_) => opencode_db().map(|_| ()),
            Harness::Other { .. } => Ok(()),
        };
        return (
            Err(read.err().unwrap_or_else(|| {
                "no start for this window: the session has no mark (`war start` sets one) and \
                 the journal no claim time"
                    .to_owned()
            })),
            next,
        );
    }
    let read = match harness {
        Harness::ClaudeCode(s) => claude_files(s)
            .and_then(|files| read_claude(&files, cursor, since_time.as_ref(), &mut next)),
        Harness::Codex(t) => {
            codex_file(t).and_then(|path| read_codex(&path, cursor, since_time.as_ref(), &mut next))
        }
        Harness::OpenCode(s) => opencode_db().and_then(|db| {
            read_opencode("sqlite3", &db, s, cursor, since_time.as_ref(), &mut next)
        }),
        Harness::Other { var, .. } => Err(format!(
            "{var} names the session, and no reader knows that harness's record; report it \
             with `war done --usage <file|->`"
        )),
    };
    (read, next)
}

fn journal_on(store: &Store, target: &str, payload: &serde_json::Value) -> Result<(), String> {
    let (tickets, _) = store.load_all().map_err(|e| e.to_string())?;
    let t = match crate::ticket::resolve(&tickets, target) {
        Ok(crate::ticket::Target::Ticket(n) | crate::ticket::Target::Item(n, _)) => &tickets[n],
        Err(d) => return Err(d.message),
    };
    store.journal(t, EVENT, payload).map_err(|e| e.to_string())
}

/// One line for the person, from a recorded payload.
fn summary(p: &serde_json::Value) -> String {
    match p.get("tokens") {
        Some(serde_json::Value::Number(n)) => {
            let models: Vec<&str> = p["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| m.get("model").and_then(|v| v.as_str()))
                .collect();
            format!(
                "usage: {n} tok recorded ({}{}{})",
                p["source"].as_str().unwrap_or("?"),
                if models.is_empty() { "" } else { ", " },
                models.join(", ")
            )
        }
        _ => format!(
            "usage: UNKNOWN ({})",
            p["reason"].as_str().unwrap_or("no reason given")
        ),
    }
}

fn attach(outcome: &mut Outcome, payload: &serde_json::Value, recorded: Result<(), String>) {
    let line = match &recorded {
        Ok(()) => summary(payload),
        Err(why) => format!("usage: not recorded ({why})"),
    };
    outcome.human = if outcome.human.is_empty() {
        line
    } else {
        format!("{}\n{line}", outcome.human)
    };
    if let Some(o) = outcome.result.as_object_mut() {
        o.insert(
            "usage".to_owned(),
            if recorded.is_ok() {
                payload.clone()
            } else {
                serde_json::Value::Null
            },
        );
    }
}

/// The target a successful `war done` closed, from its answer.
fn done_target(outcome: &Outcome) -> Option<(String, bool)> {
    let r = &outcome.result;
    if r.get("already_done").and_then(serde_json::Value::as_bool) == Some(true) {
        return None;
    }
    let ticket = r.get("ticket").and_then(|v| v.as_str())?;
    let whole = r.get("ticket_state").and_then(|v| v.as_str()) == Some("done");
    Some(match r.get("item").and_then(|v| v.as_str()) {
        // A document with no items is done as itself.
        Some(item) if item != ticket => (format!("{ticket}/{item}"), whole),
        _ => (ticket.to_owned(), whole),
    })
}

/// After a `war done` that ticked: journal the session's usage on what it
/// closed. `report` is `--usage`'s, which is recorded as given. A failure to
/// record is said, and never undoes or refuses the tick.
pub fn after_done(root: &Utf8Path, store: &Store, outcome: &mut Outcome, report: Option<&Report>) {
    if outcome.is_refused() {
        return;
    }
    let Some((target, whole)) = done_target(outcome) else {
        return;
    };
    if let Some(r) = report {
        let payload = report_payload(&target, r);
        let recorded = journal_on(store, &target, &payload);
        attach(outcome, &payload, recorded);
        return;
    }
    if !enabled(root) {
        return;
    }
    // A `war evidence go` node's usage is its `go.ended`: not counted twice.
    if std::env::var_os("WAR_GO_PACKET").is_some() {
        return;
    }
    let Some(harness) = Harness::from_env() else {
        return;
    };
    let path = cursor_path(store, &harness);
    let _lock = lock_cursor(&path);
    let cursor = load_cursor(&path);
    let since = if cursor.is_some() {
        None
    } else {
        store.load_all().ok().and_then(|(tickets, _)| {
            match crate::ticket::resolve(&tickets, &target) {
                Ok(crate::ticket::Target::Ticket(n) | crate::ticket::Target::Item(n, _)) => {
                    claimed_since(&tickets[n], &target)
                }
                Err(_) => None,
            }
        })
    };
    let from = cursor
        .as_ref()
        .map(|c| c.since.clone())
        .or_else(|| since.clone())
        .unwrap_or_else(|| "UNKNOWN".to_owned());
    let (read, mut next) = read_window(&harness, cursor.as_ref(), since.as_deref());
    let read_ok = read.is_ok();
    let payload = window_payload(&harness, &target, &from, read);
    let recorded = journal_on(store, &target, &payload);
    if recorded.is_ok() && read_ok {
        let ticket = target.split('/').next().unwrap_or(&target).to_owned();
        next.target = (!whole).then_some(ticket);
        save_cursor(&path, &next);
    }
    attach(outcome, &payload, recorded);
}

/// After a `war start` that took or resumed `ticket`: open a window for it.
/// A window the session had open on other work is closed first, and
/// journalled on that work: the hand-over.
pub fn after_start(root: &Utf8Path, store: &Store, outcome: &mut Outcome) {
    if outcome.is_refused() || !enabled(root) || std::env::var_os("WAR_GO_PACKET").is_some() {
        return;
    }
    let Some(ticket) = outcome
        .result
        .get("ticket")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
    else {
        return;
    };
    let Some(harness) = Harness::from_env() else {
        return;
    };
    if matches!(harness, Harness::Other { .. }) {
        return;
    }
    let path = cursor_path(store, &harness);
    let _lock = lock_cursor(&path);
    let cursor = load_cursor(&path);
    match cursor.as_ref().and_then(|c| c.target.clone()) {
        // A resumption: the window stays open.
        Some(t) if t == ticket => {}
        Some(previous) => {
            let from = cursor.as_ref().map(|c| c.since.clone()).unwrap_or_default();
            let (read, mut next) = read_window(&harness, cursor.as_ref(), None);
            let read_ok = read.is_ok();
            let payload = window_payload(&harness, &previous, &from, read);
            let recorded = journal_on(store, &previous, &payload);
            if recorded.is_ok() {
                let line = format!("handed over from {previous}: {}", summary(&payload));
                outcome.human = format!("{}\n{line}", outcome.human);
            }
            if read_ok {
                next.target = Some(ticket);
                save_cursor(&path, &next);
            } else if let Some(mut fresh) = mark_now(&harness) {
                fresh.target = Some(ticket);
                save_cursor(&path, &fresh);
            }
        }
        None => {
            if let Some(mut fresh) = mark_now(&harness) {
                fresh.target = Some(ticket);
                save_cursor(&path, &fresh);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Utf8Path, name: &str, text: &str) -> Utf8PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).expect("write");
        path
    }

    fn tmp() -> (tempfile::TempDir, Utf8PathBuf) {
        let d = tempfile::tempdir().expect("tempdir");
        let p = Utf8PathBuf::from_path_buf(d.path().to_path_buf()).expect("utf8");
        (d, p)
    }

    const A: &str = r#"{"type":"assistant","timestamp":"2026-11-07T10:00:00Z","message":{"id":"m1","model":"claude-x","content":[{"type":"text","text":"SECRET-CONTENT"}],"usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":20}}}"#;
    const A2: &str = r#"{"type":"assistant","timestamp":"2026-11-07T10:00:01Z","message":{"id":"m1","model":"claude-x","content":[{"type":"tool_use","input":{"cmd":"SECRET-CONTENT"}}],"usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":20}}}"#;
    const B: &str = r#"{"type":"assistant","timestamp":"2026-11-07T11:00:00Z","message":{"id":"m2","model":"claude-y","usage":{"input_tokens":1,"output_tokens":2}}}"#;
    const U: &str = r#"{"type":"user","timestamp":"2026-11-07T10:30:00Z","message":{"role":"user","content":"usage SECRET-CONTENT","usage":{"input_tokens":999}}}"#;

    #[test]
    fn a_message_split_across_lines_counts_once_and_content_is_not_kept() {
        let (_d, dir) = tmp();
        let f = write(&dir, "s.jsonl", &format!("{A}\n{A2}\n{U}\n{B}\n"));
        let mut next = Cursor::default();
        let w = read_claude(&[f], None, None, &mut next).expect("read");
        assert_eq!(w.messages, 2);
        let x = &w.by_model["claude-x"];
        assert_eq!(
            (x.input, x.output, x.cache_read, x.cache_write, x.tokens),
            (Some(10), Some(5), Some(100), Some(20), 135)
        );
        assert_eq!(w.by_model["claude-y"].tokens, 3);
        let p = window_payload(&Harness::ClaudeCode("s".into()), "t-1", "x", Ok(w));
        let text = p.to_string();
        assert!(!text.contains("SECRET"), "{text}");
        assert!(!text.contains("\"s\""), "the session id itself is not kept");
        assert_eq!(p["tokens"], 138);
    }

    /// Refusal: a second read from the cursor counts only what was added,
    /// and a line still being written waits.
    #[test]
    fn a_cursor_reads_only_what_was_appended() {
        let (_d, dir) = tmp();
        let f = write(&dir, "s.jsonl", &format!("{A}\n"));
        let mut first = Cursor::default();
        read_claude(std::slice::from_ref(&f), None, None, &mut first).expect("read");
        std::fs::write(&f, format!("{A}\n{A2}\n{B}\n{{\"partial")).expect("append");
        let mut second = Cursor::default();
        let w =
            read_claude(std::slice::from_ref(&f), Some(&first), None, &mut second).expect("read");
        assert_eq!(w.messages, 1, "m1's second line is m1, already counted");
        assert_eq!(w.by_model["claude-y"].tokens, 3);
        let mark = &second.files[f.as_str()];
        assert!(mark.offset < std::fs::metadata(&f).expect("meta").len());
    }

    #[test]
    fn a_claim_time_bounds_the_first_read() {
        let (_d, dir) = tmp();
        let f = write(&dir, "s.jsonl", &format!("{A}\n{B}\n"));
        let since = chrono::DateTime::parse_from_rfc3339("2026-11-07T10:30:00Z")
            .expect("t")
            .with_timezone(&chrono::Utc);
        let mut next = Cursor::default();
        let w = read_claude(&[f], None, Some(&since), &mut next).expect("read");
        assert_eq!(w.by_model.keys().collect::<Vec<_>>(), ["claude-y"]);
    }

    /// Refusal: nothing read is UNKNOWN with the reason, never zero.
    #[test]
    fn a_window_with_nothing_is_unknown_not_zero() {
        let h = Harness::ClaudeCode("s".into());
        let p = window_payload(&h, "t-1", "x", Ok(Window::default()));
        assert_eq!(p["tokens"], "unknown");
        let p = window_payload(&h, "t-1", "x", Err("no transcript".into()));
        assert_eq!(p["tokens"], "unknown");
        assert_eq!(p["reason"], "no transcript");
    }

    #[test]
    fn codex_is_its_running_total_less_the_mark() {
        let (_d, dir) = tmp();
        let tc = |t: &str, i: u64, c: u64, o: u64| {
            format!(
                r#"{{"timestamp":"{t}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{i},"cached_input_tokens":{c},"output_tokens":{o}}}}}}}}}"#
            )
        };
        let ctx = r#"{"timestamp":"2026-11-07T09:00:00Z","type":"turn_context","payload":{"model":"gpt-x","cwd":"SECRET"}}"#;
        let text = format!(
            "{ctx}\n{}\n{}\n{}\n",
            tc("2026-11-07T09:00:01Z", 100, 40, 10),
            tc("2026-11-07T10:00:01Z", 300, 140, 30),
            tc("2026-11-07T10:00:02Z", 300, 140, 30),
        );
        let f = write(&dir, "r.jsonl", &text);
        let since = chrono::DateTime::parse_from_rfc3339("2026-11-07T10:00:00Z")
            .expect("t")
            .with_timezone(&chrono::Utc);
        let mut next = Cursor::default();
        let w = read_codex(&f, None, Some(&since), &mut next).expect("read");
        let g = &w.by_model["gpt-x"];
        assert_eq!(
            (g.input, g.cache_read, g.output),
            (Some(100), Some(100), Some(20))
        );
        assert_eq!(next.codex_total, Some([300, 140, 30]));
        let (total, model) = codex_tail(&f, std::fs::metadata(&f).expect("m").len());
        assert_eq!(
            (total, model.as_deref()),
            (Some([300, 140, 30]), Some("gpt-x"))
        );
    }

    // ---- opencode: a fixture database built by `sqlite3` ----------------------

    fn have_sqlite3() -> bool {
        let ok = std::process::Command::new("sqlite3")
            .arg("-version")
            .output()
            .is_ok_and(|o| o.status.success());
        if !ok {
            eprintln!("SKIPPED: sqlite3 is not installed, so the opencode fixture cannot be built");
        }
        ok
    }

    fn sqlite(db: &Utf8Path, sql: &str) {
        let o = std::process::Command::new("sqlite3")
            .arg(db.as_str())
            .arg(sql)
            .output()
            .expect("sqlite3");
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }

    /// An assistant message row: session, seq, created ms, completed ms (or
    /// not yet), model, tokens (input, output, reasoning, cache read, write), cost.
    fn msg(
        sid: &str,
        seq: i64,
        at: i64,
        done: bool,
        model: &str,
        t: [u64; 5],
        cost: f64,
    ) -> String {
        let completed = if done {
            format!(r#","completed":{}"#, at + 5)
        } else {
            String::new()
        };
        format!(
            "INSERT INTO session_message VALUES ('{sid}-{seq}','{sid}','assistant',{seq},{at},{at},\
             '{{\"text\":\"SECRET-CONTENT\",\"model\":{{\"id\":\"{model}\"}},\"cost\":{cost},\
             \"time\":{{\"created\":{at}{completed}}},\"tokens\":{{\"input\":{},\"output\":{},\
             \"reasoning\":{},\"cache\":{{\"read\":{},\"write\":{}}}}}}}');",
            t[0], t[1], t[2], t[3], t[4]
        )
    }

    const OC_SCHEMA: &str = "CREATE TABLE session_v2 (id text PRIMARY KEY, parent_id text, \
        directory text, time_created integer, time_updated integer);\
        CREATE TABLE session_message (id text PRIMARY KEY, session_id text, type text, \
        seq integer, time_created integer, time_updated integer, data text);";

    fn oc_db(dir: &Utf8Path) -> Utf8PathBuf {
        let db = dir.join("opencode.db");
        let mut sql = String::from(OC_SCHEMA);
        for (id, parent) in [
            ("ses_top", "NULL"),
            ("ses_kid", "'ses_top'"),
            ("ses_grand", "'ses_kid'"),
            ("ses_other", "NULL"),
        ] {
            sql += &format!("INSERT INTO session_v2 VALUES ('{id}',{parent},'/r',1,1);");
        }
        // Before the window.
        sql += &msg("ses_top", 1, 1_000, true, "m-a", [1000, 1000, 0, 0, 0], 1.0);
        // In it: the parent, a child (cost 0, as for subagents on a paid model),
        // a grandchild; a user message with usage-looking content; another tree.
        sql += &msg("ses_top", 2, 10_000, true, "m-a", [10, 5, 2, 100, 20], 0.5);
        sql += &msg("ses_kid", 1, 11_000, true, "m-b", [1, 2, 0, 0, 0], 0.0);
        sql += &msg("ses_grand", 1, 12_000, true, "m-b", [3, 4, 0, 0, 0], 0.0);
        sql += &msg(
            "ses_other",
            1,
            12_000,
            true,
            "m-z",
            [9999, 9999, 0, 0, 0],
            9.0,
        );
        sql += "INSERT INTO session_message VALUES ('u1','ses_top','user',3,13000,13000,\
                '{\"tokens\":{\"input\":777}}');";
        sqlite(&db, &sql);
        db
    }

    fn since(ms: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp_millis(ms).expect("time")
    }

    #[test]
    fn opencode_sums_the_session_and_every_session_under_it() {
        if !have_sqlite3() {
            return;
        }
        let (_d, dir) = tmp();
        let db = oc_db(&dir);
        let mut next = Cursor::default();
        let w = read_opencode(
            "sqlite3",
            &db,
            "ses_top",
            None,
            Some(&since(5_000)),
            &mut next,
        )
        .expect("read");
        assert_eq!(
            w.messages, 3,
            "the parent's, the child's and the grandchild's"
        );
        let a = &w.by_model["m-a"];
        // Reasoning tokens are counted as output.
        assert_eq!(
            (a.input, a.output, a.cache_read, a.cache_write, a.tokens),
            (Some(10), Some(7), Some(100), Some(20), 137)
        );
        assert_eq!(w.by_model["m-b"].tokens, 10);
        assert!(!w.by_model.contains_key("m-z"), "another session's tree");
        assert_eq!(
            w.cost,
            Some(0.5),
            "recorded as the provider gave it, 0s included"
        );
        let p = window_payload(&Harness::OpenCode("ses_top".into()), "t-1", "x", Ok(w));
        assert_eq!(
            (p["tokens"].clone(), p["source"].clone()),
            (147.into(), "opencode".into())
        );
        let text = p.to_string();
        assert!(
            !text.contains("SECRET") && !text.contains("ses_top"),
            "{text}"
        );
        // The child alone is its own tree: nothing of the parent's.
        let w = read_opencode(
            "sqlite3",
            &db,
            "ses_kid",
            None,
            Some(&since(5_000)),
            &mut Cursor::default(),
        )
        .expect("read");
        assert_eq!(w.by_model.keys().collect::<Vec<_>>(), ["m-b"]);
    }

    /// Refusal: a message before the window is not counted, a cursor counts
    /// only what came after it, and a message still being written waits.
    #[test]
    fn opencode_respects_the_window_and_the_cursor() {
        if !have_sqlite3() {
            return;
        }
        let (_d, dir) = tmp();
        let db = oc_db(&dir);
        let all = read_opencode(
            "sqlite3",
            &db,
            "ses_top",
            None,
            Some(&since(0)),
            &mut Cursor::default(),
        )
        .expect("read");
        assert_eq!(
            all.by_model["m-a"].tokens,
            2000 + 137,
            "from time 0 the old message counts"
        );
        let mut first = Cursor {
            since: "1970-01-01T00:00:05Z".into(),
            ..Cursor::default()
        };
        let w = read_opencode(
            "sqlite3",
            &db,
            "ses_top",
            None,
            Some(&since(5_000)),
            &mut first,
        )
        .expect("read");
        assert_eq!(w.messages, 3);
        assert_eq!(first.opencode["ses_top"], 2);
        sqlite(
            &db,
            &(msg("ses_top", 4, 20_000, true, "m-a", [1, 1, 0, 0, 0], 0.0)
                + &msg("ses_kid", 2, 21_000, false, "m-b", [50, 50, 0, 0, 0], 0.0)
                + &msg("ses_new", 1, 22_000, true, "m-c", [4, 0, 0, 0, 0], 0.0)),
        );
        sqlite(
            &db,
            "INSERT INTO session_v2 VALUES ('ses_new','ses_grand','/r',1,1);",
        );
        let mut second = Cursor {
            since: first.since.clone(),
            ..Cursor::default()
        };
        let w = read_opencode("sqlite3", &db, "ses_top", Some(&first), None, &mut second)
            .expect("read");
        assert_eq!(w.by_model["m-a"].tokens, 2, "only the new message");
        assert_eq!(
            w.by_model["m-c"].tokens, 4,
            "a session that began after the mark"
        );
        assert!(
            !w.by_model.contains_key("m-b"),
            "the unfinished message waits"
        );
        assert_eq!(second.opencode["ses_kid"], 1, "the mark stops before it");
        // The window with nothing new is no usage, not zero.
        let mut third = Cursor {
            since: first.since.clone(),
            ..Cursor::default()
        };
        let w = read_opencode("sqlite3", &db, "ses_top", Some(&second), None, &mut third)
            .expect("read");
        assert_eq!(w.messages, 0);
        let p = window_payload(&Harness::OpenCode("ses_top".into()), "t-1", "x", Ok(w));
        assert_eq!(p["tokens"], "unknown");
    }

    /// Refusal: no database, no sqlite3, a schema that is not opencode's, and
    /// rows that are not the shape read are each UNKNOWN with the reason, never 0.
    #[test]
    fn opencode_unreadable_is_unknown_never_zero() {
        let (_d, dir) = tmp();
        let h = Harness::OpenCode("ses_top".into());
        let unknown = |why: String| {
            let p = window_payload(&h, "t-1", "x", Err(why.clone()));
            assert_eq!(p["tokens"], "unknown");
            assert_eq!(p["reason"], why);
            assert!(!why.contains(dir.as_str()), "no path in the reason: {why}");
        };
        let no_tool = opencode_rows("no-such-sqlite3-binary", &dir.join("x.db"), "ses_top")
            .expect_err("no sqlite3");
        assert!(no_tool.contains("not installed"), "{no_tool}");
        unknown(no_tool);
        assert!(opencode_rows("sqlite3", &dir.join("x.db"), "bad id'; --").is_err());
        if !have_sqlite3() {
            return;
        }
        let missing =
            opencode_rows("sqlite3", &dir.join("absent.db"), "ses_top").expect_err("no database");
        assert!(missing.contains("could not be opened"), "{missing}");
        assert!(
            !dir.join("absent.db").exists(),
            "read-only: nothing is created"
        );
        unknown(missing);
        // A schema without the columns read.
        let db = dir.join("drift.db");
        sqlite(
            &db,
            "CREATE TABLE session_v2 (id text, parent text); \
             CREATE TABLE session_message (id text, body text);",
        );
        let drift = opencode_rows("sqlite3", &db, "ses_top").expect_err("schema");
        assert!(drift.contains("schema mismatch"), "{drift}");
        unknown(drift);
        // The columns are there but a number is not one.
        let odd = dir.join("odd.db");
        sqlite(&odd, OC_SCHEMA);
        sqlite(
            &odd,
            "INSERT INTO session_v2 VALUES ('ses_top',NULL,'/r',1,1);",
        );
        sqlite(
            &odd,
            "INSERT INTO session_message VALUES ('a','ses_top','assistant',1,1,1,\
             '{\"time\":{\"completed\":2},\"tokens\":{\"input\":\"many\"}}');",
        );
        let shape = opencode_rows("sqlite3", &odd, "ses_top").expect_err("not a number");
        assert!(shape.contains("schema mismatch"), "{shape}");
        unknown(shape);
    }

    /// Refusal: a report with no count, a stray key, or a total that is not
    /// its parts is refused.
    #[test]
    fn a_report_must_be_numbers() {
        let (_d, dir) = tmp();
        let ok = write(&dir, "ok.json", r#"{"model":"m","input":3,"output":4}"#);
        let r = read_report(ok.as_str()).expect("ok");
        assert_eq!(report_payload("t-1", &r)["tokens"], 7);
        for bad in [
            r#"{"model":"m"}"#,
            r#"{"tokens":3,"content":"hi"}"#,
            r#"{"tokens":3,"input":1}"#,
            "not json",
        ] {
            let f = write(&dir, "bad.json", bad);
            assert!(read_report(f.as_str()).is_err(), "{bad}");
        }
    }
}
