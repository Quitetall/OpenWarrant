// SPDX-License-Identifier: Apache-2.0
//! Intake (OW-WAR-0141): a ticket, or a sentence, becomes a draft request.
//!
//! Work elsewhere starts from a prompt or a ticket and nothing more. This is
//! the part of `war plan` that reads the ticket. It does three things and no
//! others:
//!
//! - reads an issue from a file (`gh issue view <n> --json …` output), or
//!   through the configured `[intake] fetch_argv` — off by default, and the
//!   one process intake ever starts;
//! - turns it into the sentence `war plan` already takes, and an
//!   `oh.war/intake/v1` record naming where it came from;
//! - names the pre-Warrant store, `docs/intake/<key>/`, where a question
//!   waits when the input is too thin to draft (U-002).
//!
//! What it never does: write to a tracker, hold a credential, or decide
//! anything. §75.4: an adapter has no authority, and the fetch command is an
//! adapter. §74.8: the issue's text is recorded as the request it is — a
//! digest of it, never a claim that it is evidence of anything. A body that
//! says "pre-approved" is still just a body.

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::repo::{IntakePolicy, RepoError, Repository};

pub const SCHEMA: &str = "oh.war/intake/v1";
/// The pre-Warrant store: one directory per input, no alias allocated.
pub const STORE: &str = "docs/intake";
/// The tracker this build reads (`docs/agents/issue-tracker.md`). The field
/// leaves room for another tracker's adapter; nothing here pretends to one.
pub const TRACKER_GITHUB: &str = "github";
/// Recorded under a Warrant's `plan/` beside `request.json`.
pub const RECORD_FILE: &str = "intake.json";
/// The issue as intake read it: the four fields it uses, nothing else.
pub const ISSUE_FILE: &str = "issue.json";

/// Subcommand words that change an issue. A `fetch_argv` naming one is not a
/// read, and is refused before any process starts.
const WRITE_WORDS: [&str; 10] = [
    "close", "comment", "edit", "delete", "reopen", "lock", "unlock", "transfer", "create", "pin",
];

/// `oh.war/intake/v1`: where a drafted Warrant came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntakeRecord {
    pub schema: String,
    pub tracker: String,
    pub id: String,
    /// The issue's public URL, stripped of any user-info, query or fragment:
    /// no token rides into a record on it.
    pub url: String,
    pub title: String,
    /// `sha256:<hex>` of the body the draft request was made from. The body
    /// itself is the request's sentence, and it is not evidence.
    pub body_sha256: String,
    pub fetched_at: String,
}

/// An issue, reduced to what intake uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub url: String,
}

impl Issue {
    /// Parse `gh issue view --json` output. A missing `number` or `title` is
    /// refused by name (A-001): a ticket with no id cannot be referenced, and
    /// one with no title cannot be drafted from.
    pub fn parse(text: &str, origin: &str) -> Result<Self, RepoError> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| {
            RepoError::Message(format!("intake.issue-malformed: {origin} is not JSON: {e}"))
        })?;
        let number = v
            .get("number")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "intake.issue-missing-number: {origin} has no `number`; an issue with no id \
                     cannot be linked. Nothing was written"
                ))
            })?;
        let title = v
            .get("title")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "intake.issue-missing-title: {origin} has no `title`; there is nothing to \
                     draft from. Nothing was written"
                ))
            })?
            .to_owned();
        let body = v
            .get("body")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let url = public_url(
            v.get("url")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
        );
        Ok(Self {
            number,
            title,
            body,
            url,
        })
    }

    /// The sentence `war plan` drafts from: the title, then the body.
    #[must_use]
    pub fn sentence(&self) -> String {
        let body = self.body.trim();
        if body.is_empty() {
            self.title.clone()
        } else {
            format!("{}\n\n{body}", self.title)
        }
    }

    #[must_use]
    pub fn key(&self) -> String {
        format!("{TRACKER_GITHUB}-{}", self.number)
    }

    #[must_use]
    pub fn record(&self) -> IntakeRecord {
        IntakeRecord {
            schema: SCHEMA.to_owned(),
            tracker: TRACKER_GITHUB.to_owned(),
            id: self.number.to_string(),
            url: self.url.clone(),
            title: self.title.clone(),
            body_sha256: format!(
                "sha256:{}",
                openwarrant_compiler::sha256_hex(self.body.as_bytes())
            ),
            fetched_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        }
    }
}

