// SPDX-License-Identifier: Apache-2.0
//! The ledger's two foreign formats (OW-WAR-0148 M17; docs/LEDGER.md).
//!
//! # Agent Trace (RFC v0.1.0, <https://agent-trace.dev>)
//!
//! A trace record is `{version, id, timestamp, files[], vcs?, tool?,
//! metadata?}`; each file is `{path, conversations[]}` and each conversation
//! `{ranges[], contributor?, url?, related?}`. Storage is left to the
//! implementation.
//!
//! - **Out**: one record per commit the ledger names. `vcs` is
//!   `{type: "git", revision: <sha>}`; each file has one conversation whose
//!   contributor is `unknown` (the ledger records why, not who) and whose one
//!   range covers the file at that revision. The whys ride in `metadata`
//!   under the reverse-domain key `dev.openwarrant` (spec section 7.2).
//! - **In**: a record carrying `dev.openwarrant` gives back exactly what was
//!   written; any other gives each file an entry whose why says who
//!   contributed which lines with which tool (`source: agent-trace`).
//!
//! The spec's example says `"version": "0.1.0"` while its schema's pattern
//! (`^[0-9]+\.[0-9]+$`) admits two parts only. Out writes the example's
//! `0.1.0`; in reads any `0.1` version.
//!
//! # git-ai notes (Git AI Standard v3.0.0, `refs/notes/ai`)
//!
//! A note is an attestation section (a file path, then `  <key> <ranges>`
//! lines), a `---` line, and JSON metadata: `schema_version`
//! (`authorship/3.0.0`), `base_commit_sha`, `prompts`, and optionally
//! `sessions` (keyed by session id `s_<14 hex>`) and `humans`.
//!
//! - **Out**: the ledger adds one session, `s_` + the first 14 hex of
//!   sha256(`openwarrant:ledger`), with `agent_id {tool: openwarrant, id:
//!   ledger, model: none}` and the commit's entries in `custom_attributes`
//!   (a string map) under `dev.openwarrant.ledger`. It attests no lines: the
//!   ledger does not know who wrote them. A note git-ai already wrote is kept
//!   whole; only that session is added or replaced.
//! - **In**: the ledger's own session gives back what was written; otherwise
//!   each file with an AI attestation (`s_` or a legacy 16-hex key) gets an
//!   entry naming the tool, model and lines (`source: git-ai`).

use std::collections::BTreeMap;

use camino::Utf8Path;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{Atom, Config, Entry, Source, one_line};
use crate::go::git::git;
use crate::repo::Repository;

/// The `metadata` key (Agent Trace) and `custom_attributes` key (git-ai).
pub const VENDOR_KEY: &str = "dev.openwarrant";
pub const GIT_AI_ATTRIBUTE: &str = "dev.openwarrant.ledger";
pub const GIT_AI_REF: &str = "refs/notes/ai";
pub const GIT_AI_SCHEMA: &str = "authorship/3.0.0";
pub const AGENT_TRACE_VERSION: &str = "0.1.0";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256(s: &str) -> String {
    hex(&Sha256::digest(s.as_bytes()))
}

/// The ledger's git-ai session id: `s_` + 14 hex.
#[must_use]
pub fn session_id() -> String {
    format!("s_{}", &sha256("openwarrant:ledger")[..14])
}

/// A UUID-shaped id (version 8, RFC 9562) derived from the commit, so the
/// same ledger always exports the same record.
fn record_id(commit: &str) -> String {
    let h = sha256(&format!("openwarrant:agent-trace:{commit}"));
    let mut b: Vec<char> = h[..32].chars().collect();
    b[12] = '8';
    b[16] = ['8', '9', 'a', 'b'][(b[16].to_digit(16).unwrap_or(0) & 3) as usize];
    let s: String = b.into_iter().collect();
    format!(
        "{}-{}-{}-{}-{}",
        &s[..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..32]
    )
}

/// One file's entry within a commit, as both formats carry it.
fn carried(path: &str, e: &Entry) -> Value {
    let mut v = json!({"path": path, "date": e.date, "source": e.source.as_str(), "why": e.why});
    if let Some(w) = &e.warrant {
        v["warrant"] = json!(w);
    }
    v
}

fn uncarried(v: &Value, commit: &str) -> Option<(String, Entry)> {
    let path = v.get("path")?.as_str()?.to_owned();
    let source = Source::parse(v.get("source")?.as_str()?)?;
    Some((
        path,
        Entry {
            date: v.get("date")?.as_str()?.to_owned(),
            commit: Some(commit.to_owned()),
            warrant: v.get("warrant").and_then(Value::as_str).map(str::to_owned),
            source,
            why: v.get("why")?.as_str()?.to_owned(),
        },
    ))
}

