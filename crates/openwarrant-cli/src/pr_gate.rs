// SPDX-License-Identifier: Apache-2.0
//! `war check --pr <number>`: the PR gate (OW-WAR-0148 M14; docs/PRESETS.md).
//!
//! A PR passes when it cites a Warrant that is official at its author's
//! level (decision 6). It reads, through `gh api` and nothing else:
//!
//! - the PR: its author, body, base and head commits
//!   (`repos/{o}/{r}/pulls/{n}`);
//! - the policy as the BASE branch has it
//!   (`repos/{o}/{r}/contents/openwarrant.toml?ref=<base>`), so a PR cannot
//!   loosen `[roles]` or `[preset]` to pass itself;
//! - the author's role (`repos/{o}/{r}/collaborators/{user}/permission`);
//! - its files (`.../pulls/{n}/files`): a Warrant the PR adds is cited, and a
//!   PR that changes the roster (`docs/authority/roles.toml`,
//!   `allowed_signers`) gets no credit for a roster approval;
//! - its commits (`.../pulls/{n}/commits`): `Warrant: <id>` trailers;
//! - its reviews (`.../pulls/{n}/reviews`): each reviewer's latest, an
//!   approval counting only at the head commit, by a role that may make the
//!   Warrant's kind.
//!
//! The Warrant itself is read from this checkout (the PR's tree), so the
//! Warrant and its code may arrive in the same PR.
//!
//! A call that fails, times out or answers something unreadable makes what
//! it would have told UNKNOWN: never a pass. Every refusal names what would
//! make the PR pass.

use std::collections::BTreeMap;
use std::time::Duration;

use camino::Utf8PathBuf;
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::official::{self, Subject};
use crate::plan::intake::{Ran, first_line, run_bounded};
use crate::preset::{Kind, Policy, Role, Signing};
use crate::repo::Repository;

/// The envelope's `result` schema.
pub const SCHEMA: &str = "oh.war/pr-check/v1";

/// How long one `gh` call may take.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// What `war check --pr` is given.
#[derive(Debug, Clone, Default)]
pub struct Args {
    pub pr: String,
    /// `owner/name`; else `$GITHUB_REPOSITORY`, else `gh repo view`.
    pub repo: Option<String>,
    /// Append the Markdown summary here (`$GITHUB_STEP_SUMMARY` in CI).
    pub summary: Option<Utf8PathBuf>,
    /// Post the summary as a PR comment through `gh`.
    pub comment: bool,
}

/// One `gh` call's failure, kept for the UNKNOWN it causes.
#[derive(Debug, Clone)]
struct Failed {
    call: String,
    why: String,
    not_found: bool,
}

/// `gh`, from the repository root, bounded, recording each call.
struct Gh<'a> {
    root: &'a camino::Utf8Path,
    calls: Vec<String>,
}

