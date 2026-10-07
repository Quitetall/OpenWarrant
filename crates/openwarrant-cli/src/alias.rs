// SPDX-License-Identifier: Apache-2.0
//! Alias safety across branches (M11).
//!
//! On 2026-10-07 `main` and a feature branch each allocated `OW-WAR-0148` in
//! parallel; the unsigned one had to be renumbered by hand, and `war check`
//! said nothing. Three things close that:
//!
//! - **`war check` refuses a shared alias** (`warrant.alias-duplicate`): two
//!   Warrants whose directory names or `local_alias` fields name the same
//!   alias, both UUIDs named.
//! - **`war plan new` allocates past every branch.** The next ordinal is one past
//!   the highest under the Warrants directory in the working tree AND on
//!   every local and remote-tracking branch, read with one `git
//!   for-each-ref` and one `git cat-file --batch` (no checkout, no per-branch
//!   process).
//! - **`war admin renumber <alias> <new>`** moves an unsigned Warrant to a free
//!   alias: `local_alias`, the directory, and a journal line. A Warrant with
//!   an `authorization.toml` is refused by name: its alias is in what was
//!   signed.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read as _, Write as _};
use std::process::{Command, Stdio};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{Loaded, RepoError, Repository};

/// The ordinal of an alias (`OW-WAR-0148` → 148), as `war plan new` reads one.
#[must_use]
pub fn ordinal(alias: &str) -> Option<u32> {
    alias
        .rsplit_once("-WAR-")
        .and_then(|(_, digits)| digits.parse::<u32>().ok())
}

/// Every directory name under the Warrants directory on every local and
/// remote-tracking branch, by branch. Empty outside git, or when git cannot
/// be run: the working tree still counts, and nothing is claimed about the
/// branches.
#[must_use]
pub fn on_branches(repo: &Repository) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let rel = repo.relative(&repo.warrants_dir());
    let rel = rel.trim_end_matches('/');
    let Ok(refs) = Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(objectname) %(refname:short)",
            "refs/heads",
            "refs/remotes",
        ])
        .current_dir(&repo.root)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return out;
    };
    if !refs.status.success() {
        return out;
    }
    let listed = String::from_utf8_lossy(&refs.stdout).into_owned();
    let branches: Vec<(&str, &str)> = listed
        .lines()
        .filter_map(|l| l.split_once(' '))
        .filter(|(_, name)| !name.ends_with("/HEAD"))
        .collect();
    if branches.is_empty() {
        return out;
    }
    let Ok(mut child) = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(&repo.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return out;
    };
    let mut request = String::new();
    for (sha, _) in &branches {
        request.push_str(&format!("{sha}:{rel}\n"));
    }
    // Written from a thread: the answers can outgrow a pipe while the
    // requests are still being written.
    let mut stdin = child.stdin.take().expect("piped");
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(request.as_bytes());
    });
    let mut bytes = Vec::new();
    let _ = child.stdout.take().expect("piped").read_to_end(&mut bytes);
    let _ = writer.join();
    let _ = child.wait();
    let mut at = 0usize;
    for (_, branch) in &branches {
        let Some(nl) = bytes[at..].iter().position(|&b| b == b'\n') else {
            break;
        };
        let header = String::from_utf8_lossy(&bytes[at..at + nl]).into_owned();
        at += nl + 1;
        let mut parts = header.split(' ');
        let (Some(sha), Some(kind), Some(size)) = (parts.next(), parts.next(), parts.next()) else {
            // `<object> missing`: the branch has no Warrants directory.
            continue;
        };
        let Ok(size) = size.parse::<usize>() else {
            continue;
        };
        let body = bytes.get(at..at + size).unwrap_or_default();
        at += size + 1;
        if kind != "tree" {
            continue;
        }
        let names = out
            .entry((*branch).to_owned())
            .or_insert_with(BTreeSet::new);
        tree_dirs(body, sha.len() / 2, names);
    }
    out
}

/// The directory entries of a raw tree object.
fn tree_dirs(mut body: &[u8], hash_len: usize, names: &mut BTreeSet<String>) {
    while !body.is_empty() {
        let Some(space) = body.iter().position(|&b| b == b' ') else {
            return;
        };
        let mode = &body[..space];
        let rest = &body[space + 1..];
        let Some(nul) = rest.iter().position(|&b| b == 0) else {
            return;
        };
        let name = String::from_utf8_lossy(&rest[..nul]).into_owned();
        if mode == b"40000" {
            names.insert(name);
        }
        let next = nul + 1 + hash_len;
        if rest.len() < next {
            return;
        }
        body = &rest[next..];
    }
}