/// Every entry with a commit, grouped by commit: commit → [(path, entry)].
#[must_use]
pub fn by_commit(atoms: &[Atom]) -> BTreeMap<String, Vec<(String, Entry)>> {
    let mut out: BTreeMap<String, Vec<(String, Entry)>> = BTreeMap::new();
    for a in atoms {
        for e in &a.entries {
            if let Some(c) = &e.commit {
                out.entry(c.clone())
                    .or_default()
                    .push((a.path.clone(), e.clone()));
            }
        }
    }
    out
}

/// A full commit id for `rev`, when git knows it.
fn full_sha(root: &Utf8Path, rev: &str) -> Option<String> {
    let r = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ],
    );
    r.ok.then(|| r.stdout.trim().to_owned())
}

fn line_count(root: &Utf8Path, sha: &str, path: &str) -> usize {
    let r = git(root, &["show", &format!("{sha}:{path}")]);
    if r.ok { r.stdout.lines().count() } else { 0 }
}

// ---- Agent Trace ---------------------------------------------------------------------

/// One Agent Trace record per commit.
#[must_use]
pub fn agent_trace_out(root: &Utf8Path, atoms: &[Atom]) -> Vec<Value> {
    let mut out = Vec::new();
    for (commit, list) in by_commit(atoms) {
        let sha = full_sha(root, &commit).unwrap_or_else(|| commit.clone());
        let date = list
            .iter()
            .map(|(_, e)| e.date.clone())
            .max()
            .unwrap_or_default();
        let files: Vec<Value> = list
            .iter()
            .map(|(p, _)| {
                let n = line_count(root, &sha, p);
                let ranges = if n == 0 {
                    json!([])
                } else {
                    json!([{"start_line": 1, "end_line": n}])
                };
                json!({"path": p, "conversations": [{"contributor": {"type": "unknown"}, "ranges": ranges}]})
            })
            .collect();
        out.push(json!({
            "version": AGENT_TRACE_VERSION,
            "id": record_id(&sha),
            "timestamp": format!("{date}T00:00:00Z"),
            "vcs": {"type": "git", "revision": sha},
            "tool": {"name": "openwarrant"},
            "files": files,
            "metadata": {VENDOR_KEY: {"ledger": list.iter().map(|(p, e)| carried(p, e)).collect::<Vec<_>>()}},
        }));
    }
    out
}

