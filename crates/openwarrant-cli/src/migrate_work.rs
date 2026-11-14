// SPDX-License-Identifier: Apache-2.0
//! `war admin migrate`: tickets written before `docs/work/` become masters
//! (docs/design/one-format.md, "Layouts").
//!
//! Every ticket of the ticket directory (`docs/tickets/` unless
//! `[tickets] dir` says otherwise), in any of its three encodings, moves to
//! `docs/work/<id>/<id>-<slug>.md` with its state (journal, check output,
//! anything else in `<tickets>/<id>/`) beside it:
//!
//! | it was | its document | its state |
//! |---|---|---|
//! | `t-x.md` | moved, bytes unchanged | `t-x/` moved whole |
//! | `t-x/ticket.md` | moved, bytes unchanged | the rest of `t-x/` |
//! | `t-x/manifest.toml` and atoms | written as `war create` writes one, from the manifest, the intent and the checklist | the rest of `t-x/`; the manifest and the two atoms are removed |
//!
//! The id is kept: a migrated `t-3f2a` is `docs/work/t-3f2a/…` and still
//! `t-3f2a` to every command. The store reads `docs/work/` by the
//! frontmatter `alias`, whatever its prefix, so nothing that names the
//! ticket (a claim, a blocker, a parent, a journal, a question) changes.
//! The slug is the title's, and cosmetic.
//!
//! It is deterministic and it never guesses: the plan is computed from the
//! files alone, `--dry-run` prints it, and a real run performs exactly it,
//! with `git mv` where git tracks the file so history follows. A second run
//! finds nothing left and changes nothing.
//!
//! Nothing a person's act or another agent rests on moves. A ticket is left
//! in place, and named with the reason, when it has a recorded act
//! (`authorization.toml`, `resolution.toml`), a signed or verified tick, a
//! file a resolved Warrant pins, a claim that still holds, parts a directory
//! ticket keeps in a checks atom (the conversion does not carry them), a
//! reading fault, or a master already under its id.
//!
//! Other formats (another tracker's export, a hand-kept plan) need a drafter
//! and a review, as `war plan` has; that path is not built.

use std::collections::BTreeSet;

use camino::Utf8Path;
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use crate::ticket::{self, Store, Ticket};

pub const SCHEMA: &str = "oh.war/migrate-work/v1";

/// One file or directory that moves, relative to the root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Move {
    pub from: String,
    pub to: String,
}

/// One ticket's part of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    /// `file`, `inner` or `directory`: how it was kept.
    pub encoding: &'static str,
    /// Where its document will be.
    pub document: String,
    pub moves: Vec<Move>,
    /// A directory ticket: its document is written, not moved, and these
    /// are removed once it is.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removes: Vec<String>,
    #[serde(skip)]
    written: Option<String>,
}

/// A ticket that stays where it is, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Kept {
    pub id: String,
    pub place: String,
    pub rule: &'static str,
    pub why: String,
}

/// What a run would do, or did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub schema: &'static str,
    pub dry_run: bool,
    pub migrate: Vec<Item>,
    pub kept: Vec<Kept>,
}

/// The one document a directory ticket `t` becomes: its manifest, intent and
/// checklist, and the tests, KPIs and milestones of its checks atom
/// (`atoms/30-checks.md`) as the records `war plan add` writes into a
/// document (`CHK-<name>`, `M-<item>`). Exact or nothing: `Err` when the
/// atom does not read, or when what the ladder reads from the document is
/// not what it read from the atom, part for part.
fn converted(t: &Ticket) -> Result<String, String> {
    use openwarrant_core::ticks;
    let document = ticket::authoring::document(&t.manifest, &t.intent, &t.checklist_text);
    let path = t.dir.join(ticks::CHECKS_FILE);
    if !path.is_file() {
        return Ok(document);
    }
    let atom = std::fs::read_to_string(&path).map_err(|e| format!("it does not read: {e}"))?;
    let old = ticks::parse(&atom);
    if let Some(f) = old.faults.first() {
        return Err(format!("line {}: {}", f.line + 1, f.message));
    }
    let id = t.id();
    // A part of the whole Warrant evaluates the Warrant; one of an item, the item.
    let whole = |item: &Option<String>| item.is_none().then_some(id);
    let mut parts = Vec::new();
    for x in &old.tests {
        parts.push(
            serde_json::json!({"kind": "test", "name": x.name, "cmd": x.cmd,
            "item": x.item, "evaluates": whole(&x.item)}),
        );
    }
    for k in &old.kpis {
        parts.push(
            serde_json::json!({"kind": "kpi", "name": k.name, "cmd": k.cmd,
            "direction": k.direction.as_str(), "target": k.target, "mode": k.mode.as_str(),
            "item": k.item, "evaluates": whole(&k.item)}),
        );
    }
    for m in &old.milestones {
        parts.push(serde_json::json!({"kind": "milestone", "item": m.item,
            "min": m.min.map(ticks::Level::as_str)}));
    }
    let text = ticket::authoring::with_parts(id, &document, &parts)
        .map_err(|(rule, why)| format!("{rule}: {why}"))?;
    let new = ticket::ladder::document_checks(id, &text);
    let same = same_parts(&old, &new);
    same.map(|()| text)
}