impl Gh<'_> {
    fn run(&mut self, args: &[&str]) -> Result<String, Failed> {
        let mut argv = vec!["gh".to_owned()];
        argv.extend(args.iter().map(|a| (*a).to_owned()));
        let call = args.join(" ");
        self.calls.push(call.clone());
        match run_bounded(self.root, &argv, CALL_TIMEOUT) {
            Ok(Ran::Exited(status, out, _)) if status.success() => Ok(out),
            Ok(Ran::Exited(status, _, err)) => Err(Failed {
                not_found: err.contains("HTTP 404") || err.contains("Not Found"),
                why: format!("gh exited {status}: {}", first_line(&err)),
                call,
            }),
            Ok(Ran::TimedOut) => Err(Failed {
                call,
                why: format!("gh ran past {}s and was stopped", CALL_TIMEOUT.as_secs()),
                not_found: false,
            }),
            Err((what, e)) => Err(Failed {
                call,
                why: format!("{what}: {e}"),
                not_found: false,
            }),
        }
    }

    /// One JSON object.
    fn object(&mut self, path: &str) -> Result<serde_json::Value, Failed> {
        let out = self.run(&["api", path])?;
        serde_json::from_str(&out).map_err(|e| Failed {
            call: format!("api {path}"),
            why: format!("the answer is not JSON: {e}"),
            not_found: false,
        })
    }

    /// Every element of a paginated list. `--paginate` prints one JSON
    /// array per page; each is read and joined.
    fn list(&mut self, path: &str) -> Result<Vec<serde_json::Value>, Failed> {
        let out = self.run(&["api", "--paginate", path])?;
        let mut items = Vec::new();
        for page in serde_json::Deserializer::from_str(&out).into_iter::<serde_json::Value>() {
            match page {
                Ok(serde_json::Value::Array(a)) => items.extend(a),
                Ok(other) => {
                    return Err(Failed {
                        call: format!("api --paginate {path}"),
                        why: format!(
                            "a page is not a JSON array: {}",
                            first_line(&other.to_string())
                        ),
                        not_found: false,
                    });
                }
                Err(e) => {
                    return Err(Failed {
                        call: format!("api --paginate {path}"),
                        why: format!("the answer is not JSON: {e}"),
                        not_found: false,
                    });
                }
            }
        }
        Ok(items)
    }

    /// A file's text at a commit; `Ok(None)` when GitHub says it is not
    /// there.
    fn file_at(&mut self, repo: &str, path: &str, rev: &str) -> Result<Option<String>, Failed> {
        let api = format!("repos/{repo}/contents/{path}?ref={rev}");
        match self.run(&["api", "-H", "Accept: application/vnd.github.raw", &api]) {
            Ok(text) => Ok(Some(text)),
            Err(f) if f.not_found => Ok(None),
            Err(f) => Err(f),
        }
    }

    /// A user's role in the repository.
    fn role(&mut self, repo: &str, user: &str) -> Result<Role, Failed> {
        let path = format!("repos/{repo}/collaborators/{user}/permission");
        let v = self.object(&path)?;
        let role_name = v.get("role_name").and_then(|r| r.as_str());
        let permission = v.get("permission").and_then(|r| r.as_str());
        Role::from_github(role_name, permission).ok_or_else(|| Failed {
            call: format!("api {path}"),
            why: format!(
                "GitHub named role {role_name:?} and permission {permission:?}, neither one of \
                 admin, maintain, write, triage, read, none"
            ),
            not_found: false,
        })
    }
}

/// `owner/name`, as GitHub spells a repository.
fn repo_ok(r: &str) -> bool {
    let mut parts = r.split('/');
    let ok = |p: Option<&str>| {
        p.is_some_and(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
    };
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

/// A GitHub login.
fn login_ok(u: &str) -> bool {
    !u.is_empty()
        && u.len() <= 64
        && u.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '[' | ']'))
}

/// Every `Warrant: <id>` line in `text` (a commit message or the PR body):
/// the key in any case, `Warrants:` too, ids split at commas and spaces.
#[must_use]
pub fn cited_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim().trim_start_matches(['-', '*', '>']).trim();
        let Some((key, rest)) = t.split_once(':') else {
            continue;
        };
        if !matches!(
            key.trim().to_ascii_lowercase().as_str(),
            "warrant" | "warrants"
        ) {
            continue;
        }
        for id in rest.split([',', ' ', '\t']) {
            let id = id.trim().trim_matches('`');
            if !id.is_empty() && id.len() <= 128 && !id.contains(char::is_whitespace) {
                out.push(id.to_owned());
            }
        }
    }
    out
}

/// One reviewer's latest word on the PR.
#[derive(Debug, Clone, Serialize)]
pub struct Review {
    pub reviewer: String,
    pub state: String,
    /// Whether it was given at the PR's head commit.
    pub at_head: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// Why the role is not known, when it is not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_unknown: Option<String>,
}

/// One cited Warrant, judged.
#[derive(Debug, Clone, Serialize)]
pub struct Cited {
    pub id: String,
    /// Where the PR cites it: `body`, `commit <sha8>`, `added`.
    pub from: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Subject>,
    /// `official`, `not_official`, `unknown`, `missing`.
    pub verdict: &'static str,
    /// `author`, `formal`, `signed`, `review`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub basis: Vec<String>,
    pub why: String,
}

/// The gate's answer.
#[derive(Debug, Clone, Serialize)]
pub struct Answer {
    pub schema: &'static str,
    pub repo: String,
    pub pr: String,
    pub author: Option<String>,
    pub author_role: Option<Role>,
    pub preset: &'static str,
    pub policy_from: String,
    pub required: bool,
    pub cited: Vec<Cited>,
    pub reviews: Vec<Review>,
    /// `pass`, `refused`, `unknown`, `not_required`.
    pub verdict: &'static str,
    /// Every `gh` call made, in order.
    pub calls: Vec<String>,
    /// M17: the CI floor's answer, when the base asks for it
    /// (`[score] floor = true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor: Option<crate::score::floor::Answer>,
    /// The base commit, and whether its openwarrant.toml turns the floor on.
    #[serde(skip)]
    pub base_sha: String,
    #[serde(skip)]
    pub base_floor: bool,
}

