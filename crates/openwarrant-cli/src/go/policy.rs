// SPDX-License-Identifier: Apache-2.0
//! `[go]` in `openwarrant.toml`: how `war evidence go` and `war start` run
//! work (OW-WAR-0148 M15; decisions 10 and 11). Every key is optional, and
//! a repository without the table runs one node at a time, each in its own
//! worktree, landing on `war-go/integration`.

use std::collections::BTreeMap;

use camino::Utf8Path;
use serde::Deserialize;

use crate::diagnostic::Diagnostic;

/// The table as written.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Raw {
    /// `worktree` (each node in its own worktree, on its own branch) or
    /// `shared` (every node in this checkout; one at a time).
    #[serde(default)]
    pub isolation: Option<String>,
    /// How many nodes run at once. Above 1 only with `isolation =
    /// "worktree"`.
    #[serde(default)]
    pub max_parallel: Option<u32>,
    /// The branch finished work lands on.
    #[serde(default)]
    pub integration: Option<String>,
    /// `merge` (into the integration branch) or `pr` (push the node's branch
    /// and open a pull request with `pr_argv`).
    #[serde(default)]
    pub land: Option<String>,
    /// With `land = "pr"`: the argv that opens the pull request.
    #[serde(default)]
    pub pr_argv: Option<Vec<String>>,
    /// With `land = "pr"`: the branch the pull request targets.
    #[serde(default)]
    pub pr_base: Option<String>,
    /// The git remote `land = "pr"` pushes to and an external executor's
    /// `git:` artifacts are fetched from.
    #[serde(default)]
    pub remote: Option<String>,
    /// The local harness: `claude` (Claude Code, the first preset) or
    /// `generic` (`harness_argv`).
    #[serde(default)]
    pub harness: Option<String>,
    /// The generic harness for an unattended node: the packet on stdin, a
    /// submission on stdout.
    #[serde(default)]
    pub harness_argv: Option<Vec<String>>,
    /// The generic harness for `war start`, run at a terminal in the
    /// worktree.
    #[serde(default)]
    pub start_argv: Option<Vec<String>>,
    /// `native` (the harness above) or the name of an `[go.executors.<name>]`
    /// table: the executor of a node no `[go.route]` label names.
    #[serde(default)]
    pub executor: Option<String>,
    /// A node's time budget, in seconds; enforced by killing its process
    /// group (a native run) or abandoning the wait (an external one).
    #[serde(default)]
    pub node_timeout_secs: Option<u64>,
    /// A node's token budget; enforced only where the harness reports its
    /// usage, and UNKNOWN where it does not, never assumed to be zero.
    #[serde(default)]
    pub node_tokens: Option<u64>,
    /// How many unlanded attempts in a row a node gets before it is blocked
    /// for a person.
    #[serde(default)]
    pub max_attempts: Option<u32>,
    /// A whole run's time budget, in seconds: no node starts after it.
    #[serde(default)]
    pub max_total_secs: Option<u64>,
    /// The Warrant types that may run unattended (`untyped` for a Warrant
    /// with no type, `stage` for a directory Warrant's agent stage). Absent:
    /// every type.
    #[serde(default)]
    pub unattended: Option<Vec<String>>,
    /// `projected` (`war start` writes the Warrant's declared tools, paths
    /// and commands as session-only harness settings in the worktree) or
    /// `declared` (it writes no settings and only shows them).
    #[serde(default)]
    pub allowed_acts: Option<String>,
    /// Where worktrees are made, relative to the root.
    #[serde(default)]
    pub worktrees: Option<String>,
    /// The estimate of a node when no history says otherwise, in seconds.
    #[serde(default)]
    pub prior_secs: Option<u64>,
    /// How often an external executor is asked for its answer, in seconds.
    #[serde(default)]
    pub poll_secs: Option<u64>,
    /// A Warrant label → the executor that runs its nodes.
    #[serde(default)]
    pub route: BTreeMap<String, String>,
    #[serde(default)]
    pub executors: BTreeMap<String, RawExecutor>,
}