/// Whether two readings of a ticket's parts are the same, line numbers
/// aside; `Err` names the first part that differs.
fn same_parts(
    old: &openwarrant_core::ticks::Checks,
    new: &openwarrant_core::ticks::Checks,
) -> Result<(), String> {
    let tests = |c: &openwarrant_core::ticks::Checks| {
        let mut v: Vec<_> = c
            .tests
            .iter()
            .map(|x| (x.name.clone(), x.cmd.clone(), x.item.clone()))
            .collect();
        v.sort();
        v
    };
    let kpis = |c: &openwarrant_core::ticks::Checks| {
        let mut v: Vec<_> = c
            .kpis
            .iter()
            .map(|k| {
                format!(
                    "{} {} {} {:?} {} {:?}",
                    k.name,
                    k.cmd,
                    k.direction.as_str(),
                    k.target,
                    k.mode.as_str(),
                    k.item
                )
            })
            .collect();
        v.sort();
        v
    };
    let milestones = |c: &openwarrant_core::ticks::Checks| {
        let mut v: Vec<_> = c
            .milestones
            .iter()
            .map(|m| {
                (
                    m.item.clone(),
                    m.min.map(openwarrant_core::ticks::Level::as_str),
                )
            })
            .collect();
        v.sort();
        v
    };
    if tests(old) != tests(new) {
        return Err(format!(
            "its tests read {:?} as a document, {:?} in the atom",
            tests(new),
            tests(old)
        ));
    }
    if kpis(old) != kpis(new) {
        return Err(format!(
            "its KPIs read {:?} as a document, {:?} in the atom",
            kpis(new),
            kpis(old)
        ));
    }
    if milestones(old) != milestones(new) {
        return Err(format!(
            "its milestones read {:?} as a document, {:?} in the atom",
            milestones(new),
            milestones(old)
        ));
    }
    Ok(())
}

fn rel(store: &Store, p: &Utf8Path) -> String {
    store.rel(p)
}

/// Why `t` may not move, if anything says so.
fn held_back(
    repo: &Repository,
    store: &Store,
    t: &Ticket,
    claims: &ticket::Claims,
    pinned: &BTreeSet<String>,
    now: u64,
) -> Option<(&'static str, String)> {
    for act in ["authorization.toml", "resolution.toml"] {
        if t.dir.join(act).is_file() {
            return Some((
                "migrate.recorded-act",
                format!("{} records an act over it", rel(store, &t.dir.join(act))),
            ));
        }
    }
    match crate::journal_cmd::load(&t.dir) {
        Ok(journal) => {
            // A directory ticket's journal spells these `ticket.*`.
            if let Some(e) = journal.events.iter().find(|e| {
                e.event_type.ends_with(".tick_signed") || e.event_type.ends_with(".tick_verified")
            }) {
                return Some((
                    "migrate.signed",
                    format!(
                        "its journal holds `{}`: a signature or a verification stands on it",
                        e.event_type
                    ),
                ));
            }
        }
        Err(e) => return Some(("migrate.unreadable", format!("its journal: {e}"))),
    }
    let own = format!("{}/", rel(store, &t.dir));
    let document = rel(store, &t.intent_path);
    if let Some(pin) = pinned
        .iter()
        .find(|p| p.starts_with(&own) || **p == document)
    {
        return Some(("migrate.pinned", format!("a resolved Warrant pins {pin}")));
    }
    if let Some(c) = claims
        .values()
        .flatten()
        .find(|c| c.ticket == t.id() && c.reserves(now))
    {
        return Some((
            "migrate.claimed",
            format!(
                "{} holds {}; `war admin release` it, or let its holder finish",
                c.actor,
                c.target()
            ),
        ));
    }
    if !t.format
        && let Err(why) = converted(t)
    {
        return Some((
            "migrate.parts",
            format!(
                "its parts in {} do not convert exactly: {why}",
                rel(store, &t.dir.join(openwarrant_core::ticks::CHECKS_FILE))
            ),
        ));
    }
    let target = repo.root.join(crate::subject::WORK_ROOT).join(t.id());
    if target.exists() {
        return Some((
            "migrate.target-exists",
            format!("{} is there already", rel(store, &target)),
        ));
    }
    None
}

