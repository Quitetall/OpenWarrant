// SPDX-License-Identifier: Apache-2.0
//! Tickets (OW-WAR-0147): the working form of a delivery Warrant.
//!
//! A ticket is two atoms a human reads in an editor or on GitHub: an intent
//! (the sentence, its context, decisions and dated notes) and a checklist,
//! one Markdown task per item:
//!
//! ```markdown
//! - [ ] Parse the checklist (i-3f2a)
//! - [ ] Wire the CLI (i-9c01, after i-3f2a)
//! - [x] Write the docs (i-77be) — done by claude, 2026-09-25: docs/TICKETS.md
//! ```
//!
//! The file IS the state. Nothing here keeps a second copy: `done` is the
//! ticked box, the order is the order of the lines, and a line a human adds,
//! moves or rewords is honoured as written. A write touches exactly the line
//! it changes (or appends one) and leaves every other byte as it was, which
//! the tests below assert byte for byte.
//!
//! This module is pure (§79.1): it parses and edits text. Reading files,
//! claims and the journal are the CLI's.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// The schema a ticket's `manifest.toml` declares.
pub const TICKET_SCHEMA: &str = "oh.war/ticket/v1";

/// The profile every ticket names (`profiles/ticket.toml`).
pub const TICKET_PROFILE: &str = "ticket";

/// The role of the checklist atom.
pub const CHECKLIST_ROLE: &str = "ticket.checklist";

/// The priority a ticket gets when none is given (0 is the most urgent, 4 the
/// least, as in beads).
pub const DEFAULT_PRIORITY: u8 = 2;

/// The separator `war done` writes before `done by`. An em dash, so it reads
/// as prose; ` -- done by ` typed by hand is read the same way.
pub const DONE_SEPARATOR: &str = " — done by ";
const DONE_SEPARATOR_ASCII: &str = " -- done by ";

/// A ticket's `manifest.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TicketManifest {
    pub schema: String,
    /// The short hash id, `t-3f2a`: the ticket's name everywhere.
    pub id: String,
    /// The UUIDv7 the id is hashed from.
    pub uuid: String,
    pub title: String,
    pub profile: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    pub created_at: String,
    pub created_by: String,
    /// The Warrant this ticket was promoted into, once someone asked for
    /// sign-off (`war promote`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promoted_to: Option<String>,
    pub atoms: Vec<TicketAtom>,
}

const fn default_priority() -> u8 {
    DEFAULT_PRIORITY
}

/// One atom of a ticket, as a Warrant manifest declares one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TicketAtom {
    pub ordinal: u32,
    pub role: String,
    pub path: String,
}

impl TicketManifest {
    /// Validate against the roles the ticket profile's definition requires
    /// (`ProfileDefinition::working_roles`). Every refusal names the field.
    pub fn validate(&self, required_roles: &[String]) -> Result<(), String> {
        if self.schema != TICKET_SCHEMA {
            return Err(format!(
                "schema {:?}; this build reads {TICKET_SCHEMA:?}",
                self.schema
            ));
        }
        if !is_ticket_id(&self.id) {
            return Err(format!(
                "id {:?} is not a ticket id (t- and 3 to 16 of [0-9a-z])",
                self.id
            ));
        }
        if self.title.trim().is_empty() {
            return Err("title is empty".to_owned());
        }
        if self.profile != TICKET_PROFILE {
            return Err(format!(
                "profile {:?}; a ticket's profile is {TICKET_PROFILE:?}",
                self.profile
            ));
        }
        if self.priority > 4 {
            return Err(format!(
                "priority {} is outside 0 (most urgent) ..= 4",
                self.priority
            ));
        }
        let mut ordinals = BTreeSet::new();
        let mut roles = BTreeSet::new();
        for atom in &self.atoms {
            if !ordinals.insert(atom.ordinal) {
                return Err(format!("atom ordinal {} is declared twice", atom.ordinal));
            }
            if !roles.insert(atom.role.as_str()) {
                return Err(format!("atom role {:?} is declared twice", atom.role));
            }
            if atom.path.contains("..") || atom.path.starts_with('/') {
                return Err(format!(
                    "atom path {:?} leaves the ticket's directory",
                    atom.path
                ));
            }
        }
        for role in required_roles {
            if !roles.contains(role.as_str()) {
                return Err(format!(
                    "profile {TICKET_PROFILE} requires a {role} atom and the manifest declares \
                     none"
                ));
            }
        }
        Ok(())
    }

