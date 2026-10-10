// SPDX-License-Identifier: Apache-2.0
//! Official Warrants (OW-WAR-0148 M14, decision 6).
//!
//! A Warrant is **official** when someone allowed made it, or approved it:
//!
//! 1. its author's role allows its kind (`[roles]`: the least formal kind
//!    each role creates alone); or
//! 2. a human approved it with `war sign approve <id> --ssh-sign`, the
//!    signature verifies against the roster (`docs/authority/roles.toml`
//!    and `allowed_signers`, or the protected store), and the approver's
//!    role in `[roles.roster]` allows its kind; or
//! 3. an approving GitHub review by an allowed role is on the PR. That is
//!    read through `gh api` by `war check --pr` in CI; anywhere else it is
//!    UNKNOWN, never a pass.
//!
//! The kind is read from the record, never declared: a light Warrant is
//! `tested` when it carries a test (or a KPI that decides a tick) and
//! `vibe` otherwise; a directory Warrant is `formal` when `war check` passes
//! it, and counts as no more than its title (`vibe`) when it does not.
//!
//! An approval binds a statement of the Warrant: its id, identity, title,
//! description and items' text, and its tests. Ticking an item or adding a
//! note leaves it standing; changing the plan does not, and the approval is
//! then named stale.
//!
//! Nothing here restricts work. An unofficial Warrant is worked, claimed and
//! ticked like any other; officialness is what the PR gate reads.

use camino::Utf8PathBuf;
use serde::Serialize;

use crate::authority_check::{self, Act};
use crate::diagnostic::Diagnostic;
use crate::preset::{Kind, Policy, Role};
use crate::repo::{RepoError, Repository};
use crate::ticket::{self, Outcome, Store, Target, Ticket};

/// The schema an approval response carries.
pub const APPROVAL_SCHEMA: &str = "oh.war/warrant-approval-response/v1";

/// The journal events (a light Warrant's own journal).
pub mod event {
    /// Someone asked a person to approve the Warrant.
    pub const REQUESTED: &str = "ticket.approval_requested";
    /// A human's approval was recorded, its signature verified.
    pub const APPROVED: &str = "ticket.approved";
}

/// What a Warrant is, for officialness.
#[derive(Debug, Clone, Serialize)]
pub struct Subject {
    pub id: String,
    pub title: String,
    /// `light` or `directory`.
    pub encoding: &'static str,
    pub kind: Kind,
    /// Why it is that kind, in words.
    pub kind_why: String,
    /// Who made it, as recorded (a light Warrant's `created_by`). A name,
    /// not a proof.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// The digest an approval binds (light Warrants).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// A directory Warrant whose authorization a human signed and `war
    /// check` verifies (what signing at merge asks of one).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub signed: bool,
    #[serde(skip)]
    pub uuid: String,
}

/// The statement an approval of `t` is over: what the plan is. Ticks,
/// claims and notes are not in it.
#[must_use]
pub fn statement(t: &Ticket) -> String {
    let mut s = format!(
        "oh.war/warrant-approval/v1\n{}\n{}\n{}\n\n{}\n\n",
        t.manifest.uuid,
        t.id(),
        openwarrant_core::ticket::one_line(&t.manifest.title),
        ticket::description(&t.intent)
    );
    for item in &t.checklist.items {
        s.push_str(&format!(
            "- {} {}\n",
            item.id.as_deref().unwrap_or("-"),
            openwarrant_core::ticket::one_line(&item.text)
        ));
    }
    let checks = ticket::ladder::checks_of(t);
    for test in &checks.tests {
        s.push_str(&format!(
            "test {} {} {}\n",
            test.name,
            test.item.as_deref().unwrap_or("*"),
            test.cmd
        ));
    }
    for k in &checks.kpis {
        s.push_str(&format!(
            "kpi {} {} {} {:?} {}\n",
            k.name,
            k.item.as_deref().unwrap_or("*"),
            k.cmd,
            k.target,
            k.mode.as_str()
        ));
    }
    s
}

