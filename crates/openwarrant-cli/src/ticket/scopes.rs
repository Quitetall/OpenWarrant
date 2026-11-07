// SPDX-License-Identifier: Apache-2.0
//! Local scope admission. This coordinates cooperating writers; it grants no authority.
use std::collections::{BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::{Store, Ticket};
use crate::repo::RepoError;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coordination {
    pub workspace: String,
    pub session: String,
    pub base_revision: Option<String>,
    pub paths: Vec<String>,
    pub display_name: String,
    /// The repository the work is in: `[project] name`, else the checkout's
    /// directory. Absent on claims from before it was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// That repository's `origin` URL, without any credentials in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    /// The branch checked out in `workspace`; `detached` for none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

impl Coordination {
    /// Where the work is done, in one line: the first line of the brief and
    /// what every view of the claim says. What was not recorded is UNKNOWN.
    #[must_use]
    pub fn place(&self) -> String {
        format!(
            "work in {} (repo {}, branch {}, from {})",
            self.workspace,
            self.repo.as_deref().unwrap_or("UNKNOWN"),
            self.branch.as_deref().unwrap_or("UNKNOWN"),
            self.base_revision
                .as_deref()
                .map_or("UNKNOWN", |b| &b[..b.len().min(12)])
        )
    }

    /// Record the repository, remote and branch of `workspace` (git reads
    /// only; none of it is required).
    pub fn locate(&mut self, store: &Store, workspace: &camino::Utf8Path) {
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(workspace)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .stderr(std::process::Stdio::null())
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                .filter(|s| !s.is_empty())
        };
        self.branch = git(&["symbolic-ref", "--short", "-q", "HEAD"])
            .or_else(|| git(&["rev-parse", "--verify", "-q", "HEAD"]).map(|_| "detached".into()));
        self.remote_url =
            git(&["config", "--get", "remote.origin.url"]).map(|u| without_userinfo(&u));
        self.repo = Some(store.project.trim().to_owned())
            .filter(|p| !p.is_empty())
            .or_else(|| {
                git(&["rev-parse", "--show-toplevel"])
                    .and_then(|t| t.rsplit('/').next().map(str::to_owned))
            });
    }
}

/// A URL with any `user:secret@` taken out: a token in a remote must not reach
/// a claim, a journal or a view.
fn without_userinfo(url: &str) -> String {
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let host_end = rest.find('/').unwrap_or(rest.len());
            match rest[..host_end].rfind('@') {
                Some(at) => format!("{scheme}://{}", &rest[at + 1..]),
                None => url.to_owned(),
            }
        }
        None => url.to_owned(),
    }
}

pub fn session() -> String {
    for name in [
        "OPENWARRANT_SESSION_ID",
        "KATANA_SESSION_ID",
        "CODEX_THREAD_ID",
        "CODEX_SESSION_ID",
        "CLAUDE_CODE_SESSION_ID",
    ] {
        if let Ok(value) = std::env::var(name)
            && !value.is_empty()
        {
            return format!(
                "{name}:{}",
                openwarrant_compiler::sha256_hex(value.as_bytes())
            );
        }
    }
    #[cfg(unix)]
    if let Ok(id) = rustix::process::getsid(None) {
        return format!("local-session:{}", id.as_raw_pid());
    }
    format!("local-process:{}", std::process::id())
}

/// Whether the process asking is the one that took this claim. A harness names
/// its session, and that name decides. With no name, `session()` falls back to
/// the operating system's session, which every new terminal changes, so `war
/// start` in one and `war done` in the next would disown the holder's own
/// claim: there the worktree decides, one actor to a worktree.
pub fn same_holder(store: &Store, claim: &Coordination) -> bool {
    holder_matches(claim, &session(), &workspace(store))
}

fn holder_matches(claim: &Coordination, session: &str, workspace: &str) -> bool {
    let unnamed = |s: &str| s.starts_with("local-session:") || s.starts_with("local-process:");
    claim.session == session
        || (unnamed(&claim.session) && unnamed(session) && claim.workspace == workspace)
}

pub fn workspace(store: &Store) -> String {
    store
        .root
        .canonicalize_utf8()
        .unwrap_or_else(|_| store.root.clone())
        .to_string()
}

pub struct AdmissionLock(std::fs::File);
impl AdmissionLock {
    pub fn acquire(store: &Store) -> Result<Self, RepoError> {
        std::fs::create_dir_all(&store.claims_dir)
            .map_err(super::io("could not create claim store".into()))?;
        let path = store.claims_dir.join(".scope-admission");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(super::io("could not open scope admission lock".into()))?;
        #[cfg(unix)]
        rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)
            .map_err(|e| RepoError::Message(format!("scope lock: {e}")))?;
        Ok(Self(file))
    }
}
impl Drop for AdmissionLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        let _ = rustix::fs::flock(&self.0, rustix::fs::FlockOperation::Unlock);
    }
}