    /// The path of the atom with `role`, relative to the ticket's directory.
    #[must_use]
    pub fn atom_path(&self, role: &str) -> Option<&str> {
        self.atoms
            .iter()
            .find(|a| a.role == role)
            .map(|a| a.path.as_str())
    }
}

// ---- identifiers ---------------------------------------------------------

fn is_hash_id(s: &str, prefix: &str) -> bool {
    s.strip_prefix(prefix).is_some_and(|rest| {
        (3..=16).contains(&rest.len())
            && rest
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
    })
}

/// `t-` and 3 to 16 of `[0-9a-z]`. The tool mints lowercase hex; a human may
/// name one by hand.
#[must_use]
pub fn is_ticket_id(s: &str) -> bool {
    is_hash_id(s, "t-")
}

/// `i-` and 3 to 16 of `[0-9a-z]`.
#[must_use]
pub fn is_item_id(s: &str) -> bool {
    is_hash_id(s, "i-")
}

fn sha_hex(seed: &str) -> String {
    Sha256::digest(seed.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// How many hex characters a new ticket id takes when `existing` tickets are
/// already here: the shortest length, from 4, at which the chance that two
/// ids among them collide stays under 1% (the birthday bound `n²/2·16^L`).
/// Ids stay short while a repository is small and lengthen as it grows, so
/// agents on different branches minting ids independently do not collide.
#[must_use]
pub fn ticket_id_len(existing: usize) -> usize {
    let n = (existing + 1) as f64;
    (4..16)
        .find(|&len| n * n / (2.0 * 16f64.powi(len as i32)) < 0.01)
        .unwrap_or(16)
}

/// The ticket id of `uuid` at `len` hex characters: `t-` and the head of the
/// SHA-256 of the UUID's text. A digest rather than the UUID's own digits,
/// because a UUIDv7 opens with its timestamp and two tickets made in the same
/// minute would share a prefix.
#[must_use]
pub fn ticket_id(uuid: &str, len: usize) -> String {
    format!("t-{}", &sha_hex(uuid)[..len.clamp(3, 16)])
}

/// An item id from any unique seed (the caller passes a fresh UUID): `i-` and
/// four hex characters, lengthened only when `taken` already holds it.
#[must_use]
pub fn item_id(seed: &str, taken: &BTreeSet<String>) -> String {
    let hex = sha_hex(seed);
    (4..=16)
        .map(|len| format!("i-{}", &hex[..len]))
        .find(|id| !taken.contains(id))
        .unwrap_or_else(|| format!("i-{}", &hex[..16]))
}

// ---- references ------------------------------------------------------------

/// What an item waits on (`after ...`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Blocker {
    /// An item of the same ticket.
    Item { item: String },
    /// A whole ticket: every item of it done.
    Ticket { ticket: String },
    /// One item of another ticket.
    ItemOf { ticket: String, item: String },
}

impl Blocker {
    /// Parse `i-x`, `t-x` or `t-x/i-y`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if is_item_id(s) {
            return Some(Self::Item { item: s.to_owned() });
        }
        if is_ticket_id(s) {
            return Some(Self::Ticket {
                ticket: s.to_owned(),
            });
        }
        let (ticket, item) = s.split_once('/')?;
        (is_ticket_id(ticket) && is_item_id(item)).then(|| Self::ItemOf {
            ticket: ticket.to_owned(),
            item: item.to_owned(),
        })
    }
}

impl std::fmt::Display for Blocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Item { item } => f.write_str(item),
            Self::Ticket { ticket } => f.write_str(ticket),
            Self::ItemOf { ticket, item } => write!(f, "{ticket}/{item}"),
        }
    }
}

// ---- the checklist ---------------------------------------------------------