fn ranges_text(ranges: &[Value]) -> String {
    ranges
        .iter()
        .filter_map(|r| {
            let a = r.get("start_line")?.as_u64()?;
            let b = r.get("end_line")?.as_u64()?;
            Some(if a == b {
                a.to_string()
            } else {
                format!("{a}-{b}")
            })
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Records in a file: one JSON value, an array of them, or JSON lines.
///
/// # Errors
/// Text that is none of the three.
pub fn read_records(text: &str) -> Result<Vec<Value>, String> {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return Ok(match v {
            Value::Array(a) => a,
            other => vec![other],
        });
    }
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(n, l)| {
            serde_json::from_str(l).map_err(|e| format!("line {}: not JSON ({e})", n + 1))
        })
        .collect()
}

/// The entries one Agent Trace record gives, by file.
///
/// # Errors
/// A record that is not Agent Trace v0.1: the field it lacks.
pub fn agent_trace_in(record: &Value) -> Result<(String, Vec<(String, Entry)>), String> {
    let version = record
        .get("version")
        .and_then(Value::as_str)
        .ok_or("no `version`")?;
    if !(version == "0.1" || version.starts_with("0.1.")) {
        return Err(format!("version {version:?} is not Agent Trace 0.1"));
    }
    for k in ["id", "timestamp"] {
        record
            .get(k)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("no `{k}`"))?;
    }
    let files = record
        .get("files")
        .and_then(Value::as_array)
        .ok_or("no `files` array")?;
    let vcs = record
        .get("vcs")
        .ok_or("no `vcs`: the ledger needs the commit")?;
    if vcs.get("type").and_then(Value::as_str) != Some("git") {
        return Err("`vcs.type` is not git".to_owned());
    }
    let commit = vcs
        .get("revision")
        .and_then(Value::as_str)
        .filter(|r| r.len() >= 7 && r.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or("`vcs.revision` is not a git commit id")?
        .to_owned();
    let date = record["timestamp"]
        .as_str()
        .and_then(|t| t.get(..10))
        .filter(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok())
        .ok_or("`timestamp` is not RFC 3339")?
        .to_owned();
    if let Some(list) = record
        .pointer(&format!("/metadata/{VENDOR_KEY}/ledger"))
        .and_then(Value::as_array)
    {
        let out: Vec<(String, Entry)> = list.iter().filter_map(|v| uncarried(v, &commit)).collect();
        if out.len() == list.len() {
            return Ok((commit, out));
        }
        return Err(format!(
            "`metadata.{VENDOR_KEY}.ledger` has an entry that does not read"
        ));
    }
    let tool = record
        .pointer("/tool/name")
        .and_then(Value::as_str)
        .unwrap_or("an unnamed tool");
    let mut out = Vec::new();
    for f in files {
        let path = f
            .get("path")
            .and_then(Value::as_str)
            .ok_or("a file has no `path`")?;
        let convs = f
            .get("conversations")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{path}: no `conversations`"))?;
        let mut parts = Vec::new();
        for c in convs {
            let ranges = c
                .get("ranges")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("{path}: a conversation has no `ranges`"))?;
            let who = c
                .pointer("/contributor/type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let model = c
                .pointer("/contributor/model_id")
                .and_then(Value::as_str)
                .map(|m| format!(" ({m})"))
                .unwrap_or_default();
            let lines = ranges_text(ranges);
            parts.push(if lines.is_empty() {
                format!("{who}{model}")
            } else {
                format!("{who}{model} lines {lines}")
            });
        }
        out.push((
            path.to_owned(),
            Entry {
                date: date.clone(),
                commit: Some(commit.clone()),
                warrant: None,
                source: Source::AgentTrace,
                why: one_line(
                    &format!("Agent Trace via {tool}: {}", parts.join("; ")),
                    300,
                ),
            },
        ));
    }
    Ok((commit, out))
}

// ---- git-ai notes --------------------------------------------------------------------

/// A note, split: the attestation section's text and the metadata.
#[derive(Debug, Clone)]
pub struct Note {
    pub attestations: String,
    pub metadata: Value,
}

impl Note {
    /// # Errors
    /// A note without the `---` line, or whose metadata is not JSON.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut att = String::new();
        let mut rest = None;
        let mut lines = text.split_inclusive('\n');
        for l in lines.by_ref() {
            if l.trim_end_matches(['\n', '\r']) == "---" {
                rest = Some(lines.collect::<String>());
                break;
            }
            att.push_str(l);
        }
        let json = rest.ok_or("no `---` line between the attestations and the metadata")?;
        let metadata: Value =
            serde_json::from_str(&json).map_err(|e| format!("the metadata is not JSON: {e}"))?;
        if metadata.get("schema_version").and_then(Value::as_str) != Some(GIT_AI_SCHEMA) {
            return Err(format!("schema_version is not {GIT_AI_SCHEMA:?}"));
        }
        Ok(Self {
            attestations: att,
            metadata,
        })
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut s = self.attestations.clone();
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str("---\n");
        s.push_str(&serde_json::to_string_pretty(&self.metadata).unwrap_or_default());
        s.push('\n');
        s
    }

    /// `(path, [(key, ranges)])` in the attestation section.
    #[must_use]
    pub fn files(&self) -> Vec<(String, Vec<(String, String)>)> {
        let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for l in self.attestations.lines() {
            if l.trim().is_empty() {
                continue;
            }
            if let Some(entry) = l.strip_prefix("  ") {
                if let (Some(last), Some((k, r))) = (out.last_mut(), entry.split_once(' ')) {
                    last.1.push((k.to_owned(), r.to_owned()));
                }
            } else {
                let p = l
                    .strip_prefix('"')
                    .and_then(|p| p.strip_suffix('"'))
                    .unwrap_or(l);
                out.push((p.to_owned(), Vec::new()));
            }
        }
        out
    }
}

/// The note a commit carries under `refs/notes/ai`, if any.
#[must_use]
pub fn read_note(root: &Utf8Path, sha: &str) -> Option<String> {
    let r = git(root, &["notes", "--ref", GIT_AI_REF, "show", sha]);
    r.ok.then_some(r.stdout)
}

