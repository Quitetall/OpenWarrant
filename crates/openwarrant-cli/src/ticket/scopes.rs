// SPDX-License-Identifier: Apache-2.0
//! Local scope admission. This coordinates cooperating writers; it grants no authority.
use std::collections::{BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::{Store, Ticket};
use crate::repo::RepoError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coordination {
    pub workspace: String,
    pub session: String,
    pub base_revision: Option<String>,
    pub paths: Vec<String>,
    pub display_name: String,
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

/// Shared by admission and scheduling. An unsupported pattern is a conflict
/// until a caller can establish that the scopes are disjoint.
pub fn conflict(a: &Coordination, b: &Coordination) -> Option<String> {
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