/// One task line, as read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    /// 0-based index of the line in the atom.
    pub line: usize,
    /// `None` for a line a human added without one; the next write names it.
    pub id: Option<String>,
    pub text: String,
    pub done: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<Blocker>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip)]
    indent: String,
    #[serde(skip)]
    bullet: char,
}

/// A structural fault in a checklist, by rule. `war check` reports each as an
/// error naming the line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fault {
    /// `ticket.checklist-malformed`, `ticket.item-duplicate`,
    /// `ticket.blocker-unknown` or `ticket.blocker-cycle`.
    pub rule: &'static str,
    /// 1-based, as an editor shows it.
    pub line: usize,
    pub message: String,
}

/// A parsed checklist: the items in file order and the faults found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Checklist {
    pub items: Vec<Item>,
    pub faults: Vec<Fault>,
}

/// The lines of `text`, each with its own terminator, so that joining them
/// gives back `text` exactly.
fn lines_of(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

fn body_of(line: &str) -> (&str, &str) {
    let body = line.trim_end_matches(['\n', '\r']);
    (body, &line[body.len()..])
}

/// A task line's parts, or why it is not one. `Ok(None)`: not a task line.
fn parse_task(line: &str) -> Result<Option<(String, char, bool, String)>, String> {
    let trimmed = line.trim_start();
    let indent = line[..line.len() - trimmed.len()].to_owned();
    let mut chars = trimmed.chars();
    let Some(bullet @ ('-' | '*' | '+')) = chars.next() else {
        return Ok(None);
    };
    let rest = chars.as_str();
    let after_space = rest.trim_start_matches(' ');
    if after_space.len() == rest.len() || !after_space.starts_with('[') {
        return Ok(None);
    }
    let inner = &after_space[1..];
    // `[]` then a space: an empty box, a typo for `[ ]`.
    if let Some(tail) = inner.strip_prefix(']')
        && (tail.is_empty() || tail.starts_with(' '))
    {
        return Err("an empty box `[]`; a task is `- [ ] ...` or `- [x] ...`".to_owned());
    }
    let mut ic = inner.chars();
    let (Some(mark), Some(']')) = (ic.next(), ic.next()) else {
        return Ok(None);
    };
    let tail = ic.as_str();
    if !(tail.is_empty() || tail.starts_with(' ')) {
        // `- [a](link)`: a link, not a task.
        return Ok(None);
    }
    let done = match mark {
        ' ' => false,
        'x' | 'X' => true,
        other => {
            return Err(format!(
                "box `[{other}]`; a task is `- [ ] ...` (open) or `- [x] ...` (done)"
            ));
        }
    };
    let text = tail.trim();
    if text.is_empty() {
        return Err("a task with no text".to_owned());
    }
    Ok(Some((indent, bullet, done, text.to_owned())))
}

/// Who ticked an item, when, and with what note: the done suffix's parts.
type DonePart = (String, Option<String>, Option<String>);

/// The done suffix, split off: (before, actor, date, note).
fn split_done(text: &str) -> (&str, Option<DonePart>) {
    let found = text
        .find(DONE_SEPARATOR)
        .map(|i| (i, DONE_SEPARATOR.len()))
        .or_else(|| {
            text.find(DONE_SEPARATOR_ASCII)
                .map(|i| (i, DONE_SEPARATOR_ASCII.len()))
        });
    let Some((at, len)) = found else {
        return (text, None);
    };
    let (head, rest) = (&text[..at], &text[at + len..]);
    let (who_when, note) = match rest.split_once(": ") {
        Some((a, n)) => (a, Some(n.trim().to_owned()).filter(|n| !n.is_empty())),
        None => (rest.trim_end_matches(':'), None),
    };
    let (actor, date) = match who_when.split_once(", ") {
        Some((a, d)) => (a.trim().to_owned(), Some(d.trim().to_owned())),
        None => (who_when.trim().to_owned(), None),
    };
    (head, Some((actor, date, note)))
}

/// The trailing `(i-x, after ...)` group: (text before it, id, blockers).
/// `Ok(None)`: the text ends in no id group (a parenthesis that is prose).
#[allow(clippy::type_complexity)]
fn split_id_group(text: &str) -> Result<Option<(&str, String, Vec<Blocker>)>, String> {
    let trimmed = text.trim_end();
    let Some(inner_end) = trimmed.strip_suffix(')') else {
        return Ok(None);
    };
    let Some(open) = inner_end.rfind('(') else {
        return Ok(None);
    };
    let inner = &inner_end[open + 1..];
    let mut parts = inner.split(',').map(str::trim);
    let first = parts.next().unwrap_or_default();
    if !first.starts_with("i-") {
        return Ok(None);
    }
    if !is_item_id(first) {
        return Err(format!(
            "item id {first:?}; an id is i- and 3 to 16 of [0-9a-z]"
        ));
    }
    let mut after = Vec::new();
    let mut in_after = false;
    for part in parts {
        let reference = if let Some(r) = part.strip_prefix("after ") {
            in_after = true;
            r.trim()
        } else if in_after {
            part
        } else {
            return Err(format!(
                "{part:?} in the id group; only `after <item|ticket>` may follow the id"
            ));
        };
        let Some(blocker) = Blocker::parse(reference) else {
            return Err(format!(
                "`after {reference}` names no item (i-x), ticket (t-x) or item of a ticket \
                 (t-x/i-y)"
            ));
        };
        after.push(blocker);
    }
    let before = inner_end[..open].trim_end();
    Ok(Some((before, first.to_owned(), after)))
}

/// Parse a checklist atom. Never fails: faults are collected and reported, and
/// every well-formed line is still an item.
#[must_use]
pub fn parse(text: &str) -> Checklist {
    let mut out = Checklist::default();
    let mut fence: Option<&str> = None;
    for (index, raw) in lines_of(text).into_iter().enumerate() {
        let (line, _) = body_of(raw);
        let t = line.trim_start();
        if line.len() - t.len() <= 3 {
            for marker in ["```", "~~~"] {
                if t.starts_with(marker) {
                    fence = match fence {
                        Some(open) if open == marker => None,
                        Some(open) => Some(open),
                        None => Some(marker),
                    };
                }
            }
        }
        if fence.is_some() || t.starts_with("```") || t.starts_with("~~~") {
            continue;
        }
        let malformed = |message: String| Fault {
            rule: "ticket.checklist-malformed",
            line: index + 1,
            message,
        };
        let (indent, bullet, done, text) = match parse_task(line) {
            Ok(Some(task)) => task,
            Ok(None) => continue,
            Err(why) => {
                out.faults.push(malformed(why));
                continue;
            }
        };
        let (head, done_part) = split_done(&text);
        let (label, id, after) = match split_id_group(head) {
            Ok(Some((label, id, after))) => (label.to_owned(), Some(id), after),
            Ok(None) => (head.trim().to_owned(), None, Vec::new()),
            Err(why) => {
                out.faults.push(malformed(why));
                continue;
            }
        };
        if label.is_empty() {
            out.faults
                .push(malformed("a task with an id and no text".to_owned()));
            continue;
        }
        let (done_by, done_on, note) = match done_part {
            Some((actor, date, note)) if done => (Some(actor), date, note),
            _ => (None, None, None),
        };
        out.items.push(Item {
            line: index,
            id,
            text: label,
            done,
            after,
            done_by,
            done_on,
            note,
            indent,
            bullet,
        });
    }
    out.check_references();
    out
}

impl Checklist {
    fn check_references(&mut self) {
        let mut first_line: BTreeMap<&str, usize> = BTreeMap::new();
        for item in &self.items {
            let Some(id) = item.id.as_deref() else {
                continue;
            };
            if let Some(first) = first_line.get(id) {
                self.faults.push(Fault {
                    rule: "ticket.item-duplicate",
                    line: item.line + 1,
                    message: format!(
                        "item id {id} is also on line {}; an id names one item",
                        first + 1
                    ),
                });
            } else {
                first_line.insert(id, item.line);
            }
        }
        for item in &self.items {
            for blocker in &item.after {
                if let Blocker::Item { item: target } = blocker
                    && !first_line.contains_key(target.as_str())
                {
                    self.faults.push(Fault {
                        rule: "ticket.blocker-unknown",
                        line: item.line + 1,
                        message: format!(
                            "`after {target}`: no item of this ticket is {target}; the item \
                             waits on nothing that can finish"
                        ),
                    });
                }
            }
        }
        // A cycle among this ticket's items: none of them can ever be ready.
        let edges: BTreeMap<&str, Vec<&str>> = self
            .items
            .iter()
            .filter_map(|i| {
                i.id.as_deref().map(|id| {
                    (
                        id,
                        i.after
                            .iter()
                            .filter_map(|b| match b {
                                Blocker::Item { item } => Some(item.as_str()),
                                _ => None,
                            })
                            .collect(),
                    )
                })
            })
            .collect();
        for item in &self.items {
            let Some(id) = item.id.as_deref() else {
                continue;
            };
            let mut seen = BTreeSet::new();
            let mut stack: Vec<&str> = edges.get(id).cloned().unwrap_or_default();
            while let Some(next) = stack.pop() {
                if next == id {
                    self.faults.push(Fault {
                        rule: "ticket.blocker-cycle",
                        line: item.line + 1,
                        message: format!(
                            "{id} waits, through `after`, on itself; no item on the cycle can \
                             ever be ready"
                        ),
                    });
                    break;
                }
                if seen.insert(next) {
                    stack.extend(edges.get(next).cloned().unwrap_or_default());
                }
            }
        }
    }

    /// The item named `id`.
    #[must_use]
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id.as_deref() == Some(id))
    }

    /// Every id in use.
    #[must_use]
    pub fn ids(&self) -> BTreeSet<String> {
        self.items.iter().filter_map(|i| i.id.clone()).collect()
    }

    /// `(done, total)`.
    #[must_use]
    pub fn progress(&self) -> (usize, usize) {
        (
            self.items.iter().filter(|i| i.done).count(),
            self.items.len(),
        )
    }

    /// Every item is done, and there is at least one.
    #[must_use]
    pub fn is_done(&self) -> bool {
        !self.items.is_empty() && self.items.iter().all(|i| i.done)
    }
}