/// `sha256:<hex>` of [`statement`].
#[must_use]
pub fn statement_digest(t: &Ticket) -> String {
    format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(statement(t).as_bytes())
    )
}

/// A light Warrant, read as a subject.
#[must_use]
pub fn light(t: &Ticket) -> Subject {
    let checks = ticket::ladder::checks_of(t);
    let deciding_kpis = checks
        .kpis
        .iter()
        .filter(|k| k.target.is_some() && k.mode != openwarrant_core::ticks::KpiMode::Optimise)
        .count();
    let (kind, kind_why) = if !checks.tests.is_empty() || deciding_kpis > 0 {
        (
            Kind::Tested,
            format!(
                "{} test(s) and {deciding_kpis} deciding KPI(s) in {}",
                checks.tests.len(),
                openwarrant_core::ticks::CHECKS_FILE
            ),
        )
    } else {
        (Kind::Vibe, "a title and checklist, no test".to_owned())
    };
    Subject {
        id: t.id().to_owned(),
        title: t.manifest.title.clone(),
        encoding: "light",
        kind,
        kind_why,
        author: Some(t.manifest.created_by.clone()),
        digest: Some(statement_digest(t)),
        signed: false,
        uuid: t.manifest.uuid.clone(),
    }
}

/// A directory Warrant, read as a subject: formal when `war check` passes
/// it.
pub fn directory(repo: &Repository, alias: &str) -> Result<Subject, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let loaded = repo.load_warrant(&dir)?;
    let title = loaded
        .validated
        .as_ref()
        .map(|v| v.raw.title.clone())
        .unwrap_or_default();
    let uuid = loaded
        .validated
        .as_ref()
        .map(|v| v.uuid.to_string())
        .unwrap_or_default();
    let report = crate::check::run(repo, Some(alias), false)?;
    let errors: Vec<String> = report
        .diagnostics
        .iter()
        .filter(|d| d.severity.blocks_readiness())
        .map(|d| d.rule.clone())
        .collect();
    let signed = report.diagnostics.iter().any(|d| {
        d.severity == crate::diagnostic::Severity::Pass
            && d.rule == "authority.signed"
            && d.message.starts_with(&format!("{alias}: "))
    });
    let (kind, kind_why) = if errors.is_empty() {
        (
            Kind::Formal,
            format!("a directory Warrant `war check {alias}` passes"),
        )
    } else {
        let mut named = errors.clone();
        named.dedup();
        named.truncate(4);
        (
            Kind::Vibe,
            format!(
                "a directory Warrant `war check {alias}` refuses ({} finding(s): {}), so it counts \
                 as its title until that passes",
                errors.len(),
                named.join(", ")
            ),
        )
    };
    Ok(Subject {
        id: alias.to_owned(),
        title,
        encoding: "directory",
        kind,
        kind_why,
        author: None,
        digest: None,
        signed,
        uuid,
    })
}

/// A Warrant by id, as a subject: a light one through the store, a
/// directory one through its check. A Warrant read in place is another
/// tool's record and is refused here.
pub fn subject(repo: &Repository, id: &str) -> Result<Subject, Diagnostic> {
    match crate::warrants::kind_of(id) {
        crate::warrants::IdKind::Light => {
            let store = Store::open(repo, None).map_err(|e| {
                Diagnostic::error("tickets.config", crate::init::CONFIG_FILE, e.to_string())
            })?;
            let (tickets, _) = store.load_all().map_err(|e| {
                Diagnostic::error("tickets.config", crate::init::CONFIG_FILE, e.to_string())
            })?;
            match ticket::resolve(&tickets, id)? {
                Target::Ticket(n) | Target::Item(n, _) => Ok(light(&tickets[n])),
            }
        }
        crate::warrants::IdKind::ReadInPlace => Err(Diagnostic::error(
            "official.read-in-place",
            id.to_owned(),
            format!(
                "{id} is read in place from another tool's folder; cite a Warrant of this \
                 repository (`war create`, or `war plan new`)"
            ),
        )),
        _ => directory(repo, id).map_err(|e| {
            Diagnostic::error(
                "official.unknown",
                id.to_owned(),
                format!("{id} names no Warrant here: {e}"),
            )
        }),
    }
}