/// Drop user-info, query and fragment from a URL. What is left is the
/// issue's public address; a token pasted into a URL does not survive.
fn public_url(url: &str) -> String {
    let url = url.trim();
    let cut = url.find(['?', '#']).unwrap_or(url.len());
    let url = &url[..cut];
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
            let host = authority.rsplit('@').next().unwrap_or(authority);
            if path.is_empty() {
                format!("{scheme}://{host}")
            } else {
                format!("{scheme}://{host}/{path}")
            }
        }
        None => url.to_owned(),
    }
}

/// `--issue-file <path>`.
pub fn read_file(path: &Utf8Path) -> Result<Issue, RepoError> {
    let text = std::fs::read_to_string(path).map_err(|source| RepoError::Io {
        context: format!("intake.issue-unreadable: could not read {path}"),
        source,
    })?;
    Issue::parse(&text, path.as_str())
}

/// `--issue <id>`: run `[intake] fetch_argv` with `{id}` substituted.
///
/// With no `[intake]` table this is refused by name and no process starts.
/// The command's own store holds any token (`gh auth`); `war` passes the
/// environment through untouched and records none of it.
pub fn fetch(repo: &Repository, id: &str) -> Result<Issue, RepoError> {
    let id = id.trim().trim_start_matches('#');
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(RepoError::Message(format!(
            "intake.issue-id: {id:?} is not an issue number"
        )));
    }
    let Some(policy) = repo.intake_policy()? else {
        return Err(RepoError::Message(
            "intake.not-configured: `--issue` fetches through `[intake] fetch_argv`, and this \
             repository has no [intake] table. Nothing was started. Pass the issue as a file \
             (`gh issue view <n> --json number,title,body,url > f; war plan --issue-file f`) or \
             configure a fetch command"
                .to_owned(),
        ));
    };
    let argv = fetch_argv(&policy, id)?;
    run_fetch(repo, &argv, policy.fetch_timeout(), id)
}

/// The argv to run, `{id}` substituted, refused when it is not a read.
fn fetch_argv(policy: &IntakePolicy, id: &str) -> Result<Vec<String>, RepoError> {
    if policy.fetch_argv.is_empty() {
        return Err(RepoError::Message(
            "intake.not-configured: [intake] has no fetch_argv. Nothing was started".to_owned(),
        ));
    }
    if !policy.fetch_argv.iter().any(|a| a.contains("{id}")) {
        return Err(RepoError::Message(
            "intake.fetch-argv-no-id: no element of [intake] fetch_argv contains `{id}`, so every \
             issue would fetch the same thing. Nothing was started"
                .to_owned(),
        ));
    }
    if let Some(word) = policy.fetch_argv[1..]
        .iter()
        .find(|a| WRITE_WORDS.contains(&a.as_str()) || a.as_str() == "api")
    {
        return Err(RepoError::Message(format!(
            "intake.fetch-not-a-read: [intake] fetch_argv names `{word}`. Intake reads a ticket \
             and never writes one, and `api` can do either. Nothing was started"
        )));
    }
    Ok(policy
        .fetch_argv
        .iter()
        .map(|a| a.replace("{id}", id))
        .collect())
}

fn run_fetch(
    repo: &Repository,
    argv: &[String],
    deadline: Duration,
    id: &str,
) -> Result<Issue, RepoError> {
    let started = Instant::now();
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&repo.root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| RepoError::Io {
            context: format!("intake.fetch-failed: could not run {}", argv[0]),
            source,
        })?;
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.read_to_string(&mut out);
        let mut err = String::new();
        let _ = stderr.read_to_string(&mut err);
        (out, err)
    });
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|source| RepoError::Io {
            context: "intake.fetch-failed: could not wait for the fetch command".to_owned(),
            source,
        })? {
            break Some(s);
        }
        if started.elapsed() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let Some(status) = status else {
        // Not joined: a grandchild still holding the pipe would hold this
        // command past the bound it just enforced.
        drop(reader);
        return Err(RepoError::Message(format!(
            "intake.fetch-timeout: the fetch command did not finish within {}s and was killed. \
             Nothing was written",
            deadline.as_secs()
        )));
    };
    let (out, err) = reader.join().unwrap_or_default();
    if !status.success() {
        // The stderr of a tracker CLI can name the account; one line of it,
        // bounded, is enough to act on and is not written anywhere.
        let first = err
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(200)
            .collect::<String>();
        return Err(RepoError::Message(format!(
            "intake.fetch-failed: the fetch command failed ({status}): {first}. Nothing was \
             written"
        )));
    }
    let issue = Issue::parse(&out, &format!("the fetch of issue {id}"))?;
    if issue.number.to_string() != id {
        return Err(RepoError::Message(format!(
            "intake.fetch-mismatch: asked for issue {id}, the fetch command answered with issue \
             {}. Nothing was written",
            issue.number
        )));
    }
    Ok(issue)
}