/// The note for `sha` with the ledger's session holding `list`.
#[must_use]
pub fn git_ai_note(existing: Option<Note>, sha: &str, list: &[(String, Entry)]) -> Note {
    let mut note = existing.unwrap_or_else(|| Note {
        attestations: String::new(),
        metadata: json!({"schema_version": GIT_AI_SCHEMA, "base_commit_sha": sha, "prompts": {}}),
    });
    let carried: Vec<Value> = list.iter().map(|(p, e)| carried(p, e)).collect();
    let session = json!({
        "agent_id": {"tool": "openwarrant", "id": "ledger", "model": "none"},
        "custom_attributes": {GIT_AI_ATTRIBUTE: serde_json::to_string(&carried).unwrap_or_default()},
    });
    if !note.metadata.get("sessions").is_some_and(Value::is_object) {
        note.metadata["sessions"] = json!({});
    }
    note.metadata["sessions"][session_id()] = session;
    note
}

/// Write the ledger's session into each named commit's note. The commits
/// written, and those refused with why.
#[must_use]
pub fn git_ai_out(root: &Utf8Path, atoms: &[Atom]) -> (Vec<String>, Vec<(String, String)>) {
    let mut written = Vec::new();
    let mut refused = Vec::new();
    for (commit, list) in by_commit(atoms) {
        let Some(sha) = full_sha(root, &commit) else {
            refused.push((commit, "git does not know this commit".to_owned()));
            continue;
        };
        let existing = match read_note(root, &sha).map(|t| Note::parse(&t)) {
            Some(Ok(n)) => Some(n),
            Some(Err(e)) => {
                refused.push((
                    sha,
                    format!("its git-ai note does not read ({e}); it is left as it is"),
                ));
                continue;
            }
            None => None,
        };
        let note = git_ai_note(existing, &sha, &list);
        let tmp = std::env::temp_dir().join(format!(
            "war-git-ai-{}-{}.note",
            std::process::id(),
            &sha[..12]
        ));
        if let Err(e) = std::fs::write(&tmp, note.render()) {
            refused.push((sha, format!("could not stage the note: {e}")));
            continue;
        }
        let tmp_s = tmp.to_string_lossy().into_owned();
        let r = git(
            root,
            &[
                "notes", "--ref", GIT_AI_REF, "add", "-f", "-F", &tmp_s, &sha,
            ],
        );
        let _ = std::fs::remove_file(&tmp);
        if r.ok {
            written.push(sha);
        } else {
            refused.push((sha, format!("git notes refused: {}", r.why())));
        }
    }
    (written, refused)
}

/// Every commit with a note under `refs/notes/ai`.
#[must_use]
pub fn noted_commits(root: &Utf8Path) -> Vec<String> {
    let r = git(root, &["notes", "--ref", GIT_AI_REF, "list"]);
    if !r.ok {
        return Vec::new();
    }
    let mut v: Vec<String> = r
        .stdout
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_owned))
        .collect();
    v.sort();
    v
}

/// The entries one commit's git-ai note gives, by file.
///
/// # Errors
/// A note that does not read.
pub fn git_ai_in(
    note: &Note,
    sha: &str,
    date: &str,
    warrant: Option<&str>,
) -> Result<Vec<(String, Entry)>, String> {
    if let Some(attr) = note
        .metadata
        .pointer(&format!("/sessions/{}/custom_attributes", session_id()))
        .and_then(|a| a.get(GIT_AI_ATTRIBUTE))
        .and_then(Value::as_str)
    {
        let list: Vec<Value> = serde_json::from_str(attr)
            .map_err(|e| format!("{GIT_AI_ATTRIBUTE} is not a JSON list: {e}"))?;
        let out: Vec<(String, Entry)> = list.iter().filter_map(|v| uncarried(v, sha)).collect();
        if out.len() != list.len() {
            return Err(format!(
                "{GIT_AI_ATTRIBUTE} has an entry that does not read"
            ));
        }
        return Ok(out);
    }
    let agent = |key: &str| -> String {
        let id = key.split("::").next().unwrap_or(key);
        let rec = note
            .metadata
            .pointer(&format!("/sessions/{id}"))
            .or_else(|| note.metadata.pointer(&format!("/prompts/{id}")));
        let a = rec.and_then(|r| r.get("agent_id"));
        let tool = a
            .and_then(|a| a.get("tool"))
            .and_then(Value::as_str)
            .unwrap_or("an AI tool");
        match a.and_then(|a| a.get("model")).and_then(Value::as_str) {
            Some(m) => format!("{tool} ({m})"),
            None => tool.to_owned(),
        }
    };
    let mut out = Vec::new();
    for (path, atts) in note.files() {
        let ai: Vec<String> = atts
            .iter()
            .filter(|(k, _)| !k.starts_with("h_"))
            .map(|(k, r)| format!("{} lines {r}", agent(k)))
            .collect();
        if ai.is_empty() {
            continue;
        }
        out.push((
            path,
            Entry {
                date: date.to_owned(),
                commit: Some(sha.to_owned()),
                warrant: warrant.map(str::to_owned),
                source: Source::GitAi,
                why: one_line(&format!("AI-authored (git-ai): {}", ai.join("; ")), 300),
            },
        ));
    }
    Ok(out)
}