/// One approval on record.
#[derive(Debug, Clone, Serialize)]
pub struct Approval {
    /// Who the response says approved.
    pub approver: String,
    /// The principal the signature verified as.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    /// The role `[roles.roster]` gives that principal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    pub response: String,
    /// Whether it makes the Warrant official.
    pub counts: bool,
    pub why: String,
}

/// The approval response on record for `subject`, judged: its signature,
/// whether it is over the plan as it stands, and whether the approver's
/// role allows the kind. `None` when there is none.
#[must_use]
pub fn approval(repo: &Repository, policy: &Policy, subject: &Subject) -> Option<Approval> {
    let path = authority_check::response_path(repo, Act::Approve, &subject.id);
    let text = crate::vfs::read_to_string(&path).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let field = |k: &str| {
        value
            .get(k)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let approver = field("approved_by").unwrap_or_default();
    let response = repo.relative(&path);
    let refuse = |why: String| Approval {
        approver: approver.clone(),
        principal: None,
        role: None,
        response: response.clone(),
        counts: false,
        why,
    };
    if field("schema").as_deref() != Some(APPROVAL_SCHEMA) {
        return Some(refuse(format!(
            "{response} is not an {APPROVAL_SCHEMA} response"
        )));
    }
    let Some(digest) = subject.digest.as_deref() else {
        return Some(refuse(
            "a directory Warrant is official through `war check`, not an approval".to_owned(),
        ));
    };
    if field("sha256").as_deref() != Some(digest) {
        return Some(refuse(format!(
            "the approval is over an earlier plan ({}); the Warrant's title, description, items or \
             tests changed since, so it is stale. `war sign approve {}` asks again",
            field("sha256").unwrap_or_default(),
            subject.id
        )));
    }
    let verdict = authority_check::verify(repo, Act::Approve, &subject.id, &approver, Some(digest));
    let authority_check::Verdict::Signed { principal, .. } = &verdict else {
        return Some(refuse(format!(
            "its signature does not verify ({}): {}",
            verdict.rule(),
            verdict.why()
        )));
    };
    let role = policy.roster.get(principal).copied();
    let (counts, why) = match role {
        None => (
            false,
            format!(
                "signed by {approver} ({principal}), whose principal has no role in \
                 [roles.roster], so the approval counts as no role's"
            ),
        ),
        Some(r) if policy.allows(r, subject.kind) => (
            true,
            format!(
                "approved by {approver} ({principal}, {r}); {r} may make a {} Warrant",
                subject.kind
            ),
        ),
        Some(r) => (
            false,
            format!(
                "approved by {approver} ({principal}, {r}), and a {r} approves {} Warrants or \
                 more formal; a {} one needs {}",
                policy.least_kind(r),
                subject.kind,
                policy.roles_allowing_text(subject.kind)
            ),
        ),
    };
    Some(Approval {
        approver,
        principal: Some(principal.clone()),
        role,
        response,
        counts,
        why,
    })
}

/// The role the roster gives a name: the register's entry for that actor,
/// its `ssh_principal`, and `[roles.roster]`. `None` when any link is
/// missing.
#[must_use]
pub fn roster_role(repo: &Repository, policy: &Policy, actor: &str) -> Option<(String, Role)> {
    let register = repo.load_authority_register().ok()?;
    let principal = register
        .assignments
        .iter()
        .find(|a| a.actor == actor)
        .and_then(|a| a.ssh_principal.clone())?;
    let role = policy.roster.get(&principal).copied()?;
    Some((principal, role))
}

/// Where one Warrant stands, read from this checkout.
#[derive(Debug, Clone, Serialize)]
pub struct Standing {
    #[serde(flatten)]
    pub subject: Subject,
    /// `true` when official; `null` when nothing here makes it official and
    /// an approving review (read only in CI) still could. Never `false`
    /// from a checkout: the review is UNKNOWN here.
    pub official: Option<bool>,
    /// `formal`, `author` or `signed`, when official.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis: Option<&'static str>,
    pub why: String,
    /// What would make it official, when it is not yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<Approval>,
    /// The GitHub review, which a checkout cannot read.
    pub review: &'static str,
}

/// What a checkout can tell.
pub const REVIEW_LOCAL: &str = "unknown here; `war check --pr` reads it from GitHub in CI";

/// Where `subject` stands, from this checkout.
#[must_use]
pub fn local(repo: &Repository, policy: &Policy, subject: Subject) -> Standing {
    let approval = approval(repo, policy, &subject);
    let mut standing = Standing {
        official: None,
        basis: None,
        why: String::new(),
        remedy: None,
        approval: None,
        review: REVIEW_LOCAL,
        subject,
    };
    if standing.subject.kind == Kind::Formal {
        standing.official = Some(true);
        standing.basis = Some("formal");
        standing.why = "formal: every role may make a formal Warrant alone".to_owned();
        standing.approval = approval;
        return standing;
    }
    let kind = standing.subject.kind;
    let author_role = standing
        .subject
        .author
        .as_deref()
        .and_then(|a| roster_role(repo, policy, a).map(|(p, r)| (a.to_owned(), p, r)));
    if let Some((who, principal, role)) = &author_role
        && policy.allows(*role, kind)
    {
        standing.official = Some(true);
        standing.basis = Some("author");
        standing.why = format!(
            "made by {who} ({principal}, {role} by the roster), and {role} may make a {kind} \
             Warrant alone (the author is the name recorded at creation)"
        );
    } else if let Some(a) = approval.as_ref().filter(|a| a.counts) {
        standing.official = Some(true);
        standing.basis = Some("signed");
        standing.why = a.why.clone();
    } else {
        let author = match (&standing.subject.author, &author_role) {
            (Some(_), Some((who, _, role))) => format!(
                "made by {who} ({role} by the roster), who needs a {} Warrant to go alone",
                policy.least_kind(*role)
            ),
            (Some(who), None) => format!("made by {who}, whose role the roster does not give"),
            (None, _) => "its author's role is not recorded".to_owned(),
        };
        let approved = approval
            .as_ref()
            .map(|a| format!("; the approval on record does not count: {}", a.why))
            .unwrap_or_default();
        standing.why = format!("{author}{approved}");
        standing.remedy = Some(format!(
            "an approval by {}: an approving review on the PR, or `war sign approve {} \
             --ssh-sign` by a roster key with that role{}",
            policy.roles_allowing_text(kind),
            standing.subject.id,
            if kind < Kind::Formal {
                format!(
                    "; or a {} Warrant, which its author's role allows alone",
                    next_kind(kind)
                )
            } else {
                String::new()
            }
        ));
    }
    standing.approval = approval;
    standing
}

const fn next_kind(k: Kind) -> &'static str {
    match k {
        Kind::Vibe => "tested (`war add <id> --test \"<command>\"`) or formal",
        Kind::Tested | Kind::Formal => "formal (`war plan new`)",
    }
}

