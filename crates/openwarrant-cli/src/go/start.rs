// SPDX-License-Identifier: Apache-2.0
//! `war start <id>`: one node, at a terminal (OW-WAR-0148 M15; decision 11).
//!
//! It claims the node, makes it a worktree on a branch of its own
//! (`war-go/<node>`, from HEAD or `--base`), and starts the configured
//! harness there, or prints the command with `--print`. Everything it writes
//! is inside that worktree, or is git's own bookkeeping (the worktree's
//! metadata, `info/exclude`, and M11's claim lock, all under the clone's
//! common directory):
//!
//! - with `[go] allowed_acts = "projected"` (the default), the session's
//!   harness settings: for Claude Code, `.claude/settings.local.json` in the
//!   worktree, carrying the Warrant's declared tools, paths and commands as
//!   permissions, and the actor that holds the claim, so `war done` in the
//!   session ticks as its holder; and `.openwarrant/session.json`, which the
//!   plugin's edit guard reads to turn back an edit outside the declared
//!   paths, when the Warrant declares any. Never a user's or a global
//!   settings file;
//! - with `declared`, no settings at all: the context is shown, and nothing
//!   the harness reads is written.
//!
//! The claim is taken from the worktree, so its journal entry is on the
//! node's branch with the work, and the checkout `war start` ran in is left
//! exactly as it was. A second `war start` of the same node by the same
//! actor finds its worktree and starts the session again.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::go::{git, packet, policy, proc};
use crate::repo::{RepoError, Repository};
use crate::ticket::Store;

pub const SCHEMA: &str = "oh.war/start/v1";
pub const SESSION_SCHEMA: &str = "oh.war/session/v1";

/// What `war start` was asked.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub actor: Option<String>,
    /// Print the command and the context; start nothing.
    pub print: bool,
    /// The commit the worktree starts from (default HEAD).
    pub base: Option<String>,
}

/// The session marker the edit guard reads.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Session {
    pub schema: String,
    pub node: String,
    pub actor: String,
    pub branch: String,
    pub base: String,
    #[serde(flatten)]
    pub allowed: packet::Allowed,
    /// sha256 of the settings file as written: a later `war start` rewrites
    /// it only while it is still these bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Started {
    pub schema: &'static str,
    pub node: String,
    pub worktree: String,
    pub branch: String,
    pub base: String,
    pub actor: String,
    pub resumed: bool,
    pub allowed_acts: &'static str,
    #[serde(skip_serializing_if = "packet::Allowed::is_empty")]
    pub allowed: packet::Allowed,
    /// Files written in the worktree for the session.
    pub wrote: Vec<String>,
    pub command: Vec<String>,
    pub ran: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<i32>,
    pub brief: String,
}

/// Claude Code's permission rules for what a Warrant declares: each tool by
/// name, each path as an edit rule rooted at the worktree (`/src/**`), each
/// command as a Bash rule.
#[must_use]
pub fn claude_permissions(a: &packet::Allowed) -> Vec<String> {
    let mut allow: Vec<String> = a.tools.clone();
    allow.extend(
        a.paths
            .iter()
            .map(|p| format!("Edit(/{})", p.trim_start_matches('/'))),
    );
    allow.extend(a.commands.iter().map(|c| format!("Bash({c})")));
    allow
}

/// The settings file for the session.
#[must_use]
pub fn claude_settings(a: &packet::Allowed, actor: &str) -> serde_json::Value {
    let mut v = serde_json::json!({
        "env": {"OPENWARRANT_ACTOR": actor},
    });
    let allow = claude_permissions(a);
    if !allow.is_empty() {
        v["permissions"] = serde_json::json!({"allow": allow});
    }
    v
}

fn sha(bytes: &[u8]) -> String {
    openwarrant_compiler::sha256_hex(bytes)
}

fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:@%+,".contains(c))
    {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// `war start <id>`.
pub fn run(
    repo: &Repository,
    id: &str,
    opts: &Options,
) -> Result<(Report, serde_json::Value, String), RepoError> {
    let mut report = Report::default();
    let policy = match policy::load(&repo.root) {
        Ok(p) => p,
        Err(d) => {
            report.push(d);
            return Ok((report, serde_json::Value::Null, String::new()));
        }
    };
    let root = repo.root.clone();
    let Some(top) = git::toplevel(&root) else {
        report.push(Diagnostic::error(
            "go.no-git",
            root.to_string(),
            "`war start` gives the work a git worktree of its own, and this is not a git \
             checkout; claim it and work here instead (`war claim <id>`)",
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    };
    let below = root
        .strip_prefix(&top)
        .map(Utf8Path::to_owned)
        .unwrap_or_default();
    let here = Store::open(repo, opts.actor.as_deref())?;
    let actor = here.actor.clone();
    // What the id names, read here; the claim is taken in the worktree.
    let (tickets, _) = here.load_all()?;
    let (node, warrant_dir, allowed_here, title) = if crate::ticket::is_ticket_ref(id) {
        match crate::ticket::resolve(&tickets, id) {
            Ok(crate::ticket::Target::Item(n, item)) => {
                let t = &tickets[n];
                (
                    format!("{}/{item}", t.id()),
                    t.dir.clone(),
                    packet::allowed_of_ticket(t),
                    format!(
                        "{}: {}",
                        t.manifest.title,
                        t.item(&item).map(|i| i.text.clone()).unwrap_or_default()
                    ),
                )
            }
            Ok(crate::ticket::Target::Ticket(n)) => {
                let t = &tickets[n];
                (
                    t.id().to_owned(),
                    t.dir.clone(),
                    packet::allowed_of_ticket(t),
                    t.manifest.title.clone(),
                )
            }
            Err(d) => {
                report.push(d);
                return Ok((report, serde_json::Value::Null, String::new()));
            }
        }
    } else {
        let alias = id.split('/').next().unwrap_or(id);
        let dir = repo.warrant_dir(alias)?;
        (
            id.to_owned(),
            dir,
            packet::allowed_of_warrant(repo, alias),
            id.to_owned(),
        )
    };
    let slug = git::slug(&node);
    let worktree = root.join(&policy.worktrees).join(&slug);
    let branch = git::node_branch(&node);
    let base_rev = opts.base.clone().unwrap_or_else(|| "HEAD".to_owned());
    let Some(base) = git::rev(&root, &base_rev) else {
        report.push(Diagnostic::error(
            "go.start-base",
            root.to_string(),
            format!("{base_rev} names no commit here; a worktree is made from a commit"),
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    };
    // The Warrant has to be in the commit the worktree is made from.
    let manifest_rel = warrant_dir
        .join("manifest.toml")
        .strip_prefix(&top)
        .map(Utf8Path::to_string)
        .unwrap_or_default();
    let resumed = worktree.join(".git").exists();
    if !resumed
        && !git::git(
            &root,
            &["cat-file", "-e", &format!("{base}:{manifest_rel}")],
        )
        .ok
    {
        report.push(Diagnostic::error(
            "go.start-uncommitted",
            repo.relative(&warrant_dir),
            format!(
                "{node}'s Warrant is not in {base_rev} yet, and the worktree is made from that \
                 commit. Commit {} first (`git add {0} && git commit`), then run `war start {node}` \
                 again",
                repo.relative(&warrant_dir)
            ),
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    }
    let created = !resumed;
    if created {
        let made = if git::branch_tip(&root, &branch).is_some() {
            // Work from an earlier session is on the branch: keep it.
            git::add_worktree_on(&root, &worktree, &branch)
        } else {
            let parent = worktree.parent().map(Utf8Path::to_owned);
            if let Some(p) = parent {
                let _ = std::fs::create_dir_all(p);
            }
            let r = git::git(
                &root,
                &[
                    "worktree",
                    "add",
                    "-q",
                    "-b",
                    &branch,
                    worktree.as_str(),
                    &base,
                ],
            );
            if r.ok { Ok(()) } else { Err(r.why()) }
        };
        if let Err(e) = made {
            report.push(Diagnostic::error(
                "go.worktree",
                repo.relative(&worktree),
                format!("{node}: could not make its worktree: {e}"),
            ));
            return Ok((report, serde_json::Value::Null, String::new()));
        }
    }
    git::exclude(
        &root,
        &format!("/{}/", policy.worktrees.trim_end_matches('/')),
    );
    for f in packet::SESSION_FILES {
        git::exclude(&root, f);
    }
    let wt_root = worktree.join(&below);
    // Claim from the worktree: the shared lock set, the journal on the branch.
    let wt_repo = Repository::discover(Some(wt_root.clone()))?;
    let store = Store::open(&wt_repo, Some(&actor))?;
    let undo = |report: &mut Report| {
        if created {
            git::remove_worktree(&root, &worktree, None);
            if git::branch_tip(&root, &branch).is_some_and(|tip| tip == base) {
                let _ = git::git(&root, &["branch", "-D", &branch]);
            }
            report.note("the worktree made for it was removed again".to_owned());
        }
    };
    if crate::ticket::is_ticket_ref(&node) {
        let claimed = crate::ticket::claim_cmd(&store, &node, false)?;
        if claimed.is_refused() {
            for d in claimed.report.diagnostics {
                report.push(d);
            }
            undo(&mut report);
            return Ok((report, serde_json::Value::Null, String::new()));
        }
    }
    // The session's settings: inside the worktree, and nowhere else.
    let mut wrote = Vec::new();
    let allowed = if crate::ticket::is_ticket_ref(&node) {
        let (wt_tickets, _) = store.load_all()?;
        let (tid, _) = openwarrant_core::ticket::split_record_id(&node);
        wt_tickets
            .iter()
            .find(|t| t.id() == tid)
            .map_or(allowed_here, packet::allowed_of_ticket)
    } else {
        allowed_here
    };
    let acts = match policy.acts {
        policy::Acts::Projected => "projected",
        policy::Acts::Declared => "declared",
    };
    if policy.acts == policy::Acts::Projected {
        let settings_path = wt_root.join(".claude").join("settings.local.json");
        let session_path = wt_root.join(".openwarrant").join("session.json");
        let previous: Option<Session> = std::fs::read(&session_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let ours = match std::fs::read(&settings_path) {
            Err(_) => true,
            Ok(bytes) => previous
                .as_ref()
                .and_then(|s| s.settings_sha256.as_deref())
                .is_some_and(|d| d == sha(&bytes)),
        };
        let settings_sha = if ours {
            let body = serde_json::to_string_pretty(&claude_settings(&allowed, &actor))
                .unwrap_or_default()
                + "\n";
            let _ = std::fs::create_dir_all(wt_root.join(".claude"));
            std::fs::write(&settings_path, &body).map_err(|source| RepoError::Io {
                context: format!("could not write {settings_path}"),
                source,
            })?;
            wrote.push(settings_path.to_string());
            Some(sha(body.as_bytes()))
        } else {
            report.push(Diagnostic::warn(
                "go.settings-kept",
                settings_path.to_string(),
                format!(
                    "{settings_path} was not written by `war start`, so it is left as it is; the \
                     Warrant's declared acts are shown below instead"
                ),
            ));
            None
        };
        let session = Session {
            schema: SESSION_SCHEMA.to_owned(),
            node: node.clone(),
            actor: actor.clone(),
            branch: branch.clone(),
            base: base.clone(),
            allowed: allowed.clone(),
            settings_sha256: settings_sha,
        };
        let _ = std::fs::create_dir_all(wt_root.join(".openwarrant"));
        std::fs::write(
            &session_path,
            serde_json::to_string_pretty(&session).unwrap_or_default() + "\n",
        )
        .map_err(|source| RepoError::Io {
            context: format!("could not write {session_path}"),
            source,
        })?;
        wrote.push(session_path.to_string());
    }
    // The brief: the same context `war evidence go` hands a performer.
    let brief = brief_for(&node, &title, &worktree, &branch, &base, &allowed, &actor);
    let argv = proc::fill(
        &policy.start_argv,
        &[
            ("node", node.as_str()),
            ("worktree", worktree.as_str()),
            ("brief", brief.as_str()),
            ("title", title.as_str()),
        ],
    );
    let mut human = String::new();
    human.push_str(&format!(
        "{} {node} in {} (branch {branch}, from {})\n",
        if resumed { "resumed" } else { "started" },
        repo.relative(&worktree),
        &base[..base.len().min(12)]
    ));
    match policy.acts {
        policy::Acts::Projected if wrote.iter().any(|w| w.ends_with("settings.local.json")) => {
            human.push_str(&format!(
                "session settings: {} (this worktree only)\n",
                repo.relative(&wt_root.join(".claude/settings.local.json"))
            ));
        }
        policy::Acts::Declared => {
            human.push_str("allowed acts are declared only ([go] allowed_acts = \"declared\"): no settings were written\n");
        }
        policy::Acts::Projected => {}
    }
    if !allowed.is_empty() {
        for (what, list) in [
            ("tools", &allowed.tools),
            ("paths", &allowed.paths),
            ("commands", &allowed.commands),
        ] {
            if !list.is_empty() {
                human.push_str(&format!("declared {what}: {}\n", list.join(", ")));
            }
        }
    }
    let command_line = format!(
        "cd {} && {}",
        shell_quote(wt_root.as_str()),
        if argv.is_empty() {
            "<your harness>".to_owned()
        } else {
            argv.iter()
                .map(|a| shell_quote(a))
                .collect::<Vec<_>>()
                .join(" ")
        }
    );
    let mut ran = false;
    let mut exit = None;
    if opts.print || argv.is_empty() {
        if argv.is_empty() && !opts.print {
            report.note(
                "no harness for a session is configured (`[go] harness = \"claude\"`, or \
                 `start_argv`); the command below is where to start one"
                    .to_owned(),
            );
        }
        human.push_str(&format!("{command_line}\n\n{brief}"));
    } else {
        human.push_str(&format!("{command_line}\n"));
        let status = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .current_dir(&wt_root)
            .env("OPENWARRANT_ACTOR", &actor)
            .env("WAR_GO_NODE", &node)
            .status()
            .map_err(|source| RepoError::Io {
                context: format!("could not run {}", argv[0]),
                source,
            })?;
        ran = true;
        exit = status.code();
    }
    report.push(Diagnostic::pass(
        "start.ready",
        format!(
            "{node}: claimed by {actor}, worktree {}",
            repo.relative(&worktree)
        ),
    ));
    let started = Started {
        schema: SCHEMA,
        node,
        worktree: worktree.to_string(),
        branch,
        base,
        actor,
        resumed,
        allowed_acts: acts,
        allowed,
        wrote,
        command: argv,
        ran,
        exit,
        brief,
    };
    Ok((
        report,
        serde_json::to_value(&started).unwrap_or_default(),
        human,
    ))
}

fn brief_for(
    node: &str,
    title: &str,
    worktree: &Utf8PathBuf,
    branch: &str,
    base: &str,
    allowed: &packet::Allowed,
    actor: &str,
) -> String {
    let mut md = format!(
        "# {title}\n\nTracked work `{node}`, claimed by `{actor}`, in its own worktree `{worktree}` \
         (branch `{branch}`, from `{}`). When it is done: `war done {node} --note \"...\"`, or \
         `war done {node} --check` to run its tests and tick it at observed.\n",
        &base[..base.len().min(12)]
    );
    if !allowed.is_empty() {
        md.push_str("\n## What the Warrant declares this work touches\n\n");
        for (what, list) in [
            ("tools", &allowed.tools),
            ("paths", &allowed.paths),
            ("commands", &allowed.commands),
        ] {
            if !list.is_empty() {
                md.push_str(&format!("- {what}: {}\n", list.join(", ")));
            }
        }
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_acts_become_claude_permission_rules() {
        let a = packet::Allowed {
            tools: vec!["Read".into()],
            paths: vec!["src/parser/**".into()],
            commands: vec!["cargo test -p parser".into()],
        };
        assert_eq!(
            claude_permissions(&a),
            ["Read", "Edit(/src/parser/**)", "Bash(cargo test -p parser)"]
        );
        let s = claude_settings(&a, "claude");
        assert_eq!(s["env"]["OPENWARRANT_ACTOR"], "claude");
        assert_eq!(s["permissions"]["allow"][1], "Edit(/src/parser/**)");
        let none = claude_settings(&packet::Allowed::default(), "claude");
        assert!(
            none.get("permissions").is_none(),
            "nothing declared, nothing restricted"
        );
    }

    #[test]
    fn a_command_line_quotes_what_a_shell_would_split() {
        assert_eq!(shell_quote("claude"), "claude");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