/// Run the gate. The report carries the verdict (an error refuses, an
/// UNKNOWN never passes); the answer is the envelope's `result`.
pub fn run(repository: &Repository, args: &Args) -> (Report, Answer) {
    let mut report = Report::default();
    let mut gh = Gh {
        root: &repository.root,
        calls: Vec::new(),
    };
    let mut answer = Answer {
        schema: SCHEMA,
        repo: String::new(),
        pr: args.pr.clone(),
        author: None,
        author_role: None,
        preset: "none",
        policy_from: String::new(),
        required: true,
        cited: Vec::new(),
        reviews: Vec::new(),
        verdict: "unknown",
        calls: Vec::new(),
        floor: None,
        base_sha: String::new(),
        base_floor: false,
    };
    let unknown = |report: &mut Report, rule: &str, f: &Failed, what: &str| {
        report.push(Diagnostic::unknown(
            rule,
            format!("gh {}", f.call),
            format!(
                "{what} is UNKNOWN: `gh {}` did not answer ({}). The gate never passes on what \
                 it could not read; it passes once GitHub answers",
                f.call, f.why
            ),
        ));
    };
    if args.pr.is_empty() || !args.pr.chars().all(|c| c.is_ascii_digit()) {
        report.push(Diagnostic::error(
            "pr.number",
            "war check --pr".to_owned(),
            format!(
                "{:?} is not a PR number; `war check --pr 12` checks PR #12",
                args.pr
            ),
        ));
        answer.verdict = "refused";
        return (report, answer);
    }
    // The repository: the flag, CI's variable, or gh's answer.
    let repo_name = match args
        .repo
        .clone()
        .or_else(|| std::env::var("GITHUB_REPOSITORY").ok())
        .filter(|r| !r.trim().is_empty())
    {
        Some(r) => r,
        None => match gh.run(&[
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "-q",
            ".nameWithOwner",
        ]) {
            Ok(out) => out.trim().to_owned(),
            Err(f) => {
                unknown(&mut report, "pr.unknown", &f, "which repository this is");
                answer.calls = gh.calls;
                return (report, answer);
            }
        },
    };
    if !repo_ok(&repo_name) {
        report.push(Diagnostic::error(
            "pr.repo",
            "war check --pr".to_owned(),
            format!(
                "{repo_name:?} is not a GitHub repository name; pass `--repo owner/name` (in CI, \
                 $GITHUB_REPOSITORY)"
            ),
        ));
        answer.verdict = "refused";
        answer.calls = gh.calls;
        return (report, answer);
    }
    answer.repo.clone_from(&repo_name);
    let n = &args.pr;

    // The PR itself: without it nothing is known.
    let pr = match gh.object(&format!("repos/{repo_name}/pulls/{n}")) {
        Ok(v) => v,
        Err(f) => {
            unknown(&mut report, "pr.unknown", &f, &format!("PR #{n}"));
            answer.calls = gh.calls;
            return (report, answer);
        }
    };
    let s = |v: &serde_json::Value, path: &[&str]| {
        path.iter()
            .try_fold(v, |v, k| v.get(*k))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    let author = s(&pr, &["user", "login"]).filter(|u| login_ok(u));
    let head = s(&pr, &["head", "sha"]).unwrap_or_default();
    let base = s(&pr, &["base", "sha"]).unwrap_or_default();
    let body = s(&pr, &["body"]).unwrap_or_default();
    answer.author.clone_from(&author);

    // The policy, as the base branch has it.
    let policy = if base.is_empty() || !base.chars().all(|c| c.is_ascii_hexdigit()) {
        report.push(Diagnostic::unknown(
            "pr.unknown",
            format!("gh api repos/{repo_name}/pulls/{n}"),
            "the PR names no base commit, so the policy it is held to is UNKNOWN",
        ));
        None
    } else {
        match gh.file_at(&repo_name, crate::init::CONFIG_FILE, &base) {
            Ok(Some(text)) => match Policy::from_text(&text) {
                Ok(p) => {
                    answer.base_floor =
                        crate::score::Config::from_text(&text).is_ok_and(|c| c.floor);
                    answer.base_sha.clone_from(&base);
                    answer.policy_from = format!("openwarrant.toml at {}", short(&base));
                    Some(p)
                }
                Err(e) => {
                    report.push(Diagnostic::error(
                        crate::preset::CONFIG_RULE,
                        format!("openwarrant.toml at {}", short(&base)),
                        format!(
                            "the base branch's openwarrant.toml does not read ({e}); the gate \
                             holds a PR to the base's policy, so it passes once the base's file \
                             is fixed"
                        ),
                    ));
                    None
                }
            },
            Ok(None) => {
                answer.policy_from = "defaults (the base has no openwarrant.toml)".to_owned();
                report.push(Diagnostic::warn(
                    "pr.no-policy",
                    format!("openwarrant.toml at {}", short(&base)),
                    "the base branch has no openwarrant.toml; the gate reads the team preset's \
                     defaults",
                ));
                Some(Policy::default())
            }
            Err(f) => {
                unknown(
                    &mut report,
                    "pr.unknown",
                    &f,
                    "the base branch's [preset] and [roles]",
                );
                None
            }
        }
    };
    let Some(policy) = policy else {
        answer.calls = gh.calls;
        answer.verdict = if report.count(Severity::Error) > 0 {
            "refused"
        } else {
            "unknown"
        };
        return (report, answer);
    };
    answer.preset = policy.preset_name();
    answer.required = policy.pr_requires_official;

    // The author's role.
    let author_role = match &author {
        None => {
            report.push(Diagnostic::unknown(
                "pr.unknown",
                format!("gh api repos/{repo_name}/pulls/{n}"),
                "the PR names no author GitHub login, so the author's role is UNKNOWN",
            ));
            None
        }
        Some(user) => match gh.role(&repo_name, user) {
            Ok(r) => Some(r),
            Err(f) => {
                unknown(
                    &mut report,
                    "pr.unknown",
                    &f,
                    &format!("@{user}'s role in {repo_name}"),
                );
                None
            }
        },
    };
    answer.author_role = author_role;

    // What the PR cites: its body, its commits' trailers, the Warrants it adds.
    let mut cited: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut cite = |id: String, from: String| {
        if !cited.contains_key(&id) {
            order.push(id.clone());
        }
        let e = cited.entry(id).or_default();
        if !e.contains(&from) {
            e.push(from);
        }
    };
    for id in cited_in(&body) {
        cite(id, "body".to_owned());
    }
    let mut inputs_unknown = false;
    match gh.list(&format!("repos/{repo_name}/pulls/{n}/commits")) {
        Ok(commits) => {
            for c in &commits {
                let sha = s(c, &["sha"]).unwrap_or_default();
                for id in cited_in(&s(c, &["commit", "message"]).unwrap_or_default()) {
                    cite(id, format!("commit {}", short(&sha)));
                }
            }
        }
        Err(f) => {
            inputs_unknown = true;
            unknown(
                &mut report,
                "pr.unknown",
                &f,
                "the `Warrant:` trailers in the PR's commits",
            );
        }
    }
    let tickets_dir = crate::ticket::Store::open(repository, None)
        .ok()
        .and_then(|st| {
            st.dir
                .strip_prefix(&repository.root)
                .ok()
                .map(|p| p.as_str().to_owned())
        })
        .unwrap_or_else(|| crate::ticket::DEFAULT_DIR.to_owned());
    let warrants_dir = repository
        .warrants_dir()
        .strip_prefix(&repository.root)
        .map_or_else(|_| "docs/warrants".to_owned(), |p| p.as_str().to_owned());
    let mut roster_changed = None;
    match gh.list(&format!("repos/{repo_name}/pulls/{n}/files")) {
        Ok(files) => {
            for f in &files {
                let name = s(f, &["filename"]).unwrap_or_default();
                let status = s(f, &["status"]).unwrap_or_default();
                if matches!(
                    name.as_str(),
                    "docs/authority/roles.toml" | "docs/authority/allowed_signers"
                ) {
                    roster_changed = Some(name.clone());
                }
                if status != "added" {
                    continue;
                }
                for dir in [&tickets_dir, &warrants_dir] {
                    if let Some(id) = name
                        .strip_prefix(&format!("{dir}/"))
                        .and_then(|r| r.strip_suffix("/manifest.toml"))
                        .filter(|id| !id.contains('/'))
                    {
                        cite(id.to_owned(), "added".to_owned());
                    }
                }
            }
        }
        Err(f) => {
            inputs_unknown = true;
            unknown(
                &mut report,
                "pr.unknown",
                &f,
                "the Warrants the PR adds, and whether it changes the roster",
            );
        }
    }
    if let Some(file) = &roster_changed {
        report.push(Diagnostic::warn(
            "pr.roster-changed",
            file.clone(),
            format!(
                "this PR changes {file}, so an approval signed by a roster key is not counted for \
                 it: the roster that would verify it is the one being changed. An approving \
                 review, or a Warrant the author's role allows, still counts"
            ),
        ));
    }

    // The reviews: each reviewer's latest, and the role of each who approved.
    let mut reviews_unknown = false;
    match gh.list(&format!("repos/{repo_name}/pulls/{n}/reviews")) {
        Ok(list) => {
            let mut latest: BTreeMap<String, (String, String)> = BTreeMap::new();
            for r in &list {
                let (Some(user), Some(state)) = (s(r, &["user", "login"]), s(r, &["state"])) else {
                    continue;
                };
                // A comment does not change a reviewer's verdict.
                if state == "COMMENTED" || state == "PENDING" {
                    continue;
                }
                latest.insert(user, (state, s(r, &["commit_id"]).unwrap_or_default()));
            }
            for (reviewer, (state, commit)) in latest {
                let mut review = Review {
                    reviewer: reviewer.clone(),
                    state: state.clone(),
                    at_head: !head.is_empty() && commit == head,
                    role: None,
                    role_unknown: None,
                };
                if state == "APPROVED" && Some(&reviewer) != author.as_ref() {
                    if !login_ok(&reviewer) {
                        review.role_unknown = Some("not a GitHub login".to_owned());
                    } else {
                        match gh.role(&repo_name, &reviewer) {
                            Ok(r) => review.role = Some(r),
                            Err(f) => {
                                review.role_unknown = Some(f.why.clone());
                                unknown(
                                    &mut report,
                                    "pr.unknown",
                                    &f,
                                    &format!("@{reviewer}'s role, who approved"),
                                );
                            }
                        }
                    }
                }
                answer.reviews.push(review);
            }
        }
        Err(f) => {
            reviews_unknown = true;
            unknown(&mut report, "pr.unknown", &f, "the PR's reviews");
        }
    }

    // Each cited Warrant, judged at the author's level.
    let author_text = author.as_deref().map_or_else(
        || "the author".to_owned(),
        |a| match author_role {
            Some(Role::None) => format!("@{a} (no role in {repo_name}: an outsider)"),
            Some(r) => format!("@{a} ({r})"),
            None => format!("@{a} (role UNKNOWN)"),
        },
    );
    for id in &order {
        let from = cited.get(id).cloned().unwrap_or_default();
        let subject = match official::subject(repository, id) {
            Ok(s) => s,
            Err(d) => {
                answer.cited.push(Cited {
                    id: id.clone(),
                    from,
                    subject: None,
                    verdict: "missing",
                    basis: Vec::new(),
                    why: d.message,
                });
                continue;
            }
        };
        answer.cited.push(judge(
            repository,
            &policy,
            subject,
            from,
            &JudgeInput {
                author_role,
                author_text: &author_text,
                reviews: &answer.reviews,
                reviews_unknown,
                roster_changed: roster_changed.is_some(),
            },
        ));
    }

    // The verdict.
    for c in &answer.cited {
        let at = c
            .subject
            .as_ref()
            .map_or_else(|| c.id.clone(), |s| format!("{} [{}]", s.id, s.kind));
        let line = format!("{at} (cited in {}): {}", c.from.join(", "), c.why);
        report.push(match c.verdict {
            "official" => Diagnostic::pass("pr.cited", line),
            "missing" => Diagnostic::warn("pr.warrant-unknown", c.id.clone(), line),
            _ => Diagnostic::warn("pr.cited", c.id.clone(), line),
        });
    }
    let official: Vec<&Cited> = answer
        .cited
        .iter()
        .filter(|c| c.verdict == "official")
        .collect();
    let maybe: Vec<&Cited> = answer
        .cited
        .iter()
        .filter(|c| c.verdict == "unknown")
        .collect();
    answer.calls.clone_from(&gh.calls);
    if !policy.pr_requires_official {
        report.diagnostics.retain(|d| {
            !matches!(d.severity, Severity::Unknown | Severity::Error) || d.rule == "pr.number"
        });
        report.push(Diagnostic::pass(
            "pr.not-required",
            format!(
                "the {} preset (as the base has it) does not require an official Warrant, so PR \
                 #{n} passes; {} of {} cited Warrant(s) official",
                policy.preset_name(),
                official.len(),
                answer.cited.len()
            ),
        ));
        answer.verdict = "not_required";
        return (report, answer);
    }
    if let Some(c) = official.first() {
        report.push(Diagnostic::pass(
            "pr.official",
            format!(
                "PR #{n} by {author_text} cites {}, official ({}); it passes",
                c.id,
                c.basis.join(", ")
            ),
        ));
        // What could not be read no longer decides anything.
        report
            .diagnostics
            .retain(|d| d.severity != Severity::Unknown);
        answer.verdict = "pass";
        return (report, answer);
    }
    let remedy = remedy(&policy, author_role, &answer.cited, n);
    if answer.cited.is_empty() {
        if inputs_unknown {
            report.push(Diagnostic::unknown(
                "pr.unknown",
                format!("PR #{n}"),
                format!(
                    "PR #{n} cites no Warrant in its body, and its commits or files could not be \
                     read; whether it cites one is UNKNOWN"
                ),
            ));
            answer.verdict = "unknown";
        } else {
            report.push(Diagnostic::error(
                "pr.no-warrant",
                format!("PR #{n}"),
                format!(
                    "PR #{n} by {author_text} cites no Warrant: no `Warrant: <id>` line in its body \
                     or commits, and it adds none. It passes with {remedy}"
                ),
            ));
            answer.verdict = "refused";
        }
    } else if !maybe.is_empty() || author_role.is_none() {
        report.push(Diagnostic::unknown(
            "pr.unknown",
            format!("PR #{n}"),
            format!(
                "PR #{n} by {author_text}: no cited Warrant is official on what could be read, and \
                 what could not be read might make one official; UNKNOWN, never a pass. It passes \
                 with {remedy}"
            ),
        ));
        answer.verdict = "unknown";
    } else {
        report.push(Diagnostic::error(
            "pr.not-official",
            format!("PR #{n}"),
            format!(
                "PR #{n} by {author_text} cites {}, and none is official at that level. It passes \
                 with {remedy}",
                answer
                    .cited
                    .iter()
                    .map(|c| c.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
        answer.verdict = "refused";
    }
    if answer.verdict == "refused"
        && let Some(d) = crate::notify::human_waits(
            &repository.root,
            &crate::notify::Wait {
                event: "pr.approval-needed",
                subject: &format!("{repo_name}#{n}"),
                message: &format!("PR #{n} waits on an official Warrant: {remedy}"),
                command: &format!("war check --pr {n}"),
            },
        )
    {
        report.push(d);
    }
    (report, answer)
}

/// The CI floor (OW-WAR-0148 M17): when the base's openwarrant.toml says
/// `[score] floor = true`, the PR passes only if it keeps the base's level.
/// A floor that refuses refuses the PR; one that is UNKNOWN keeps a pass
/// from being a pass.
pub fn apply_floor(repository: &Repository, report: &mut Report, answer: &mut Answer) {
    if !answer.base_floor || answer.base_sha.is_empty() {
        return;
    }
    let (r, floor) = crate::score::floor::check(repository, &answer.base_sha);
    report.diagnostics.extend(r.diagnostics);
    match floor.verdict {
        "refused" => answer.verdict = "refused",
        "unknown" if matches!(answer.verdict, "pass" | "not_required") => {
            answer.verdict = "unknown";
        }
        _ => {}
    }
    answer.floor = Some(floor);
}

/// What the judge needs beside the Warrant.
struct JudgeInput<'a> {
    author_role: Option<Role>,
    author_text: &'a str,
    reviews: &'a [Review],
    reviews_unknown: bool,
    roster_changed: bool,
}

/// One cited Warrant at the author's level.
fn judge(
    repository: &Repository,
    policy: &Policy,
    subject: Subject,
    from: Vec<String>,
    input: &JudgeInput<'_>,
) -> Cited {
    let kind = subject.kind;
    let mut basis = Vec::new();
    let mut why = Vec::new();
    if kind == Kind::Formal {
        basis.push("formal".to_owned());
        why.push("formal: every role may make one alone".to_owned());
    } else {
        match input.author_role {
            Some(r) if policy.allows(r, kind) => {
                basis.push("author".to_owned());
                why.push(format!(
                    "{} may make a {kind} Warrant alone",
                    input.author_text
                ));
            }
            Some(r) => why.push(format!(
                "{} needs a {} Warrant to go alone, and this one is {kind} ({})",
                input.author_text,
                policy.least_kind(r),
                kind.describe()
            )),
            None => {}
        }
    }
    // A signed approval: verified against this tree's roster, the role read
    // from the base's [roles.roster], and only when the PR leaves the
    // roster alone.
    let approval = official::approval(repository, policy, &subject);
    let mut signed = subject.signed;
    if let Some(a) = &approval {
        if input.roster_changed {
            why.push(format!(
                "the approval in {} is not counted: this PR changes the roster",
                a.response
            ));
        } else if a.counts {
            signed = true;
            if !basis.contains(&"formal".to_owned()) {
                basis.push("signed".to_owned());
            }
            why.push(a.why.clone());
        } else {
            why.push(format!("the approval on record does not count: {}", a.why));
        }
    }
    // Reviews, at the head commit, by a role that may make this kind.
    let mut review_unknown = input.reviews_unknown;
    for r in input.reviews.iter().filter(|r| r.state == "APPROVED") {
        match (r.role, r.at_head) {
            (Some(role), true) if policy.allows(role, kind) => {
                if !basis.contains(&"review".to_owned()) && kind != Kind::Formal {
                    basis.push("review".to_owned());
                }
                why.push(format!("approved in review by @{} ({role})", r.reviewer));
            }
            (Some(role), true) => why.push(format!(
                "@{}'s approving review does not count: a {role} approves {} Warrants or more \
                 formal, and this one is {kind}",
                r.reviewer,
                policy.least_kind(role)
            )),
            (Some(_), false) => why.push(format!(
                "@{}'s approval is of an earlier commit, not the head; a new approval counts",
                r.reviewer
            )),
            (None, _) => review_unknown = true,
        }
    }
    let official = !basis.is_empty();
    // Signing at merge: an official Warrant also carries a verified
    // signature by merge time.
    let needs_signature = official && policy.signing == Signing::Merge && !signed;
    let verdict = if official && !needs_signature {
        "official"
    } else if !official && (review_unknown || input.author_role.is_none()) {
        "unknown"
    } else {
        "not_official"
    };
    if needs_signature {
        why.push(format!(
            "official, and the preset signs at merge: it also needs {}",
            if subject.encoding == "light" {
                format!(
                    "`war sign approve {} --ssh-sign` by a roster key",
                    subject.id
                )
            } else {
                format!(
                    "its authorization signed: `war sign {} --ssh-sign`",
                    subject.id
                )
            }
        ));
    }
    Cited {
        id: subject.id.clone(),
        from,
        subject: Some(subject),
        verdict,
        basis,
        why: if why.is_empty() {
            "nothing makes it official".to_owned()
        } else {
            why.join("; ")
        },
    }
}

/// What would make the PR pass, said once.
fn remedy(policy: &Policy, author_role: Option<Role>, cited: &[Cited], n: &str) -> String {
    let kind_alone = author_role.map(|r| policy.least_kind(r));
    let approvers = |k: Kind| policy.roles_allowing_text(k);
    let lowest = cited
        .iter()
        .filter_map(|c| c.subject.as_ref().map(|s| s.kind))
        .min()
        .unwrap_or(Kind::Vibe);
    let mut ways = Vec::new();
    match kind_alone {
        Some(Kind::Formal) | None => ways.push(
            "a formal Warrant in the PR (`war plan new \"...\"`, its atoms written, passing `war \
             check`) and its id in the body as `Warrant: <id>`"
                .to_owned(),
        ),
        Some(Kind::Tested) => ways.push(
            "a Warrant with a test (`war create \"...\"`, then `war add <id> --test \
             \"<command>\"`) cited as `Warrant: <id>` in the PR body"
                .to_owned(),
        ),
        Some(Kind::Vibe) => {
            ways.push("any Warrant cited as `Warrant: <id>` in the PR body".to_owned());
        }
    }
    ways.push(format!(
        "an approving review at the head commit by {}",
        approvers(lowest)
    ));
    if let Some(c) = cited.iter().find(|c| {
        c.subject
            .as_ref()
            .is_some_and(|s| s.encoding == "light" && s.kind < Kind::Formal)
    }) {
        ways.push(format!(
            "`war sign approve {} --ssh-sign` by a roster key whose [roles.roster] role is {}",
            c.id,
            approvers(lowest)
        ));
    }
    if policy.signing == Signing::Merge {
        ways.push(format!(
            "(the preset signs at merge, so the Warrant also needs a verified signature before \
             `war check --pr {n}` passes)"
        ));
    }
    ways.join("; or ")
}

/// The first eight characters of a commit id.
fn short(sha: &str) -> &str {
    sha.get(..8).unwrap_or(sha)
}

/// The Markdown summary: the job summary, or the PR comment.
#[must_use]
pub fn summary(report: &Report, answer: &Answer) -> String {
    let verdict = match answer.verdict {
        "pass" => "passes",
        "not_required" => "passes (not required by the preset)",
        "refused" => "is refused",
        _ => "is UNKNOWN (never a pass)",
    };
    let mut s = format!(
        "### OpenWarrant: PR #{} {verdict}\n\nAuthor: {} · preset {} ({})\n\n",
        answer.pr,
        answer.author.as_deref().map_or_else(
            || "unknown".to_owned(),
            |a| format!(
                "@{a} ({})",
                answer
                    .author_role
                    .map_or_else(|| "role unknown".to_owned(), |r| r.as_str().to_owned())
            )
        ),
        answer.preset,
        answer.policy_from
    );
    if answer.cited.is_empty() {
        s.push_str("No Warrant is cited.\n\n");
    } else {
        s.push_str("| Warrant | kind | official | why |\n|---|---|---|---|\n");
        for c in &answer.cited {
            s.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                c.id,
                c.subject.as_ref().map_or("?", |s| s.kind.as_str()),
                c.verdict,
                c.why.replace('|', "\\|")
            ));
        }
        s.push('\n');
    }
    if let Some(f) = &answer.floor {
        s.push_str(&format!(
            "Score floor: level {} at the base, {} ({} / 1000) here: {}\n\n",
            f.base_score.as_ref().map_or_else(
                || "UNKNOWN".to_owned(),
                |b| format!("{} ({} / 1000)", b.level.number, b.score)
            ),
            f.head_score.level.number,
            f.head_score.score,
            f.verdict
        ));
    }
    for d in report
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Unknown))
    {
        s.push_str(&format!(
            "- **{}** `{}`: {}\n",
            d.severity, d.rule, d.message
        ));
    }
    s
}