/// `war check`'s alias rule over the whole corpus: an alias names one
/// Warrant. Each Warrant answers to its directory's name and to its
/// manifest's `local_alias`; two Warrants answering to one alias are each an
/// error, both UUIDs named. With `only`, the findings that touch that alias.
pub fn check_duplicates(
    repo: &Repository,
    corpus: &[Loaded],
    only: Option<&str>,
    report: &mut Report,
) {
    let mut by_alias: BTreeMap<String, Vec<(&Loaded, String)>> = BTreeMap::new();
    for one in corpus {
        let dir_name = one.dir.file_name().unwrap_or_default().to_owned();
        let uuid = one
            .validated
            .as_ref()
            .map_or_else(|| "(no valid uuid)".to_owned(), |v| v.uuid.to_string());
        let mut aliases = BTreeSet::from([dir_name]);
        if let Some(v) = &one.validated {
            aliases.insert(v.raw.local_alias.clone());
        }
        for a in aliases {
            by_alias.entry(a).or_default().push((one, uuid.clone()));
        }
    }
    for (alias, holders) in by_alias {
        if holders.len() < 2 {
            continue;
        }
        if only.is_some_and(|o| {
            o != alias && !holders.iter().any(|(l, _)| l.dir.file_name() == Some(o))
        }) {
            continue;
        }
        let named: Vec<String> = holders
            .iter()
            .map(|(l, uuid)| {
                let local = l
                    .validated
                    .as_ref()
                    .map(|v| v.raw.local_alias.clone())
                    .unwrap_or_default();
                format!(
                    "{} (uuid {uuid}, local_alias {local})",
                    repo.relative(&l.dir)
                )
            })
            .collect();
        let unsigned: Vec<String> = holders
            .iter()
            .filter(|(l, _)| !crate::vfs::is_file(l.dir.join("authorization.toml")))
            .filter_map(|(l, _)| l.dir.file_name().map(str::to_owned))
            .collect();
        let fix = match unsigned.first() {
            Some(a) => format!(
                "; give one a free alias with `war admin renumber {a} <new alias>` (an unsigned \
                 Warrant only)"
            ),
            None => String::new(),
        };
        report.push(Diagnostic::error(
            "warrant.alias-duplicate",
            repo.relative(&holders[0].0.dir.join("manifest.toml")),
            format!(
                "{alias} names {} Warrants: {}. An alias names one Warrant{fix}",
                holders.len(),
                named.join(" and ")
            ),
        ));
    }
}

/// What `war admin renumber` did.
#[derive(Debug)]
pub struct Renumbered {
    pub report: Report,
    pub human: String,
    pub result: serde_json::Value,
}

fn refused(rule: &str, file: String, message: String) -> Renumbered {
    let mut report = Report::default();
    report.push(Diagnostic::error(rule, file, message.clone()));
    Renumbered {
        report,
        human: message,
        result: serde_json::Value::Null,
    }
}