/// The store key for a sentence with no ticket: its digest, so the same
/// sentence finds the same questions.
#[must_use]
pub fn sentence_key(sentence: &str) -> String {
    let digest = openwarrant_compiler::sha256_hex(sentence.trim().as_bytes());
    format!("sentence-{}", &digest[..12])
}

/// A key names a directory directly under `docs/intake/`, and nothing else.
#[must_use]
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && (key.starts_with("github-") || key.starts_with("sentence-"))
}

/// A drafter's question id becomes a file name under `questions/`.
#[must_use]
pub fn valid_question_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[must_use]
pub fn store_dir(repo: &Repository, key: &str) -> Utf8PathBuf {
    repo.root.join(STORE).join(key)
}

/// The intake directory for `key`, when one exists.
#[must_use]
pub fn existing_dir(repo: &Repository, key: &str) -> Option<Utf8PathBuf> {
    if !valid_key(key) {
        return None;
    }
    let dir = store_dir(repo, key);
    std::fs::symlink_metadata(&dir)
        .ok()
        .filter(std::fs::Metadata::is_dir)
        .map(|_| dir)
}

/// Every key with a directory under `docs/intake/`, sorted.
pub fn keys(repo: &Repository) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(repo.root.join(STORE)) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|k| valid_key(k))
        .collect();
    keys.sort();
    keys
}

/// Quote one argument for a POSIX shell, so a re-draft command copied from
/// `war next` runs the sentence it names and nothing else.
#[must_use]
pub fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:@".contains(c))
    {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_issue_needs_a_number_and_a_title_by_name() {
        let ok = Issue::parse(
            r#"{"number":12,"title":"Add a changelog","body":"b","url":"https://github.com/o/r/issues/12","labels":[{"name":"approved"}]}"#,
            "f",
        )
        .unwrap();
        assert_eq!(ok.number, 12);
        assert_eq!(ok.key(), "github-12");
        let e = Issue::parse(r#"{"number":12,"body":"b"}"#, "f").unwrap_err();
        assert!(e.to_string().starts_with("intake.issue-missing-title"));
        let e = Issue::parse(r#"{"title":"t"}"#, "f").unwrap_err();
        assert!(e.to_string().starts_with("intake.issue-missing-number"));
        let e = Issue::parse(r#"{"number":12,"title":"   "}"#, "f").unwrap_err();
        assert!(e.to_string().starts_with("intake.issue-missing-title"));
    }

    #[test]
    fn a_url_keeps_its_address_and_loses_any_secret() {
        assert_eq!(
            public_url("https://x-access-token:ghp_secret@github.com/o/r/issues/1?token=t#c"),
            "https://github.com/o/r/issues/1"
        );
        assert_eq!(
            public_url("https://github.com/o/r/issues/1"),
            "https://github.com/o/r/issues/1"
        );
    }

    #[test]
    fn a_fetch_argv_that_writes_is_refused_before_it_runs() {
        let p = |argv: &[&str]| IntakePolicy {
            fetch_argv: argv.iter().map(|s| (*s).to_owned()).collect(),
            ..IntakePolicy::default()
        };
        assert_eq!(
            fetch_argv(
                &p(&["gh", "issue", "view", "{id}", "--json", "number,title"]),
                "7"
            )
            .unwrap()[3],
            "7"
        );
        for bad in [
            &["gh", "issue", "close", "{id}"][..],
            &["gh", "issue", "comment", "{id}"][..],
            &["gh", "api", "repos/o/r/issues/{id}"][..],
        ] {
            let e = fetch_argv(&p(bad), "7").unwrap_err().to_string();
            assert!(e.starts_with("intake.fetch-not-a-read"), "{e}");
        }
        assert!(
            fetch_argv(&p(&["gh", "issue", "view"]), "7")
                .unwrap_err()
                .to_string()
                .starts_with("intake.fetch-argv-no-id")
        );
    }

    #[test]
    fn keys_name_one_directory_and_quoting_survives_a_quote() {
        assert!(valid_key("github-12"));
        assert!(valid_key(&sentence_key("add a changelog")));
        assert!(!valid_key("../x"));
        assert!(!valid_key("OW-WAR-0001"));
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(
            shell_quote("docs/intake/github-1/issue.json"),
            "docs/intake/github-1/issue.json"
        );
    }
}