/// Every atom, read (the ones that do not read are skipped: `war check`
/// names them).
#[must_use]
pub fn atoms(repo: &Repository, cfg: &Config) -> Vec<Atom> {
    super::load_all(repo, cfg)
        .into_iter()
        .filter_map(|(_, a)| a.ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(why: &str) -> Entry {
        Entry {
            date: "2026-10-01".to_owned(),
            commit: Some("a".repeat(40)),
            warrant: Some("t-abcd".to_owned()),
            source: Source::Commit,
            why: why.to_owned(),
        }
    }

    #[test]
    fn a_git_ai_note_round_trips_and_keeps_what_git_ai_wrote() {
        let theirs = "src/main.rs\n  abcd1234abcd1234 1-10\n---\n{\"schema_version\":\"authorship/3.0.0\",\"base_commit_sha\":\"x\",\"prompts\":{\"abcd1234abcd1234\":{\"agent_id\":{\"tool\":\"cursor\",\"id\":\"1\",\"model\":\"m\"}}}}\n";
        let n = Note::parse(theirs).unwrap();
        // Without the ledger's session, git-ai's attestation flows in.
        let got = git_ai_in(&n, &"a".repeat(40), "2026-10-01", None).unwrap();
        assert_eq!(got.len(), 1);
        assert!(
            got[0].1.why.contains("cursor (m) lines 1-10"),
            "{}",
            got[0].1.why
        );
        let list = vec![("src/main.rs".to_owned(), e("the parser"))];
        let ours = git_ai_note(Some(n), &"a".repeat(40), &list);
        let text = ours.render();
        assert!(text.starts_with("src/main.rs\n  abcd1234abcd1234 1-10\n---\n"));
        let back = Note::parse(&text).unwrap();
        assert!(back.metadata.pointer("/prompts/abcd1234abcd1234").is_some());
        assert_eq!(git_ai_in(&back, &"a".repeat(40), "x", None).unwrap(), list);
        assert!(
            Note::parse("src/a\n  k 1\n{}").is_err(),
            "no divider is refused"
        );
    }

    #[test]
    fn an_agent_trace_record_round_trips_and_a_foreign_one_reads() {
        let atoms = vec![Atom {
            path: "src/a.rs".to_owned(),
            earlier: None,
            entries: vec![e("why a")],
        }];
        let recs = agent_trace_out(Utf8Path::new("/nonexistent"), &atoms);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0]["version"], "0.1.0");
        let (c, got) = agent_trace_in(&recs[0]).unwrap();
        assert_eq!(c, "a".repeat(40));
        assert_eq!(got, vec![("src/a.rs".to_owned(), e("why a"))]);
        let foreign = json!({"version": "0.1.0", "id": "x", "timestamp": "2026-01-02T03:04:05Z",
            "vcs": {"type": "git", "revision": "b".repeat(40)}, "tool": {"name": "cursor"},
            "files": [{"path": "f", "conversations": [{"contributor": {"type": "ai", "model_id": "anthropic/x"},
            "ranges": [{"start_line": 3, "end_line": 9}]}]}]});
        let (_, got) = agent_trace_in(&foreign).unwrap();
        assert_eq!(
            got[0].1.why,
            "Agent Trace via cursor: ai (anthropic/x) lines 3-9"
        );
        assert_eq!(got[0].1.source, Source::AgentTrace);
        let mut bad = foreign.clone();
        bad["version"] = json!("2.0");
        assert!(agent_trace_in(&bad).is_err());
        let mut bad = foreign;
        bad.as_object_mut().unwrap().remove("vcs");
        assert!(agent_trace_in(&bad).is_err());
    }

    #[test]
    fn the_record_id_is_uuid_shaped_and_stable() {
        let a = record_id("abc");
        assert_eq!(a, record_id("abc"));
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "8");
    }
}