impl Standing {
    /// One line: `official (author): ...` or `official: not established
    /// here: ...`.
    #[must_use]
    pub fn line(&self) -> String {
        match (self.official, self.basis) {
            (Some(true), Some(basis)) => format!("official ({basis}): {}", self.why),
            _ => format!("official: not established here: {}", self.why),
        }
    }

    /// The `## Official` section `war show` appends.
    #[must_use]
    pub fn section(&self, policy: &Policy) -> String {
        let mut s = format!(
            "\n## Official\n\n{}\nkind: {} ({})\nGitHub review: {}\n",
            self.line(),
            self.subject.kind,
            self.subject.kind_why,
            self.review
        );
        if let Some(r) = &self.remedy {
            s.push_str(&format!("official with: {r}\n"));
        }
        s.push_str(&format!("preset: {}\n", policy.preset_name()));
        s
    }
}

/// Every Warrant's standing, for `war status`: light ones first.
pub fn corpus(repo: &Repository, policy: &Policy) -> Vec<Standing> {
    let mut out = Vec::new();
    if let Ok(store) = Store::open(repo, None)
        && let Ok((tickets, _)) = store.load_all()
    {
        for t in &tickets {
            out.push(local(repo, policy, light(t)));
        }
    }
    if let Ok(dirs) = repo.warrant_dirs() {
        for dir in dirs {
            let Some(alias) = dir.file_name() else {
                continue;
            };
            if let Ok(s) = directory(repo, alias) {
                out.push(local(repo, policy, s));
            }
        }
    }
    out
}