/// The plan for `repo`: every ticket of the ticket directory, moved or kept.
pub fn plan(repo: &Repository, store: &Store) -> Result<Plan, RepoError> {
    let mut out = Plan {
        schema: SCHEMA,
        ..Plan::default()
    };
    let work = repo.root.join(crate::subject::WORK_ROOT);
    if store.dir == work {
        return Ok(out);
    }
    let (tickets, faults) = store.load_all_uncached()?;
    for f in &faults {
        let place = f.file.clone().unwrap_or_default();
        if place.starts_with(&format!("{}/", rel(store, &store.dir))) {
            out.kept.push(Kept {
                id: Utf8Path::new(&place)
                    .strip_prefix(rel(store, &store.dir))
                    .ok()
                    .and_then(|p| p.components().next())
                    .map(|c| c.as_str().trim_end_matches(".md").to_owned())
                    .unwrap_or_default(),
                place,
                rule: "migrate.unreadable",
                why: f.message.clone(),
            });
        }
    }
    let claims = store.claims()?;
    let pinned: BTreeSet<String> = crate::pins::list(repo, true)
        .map(|p| p.pins.into_iter().map(|p| p.path).collect())
        .unwrap_or_default();
    let now = ticket::now_secs();
    for t in tickets
        .iter()
        .filter(|t| t.dir.parent() == Some(store.dir.as_path()))
    {
        if let Some((rule, why)) = held_back(repo, store, t, &claims, &pinned, now) {
            out.kept.push(Kept {
                id: t.id().to_owned(),
                place: rel(store, if t.format { &t.intent_path } else { &t.dir }),
                rule,
                why,
            });
            continue;
        }
        let to_dir = work.join(t.id());
        let document = crate::subject::master_path_at(&repo.root, t.id(), &t.manifest.title);
        let mut item = Item {
            id: t.id().to_owned(),
            title: t.manifest.title.clone(),
            encoding: "file",
            document: rel(store, &document),
            moves: Vec::new(),
            removes: Vec::new(),
            written: None,
        };
        let state_exists = t.dir.is_dir();
        if state_exists {
            item.moves.push(Move {
                from: format!("{}/", rel(store, &t.dir)),
                to: format!("{}/", rel(store, &to_dir)),
            });
        }
        if !t.format {
            item.encoding = "directory";
            // `held_back` refused one whose parts do not convert exactly.
            let Ok(text) = converted(t) else { continue };
            item.written = Some(text);
            let checks = t.dir.join(openwarrant_core::ticks::CHECKS_FILE);
            for old in [
                t.dir.join("manifest.toml"),
                t.intent_path.clone(),
                t.checklist_path.clone(),
            ]
            .into_iter()
            .chain(checks.is_file().then_some(checks))
            {
                let moved = to_dir.join(old.strip_prefix(&t.dir).unwrap_or(&old));
                item.removes.push(rel(store, &moved));
            }
        } else if t.intent_path.parent() == Some(t.dir.as_path()) {
            item.encoding = "inner";
            let moved = to_dir.join(t.intent_path.file_name().unwrap_or_default());
            item.moves.push(Move {
                from: rel(store, &moved),
                to: rel(store, &document),
            });
        } else {
            item.moves.push(Move {
                from: rel(store, &t.intent_path),
                to: rel(store, &document),
            });
        }
        out.migrate.push(item);
    }
    out.kept.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn tracked(root: &Utf8Path, path: &str) -> bool {
    std::process::Command::new("git")
        .args(["-C", root.as_str(), "ls-files", "--error-unmatch", "--"])
        .arg(path.trim_end_matches('/'))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn git(root: &Utf8Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(["-C", root.as_str()])
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Move `from` to `to` under `root`: `git mv` when git tracks it, so its
/// history follows; a rename otherwise.
fn mv(root: &Utf8Path, from: &str, to: &str) -> Result<(), RepoError> {
    let (from, to) = (from.trim_end_matches('/'), to.trim_end_matches('/'));
    let dest = root.join(to);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    if tracked(root, from) && git(root, &["mv", "--", from, to]) {
        return Ok(());
    }
    std::fs::rename(root.join(from), &dest).map_err(|source| RepoError::Io {
        context: format!("could not move {from} to {to}"),
        source,
    })
}

/// Perform `plan`. Each ticket is moved whole or, on a failure, the run
/// stops there and says which; what already moved is a finished migration of
/// those tickets, and a second run takes up the rest.
pub fn apply(repo: &Repository, store: &Store, plan: &Plan) -> Result<(), RepoError> {
    let root = &repo.root;
    for item in &plan.migrate {
        for m in &item.moves {
            mv(root, &m.from, &m.to)?;
        }
        if let Some(text) = &item.written {
            let document = root.join(&item.document);
            if let Some(parent) = document.parent() {
                std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                    context: format!("could not create {parent}"),
                    source,
                })?;
            }
            ticket::authoring::create(&document, text)?;
            let was_tracked = item.removes.iter().any(|r| tracked(root, r));
            for old in &item.removes {
                if !(tracked(root, old) && git(root, &["rm", "-q", "--", old])) {
                    let _ = std::fs::remove_file(root.join(old));
                }
            }
            let _ = std::fs::remove_dir(root.join(&item.removes[1]).parent().unwrap_or(root));
            if was_tracked {
                let _ = git(root, &["add", "--", &item.document]);
            }
        }
        // The clone's register says where it lives now.
        if let Ok(t) = store.load(&root.join(crate::subject::WORK_ROOT).join(&item.id)) {
            ticket::shared::moved(store, &t);
        }
    }
    Ok(())
}

/// The text a person reads: each move, then each ticket kept and why.
#[must_use]
pub fn render(plan: &Plan) -> String {
    let mut out = String::new();
    let verb = if plan.dry_run { "would move" } else { "moved" };
    for item in &plan.migrate {
        out.push_str(&format!("{}  {}\n", item.id, item.title));
        for m in &item.moves {
            out.push_str(&format!("  {verb} {} -> {}\n", m.from, m.to));
        }
        if item.written.is_some() || item.encoding == "directory" {
            out.push_str(&format!(
                "  {} {} (from the manifest and atoms)\n",
                if plan.dry_run { "would write" } else { "wrote" },
                item.document
            ));
            for r in &item.removes {
                out.push_str(&format!(
                    "  {} {r}\n",
                    if plan.dry_run {
                        "would remove"
                    } else {
                        "removed"
                    }
                ));
            }
        }
    }
    for k in &plan.kept {
        out.push_str(&format!(
            "{}  left in place ({}): {}\n",
            k.id, k.rule, k.why
        ));
    }
    if plan.migrate.is_empty() && plan.kept.is_empty() {
        out.push_str("nothing to migrate: no ticket is kept outside docs/work/\n");
    } else {
        out.push_str(&format!(
            "{} ticket(s) {}, {} left in place{}\n",
            plan.migrate.len(),
            if plan.dry_run { "would move" } else { "moved" },
            plan.kept.len(),
            if plan.dry_run {
                "; nothing was written (`war admin migrate` does it)"
            } else {
                "; `war check` and `git status` show the result"
            }
        ));
    }
    out
}

/// `war admin migrate [--dry-run]`.
pub fn run(repo: &Repository, dry_run: bool) -> Result<(Plan, Report), RepoError> {
    let store = Store::open(repo, None)?;
    let mut plan = plan(repo, &store)?;
    plan.dry_run = dry_run;
    let mut report = Report::default();
    for k in &plan.kept {
        report.push(Diagnostic::warn(
            k.rule,
            k.place.clone(),
            format!("{} left in place: {}", k.id, k.why),
        ));
    }
    if !dry_run {
        apply(repo, &store, &plan)?;
    }
    Ok((plan, report))
}
