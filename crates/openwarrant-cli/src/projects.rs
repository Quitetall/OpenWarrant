// SPDX-License-Identifier: Apache-2.0
//! The per-user list of OpenWarrant repositories (OW-WAR-0115).
//!
//! `war` from anywhere opens the hub, and the hub needs to know which
//! repositories this user works in. Nobody registers them: any `war` command
//! that opened a repository touches the list, best-effort — a failure to
//! write it never changes a command's result. `war projects` lists, adds and
//! forgets entries.
//!
//! The list stores where a project is and when it was last seen, never what
//! it says: every fact the hub shows is read from that project's own records
//! through the CLI's functions. A path that no longer holds a repository is
//! reported as missing, not dropped — a moved checkout should be found, not
//! forgotten silently.
//!
//! `$XDG_CONFIG_HOME/openwarrant/projects.toml` (default `~/.config/…`).
//! `OPENWARRANT_NO_PROJECTS=1` turns the touching off.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};

pub const SCHEMA: &str = "oh.war/projects/v1";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projects {
    #[serde(default = "schema")]
    pub schema: String,
    /// Absolute repository root → last seen (RFC 3339).
    #[serde(default)]
    pub projects: BTreeMap<String, String>,
}

fn schema() -> String {
    SCHEMA.to_owned()
}

/// Where the list lives, or `None` when neither XDG_CONFIG_HOME nor HOME is
/// set (then there is no list, and nothing is touched).
#[must_use]
pub fn path() -> Option<Utf8PathBuf> {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(Utf8PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|h| Utf8PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("openwarrant").join("projects.toml"))
}

#[must_use]
pub fn load() -> Projects {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_else(|| Projects {
            schema: SCHEMA.to_owned(),
            projects: BTreeMap::new(),
        })
}

fn save(p: &Projects) -> Result<(), String> {
    let path = path().ok_or("no XDG_CONFIG_HOME or HOME")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = toml::to_string_pretty(p).map_err(|e| e.to_string())?;
    // Write-then-rename: a crash never leaves half a list.
    let tmp = path.with_extension(format!("toml.{}", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

fn now() -> String {
    crate::gate_cmd::receipt::now_rfc3339_public()
}

/// Remember a repository this user just used. Best-effort by design: the
/// caller ignores the result, and the opt-out writes nothing.
pub fn touch(root: &Utf8Path) {
    if std::env::var("OPENWARRANT_NO_PROJECTS").is_ok_and(|v| !v.is_empty() && v != "0") {
        return;
    }
    let Ok(root) = root.canonicalize_utf8() else {
        return;
    };
    let mut p = load();
    p.projects.insert(root.to_string(), now());
    let _ = save(&p);
}

/// One row of `war projects`.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub root: String,
    pub last_seen: String,
    pub missing: bool,
    pub program: Option<String>,
    pub namespace: Option<String>,
}

#[must_use]
pub fn rows() -> Vec<Row> {
    load()
        .projects
        .into_iter()
        .map(|(root, last_seen)| {
            let config = Utf8Path::new(&root).join(crate::init::CONFIG_FILE);
            let parsed = std::fs::read_to_string(&config)
                .ok()
                .and_then(|t| toml::from_str::<toml::Value>(&t).ok());
            let field = |k: &str| {
                parsed
                    .as_ref()
                    .and_then(|v| v.get("project")?.get(k)?.as_str().map(str::to_owned))
            };
            Row {
                missing: parsed.is_none(),
                program: field("name"),
                namespace: field("namespace"),
                root,
                last_seen,
            }
        })
        .collect()
}

/// `war projects [--add <path> | --forget <path>]`.
pub fn run(add: Option<&Utf8Path>, forget: Option<&Utf8Path>) -> (Report, Vec<Row>) {
    let mut report = Report::default();
    if let Some(a) = add {
        match a.canonicalize_utf8() {
            Ok(root) if root.join(crate::init::CONFIG_FILE).is_file() => {
                let mut p = load();
                p.projects.insert(root.to_string(), now());
                match save(&p) {
                    Ok(()) => report.push(Diagnostic::pass("projects.added", root.to_string())),
                    Err(e) => report.push(Diagnostic::error(
                        "projects.unwritable",
                        root.to_string(),
                        e,
                    )),
                }
            }
            _ => report.push(Diagnostic::error(
                "projects.not-a-repository",
                a.to_string(),
                format!("{a} holds no openwarrant.toml"),
            )),
        }
    }
    if let Some(f) = forget {
        let key = f
            .canonicalize_utf8()
            .map_or_else(|_| f.to_string(), |p| p.to_string());
        let mut p = load();
        if p.projects.remove(&key).is_some() {
            match save(&p) {
                Ok(()) => report.push(Diagnostic::pass("projects.forgotten", key)),
                Err(e) => report.push(Diagnostic::error("projects.unwritable", key, e)),
            }
        } else {
            report.push(Diagnostic::error(
                "projects.unknown",
                key.clone(),
                format!("{key} is not on the list; `war projects` shows it"),
            ));
        }
    }
    let rows = rows();
    for r in &rows {
        if r.missing {
            report.push(Diagnostic::warn(
                "projects.missing",
                r.root.clone(),
                format!(
                    "{} holds no openwarrant.toml any more; `war projects --forget {}` removes it",
                    r.root, r.root
                ),
            ));
        }
    }
    (report, rows)
}

#[must_use]
pub fn render(rows: &[Row]) -> String {
    if rows.is_empty() {
        return "no projects yet: run any `war` command inside a repository and it is remembered\n"
            .to_owned();
    }
    let mut s = String::new();
    for r in rows {
        s.push_str(&format!(
            "{:<28} {:<6} {}{}\n",
            r.program.as_deref().unwrap_or("?"),
            r.namespace.as_deref().unwrap_or(""),
            r.root,
            if r.missing { "  (missing)" } else { "" }
        ));
    }
    s
}
