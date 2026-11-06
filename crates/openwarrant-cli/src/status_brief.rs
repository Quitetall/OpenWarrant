// SPDX-License-Identifier: Apache-2.0
//! Bare `war status` in human mode: a few lines an agent can afford to read.
//!
//! The corpus projection this stands in for is tens of kilobytes on a real
//! repository, and an agent reads every byte. It stays behind `--verbose`,
//! and under `--json`. Every figure here comes from the records
//! (`status::build`, `console::board`, [`crate::work`]). One that cannot be
//! read prints `?` and says why on stderr, and the command exits as an UNKNOWN
//! does: nothing here stands in for a number it did not read.
//!
//! The Warrant line stays `status::build`'s: its `resolved` is computed against
//! the contract as it compiles now, where a list row is what the journal
//! recorded, and a brief must not read higher than `--verbose` does.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::corpus::Corpus;
use crate::repo::Repository;
use crate::ticket;
use crate::work::{LevelCounts, Source, Work};

/// What the brief says. `None` is a figure that could not be read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Brief {
    /// The SAS revision, when the program has one.
    pub sas: Option<String>,
    pub warrants: Option<usize>,
    pub resolved: Option<usize>,
    pub open_tickets: Option<usize>,
    /// Tickets the store could not parse, so not counted as open.
    pub unreadable_tickets: usize,
    pub ready_tickets: Option<usize>,
    pub ready_stages: Option<usize>,
    /// Open items of every encoding, by level.
    pub levels: Option<LevelCounts>,
    pub awaiting_signature: Option<usize>,
    pub blocking_questions: Option<usize>,
    /// Why each `?` is one.
    pub unknown: Vec<String>,
}

/// A count, or `?`.
fn figure(n: Option<usize>) -> String {
    n.map_or_else(|| "?".to_owned(), |n| n.to_string())
}

fn counted(n: Option<usize>, one: &str) -> String {
    format!("{} {one}{}", figure(n), if n == Some(1) { "" } else { "s" })
}

impl Brief {
    /// Read every figure. Never fails: a source that cannot be read leaves
    /// its figures `None` and says why in `unknown`.
    #[must_use]
    pub fn read(repo: &Repository) -> Self {
        let corpus = Corpus::new(repo);
        let mut brief = Self::default();
        match corpus.status() {
            Ok(status) => {
                brief.sas.clone_from(&status.release.version);
                let ladder = status.warrant_ladder();
                brief.warrants = Some(ladder.total());
                brief.resolved = Some(ladder.resolved);
            }
            Err(e) => brief.unknown.push(format!("warrants: {e}")),
        }
        brief.read_tickets(repo);
        // The board reads an unreadable frontier as no stages, which a count
        // must not repeat.
        let frontier = match corpus.frontier() {
            Ok(_) => true,
            Err(e) => {
                brief.unknown.push(format!("stages: {e}"));
                false
            }
        };
        match crate::console::board_with(&corpus) {
            Ok(board) => {
                brief.ready_stages = frontier.then_some(board.stages.len());
                brief.awaiting_signature = Some(board.acts.len());
                brief.blocking_questions =
                    Some(board.questions.iter().filter(|q| q.blocking).count());
            }
            Err(e) => brief.unknown.push(format!("acts and questions: {e}")),
        }
        brief
    }

    fn read_tickets(&mut self, repo: &Repository) {
        let store = match ticket::Store::open(repo, None) {
            Ok(store) => store,
            Err(e) => {
                self.unknown.push(format!("tickets: {e}"));
                self.read_levels(&Work::read(repo));
                return;
            }
        };
        let work = Work::read_with(repo, &store);
        self.read_levels(&work);
        if let Some(why) = work.unread(Source::Tickets) {
            self.unknown.push(format!("tickets: {why}"));
            return;
        }
        self.open_tickets = Some(work.tickets().iter().filter(|t| !t.is_done()).count());
        self.unreadable_tickets = work.ticket_faults().len();
        if let Some(why) = work.unread(Source::Claims) {
            self.unknown.push(format!("ready work: {why}"));
            return;
        }
        match ticket::ready_rows_with(&store, work.tickets(), work.claims().clone()) {
            Ok(rows) => {
                let ready: BTreeSet<&str> = rows.iter().map(|r| r.ticket.as_str()).collect();
                self.ready_tickets = Some(ready.len());
            }
            Err(e) => self.unknown.push(format!("ready work: {e}")),
        }
    }

    /// The level line needs every source: a part of it would read as all.
    fn read_levels(&mut self, work: &Work) {
        if let Some(why) = work.unread(Source::Others) {
            self.unknown.push(format!("warrants: {why}"));
        }
        if work.complete() {
            self.levels = Some(LevelCounts::open(work.items()));
        }
    }

    /// The lines bare `war status` prints.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        if let Some(sas) = &self.sas {
            let _ = writeln!(out, "SAS revision: {sas}");
        }
        let _ = writeln!(
            out,
            "Warrants: {}, {} resolved",
            figure(self.warrants),
            figure(self.resolved)
        );
        let _ = write!(out, "Tickets: {} open", figure(self.open_tickets));
        if self.unreadable_tickets > 0 {
            let _ = write!(out, ", {} unreadable", self.unreadable_tickets);
        }
        out.push('\n');
        let _ = writeln!(
            out,
            "Ready: {}, {}",
            counted(self.ready_tickets, "ticket"),
            counted(self.ready_stages, "stage")
        );
        let _ = writeln!(
            out,
            "Open by level: {}",
            self.levels.map_or_else(|| "?".to_owned(), |l| l.describe())
        );
        let _ = writeln!(
            out,
            "Awaiting a signature: {}",
            figure(self.awaiting_signature)
        );
        let _ = writeln!(
            out,
            "Blocking questions: {}",
            figure(self.blocking_questions)
        );
        out.push_str("`war status --verbose` shows everything.\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_figure_read_prints_as_read() {
        let brief = Brief {
            sas: Some("1.3.0".to_owned()),
            warrants: Some(145),
            resolved: Some(0),
            open_tickets: Some(12),
            unreadable_tickets: 0,
            ready_tickets: Some(1),
            ready_stages: Some(3),
            levels: Some(LevelCounts {
                tickets: 11,
                warrants: 1,
                orders: 116,
                unknown: 0,
            }),
            awaiting_signature: Some(7),
            blocking_questions: Some(2),
            unknown: Vec::new(),
        };
        assert_eq!(
            brief.render(),
            "SAS revision: 1.3.0\n\
             Warrants: 145, 0 resolved\n\
             Tickets: 12 open\n\
             Ready: 1 ticket, 3 stages\n\
             Open by level: 11 Tickets, 1 Warrant, 116 Orders\n\
             Awaiting a signature: 7\n\
             Blocking questions: 2\n\
             `war status --verbose` shows everything.\n"
        );
    }

    #[test]
    fn a_figure_not_read_is_a_question_mark_never_a_zero() {
        let text = Brief::default().render();
        assert_eq!(
            text,
            "Warrants: ?, ? resolved\n\
             Tickets: ? open\n\
             Ready: ? tickets, ? stages\n\
             Open by level: ?\n\
             Awaiting a signature: ?\n\
             Blocking questions: ?\n\
             `war status --verbose` shows everything.\n"
        );
    }

    #[test]
    fn an_unreadable_ticket_is_said_beside_the_count() {
        let brief = Brief {
            open_tickets: Some(4),
            unreadable_tickets: 2,
            ..Brief::default()
        };
        assert!(brief.render().contains("Tickets: 4 open, 2 unreadable\n"));
    }
}