/// One line on the page: no newline, no leading or trailing blanks, and no
/// `(` `)` that would read back as an id group.
#[must_use]
pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Item {
    /// The line as `war` writes it (no terminator).
    #[must_use]
    pub fn render(&self) -> String {
        let mark = if self.done { 'x' } else { ' ' };
        let mut group = self.id.clone().unwrap_or_default();
        if !self.after.is_empty() {
            let refs: Vec<String> = self.after.iter().map(ToString::to_string).collect();
            group.push_str(&format!(", after {}", refs.join(", ")));
        }
        let mut line = format!(
            "{}{} [{mark}] {} ({group})",
            self.indent, self.bullet, self.text
        );
        if self.done
            && let Some(by) = &self.done_by
        {
            line.push_str(DONE_SEPARATOR);
            line.push_str(by);
            if let Some(on) = &self.done_on {
                line.push_str(", ");
                line.push_str(on);
            }
            if let Some(note) = &self.note {
                line.push_str(": ");
                line.push_str(note);
            }
        }
        line
    }

    /// This item, ticked by `actor` on `date` with an optional note.
    #[must_use]
    pub fn ticked(&self, actor: &str, date: &str, note: Option<&str>) -> Self {
        Self {
            done: true,
            done_by: Some(one_line(actor)),
            done_on: Some(date.to_owned()),
            note: note.map(one_line).filter(|n| !n.is_empty()),
            ..self.clone()
        }
    }

    /// This item with an id (for a line a human added without one).
    #[must_use]
    pub fn named(&self, id: &str) -> Self {
        Self {
            id: Some(id.to_owned()),
            ..self.clone()
        }
    }

    /// A new open item.
    #[must_use]
    pub fn new(id: &str, text: &str, after: Vec<Blocker>) -> Self {
        Self {
            line: usize::MAX,
            id: Some(id.to_owned()),
            text: one_line(text),
            done: false,
            after,
            done_by: None,
            done_on: None,
            note: None,
            indent: String::new(),
            bullet: '-',
        }
    }
}