/// The `## Official` block `war status` prints after the projection.
#[must_use]
pub fn corpus_block(standings: &[Standing], policy: &Policy) -> String {
    let official = standings
        .iter()
        .filter(|s| s.official == Some(true))
        .count();
    let mut s = format!(
        "## Official\n\npreset {}: {official} of {} Warrant(s) official from this checkout; an \
         approving GitHub review is read only in CI (`war check --pr`).\n\n",
        policy.preset_name(),
        standings.len()
    );
    for st in standings {
        s.push_str(&format!(
            "- {} [{}] {}\n",
            st.subject.id,
            st.subject.kind,
            match (st.official, st.basis) {
                (Some(true), Some(b)) => format!("official ({b})"),
                _ => format!(
                    "not established here; {}",
                    st.remedy.as_deref().unwrap_or("an approval")
                ),
            }
        ));
    }
    s
}

// ---- `war sign approve <id>` ----------------------------------------------------

/// What `war sign approve` is given.
#[derive(Debug, Clone, Default)]
pub struct ApproveArgs {
    pub actor: Option<String>,
    pub meaning: Option<String>,
    pub ssh_sign: bool,
    pub dry_run: bool,
}

/// The humans who may approve `subject`: a human in the register with an
/// `ssh_principal` whose role in `[roles.roster]` allows its kind. The
/// second list names everyone left out, and why.
fn approvers(repo: &Repository, policy: &Policy, kind: Kind) -> (Vec<String>, Vec<String>) {
    let Ok(register) = repo.load_authority_register() else {
        return (Vec::new(), Vec::new());
    };
    let mut ok = Vec::new();
    let mut out = Vec::new();
    for a in &register.assignments {
        if a.actor_kind != openwarrant_core::authority::ActorKind::Human {
            continue;
        }
        let Some(p) = &a.ssh_principal else {
            out.push(format!("{} (no ssh_principal)", a.actor));
            continue;
        };
        match policy.roster.get(p) {
            Some(r) if policy.allows(*r, kind) => ok.push(a.actor.clone()),
            Some(r) => out.push(format!("{} ({p}, {r})", a.actor)),
            None => out.push(format!("{} ({p}, not in [roles.roster])", a.actor)),
        }
    }
    (ok, out)
}

/// Whether `actor` may approve a Warrant of `kind` (for `war sign inbox
/// --as`).
#[must_use]
pub fn may_approve(repo: &Repository, policy: &Policy, kind: Kind, actor: &str) -> bool {
    approvers(repo, policy, kind).0.iter().any(|a| a == actor)
}

