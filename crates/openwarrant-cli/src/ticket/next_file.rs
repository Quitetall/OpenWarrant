// SPDX-License-Identifier: Apache-2.0
//! `.openwarrant/next.md`: what this worktree's agent would otherwise ask for.
//!
//! A harness that loads instruction files before the first turn (opencode's
//! `instructions`) is pointed at it, so the work is in front of the agent
//! without a hook and without a call. It holds the brief of the Warrant the
//! acting agent holds (what `war start` printed), else the ready set, and is
//! removed when neither exists. `war create`, `start` and `done` (and a
//! release) rewrite it.
//!
//! It is a session's file, never a record: untracked, in the clone's
//! `info/exclude`, outside every source basis and worktree-dirt check
//! ([`crate::go::packet::SESSION_FILES`]), and within
//! [`super::prompt::BRIEF_BUDGET`] bytes. One worktree has one file, so with
//! several actors in one worktree it follows whoever acted last.

use super::prompt::{BRIEF_BUDGET, fitted};
use super::{RepoError, Store, Target, held_by_me, ready_lines, render, resolve};

/// Where it lives, relative to the root.
pub const FILE: &str = ".openwarrant/next.md";

/// What the file should say now; `None` when nothing is held and nothing is
/// ready.
pub fn text(store: &Store) -> Result<Option<String>, RepoError> {
    let held = held_by_me(store)?;
    // The act this follows is written: its other readers share this reading.
    let after = store.after_act()?;
    let tickets = &after.tickets;
    if let Some(id) = held.first()
        && let Ok(Target::Ticket(n) | Target::Item(n, _)) = resolve(tickets, id)
    {
        let brief = render::brief(store, tickets, &tickets[n], &store.actor);
        let head = format!(
            "OpenWarrant: {} holds {id}; carry on with it (no `war start` needed).",
            store.actor
        );
        let cut_note =
            format!("\n\n… cut to {BRIEF_BUDGET} bytes; `war start {id}` prints the rest.");
        let room = BRIEF_BUDGET - head.len() - cut_note.len() - 3;
        let (kept, cut) = fitted(&brief.human, room);
        let tail = if cut == 0 { "" } else { cut_note.as_str() };
        return Ok(Some(format!("{head}\n\n{kept}{tail}\n")));
    }
    let rows = after.ready(store)?;
    if rows.is_empty() {
        return Ok(None);
    }
    let head = "OpenWarrant: nothing is held here. Ready work, first to take first; `war start` \
                takes it and prints only what it needs.";
    let lines = ready_lines(rows);
    let more = "\n… more: `war view prime --ready`";
    let (kept, cut) = fitted(lines.trim_end(), BRIEF_BUDGET - head.len() - more.len() - 3);
    let tail = if cut == 0 { "" } else { more };
    Ok(Some(format!("{head}\n\n{kept}{tail}\n")))
}

/// Bring the file up to date after a command that changed what is held or
/// ready. Best effort and silent: the command it follows already happened,
/// and a file that could not be written is one an agent asks `war` instead.
pub fn refresh(store: &Store) {
    let path = store.root.join(FILE);
    match text(store) {
        Ok(Some(want)) => {
            if std::fs::read_to_string(&path).is_ok_and(|have| have == want) {
                return;
            }
            // Never a change to the checkout: excluded before it exists.
            crate::go::git::exclude(&store.root, FILE);
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = crate::compile::atomic::write(&path, want.into_bytes());
        }
        Ok(None) => {
            let _ = std::fs::remove_file(&path);
        }
        // Unknown is not "nothing": a stale file would be read as current.
        Err(_) => {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// [`refresh`] after `outcome`, unless it was refused (nothing changed).
pub fn after(store: &Store, outcome: &super::Outcome) {
    if !outcome.is_refused() {
        refresh(store);
    }
}