/// `text` with line `index` replaced by `line`, keeping that line's own
/// terminator. Every other byte is unchanged.
#[must_use]
pub fn replace_line(text: &str, index: usize, line: &str) -> String {
    let mut out = String::with_capacity(text.len() + line.len());
    for (i, raw) in lines_of(text).into_iter().enumerate() {
        if i == index {
            let (_, terminator) = body_of(raw);
            out.push_str(line);
            out.push_str(terminator);
        } else {
            out.push_str(raw);
        }
    }
    out
}

/// `text` with `line` added after the last task line (or at the end when there
/// is none). Every existing byte is kept; a missing final newline is supplied
/// only where the new line has to follow it.
#[must_use]
pub fn append_item(text: &str, checklist: &Checklist, line: &str) -> String {
    let lines = lines_of(text);
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let at = checklist.items.iter().map(|i| i.line).max();
    let mut out = String::with_capacity(text.len() + line.len() + 2);
    let insert_after = at.unwrap_or(lines.len().saturating_sub(1));
    if lines.is_empty() {
        out.push_str(line);
        out.push_str(newline);
        return out;
    }
    for (i, raw) in lines.iter().enumerate() {
        out.push_str(raw);
        if i == insert_after {
            if !raw.ends_with('\n') {
                out.push_str(newline);
            }
            out.push_str(line);
            out.push_str(newline);
        }
    }
    out
}