/// `--summary <file>`: append the summary (GitHub's step summary is
/// appended to); `--comment`: post it through `gh`. Either failing is a
/// warning and never changes the verdict.
pub fn publish(repository: &Repository, args: &Args, report: &mut Report, answer: &Answer) {
    let text = summary(report, answer);
    if let Some(path) = &args.summary {
        use std::io::Write as _;
        let written = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(text.as_bytes()));
        if let Err(e) = written {
            report.push(Diagnostic::warn(
                "pr.summary-failed",
                path.to_string(),
                format!("the summary was not written ({e}); the verdict stands"),
            ));
        }
    }
    if args.comment && repo_ok(&answer.repo) {
        let mut gh = Gh {
            root: &repository.root,
            calls: Vec::new(),
        };
        let path = format!("repos/{}/issues/{}/comments", answer.repo, answer.pr);
        let body = format!("body={text}");
        if let Err(f) = gh.run(&["api", "-X", "POST", &path, "-f", &body]) {
            report.push(Diagnostic::warn(
                "pr.comment-failed",
                format!("gh api {path}"),
                format!("the comment was not posted ({}); the verdict stands", f.why),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailers_and_body_lines_are_read() {
        let msg = "Fix the redirect\n\nWarrant: t-3f2a\nwarrants: OW-WAR-0001, t-9c01\n- Warrant: `t-77be`\nNot a Warrant: here\n";
        assert_eq!(cited_in(msg), ["t-3f2a", "OW-WAR-0001", "t-9c01", "t-77be"]);
        assert!(cited_in("Warrant:\nno ids").is_empty());
    }

    #[test]
    fn names_are_checked_before_they_reach_gh() {
        assert!(repo_ok("Quitetall/OpenWarrant"));
        assert!(!repo_ok("a/b/c") && !repo_ok("a") && !repo_ok("a/b c") && !repo_ok("/b"));
        assert!(login_ok("dependabot[bot]") && login_ok("ada-l"));
        assert!(!login_ok("a/b") && !login_ok(""));
    }
}