/// Whether an approval of the plan as it stands was already asked for.
fn requested_at(t: &Ticket, digest: &str) -> bool {
    let Ok(text) = crate::vfs::read_to_string(t.dir.join(crate::journal_cmd::FILE)) else {
        return false;
    };
    if !text.contains(event::REQUESTED) {
        return false;
    }
    crate::journal_cmd::parse(&text).is_ok_and(|j| {
        j.events.iter().any(|e| {
            e.event_type == event::REQUESTED
                && serde_json::from_str::<serde_json::Value>(&e.payload)
                    .ok()
                    .and_then(|p| p.get("digest").and_then(|d| d.as_str()).map(str::to_owned))
                    .as_deref()
                    == Some(digest)
        })
    })
}

/// `war sign approve <id> [--ssh-sign] [--dry-run] [--as <human>]`.
///
/// Without `--ssh-sign`: the request. It names what an approval binds and
/// who may sign it, records `ticket.approval_requested` in the Warrant's
/// journal (once per plan) so `war next` and `war sign inbox` list it, and
/// runs `[notify]`. Writes no response and touches no key. With
/// `--ssh-sign`: a human's approval, signed with their key through the ssh
/// agent and recorded only when the signature verifies as theirs.
pub fn approve(
    repo: &Repository,
    store: &Store,
    query: &str,
    args: &ApproveArgs,
) -> Result<Outcome, RepoError> {
    if crate::warrants::kind_of(query) != crate::warrants::IdKind::Light {
        return Ok(Outcome::refused(
            "approve.directory",
            query.to_owned(),
            format!(
                "{query} is not a light Warrant. A directory Warrant is official when `war check \
                 {query}` passes (it is formal); its authorization is `war sign {query}`"
            ),
        ));
    }
    let policy = Policy::read(&repo.root)
        .map_err(|e| RepoError::Message(format!("{}: {e}", crate::preset::CONFIG_RULE)))?;
    let (tickets, _) = store.load_all()?;
    let t = match ticket::resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => &tickets[n],
        Ok(Target::Item(n, _)) => {
            return Ok(Outcome::refused(
                "approve.item",
                query.to_owned(),
                format!(
                    "{query} names an item; an approval is of the whole Warrant: `war sign approve \
                     {}`",
                    tickets[n].id()
                ),
            ));
        }
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let subject = light(t);
    let digest = subject.digest.clone().unwrap_or_default();
    let standing = local(repo, &policy, subject.clone());
    let what = t.id().to_owned();
    if standing.approval.as_ref().is_some_and(|a| a.counts) {
        return Ok(Outcome::ok(
            format!(
                "{what} is already approved, and the signature verifies: {}",
                standing
                    .approval
                    .as_ref()
                    .map(|a| a.why.as_str())
                    .unwrap_or("")
            ),
            serde_json::json!({"schema": "oh.war/warrant-approval/v1", "warrant": what,
                "already": true, "standing": standing}),
        ));
    }
    let (eligible, left_out) = approvers(repo, &policy, subject.kind);
    let who = if eligible.is_empty() {
        format!(
            "nobody in docs/authority/roles.toml can sign it yet: a human with an ssh_principal \
             whose role in [roles.roster] is {}{}",
            policy.roles_allowing_text(subject.kind),
            if left_out.is_empty() {
                String::new()
            } else {
                format!(" (left out: {})", left_out.join(", "))
            }
        )
    } else {
        format!("may be signed by {}", eligible.join(", "))
    };
    let command_for = |signer: Option<&str>| {
        format!(
            "war sign approve {what} --ssh-sign{}",
            signer.map(|s| format!(" --as {s:?}")).unwrap_or_default()
        )
    };
    // The request: no key, no response, one journal line per plan.
    if !args.ssh_sign && !args.dry_run {
        let fresh = !requested_at(t, &digest);
        if fresh {
            store.journal(
                t,
                event::REQUESTED,
                &serde_json::json!({"warrant": what, "kind": subject.kind, "digest": digest}),
            )?;
        }
        let command = command_for(if eligible.len() == 1 {
            Some(eligible[0].as_str())
        } else {
            None
        });
        let mut out = Outcome::ok(
            format!(
                "approval requested for {what} (\"{}\"), a {} Warrant: {}.\nIt binds {digest}: \
                 the title, description, items and tests as they stand.\nA person runs: {command}\n\
                 {}",
                t.manifest.title,
                subject.kind,
                who,
                if fresh {
                    "Recorded in its journal; `war next` and `war sign inbox` list it. Your work \
                     goes on meanwhile."
                } else {
                    "Already requested for this plan; `war next` lists it. Your work goes on \
                     meanwhile."
                }
            ),
            serde_json::json!({"schema": "oh.war/warrant-approval-request/v1", "warrant": what,
                "kind": subject.kind, "digest": digest, "eligible": eligible,
                "left_out": left_out, "command": command, "recorded": fresh}),
        );
        if fresh
            && let Some(d) = crate::notify::human_waits(
                &repo.root,
                &crate::notify::Wait {
                    event: "approval.requested",
                    subject: &what,
                    message: &format!("{what} waits on an approval: {}", t.manifest.title),
                    command: &command,
                },
            )
        {
            out.report.push(d);
        }
        return Ok(out);
    }
    let opts = crate::sign::Options {
        actor: args.actor.clone(),
        dry_run: args.dry_run,
        ssh_sign: args.ssh_sign,
        ..crate::sign::Options::default()
    };
    let signer = match crate::sign::choose_actor(&eligible, &opts) {
        Ok(s) => s,
        Err(why) => {
            let why = if eligible.is_empty() {
                who.clone()
            } else {
                why
            };
            return Ok(Outcome::refused(
                "approve.no-eligible-approver",
                "docs/authority/roles.toml",
                format!(
                    "{what}: {why}. A person adds the principal's role to [roles.roster] in \
                     openwarrant.toml (and the key's line to docs/authority/allowed_signers). \
                     This blocks only the approval, not your work"
                ),
            ));
        }
    };
    let now = crate::gate_cmd::receipt::rfc3339_from_secs(ticket::now_secs());
    let mut meaning = format!(
        "I, {signer}, approve {what} (\"{}\") as an official plan of this repository: a {} \
         Warrant made by {}.",
        t.manifest.title, subject.kind, t.manifest.created_by
    );
    if let Some(m) = args
        .meaning
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
    {
        meaning.push(' ');
        meaning.push_str(m);
    }
    let q = openwarrant_core::ticket::toml_string;
    let body = format!(
        "# A human's approval of one Warrant (OW-WAR-0148 M14). Signed with\n\
         # `war sign approve {what} --ssh-sign`; the .sig beside it is the signature.\n\
         schema = {}\nwarrant = {}\nwarrant_uuid = {}\ntitle = {}\nkind = {}\nsha256 = {}\n\
         author = {}\napproved_by = {}\napproved_at = {}\nmeaning = {}\n",
        q(APPROVAL_SCHEMA),
        q(&what),
        q(&t.manifest.uuid),
        q(&t.manifest.title),
        q(subject.kind.as_str()),
        q(&digest),
        q(&t.manifest.created_by),
        q(&signer),
        q(&now),
        q(&meaning),
    );
    let path = authority_check::response_path(repo, Act::Approve, &what);
    if args.dry_run {
        let mut out = Outcome::ok(
            format!(
                "would record an approval of {what} by {signer}, writing {} and its .sig; nothing \
                 was written and no key was asked\n\n{body}",
                repo.relative(&path)
            ),
            serde_json::json!({"schema": "oh.war/warrant-approval/v1", "warrant": what,
                "signer": signer, "response": repo.relative(&path), "digest": digest,
                "dry_run": true}),
        );
        out.report.push(Diagnostic::pass(
            "sign.would-record",
            format!("{what}: an approval by {signer} would be recorded"),
        ));
        return Ok(out);
    }
    if let Err(why) = crate::sign::retire_prior(&path, &digest) {
        return Ok(Outcome::refused(
            "sign.response-exists",
            repo.relative(&path),
            why,
        ));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| RepoError::Io {
            context: format!("could not create {dir}"),
            source,
        })?;
    }
    crate::compile::atomic::write(&path, &body)?;
    let signed = match crate::sign::ssh_sign_act(repo, &signer, &[Act::Approve], &path) {
        Ok(s) => s,
        Err(refusal) => {
            let _ = std::fs::remove_file(&path);
            return Ok(Outcome::refused(
                refusal.rule,
                repo.relative(&path),
                format!(
                    "{}; nothing was recorded. This blocks only the approval, not your work",
                    refusal.why
                ),
            ));
        }
    };
    let verdict = authority_check::verify(repo, Act::Approve, &what, &signer, Some(&digest));
    if !verdict.is_signed() {
        let _ = std::fs::remove_file(&signed.sig);
        let _ = std::fs::remove_file(&path);
        return Ok(Outcome::refused(
            verdict.rule(),
            repo.relative(&path),
            format!(
                "the signature was made and does not verify as {signer}'s: {}; nothing was \
                 recorded",
                verdict.why()
            ),
        ));
    }
    store.journal(
        t,
        event::APPROVED,
        &serde_json::json!({"warrant": what, "approver": signer, "digest": digest,
            "response": repo.relative(&path), "kind": subject.kind}),
    )?;
    let after = local(repo, &policy, light(t));
    Ok(Outcome::ok(
        format!(
            "{what} approved by {signer}; {} verifies. {}",
            repo.relative(&signed.sig),
            after.line()
        ),
        serde_json::json!({"schema": "oh.war/warrant-approval/v1", "warrant": what,
            "signer": signer, "response": repo.relative(&path), "digest": digest,
            "standing": after}),
    ))
}