/// `text` with every unnamed item given an id, and the ids given, in order.
/// Only those lines change.
#[must_use]
pub fn name_unnamed(
    text: &str,
    fresh: &mut dyn FnMut(&BTreeSet<String>) -> String,
) -> (String, Vec<String>) {
    let checklist = parse(text);
    let mut taken = checklist.ids();
    let mut out = text.to_owned();
    let mut named = Vec::new();
    for item in checklist.items.iter().filter(|i| i.id.is_none()) {
        let id = fresh(&taken);
        taken.insert(id.clone());
        out = replace_line(&out, item.line, &item.named(&id).render());
        named.push(id);
    }
    (out, named)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Checklist\n\nSome prose a human wrote (with a parenthesis).\n\n- [ ] Parse the checklist (i-3f2a)\n- [ ] Wire the CLI (i-9c01, after i-3f2a)\n  - [x] Nested and done (i-77be) — done by claude, 2026-09-25: wrote it\n* [X] Star bullet, ticked by hand (i-aaaa)\n- [ ] A line with no id\n- [link](https://example.com) is not a task\n\n```\n- [ ] inside a fence (i-ffff)\n```\n";

    #[test]
    fn items_are_read_in_file_order_with_their_parts() {
        let c = parse(DOC);
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        let ids: Vec<Option<&str>> = c.items.iter().map(|i| i.id.as_deref()).collect();
        assert_eq!(
            ids,
            vec![
                Some("i-3f2a"),
                Some("i-9c01"),
                Some("i-77be"),
                Some("i-aaaa"),
                None
            ]
        );
        let wire = c.item("i-9c01").expect("wire");
        assert_eq!(wire.text, "Wire the CLI");
        assert_eq!(
            wire.after,
            vec![Blocker::Item {
                item: "i-3f2a".into()
            }]
        );
        let nested = c.item("i-77be").expect("nested");
        assert!(nested.done);
        assert_eq!(nested.done_by.as_deref(), Some("claude"));
        assert_eq!(nested.done_on.as_deref(), Some("2026-09-25"));
        assert_eq!(nested.note.as_deref(), Some("wrote it"));
        // Ticked by hand: done, by nobody the file names.
        let star = c.item("i-aaaa").expect("star");
        assert!(star.done && star.done_by.is_none());
        assert_eq!(c.progress(), (2, 5));
        assert!(!c.is_done());
    }

    #[test]
    fn rendering_an_unchanged_item_gives_back_its_line() {
        let c = parse(DOC);
        for item in c.items.iter().filter(|i| i.id.is_some() && i.bullet == '-') {
            let line = DOC.lines().nth(item.line).expect("line");
            assert_eq!(item.render(), line);
        }
    }

    #[test]
    fn ticking_one_item_changes_only_its_line_byte_for_byte() {
        let c = parse(DOC);
        let item = c.item("i-3f2a").expect("item");
        let out = replace_line(
            DOC,
            item.line,
            &item
                .ticked("agent-b", "2026-09-26", Some("done\nwith care"))
                .render(),
        );
        let before: Vec<&str> = DOC.split_inclusive('\n').collect();
        let after: Vec<&str> = out.split_inclusive('\n').collect();
        assert_eq!(before.len(), after.len());
        for (i, (b, a)) in before.iter().zip(&after).enumerate() {
            if i == item.line {
                assert_eq!(
                    *a,
                    "- [x] Parse the checklist (i-3f2a) — done by agent-b, 2026-09-26: done with care\n"
                );
            } else {
                assert_eq!(b, a, "line {i} moved");
            }
        }
        let again = parse(&out);
        let ticked = again.item("i-3f2a").expect("item");
        assert!(ticked.done);
        assert_eq!(ticked.note.as_deref(), Some("done with care"));
    }

    #[test]
    fn crlf_and_a_missing_final_newline_survive_a_write() {
        let text = "# Checklist\r\n\r\n- [ ] one (i-0001)\r\n- [ ] two (i-0002)";
        let c = parse(text);
        let two = c.item("i-0002").expect("two");
        let out = replace_line(
            text,
            two.line,
            &two.ticked("a", "2026-01-01", None).render(),
        );
        assert_eq!(
            out,
            "# Checklist\r\n\r\n- [ ] one (i-0001)\r\n- [x] two (i-0002) — done by a, 2026-01-01"
        );
        let appended = append_item(text, &c, "- [ ] three (i-0003)");
        assert_eq!(
            appended,
            "# Checklist\r\n\r\n- [ ] one (i-0001)\r\n- [ ] two (i-0002)\r\n- [ ] three (i-0003)\r\n"
        );
    }

    #[test]
    fn an_appended_item_follows_the_last_task_and_moves_nothing() {
        let c = parse(DOC);
        let line = Item::new(
            "i-beef",
            "A new one",
            vec![Blocker::Item {
                item: "i-3f2a".into(),
            }],
        )
        .render();
        assert_eq!(line, "- [ ] A new one (i-beef, after i-3f2a)");
        let out = append_item(DOC, &c, &line);
        let last_task = c.items.iter().map(|i| i.line).max().expect("items");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[last_task + 1], line);
        let mut expected: Vec<&str> = DOC.lines().collect();
        expected.insert(last_task + 1, &line);
        assert_eq!(lines, expected);
        assert_eq!(parse(&out).items.len(), 6);
        // An empty checklist gets its first item at the end.
        let empty = "# Checklist\n\n";
        assert_eq!(
            append_item(empty, &parse(empty), "- [ ] x (i-0001)"),
            "# Checklist\n\n- [ ] x (i-0001)\n"
        );
    }

    #[test]
    fn an_unnamed_line_is_named_in_place() {
        let mut n = 0;
        let (out, named) = name_unnamed(DOC, &mut |_| {
            n += 1;
            format!("i-000{n}")
        });
        assert_eq!(named, vec!["i-0001".to_owned()]);
        assert!(out.contains("- [ ] A line with no id (i-0001)\n"));
        assert_eq!(out.len(), DOC.len() + " (i-0001)".len());
    }

    #[test]
    fn every_fault_is_named_with_its_line() {
        let text = "- [y] odd box (i-0001)\n- [] empty box\n- [ ] bad id (i-NOPE)\n- [ ] dup (i-0002)\n- [ ] dup again (i-0002)\n- [ ] waits (i-0003, after i-9999)\n- [ ] bad ref (i-0004, after nothing)\n- [ ] extra (i-0005, maybe)\n- [ ] a (i-000a, after i-000b)\n- [ ] b (i-000b, after i-000a)\n";
        let c = parse(text);
        let rules: Vec<(&str, usize)> = c.faults.iter().map(|f| (f.rule, f.line)).collect();
        assert!(
            rules.contains(&("ticket.checklist-malformed", 1)),
            "{rules:?}"
        );
        assert!(
            rules.contains(&("ticket.checklist-malformed", 2)),
            "{rules:?}"
        );
        assert!(
            rules.contains(&("ticket.checklist-malformed", 3)),
            "{rules:?}"
        );
        assert!(rules.contains(&("ticket.item-duplicate", 5)), "{rules:?}");
        assert!(rules.contains(&("ticket.blocker-unknown", 6)), "{rules:?}");
        assert!(
            rules.contains(&("ticket.checklist-malformed", 7)),
            "{rules:?}"
        );
        assert!(
            rules.contains(&("ticket.checklist-malformed", 8)),
            "{rules:?}"
        );
        assert!(rules.contains(&("ticket.blocker-cycle", 9)), "{rules:?}");
        assert!(rules.contains(&("ticket.blocker-cycle", 10)), "{rules:?}");
        // A sound checklist has none.
        assert!(parse(DOC).faults.is_empty());
    }

    #[test]
    fn cross_ticket_blockers_parse() {
        let c = parse("- [ ] x (i-0001, after t-3f2a, t-9c01/i-0002)\n");
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        assert_eq!(
            c.items[0].after,
            vec![
                Blocker::Ticket {
                    ticket: "t-3f2a".into()
                },
                Blocker::ItemOf {
                    ticket: "t-9c01".into(),
                    item: "i-0002".into()
                }
            ]
        );
    }

    #[test]
    fn ids_are_well_formed_and_lengthen_as_the_repository_grows() {
        assert_eq!(ticket_id_len(0), 4);
        assert!(ticket_id_len(50) >= 5);
        assert!(ticket_id_len(5000) > ticket_id_len(50));
        let t = ticket_id("0199a0d2-1e89-7990-8b2c-5a43577b5ce5", 4);
        assert!(is_ticket_id(&t) && t.len() == 6, "{t}");
        // Two UUIDs minted in the same millisecond share their head; their
        // ticket ids do not.
        assert_ne!(
            ticket_id("0199a0d2-1e89-7990-8b2c-5a43577b5ce5", 4),
            ticket_id("0199a0d2-1e89-7990-8b2c-5a43577b5ce6", 4)
        );
        let mut taken = BTreeSet::new();
        let first = item_id("seed", &taken);
        assert!(is_item_id(&first) && first.len() == 6);
        taken.insert(first.clone());
        let second = item_id("seed", &taken);
        assert_ne!(first, second, "a taken id is lengthened, never reused");
        assert!(second.starts_with(&first));
        for bad in [
            "t-",
            "t-ab",
            "t-ABCD",
            "i-12 3",
            "x-1234",
            "t-1234567890abcdefg",
        ] {
            assert!(!is_ticket_id(bad) && !is_item_id(bad), "{bad}");
        }
    }

    #[test]
    fn a_manifest_is_validated_against_the_profile_roles() {
        let m = TicketManifest {
            schema: TICKET_SCHEMA.into(),
            id: "t-3f2a".into(),
            uuid: "0199a0d2-1e89-7990-8b2c-5a43577b5ce5".into(),
            title: "A ticket".into(),
            profile: TICKET_PROFILE.into(),
            priority: 2,
            created_at: "2026-09-25T00:00:00Z".into(),
            created_by: "claude".into(),
            promoted_to: None,
            atoms: vec![
                TicketAtom {
                    ordinal: 10,
                    role: "intent".into(),
                    path: "atoms/10-intent.md".into(),
                },
                TicketAtom {
                    ordinal: 15,
                    role: CHECKLIST_ROLE.into(),
                    path: "atoms/15-checklist.md".into(),
                },
            ],
        };
        let roles = vec!["intent".to_owned(), CHECKLIST_ROLE.to_owned()];
        assert_eq!(m.validate(&roles), Ok(()));
        let mut missing = m.clone();
        missing.atoms.pop();
        assert!(
            missing
                .validate(&roles)
                .unwrap_err()
                .contains("ticket.checklist")
        );
        let mut escape = m.clone();
        escape.atoms[0].path = "../../warrants/x.md".into();
        assert!(escape.validate(&roles).is_err());
        let mut bad = m;
        bad.priority = 9;
        assert!(bad.validate(&roles).is_err());
    }
}