/// `war admin renumber <alias> <new>`: give an unsigned Warrant a free alias.
pub fn renumber(repo: &Repository, alias: &str, new: &str) -> Result<Renumbered, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let manifest_path = dir.join("manifest.toml");
    let rel_manifest = repo.relative(&manifest_path);
    for signed in ["authorization.toml", "resolution.toml"] {
        if dir.join(signed).is_file() {
            return Ok(refused(
                "warrant.renumber-signed",
                repo.relative(&dir.join(signed)),
                format!(
                    "{alias} has {signed}: its alias is part of what was signed, so it keeps \
                     it. Renumber the other Warrant that shares it, if it is unsigned"
                ),
            ));
        }
    }
    let namespace = repo.config.project.namespace.as_str();
    if openwarrant_core::LocalAlias::parse_in(new, namespace).is_err() {
        return Ok(refused(
            "warrant.renumber-alias",
            rel_manifest,
            format!(
                "{new:?} is not an alias of this repository; one looks like \
                 {namespace}-WAR-0001"
            ),
        ));
    }
    if new == alias {
        return Ok(refused(
            "warrant.renumber-alias",
            rel_manifest,
            format!("{alias} is already {new}; nothing to change"),
        ));
    }
    // Free here, and on every branch this clone knows.
    let target = repo.warrants_dir().join(new);
    let mut taken_by: Vec<String> = Vec::new();
    if target.exists() {
        taken_by.push(format!("this working tree ({})", repo.relative(&target)));
    }
    for d in repo.warrant_dirs()? {
        if d == dir {
            continue;
        }
        let text = std::fs::read_to_string(d.join("manifest.toml")).unwrap_or_default();
        if local_alias_of(&text).as_deref() == Some(new) {
            taken_by.push(format!("{}'s local_alias", repo.relative(&d)));
        }
    }
    for (branch, names) in on_branches(repo) {
        if names.contains(new) {
            taken_by.push(format!("branch {branch}"));
        }
    }
    if !taken_by.is_empty() {
        return Ok(refused(
            "warrant.alias-taken",
            rel_manifest,
            format!(
                "{new} is taken: {}. Pick another (`war plan new` allocates past every branch)",
                taken_by.join(", ")
            ),
        ));
    }
    let text = std::fs::read_to_string(&manifest_path).map_err(|source| RepoError::Io {
        context: format!("could not read {manifest_path}"),
        source,
    })?;
    let old_local = local_alias_of(&text).unwrap_or_default();
    let Some(next) = with_local_alias(&text, new) else {
        let message = format!(
            "{rel_manifest} has no `local_alias = \"...\"` line to change; edit it by hand"
        );
        return Ok(refused("warrant.renumber-alias", rel_manifest, message));
    };
    crate::compile::atomic::write(&manifest_path, next.as_bytes())
        .map_err(|r| RepoError::Message(format!("{}: {}", r.rule, r.message)))?;
    std::fs::rename(&dir, &target).map_err(|source| RepoError::Io {
        context: format!("could not move {dir} to {target}"),
        source,
    })?;
    let uuid = toml::from_str::<openwarrant_core::Manifest>(&next)
        .map(|m| m.uuid)
        .unwrap_or_default();
    crate::journal_cmd::record(
        &target,
        &uuid,
        "draft.renumbered",
        &format!("agent://{}", repo.performer()),
        &serde_json::json!({"from": alias, "from_local_alias": old_local, "to": new}).to_string(),
    )?;
    let human = format!(
        "renumbered {alias} to {new}: {} is now {}, and its local_alias {new}. \
         `war admin compile` regenerates its views; a reference to {alias} written by hand elsewhere \
         still says {alias}",
        repo.relative(&dir),
        repo.relative(&target)
    );
    Ok(Renumbered {
        report: Report::default(),
        human,
        result: serde_json::json!({
            "schema": "oh.war/renumber/v1",
            "from": alias,
            "to": new,
            "dir": repo.relative(&target),
            "uuid": uuid,
        }),
    })
}

/// The `local_alias` a manifest's text gives.
fn local_alias_of(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "local_alias").then(|| v.trim().trim_matches('"').to_owned())
    })
}

/// The manifest with its `local_alias` line set to `new`, every other byte
/// as it was; `None` when there is no such line.
fn with_local_alias(text: &str, new: &str) -> Option<String> {
    let mut found = false;
    let out: String = text
        .split_inclusive('\n')
        .map(|l| {
            let key = l.split_once('=').map(|(k, _)| k.trim());
            if !found && key == Some("local_alias") {
                found = true;
                let end = if l.ends_with('\n') { "\n" } else { "" };
                format!("local_alias = \"{new}\"{end}")
            } else {
                l.to_owned()
            }
        })
        .collect();
    found.then_some(out)
}

/// Whether `dir` is a Warrant directory this module would read: for tests.
#[cfg(test)]
fn is_dir_entry(mode: &[u8]) -> bool {
    mode == b"40000"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_raw_tree_lists_its_directories() {
        let mut body = Vec::new();
        for (mode, name) in [
            ("40000", "OW-WAR-0001"),
            ("100644", "README.md"),
            ("40000", "OW-WAR-0148"),
        ] {
            body.extend_from_slice(mode.as_bytes());
            body.push(b' ');
            body.extend_from_slice(name.as_bytes());
            body.push(0);
            body.extend_from_slice(&[7u8; 20]);
        }
        let mut names = BTreeSet::new();
        tree_dirs(&body, 20, &mut names);
        assert_eq!(
            names.into_iter().collect::<Vec<_>>(),
            vec!["OW-WAR-0001".to_owned(), "OW-WAR-0148".to_owned()]
        );
        assert!(is_dir_entry(b"40000") && !is_dir_entry(b"100644"));
    }

    #[test]
    fn local_alias_is_rewritten_in_one_line() {
        let text = "schema = \"x\"\nlocal_alias = \"OW-WAR-0148\"\ntitle = \"t\"\n";
        assert_eq!(local_alias_of(text).as_deref(), Some("OW-WAR-0148"));
        let next = with_local_alias(text, "OW-WAR-0150").expect("has the line");
        assert_eq!(
            next,
            "schema = \"x\"\nlocal_alias = \"OW-WAR-0150\"\ntitle = \"t\"\n"
        );
        assert!(with_local_alias("title = \"t\"\n", "OW-WAR-0150").is_none());
        assert_eq!(ordinal("OW-WAR-0148"), Some(148));
    }
}