fn normal(store: &Store, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 4096
        || value.contains('\0')
        || value.starts_with('/')
        || value.split('/').any(|p| p == "..")
    {
        return Err(format!(
            "{value:?}: scope needs a nonempty repository-relative path/glob without traversal"
        ));
    }
    let mut value = value
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect::<Vec<_>>()
        .join("/");
    if value.is_empty() {
        value = "**".into();
    }
    if !value.contains(['*', '?', '[', '{']) {
        let path = store.root.join(&value);
        if let Ok(real) = path.canonicalize_utf8() {
            let root = store.root.canonicalize_utf8().map_err(|e| e.to_string())?;
            value = real
                .strip_prefix(&root)
                .map_err(|_| "scope resolves outside this repository".to_owned())?
                .to_string();
            if real.is_dir() {
                value.push_str("/**");
            }
        }
    }
    Ok(value)
}

pub fn of(store: &Store, t: &Ticket, provided: &[String]) -> Result<Coordination, String> {
    let mut paths = provided.to_vec();
    if paths.is_empty() && t.format {
        let doc = openwarrant_core::format::parse(t.checklist_text.as_bytes());
        if let Some(value) = doc.fields.get("paths") {
            paths = match value {
                openwarrant_core::format::FieldValue::One(v) => vec![v.clone()],
                openwarrant_core::format::FieldValue::Many(v) => v.clone(),
            };
        }
    }
    if paths.is_empty() {
        paths = crate::go::packet::allowed_of_ticket(t).paths;
    }
    if paths.is_empty() {
        paths.push("**".into());
    }
    let mut paths = paths
        .iter()
        .map(|p| normal(store, p))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    paths.dedup();
    let title = crate::interop::slug(&t.manifest.title);
    Ok(Coordination {
        workspace: workspace(store),
        session: session(),
        base_revision: super::ladder::head_commit(&store.root),
        paths,
        repo: None,
        remote_url: None,
        branch: None,
        display_name: format!(
            "{}-worker-{}",
            title.chars().take(48).collect::<String>(),
            t.id()
        ),
    })
}

#[derive(Clone, Copy)]
enum Char {
    Literal(char),
    NotSlash,
    Any,
}
#[derive(Default)]
struct State {
    epsilon: Vec<usize>,
    edges: Vec<(Char, usize)>,
}
fn edge_overlap(a: Char, b: Char) -> bool {
    match (a, b) {
        (Char::Literal(a), Char::Literal(b)) => a == b,
        (Char::NotSlash, Char::Literal('/')) | (Char::Literal('/'), Char::NotSlash) => false,
        _ => true,
    }
}
fn automaton(pattern: &str) -> Result<Vec<State>, String> {
    let chars: Vec<_> = pattern.chars().collect();
    let mut states = vec![State::default()];
    let mut at = 0;
    let mut i = 0;
    while i < chars.len() {
        let next = states.len();
        states.push(State::default());
        match chars[i] {
            '[' | ']' | '{' | '}' | '\\' => {
                return Err(
                    "scope uses an unsupported glob construct; use literal paths, *, ** or ?"
                        .into(),
                );
            }
            '*' if chars.get(i + 1) == Some(&'*') && chars.get(i + 2) == Some(&'/') => {
                let part = states.len();
                states.push(State::default());
                states[at].epsilon.push(next);
                states[at].edges.push((Char::NotSlash, part));
                states[part].edges.push((Char::NotSlash, part));
                states[part].edges.push((Char::Literal('/'), at));
                i += 2;
            }
            '*' if chars.get(i + 1) == Some(&'*') => {
                states[at].epsilon.push(next);
                states[at].edges.push((Char::Any, at));
                i += 1;
            }
            '*' => {
                states[at].epsilon.push(next);
                states[at].edges.push((Char::NotSlash, at));
            }
            '?' => states[at].edges.push((Char::NotSlash, next)),
            c => states[at].edges.push((Char::Literal(c), next)),
        }
        at = next;
        i += 1;
    }
    // The accepting state is `at`, not necessarily the last state (the **/ helper).
    let accept = states.len();
    states.push(State::default());
    states[at].epsilon.push(accept);
    Ok(states)
}