/// `[go.executors.<name>]`: an external executor.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawExecutor {
    /// `agent-hq`, `linear`, `gastown` or `argv`.
    pub adapter: String,
    #[serde(default)]
    pub dispatch_argv: Option<Vec<String>>,
    #[serde(default)]
    pub poll_argv: Option<Vec<String>>,
    /// What the dispatch argv reads on stdin: `packet`, `brief` or `beads`.
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub poll_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Isolation {
    Worktree,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Land {
    Merge,
    Pr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    Claude,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acts {
    Projected,
    Declared,
}

/// What an external executor's dispatch reads on stdin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// The packet, `oh.war/go-packet/v1` JSON.
    Packet,
    /// The packet's brief: Markdown a person or an agent reads.
    Brief,
    /// The node as Beads issue JSONL (`war admin export beads`'s line).
    Beads,
}

/// An external executor, resolved.
#[derive(Debug, Clone)]
pub struct Executor {
    pub name: String,
    pub adapter: String,
    pub dispatch_argv: Vec<String>,
    pub poll_argv: Vec<String>,
    pub stdin: Input,
    pub poll_secs: u64,
}

/// `[go]`, every default applied.
#[derive(Debug, Clone)]
pub struct Policy {
    pub isolation: Isolation,
    pub max_parallel: u32,
    pub integration: String,
    pub land: Land,
    pub pr_argv: Vec<String>,
    pub pr_base: Option<String>,
    pub remote: String,
    pub harness: Harness,
    pub harness_argv: Vec<String>,
    pub start_argv: Vec<String>,
    /// `None`: native.
    pub executor: Option<String>,
    pub node_timeout_secs: u64,
    pub node_tokens: Option<u64>,
    pub max_attempts: u32,
    pub max_total_secs: Option<u64>,
    pub unattended: Option<Vec<String>>,
    pub acts: Acts,
    pub worktrees: String,
    pub prior_secs: u64,
    pub route: BTreeMap<String, String>,
    pub executors: BTreeMap<String, Executor>,
}

pub const DEFAULT_INTEGRATION: &str = "war-go/integration";
pub const DEFAULT_WORKTREES: &str = ".openwarrant/worktrees";
pub const DEFAULT_NODE_TIMEOUT_SECS: u64 = 3600;
pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;
pub const DEFAULT_POLL_SECS: u64 = 30;

/// Claude Code, unattended: the brief on stdin, the answer as the result of
/// `--output-format json`, whose `usage` is the token count the budget
/// reads. Edits are accepted in the worktree; the session's settings there
/// carry the Warrant's declared tools and commands.
pub const CLAUDE_UNATTENDED: &[&str] = &[
    "claude",
    "-p",
    "--output-format",
    "json",
    "--permission-mode",
    "acceptEdits",
];

/// Claude Code at a terminal, for `war start`: the brief as its first prompt.
pub const CLAUDE_START: &[&str] = &["claude", "{brief}"];

/// `gh pr create` for `land = "pr"`.
pub const GH_PR: &[&str] = &[
    "gh", "pr", "create", "--head", "{branch}", "--base", "{base}", "--title", "{title}", "--body",
    "{body}",
];

/// GitHub Agent HQ: open an issue for the node and assign it to the coding
/// agent; the brief is the issue's body. `gh issue create` prints the issue's
/// URL, which is the reference polled.
pub const AGENT_HQ_DISPATCH: &[&str] = &[
    "gh",
    "issue",
    "create",
    "--title",
    "{title}",
    "--body-file",
    "-",
    "--assignee",
    "@copilot",
];

/// GitHub Agent HQ: read the issue's comments, and answer with the last
/// fenced `json war-submission` block as the submission, else pending.
pub const AGENT_HQ_POLL: &[&str] = &[
    "gh",
    "issue",
    "view",
    "{ref}",
    "--json",
    "comments",
    "--jq",
    "[.comments[].body | select(contains(\"```json war-submission\"))] | last \
     | if . == null then {\"status\": \"pending\"} else {\"status\": \"submitted\", \
     \"submission\": (capture(\"```json war-submission\\\\s*(?<s>[\\\\s\\\\S]*?)```\").s \
     | fromjson)} end",
];

fn owned(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

/// Read `[go]` from `openwarrant.toml`. A malformed table, or a value outside
/// its words, is refused `go.config` naming the key.
pub fn load(root: &Utf8Path) -> Result<Policy, Diagnostic> {
    let path = root.join(crate::init::CONFIG_FILE);
    let text = crate::vfs::read_to_string(&path).map_err(|e| {
        Diagnostic::error(
            "go.config",
            crate::init::CONFIG_FILE.to_owned(),
            format!("could not read {path}: {e}"),
        )
    })?;
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        go: Option<Raw>,
    }
    let file: File = toml::from_str(&text).map_err(|e| {
        Diagnostic::error(
            "go.config",
            crate::init::CONFIG_FILE.to_owned(),
            format!("the [go] table: {e}"),
        )
    })?;
    resolve(file.go.unwrap_or_default())
}

fn bad(key: &str, message: String) -> Diagnostic {
    Diagnostic::error(
        "go.config",
        crate::init::CONFIG_FILE.to_owned(),
        format!("[go] {key}: {message}"),
    )
}

fn word<T: Copy>(
    key: &str,
    v: Option<&str>,
    default: T,
    words: &[(&str, T)],
) -> Result<T, Diagnostic> {
    match v {
        None => Ok(default),
        Some(w) => words
            .iter()
            .find(|(name, _)| *name == w)
            .map(|(_, t)| *t)
            .ok_or_else(|| {
                bad(
                    key,
                    format!(
                        "{w:?} is not one of {}",
                        words
                            .iter()
                            .map(|(n, _)| format!("{n:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            }),
    }
}

/// Apply the defaults and refuse what cannot be honoured.
pub fn resolve(raw: Raw) -> Result<Policy, Diagnostic> {
    let isolation = word(
        "isolation",
        raw.isolation.as_deref(),
        Isolation::Worktree,
        &[
            ("worktree", Isolation::Worktree),
            ("shared", Isolation::Shared),
        ],
    )?;
    let land = word(
        "land",
        raw.land.as_deref(),
        Land::Merge,
        &[("merge", Land::Merge), ("pr", Land::Pr)],
    )?;
    let harness = word(
        "harness",
        raw.harness.as_deref(),
        Harness::Generic,
        &[("claude", Harness::Claude), ("generic", Harness::Generic)],
    )?;
    let acts = word(
        "allowed_acts",
        raw.allowed_acts.as_deref(),
        Acts::Projected,
        &[("projected", Acts::Projected), ("declared", Acts::Declared)],
    )?;
    let integration = raw
        .integration
        .clone()
        .unwrap_or_else(|| DEFAULT_INTEGRATION.to_owned());
    if !crate::go::git::is_branch_name(&integration) {
        return Err(bad(
            "integration",
            format!("{integration:?} is not a branch name git accepts"),
        ));
    }
    if land == Land::Pr && isolation == Isolation::Shared {
        return Err(bad(
            "land",
            "\"pr\" opens a pull request from each node's branch, and `isolation = \"shared\"` \
             gives a node none"
                .to_owned(),
        ));
    }
    let worktrees = raw
        .worktrees
        .clone()
        .unwrap_or_else(|| DEFAULT_WORKTREES.to_owned());
    if worktrees.split('/').any(|p| p == "..") || worktrees.starts_with('/') {
        return Err(bad(
            "worktrees",
            format!("{worktrees:?} leaves the repository; give a path inside it"),
        ));
    }
    let mut executors = BTreeMap::new();
    for (name, e) in raw.executors {
        let (dispatch, poll, input) = match e.adapter.as_str() {
            "agent-hq" => (
                Some(owned(AGENT_HQ_DISPATCH)),
                Some(owned(AGENT_HQ_POLL)),
                Input::Brief,
            ),
            "linear" | "argv" => (None, None, Input::Packet),
            "gastown" => (None, None, Input::Beads),
            other => {
                return Err(bad(
                    &format!("executors.{name}.adapter"),
                    format!(
                        "{other:?} is not one of \"agent-hq\", \"linear\", \"gastown\", \"argv\""
                    ),
                ));
            }
        };
        let stdin = word(
            &format!("executors.{name}.stdin"),
            e.stdin.as_deref(),
            input,
            &[
                ("packet", Input::Packet),
                ("brief", Input::Brief),
                ("beads", Input::Beads),
            ],
        )?;
        let dispatch_argv = e.dispatch_argv.or(dispatch).unwrap_or_default();
        let poll_argv = e.poll_argv.or(poll).unwrap_or_default();
        for (key, argv) in [("dispatch_argv", &dispatch_argv), ("poll_argv", &poll_argv)] {
            if argv.is_empty() {
                return Err(bad(
                    &format!("executors.{name}.{key}"),
                    format!(
                        "the {} adapter has no default {key}; give the argv that {} (docs/GO.md, \
                         \"External executors\")",
                        e.adapter,
                        if key == "dispatch_argv" {
                            "hands the node over and prints its reference"
                        } else {
                            "prints the answer for `{ref}`"
                        }
                    ),
                ));
            }
        }
        executors.insert(
            name.clone(),
            Executor {
                name,
                adapter: e.adapter,
                dispatch_argv,
                poll_argv,
                stdin,
                poll_secs: e
                    .poll_secs
                    .or(raw.poll_secs)
                    .unwrap_or(DEFAULT_POLL_SECS)
                    .max(1),
            },
        );
    }
    let executor = match raw.executor.as_deref() {
        None | Some("native") => None,
        Some(name) if executors.contains_key(name) => Some(name.to_owned()),
        Some(name) => {
            return Err(bad(
                "executor",
                format!("{name:?} is neither \"native\" nor an [go.executors.{name}] table"),
            ));
        }
    };
    for (label, name) in &raw.route {
        if name != "native" && !executors.contains_key(name) {
            return Err(bad(
                &format!("route.{label}"),
                format!("{name:?} is neither \"native\" nor an [go.executors.{name}] table"),
            ));
        }
    }
    Ok(Policy {
        isolation,
        max_parallel: raw.max_parallel.unwrap_or(1).max(1),
        integration,
        land,
        pr_argv: raw.pr_argv.unwrap_or_else(|| owned(GH_PR)),
        pr_base: raw.pr_base,
        remote: raw.remote.unwrap_or_else(|| "origin".to_owned()),
        harness,
        harness_argv: match harness {
            Harness::Claude => owned(CLAUDE_UNATTENDED),
            Harness::Generic => raw.harness_argv.unwrap_or_default(),
        },
        start_argv: match harness {
            Harness::Claude => owned(CLAUDE_START),
            Harness::Generic => raw.start_argv.unwrap_or_default(),
        },
        executor,
        node_timeout_secs: raw
            .node_timeout_secs
            .unwrap_or(DEFAULT_NODE_TIMEOUT_SECS)
            .max(1),
        node_tokens: raw.node_tokens.filter(|t| *t > 0),
        max_attempts: raw.max_attempts.unwrap_or(DEFAULT_MAX_ATTEMPTS).max(1),
        max_total_secs: raw.max_total_secs.filter(|t| *t > 0),
        unattended: raw.unattended,
        acts,
        worktrees,
        prior_secs: raw
            .prior_secs
            .unwrap_or(crate::estimate::DEFAULT_PRIOR_SECS),
        route: raw.route,
        executors,
    })
}

impl Policy {
    /// The executor that runs a node with these labels: the first label
    /// `[go.route]` names, else `[go] executor`; `None` is native.
    #[must_use]
    pub fn executor_for(&self, labels: &[String]) -> Option<&Executor> {
        let routed = labels.iter().find_map(|l| self.route.get(l));
        let name = match routed {
            Some(n) if n == "native" => return None,
            Some(n) => Some(n.as_str()),
            None => self.executor.as_deref(),
        };
        name.and_then(|n| self.executors.get(n))
    }

    /// Whether a node of `work_type` may run unattended.
    #[must_use]
    pub fn unattended_admits(&self, work_type: Option<&str>) -> bool {
        self.unattended
            .as_ref()
            .is_none_or(|kinds| kinds.iter().any(|k| k == work_type.unwrap_or("untyped")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Policy, Diagnostic> {
        #[derive(Deserialize)]
        struct File {
            go: Raw,
        }
        let f: File = toml::from_str(text).map_err(|e| bad("table", e.to_string()))?;
        resolve(f.go)
    }

    #[test]
    fn the_defaults_run_one_node_in_a_worktree() {
        let p = parse("[go]\n").unwrap();
        assert_eq!(p.isolation, Isolation::Worktree);
        assert_eq!(p.max_parallel, 1);
        assert_eq!(p.integration, DEFAULT_INTEGRATION);
        assert_eq!(p.land, Land::Merge);
        assert_eq!(p.acts, Acts::Projected);
        assert!(p.harness_argv.is_empty(), "no harness is assumed");
        assert!(p.executor_for(&[]).is_none());
        assert!(p.unattended_admits(None));
    }

    /// The Claude preset builds `claude -p` and reads usage from its JSON;
    /// nothing here runs it.
    #[test]
    fn the_claude_preset_is_claude_p() {
        let p = parse("[go]\nharness = \"claude\"\n").unwrap();
        assert_eq!(&p.harness_argv[..2], ["claude", "-p"]);
        assert!(
            p.harness_argv
                .windows(2)
                .any(|w| w == ["--output-format", "json"])
        );
        assert_eq!(p.start_argv, ["claude", "{brief}"]);
    }

    #[test]
    fn a_word_outside_its_set_is_refused_by_key() {
        for (text, key) in [
            ("[go]\nisolation = \"cgroup\"\n", "isolation"),
            ("[go]\nland = \"rebase\"\n", "land"),
            ("[go]\nallowed_acts = \"all\"\n", "allowed_acts"),
            ("[go]\nexecutor = \"nope\"\n", "executor"),
            ("[go]\nworktrees = \"../x\"\n", "worktrees"),
            (
                "[go.executors.l]\nadapter = \"linear\"\n",
                "executors.l.dispatch_argv",
            ),
            ("[go]\nland = \"pr\"\nisolation = \"shared\"\n", "land"),
        ] {
            let e = parse(text).unwrap_err();
            assert_eq!(e.rule, "go.config");
            assert!(
                e.message.contains(&format!("[go] {key}")),
                "{text}: {}",
                e.message
            );
        }
    }

    #[test]
    fn a_label_routes_to_its_executor() {
        let p = parse(
            "[go.executors.hq]\nadapter = \"agent-hq\"\n[go.route]\nhq = \"hq\"\nlocal = \"native\"\n",
        )
        .unwrap();
        assert_eq!(p.executor_for(&["hq".into()]).unwrap().adapter, "agent-hq");
        assert!(p.executor_for(&["local".into()]).is_none());
        assert_eq!(p.executor_for(&["hq".into()]).unwrap().stdin, Input::Brief);
        let p = parse("[go]\nunattended = [\"bug\"]\n").unwrap();
        assert!(p.unattended_admits(Some("bug")));
        assert!(!p.unattended_admits(Some("feature")));
        assert!(!p.unattended_admits(None));
    }
}