/// One approval someone asked for and nobody has given (for `war next` and
/// `war sign inbox`).
#[derive(Debug, Clone, Serialize)]
pub struct Requested {
    pub warrant: String,
    pub title: String,
    pub kind: Kind,
    pub requested_by: String,
    pub requested_at: String,
    pub command: String,
}

/// The approvals asked for on light Warrants whose plan still stands and
/// that are not yet official by a counted approval.
pub fn requested(repo: &Repository) -> Vec<Requested> {
    let policy = Policy::read_or_default(&repo.root);
    let Ok(store) = Store::open(repo, None) else {
        return Vec::new();
    };
    let Ok((tickets, _)) = store.load_all() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for t in &tickets {
        let path: Utf8PathBuf = t.dir.join(crate::journal_cmd::FILE);
        let Ok(text) = crate::vfs::read_to_string(&path) else {
            continue;
        };
        if !text.contains(event::REQUESTED) {
            continue;
        }
        let digest = statement_digest(t);
        let Ok(journal) = crate::journal_cmd::parse(&text) else {
            continue;
        };
        let asked = journal.events.iter().rev().find(|e| {
            e.event_type == event::REQUESTED
                && serde_json::from_str::<serde_json::Value>(&e.payload)
                    .ok()
                    .and_then(|p| p.get("digest").and_then(|d| d.as_str()).map(str::to_owned))
                    .as_deref()
                    == Some(digest.as_str())
        });
        let Some(asked) = asked else {
            continue;
        };
        let subject = light(t);
        if approval(repo, &policy, &subject).is_some_and(|a| a.counts) {
            continue;
        }
        out.push(Requested {
            warrant: t.id().to_owned(),
            title: t.manifest.title.clone(),
            kind: subject.kind,
            requested_by: asked.actor_ref.clone(),
            requested_at: asked.occurred_at.clone(),
            command: format!("war sign approve {} --ssh-sign", t.id()),
        });
    }
    out
}