/// Intersection, including paths not created yet. An undecidable/bounded query
/// is never interpreted as disjoint. On macOS unknown Unicode normalisation is conservative.
pub fn overlaps(a: &str, b: &str) -> Result<bool, String> {
    if cfg!(target_os = "macos") && (!a.is_ascii() || !b.is_ascii()) {
        return Err("Unicode volume normalisation is unknown; use isolated worktrees".into());
    }
    let a = if cfg!(target_os = "macos") {
        a.to_lowercase()
    } else {
        a.into()
    };
    let b = if cfg!(target_os = "macos") {
        b.to_lowercase()
    } else {
        b.into()
    };
    let a = automaton(&a)?;
    let b = automaton(&b)?;
    let mut queue = VecDeque::from([(0, 0)]);
    let mut seen = BTreeSet::new();
    while let Some((x, y)) = queue.pop_front() {
        if !seen.insert((x, y)) {
            continue;
        }
        if seen.len() > 65_536 {
            return Err("scope intersection exceeds the computation bound; use simpler scopes or isolated worktrees".into());
        }
        if x + 1 == a.len() && y + 1 == b.len() {
            return Ok(true);
        }
        queue.extend(a[x].epsilon.iter().map(|n| (*n, y)));
        queue.extend(b[y].epsilon.iter().map(|n| (x, *n)));
        for (ca, na) in &a[x].edges {
            for (cb, nb) in &b[y].edges {
                if edge_overlap(*ca, *cb) {
                    queue.push_back((*na, *nb));
                }
            }
        }
    }
    Ok(false)
}

/// Whether a claim says nothing about what it will write: `**` is what a
/// claim gets when nobody declared paths. It overlaps everything, so treating
/// it as a real scope would refuse every second writer and hide all other
/// work from `war next`; ordinary work is never blocked by a default.
pub fn undeclared(c: &Coordination) -> bool {
    c.paths.iter().any(|p| p == "**")
}

/// Shared by scheduling. An unsupported pattern is a conflict until a caller
/// can establish that the scopes are disjoint; a claim that declared nothing
/// conflicts with nothing ([`undeclared`]).
pub fn conflict(a: &Coordination, b: &Coordination) -> Option<String> {
    if undeclared(a) || undeclared(b) {
        return None;
    }
    for wanted in &a.paths {
        for held in &b.paths {
            match overlaps(wanted, held) {
                Ok(false) => {}
                Ok(true) => return Some(format!("{wanted} overlaps {held}")),
                Err(why) => return Some(format!("overlap cannot be ruled out: {why}")),
            }
        }
    }
    None
}

#[cfg(test)]
mod holder_tests {
    use super::{Coordination, holder_matches};

    fn claim(session: &str, workspace: &str) -> Coordination {
        Coordination {
            workspace: workspace.into(),
            session: session.into(),
            base_revision: None,
            paths: vec![],
            display_name: String::new(),
            repo: None,
            remote_url: None,
            branch: None,
        }
    }

    #[test]
    fn a_remote_loses_its_credentials_and_a_place_says_unknown() {
        assert_eq!(
            super::without_userinfo("https://u:tok@github.com/o/r.git"),
            "https://github.com/o/r.git"
        );
        assert_eq!(
            super::without_userinfo("git@github.com:o/r.git"),
            "git@github.com:o/r.git"
        );
        let mut c = claim("s", "/w");
        assert_eq!(
            c.place(),
            "work in /w (repo UNKNOWN, branch UNKNOWN, from UNKNOWN)"
        );
        c.repo = Some("wmd".into());
        c.branch = Some("agent/x".into());
        c.base_revision = Some("1a2b3c4d5e6f7a8b".into());
        assert_eq!(
            c.place(),
            "work in /w (repo wmd, branch agent/x, from 1a2b3c4d5e6f)"
        );
    }

    #[test]
    fn a_new_terminal_in_the_same_worktree_is_the_same_holder() {
        let c = claim("local-session:10", "/w");
        assert!(holder_matches(&c, "local-session:20", "/w"));
        assert!(holder_matches(&c, "local-process:30", "/w"));
    }

    #[test]
    fn another_worktree_or_a_named_session_is_not() {
        let unnamed = claim("local-session:10", "/w");
        assert!(!holder_matches(&unnamed, "local-session:20", "/other"));
        assert!(!holder_matches(&unnamed, "CODEX_THREAD_ID:ab", "/w"));
        let named = claim("CODEX_THREAD_ID:ab", "/w");
        assert!(!holder_matches(&named, "local-session:20", "/w"));
        assert!(!holder_matches(&named, "CODEX_THREAD_ID:cd", "/w"));
        assert!(holder_matches(&named, "CODEX_THREAD_ID:ab", "/other"));
    }
}
