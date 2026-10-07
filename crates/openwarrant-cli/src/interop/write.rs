// SPDX-License-Identifier: Apache-2.0
//! Writing imported work as Warrants in the light encoding (OW-WAR-0148
//! M10): the same three files `war create` writes, so an imported Warrant
//! is worked with `war ready`, `claim` and `done` like any other.
//!
//! Deterministic: a Warrant's id is derived from where it came from
//! (`beads:bd-a1b2`), its UUID from that and when it was created, its item
//! ids from its source and the task's own key, so the same input gives the
//! same files in any repository. A source that names no creation time takes
//! the import's own, which then moves only `created_at` and the UUID. Idempotent: a Warrant whose `imported_from` is
//! already in the store is named and left alone. All-or-nothing: every
//! refusal is found before the first file is written.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8PathBuf;
use openwarrant_core::ticket::{
    self, Blocker, CHECKLIST_ROLE, Item, TICKET_PROFILE, TICKET_SCHEMA, TicketAtom, TicketManifest,
};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use crate::ticket::{INTENT_FILE, NOTES_HEADING, Outcome, Store, event, toml_of};

/// One dated note, as the intent's `## Notes` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// `YYYY-MM-DD HH:MM` or `YYYY-MM-DD HH:MM:SS`, UTC.
    pub at: String,
    pub who: String,
    pub text: String,
}

/// What an incoming item waits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ref {
    /// Another item of the same Warrant, by its key.
    Item(String),
    /// A whole Warrant, by where it came from (`beads:bd-a1b2`).
    Warrant(String),
}

/// One item of an incoming Warrant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingItem {
    /// Unique within its Warrant and stable across runs: a task number
    /// (`1.1`, `T001`), or `issue` for a Beads issue's one item.
    pub key: String,
    pub text: String,
    pub done: bool,
    pub done_by: Option<String>,
    pub done_on: Option<String>,
    pub note: Option<String>,
    pub after: Vec<Ref>,
}

/// One Warrant to bring in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    /// `<format>:<id>`: what the manifest's `imported_from` records.
    pub source: String,
    pub title: String,
    /// The description, Markdown; may be empty.
    pub body: String,
    pub notes: Vec<Note>,
    pub priority: u8,
    pub kind: Option<String>,
    pub labels: Vec<String>,
    /// The Warrant this one is part of, by its source.
    pub part_of: Option<String>,
    /// RFC 3339, UTC.
    pub created_at: String,
    pub created_by: String,
    pub items: Vec<IncomingItem>,
}

/// One import, read and mapped, not yet written.
#[derive(Debug, Clone, Default)]
pub struct Import {
    /// `beads`, `openspec` or `speckit`.
    pub format: &'static str,
    /// What was read, as the message names it.
    pub origin: String,
    pub incoming: Vec<Incoming>,
    /// Record atoms to write beside the Warrants: (repository-relative
    /// path, text). One already there with the same bytes is unchanged.
    pub records: Vec<(String, String)>,
    /// Said, never swallowed: what the mapping dropped or read loosely.
    pub warnings: Vec<Diagnostic>,
}

/// A refusal naming every fault at once; nothing was written.
#[must_use]
pub fn refused(faults: &[super::Fault]) -> Outcome {
    let mut report = Report::default();
    for f in faults {
        // The place in the words too: a person reads the message alone.
        let place = f.place();
        report.push(Diagnostic::error(
            f.rule,
            place.clone(),
            if place.is_empty() {
                f.message.clone()
            } else {
                format!("{place}: {}", f.message)
            },
        ));
    }
    let human = faults
        .iter()
        .map(|f| format!("{} ({}): {}", f.place(), f.rule, f.message))
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = Outcome::ok(
        format!("{human}\nNothing was written."),
        serde_json::Value::Null,
    );
    out.report = report;
    out
}

/// A UUIDv7 derived from `seed` and `created_at`: the timestamp bits are
/// the creation time, the rest a digest of the seed, so the same Warrant
/// gets the same identity in every repository it is imported into.
#[must_use]
pub fn derived_uuid(seed: &str, created_at: &str) -> String {
    let millis = chrono::DateTime::parse_from_rfc3339(created_at)
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
        .clamp(0, (1_i64 << 48) - 1) as u64;
    let h =
        <sha2::Sha256 as sha2::Digest>::digest(format!("openwarrant-import\0{seed}").as_bytes());
    let mut b = [0u8; 16];
    b[..6].copy_from_slice(&millis.to_be_bytes()[2..]);
    b[6] = 0x70 | (h[0] & 0x0f);
    b[7] = h[1];
    b[8] = 0x80 | (h[2] & 0x3f);
    b[9..].copy_from_slice(&h[3..10]);
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// The length of a derived id: long enough that two imports of a few
/// thousand Warrants each stay clear of each other and of `war create`'s
/// shorter ids.
const ID_LEN: usize = 8;

struct Planned<'a> {
    incoming: &'a Incoming,
    id: String,
    uuid: String,
    items: Vec<(String, &'a IncomingItem)>,
}

/// Bring `import` in: plan every Warrant, refuse by rule if anything cannot
/// be written as asked, else write them all.
pub fn apply(repo: &Repository, store: &Store, import: &Import) -> Result<Outcome, RepoError> {
    let (existing, _) = store.load_all()?;
    let mut taken: BTreeSet<String> = store
        .ticket_dirs()?
        .iter()
        .filter_map(|d| d.file_name().map(str::to_owned))
        .collect();
    let mut by_source: BTreeMap<String, String> = existing
        .iter()
        .filter_map(|t| {
            t.manifest
                .imported_from
                .clone()
                .map(|s| (s, t.id().to_owned()))
        })
        .collect();
    let parents: BTreeMap<String, Option<String>> = existing
        .iter()
        .map(|t| (t.id().to_owned(), t.manifest.part_of.clone()))
        .collect();
    let file = import.origin.clone();
    let mut faults = Vec::new();
    let mut unchanged = Vec::new();
    let mut planned: Vec<Planned> = Vec::new();
    let mut seen = BTreeSet::new();
    for inc in &import.incoming {
        if !seen.insert(inc.source.clone()) {
            faults.push(super::Fault::new(
                "import.duplicate",
                &file,
                0,
                format!(
                    "{} appears twice in the input; one source, one Warrant",
                    inc.source
                ),
            ));
            continue;
        }
        if let Some(id) = by_source.get(&inc.source) {
            unchanged.push((inc.source.clone(), id.clone(), inc.title.clone()));
            continue;
        }
        let uuid = derived_uuid(&inc.source, &inc.created_at);
        // The id is the source's alone, so it is the same in every
        // repository even when the source names no creation time.
        let seed = format!("openwarrant-import\0{}", inc.source);
        let id = (ID_LEN..=16)
            .map(|len| ticket::ticket_id(&seed, len))
            .find(|c| !taken.contains(c));
        let Some(id) = id else {
            faults.push(super::Fault::new(
                "import.id-taken",
                &file,
                0,
                format!("every length of {}'s id is taken", inc.source),
            ));
            continue;
        };
        taken.insert(id.clone());
        by_source.insert(inc.source.clone(), id.clone());
        let mut keys = BTreeSet::new();
        let mut item_ids = BTreeSet::new();
        let mut items = Vec::new();
        for it in &inc.items {
            if !keys.insert(it.key.clone()) {
                faults.push(super::Fault::new(
                    "import.duplicate",
                    &file,
                    0,
                    format!("{}: task {} appears twice", inc.source, it.key),
                ));
                continue;
            }
            let item = ticket::item_id(&format!("{}/{}", inc.source, it.key), &item_ids);
            item_ids.insert(item.clone());
            items.push((item, it));
        }
        planned.push(Planned {
            incoming: inc,
            id,
            uuid,
            items,
        });
    }
    // Every reference resolves, to this input or to what was imported before.
    let mut rendered: Vec<(Planned, TicketManifest, String, String)> = Vec::new();
    let mut parent_of: BTreeMap<String, Option<String>> = parents;
    for p in &planned {
        let part_of = match &p.incoming.part_of {
            None => None,
            Some(src) => match by_source.get(src) {
                Some(id) if id != &p.id => Some(id.clone()),
                Some(_) => {
                    faults.push(super::Fault::new(
                        "import.part-of-cycle",
                        &file,
                        0,
                        format!("{} is part of itself", p.incoming.source),
                    ));
                    None
                }
                None => {
                    faults.push(super::Fault::new(
                        "import.target-unknown",
                        &file,
                        0,
                        format!(
                            "{} is part of {src}, which is neither in this input nor imported \
                             before",
                            p.incoming.source
                        ),
                    ));
                    None
                }
            },
        };
        parent_of.insert(p.id.clone(), part_of.clone());
    }
    for p in &planned {
        // A part_of chain that comes back to itself can never finish.
        let mut at = parent_of.get(&p.id).cloned().flatten();
        let mut hops = BTreeSet::new();
        while let Some(id) = at {
            if id == p.id {
                faults.push(super::Fault::new(
                    "import.part-of-cycle",
                    &file,
                    0,
                    format!(
                        "{} is part of a Warrant that is, through part_of, part of it",
                        p.incoming.source
                    ),
                ));
                break;
            }
            if !hops.insert(id.clone()) {
                break;
            }
            at = parent_of.get(&id).cloned().flatten();
        }
    }
    for p in planned {
        let keyed: BTreeMap<&str, &str> = p
            .items
            .iter()
            .map(|(id, it)| (it.key.as_str(), id.as_str()))
            .collect();
        let mut checklist = store.checklist_stub();
        if !checklist.ends_with('\n') {
            checklist.push('\n');
        }
        for (id, it) in &p.items {
            let mut after = Vec::new();
            for r in &it.after {
                match r {
                    Ref::Item(key) => match keyed.get(key.as_str()) {
                        Some(i) if *i != id.as_str() => after.push(Blocker::Item {
                            item: (*i).to_owned(),
                        }),
                        Some(_) => faults.push(super::Fault::new(
                            "import.blocker-cycle",
                            &file,
                            0,
                            format!("{}: task {} waits on itself", p.incoming.source, it.key),
                        )),
                        None => faults.push(super::Fault::new(
                            "import.target-unknown",
                            &file,
                            0,
                            format!(
                                "{}: task {} waits on {key}, which is no task of it",
                                p.incoming.source, it.key
                            ),
                        )),
                    },
                    Ref::Warrant(src) => match by_source.get(src) {
                        Some(t) if *t != p.id => after.push(Blocker::Ticket { ticket: t.clone() }),
                        Some(_) => faults.push(super::Fault::new(
                            "import.blocker-cycle",
                            &file,
                            0,
                            format!("{} waits on itself", p.incoming.source),
                        )),
                        None => faults.push(super::Fault::new(
                            "import.target-unknown",
                            &file,
                            0,
                            format!(
                                "{} waits on {src}, which is neither in this input nor imported \
                                 before",
                                p.incoming.source
                            ),
                        )),
                    },
                }
            }
            let text = ticket::one_line(&it.text);
            if text.is_empty() {
                faults.push(super::Fault::new(
                    "import.item-empty",
                    &file,
                    0,
                    format!("{}: task {} has no text", p.incoming.source, it.key),
                ));
                continue;
            }
            let mut item = Item::new(id, &text, after);
            if it.done {
                item = item.ticked(
                    it.done_by.as_deref().unwrap_or("unknown"),
                    it.done_on.as_deref().unwrap_or_default(),
                    it.note.as_deref(),
                );
                if it.done_on.is_none() {
                    item.done_on = None;
                }
            }
            checklist.push_str(&item.render());
            checklist.push('\n');
        }
        let created_by = Some(ticket::one_line(&p.incoming.created_by))
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| store.actor.clone());
        let manifest = TicketManifest {
            schema: TICKET_SCHEMA.to_owned(),
            id: p.id.clone(),
            uuid: p.uuid.clone(),
            title: ticket::one_line(&p.incoming.title),
            profile: TICKET_PROFILE.to_owned(),
            priority: p.incoming.priority,
            created_at: p.incoming.created_at.clone(),
            created_by,
            promoted_to: None,
            kind: p.incoming.kind.clone(),
            labels: p.incoming.labels.clone(),
            part_of: parent_of.get(&p.id).cloned().flatten(),
            issue: None,
            issue_url: None,
            imported_from: Some(p.incoming.source.clone()),
            atoms: vec![
                TicketAtom {
                    ordinal: 10,
                    role: "intent".to_owned(),
                    path: INTENT_FILE.to_owned(),
                },
                TicketAtom {
                    ordinal: store.checklist_ordinal(),
                    role: CHECKLIST_ROLE.to_owned(),
                    path: format!("atoms/{}", store.checklist_file()),
                },
            ],
        };
        if let Err(why) = manifest.validate(&store.definition.working_roles()) {
            faults.push(super::Fault::new(
                "import.invalid",
                &file,
                0,
                format!("{}: {why}", p.incoming.source),
            ));
            continue;
        }
        if let Some(k) = &manifest.kind
            && let Some(why) = store.definition.fields.refuse_type(k)
        {
            faults.push(super::Fault::new(
                "import.type-unmapped",
                &file,
                0,
                format!("{}: {why}", p.incoming.source),
            ));
        }
        for l in &manifest.labels {
            if let Some(why) = store.definition.fields.refuse_label(l) {
                faults.push(super::Fault::new(
                    "import.label-unmapped",
                    &file,
                    0,
                    format!("{}: {why}", p.incoming.source),
                ));
            }
        }
        let intent = intent_of(p.incoming);
        rendered.push((p, manifest, intent, checklist));
    }
    // Record atoms: a new one, or one already there with the same bytes.
    let mut records_new = Vec::new();
    let mut records_same = Vec::new();
    for (rel, text) in &import.records {
        let path = repo.root.join(rel);
        match crate::vfs::read(&path) {
            Ok(bytes) if bytes == text.as_bytes() => records_same.push(rel.clone()),
            Ok(_) => faults.push(super::Fault::new(
                "import.records-exist",
                rel,
                0,
                "a different file is already here; nothing is overwritten",
            )),
            Err(_) => records_new.push((rel.clone(), text.clone())),
        }
    }
    if !faults.is_empty() {
        faults.sort();
        faults.dedup();
        return Ok(refused(&faults));
    }

    // ---- write: every refusal is behind us.
    std::fs::create_dir_all(&store.dir).map_err(|source| RepoError::Io {
        context: format!("could not create {}", store.dir),
        source,
    })?;
    let mut created = Vec::new();
    for (p, manifest, intent, checklist) in &rendered {
        let dir = store.dir.join(&p.id);
        std::fs::create_dir(&dir).map_err(|source| RepoError::Io {
            context: format!("could not create {dir}"),
            source,
        })?;
        std::fs::create_dir_all(dir.join("atoms")).map_err(|source| RepoError::Io {
            context: format!("could not create {dir}/atoms"),
            source,
        })?;
        crate::compile::atomic::write(&dir.join(INTENT_FILE), intent.clone())?;
        crate::compile::atomic::write(
            &dir.join(format!("atoms/{}", store.checklist_file())),
            checklist.clone(),
        )?;
        crate::compile::atomic::write(&dir.join("manifest.toml"), toml_of(manifest)?)?;
        let t = store
            .load(&dir)
            .map_err(|d| RepoError::Message(format!("{}: {}", d.rule, d.message)))?;
        store.journal(
            &t,
            event::CREATED,
            &serde_json::json!({
                "ticket": p.id,
                "title": manifest.title,
                "items": p.items.len(),
                "imported_from": p.incoming.source,
            }),
        )?;
        created.push(serde_json::json!({
            "id": p.id,
            "imported_from": p.incoming.source,
            "title": manifest.title,
            "items": p.items.len(),
            "dir": store.rel(&dir),
        }));
    }
    for (rel, text) in &records_new {
        let path: Utf8PathBuf = repo.root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                context: format!("could not create {parent}"),
                source,
            })?;
        }
        crate::compile::atomic::write(&path, text.clone())?;
    }

    // ---- what was done, in words.
    let mut human = String::new();
    if created.is_empty() && records_new.is_empty() {
        human.push_str(&format!(
            "nothing new from {} ({}): every Warrant in it is already here",
            import.format, import.origin
        ));
    } else {
        human.push_str(&format!(
            "imported {} Warrant(s) from {} ({})",
            created.len(),
            import.format,
            import.origin
        ));
    }
    for c in &created {
        human.push_str(&format!(
            "\n  {}  {}  {}",
            c["id"].as_str().unwrap_or_default(),
            c["imported_from"].as_str().unwrap_or_default(),
            c["title"].as_str().unwrap_or_default()
        ));
    }
    if !unchanged.is_empty() {
        human.push_str(&format!(
            "\nalready here, left as they are: {}",
            unchanged
                .iter()
                .map(|(s, id, _)| format!("{s} ({id})"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for (rel, _) in &records_new {
        human.push_str(&format!("\nrecords: {rel}"));
    }
    if !created.is_empty() {
        human.push_str("\n`war ready` lists their open items");
    }
    let mut out = Outcome::ok(
        human,
        serde_json::json!({
            "schema": "oh.war/import/v1",
            "format": import.format,
            "from": import.origin,
            "created": created,
            "unchanged": unchanged
                .iter()
                .map(|(s, id, title)| serde_json::json!({"id": id, "imported_from": s, "title": title}))
                .collect::<Vec<_>>(),
            "records": records_new.iter().map(|(r, _)| r.clone()).collect::<Vec<_>>(),
            "records_unchanged": records_same,
        }),
    );
    for w in &import.warnings {
        out.report.push(w.clone());
    }
    Ok(out)
}

/// The intent atom: the title, the description, and the dated notes.
fn intent_of(inc: &Incoming) -> String {
    let mut out = format!("# {}\n", ticket::one_line(&inc.title));
    let body = inc.body.trim();
    if !body.is_empty() {
        out.push('\n');
        // A description's own `## Notes` would be read as the notes
        // section; it is kept, under a heading that is not that one.
        for line in body.lines() {
            if line.trim_end() == NOTES_HEADING {
                out.push_str("## Notes (as written)");
            } else {
                out.push_str(line.trim_end());
            }
            out.push('\n');
        }
    }
    if !inc.notes.is_empty() {
        out.push('\n');
        out.push_str(NOTES_HEADING);
        out.push_str("\n\n");
        for n in &inc.notes {
            let mut lines = n.text.trim().lines();
            out.push_str(&format!(
                "- **{} UTC, {}:** {}\n",
                n.at,
                ticket::one_line(&n.who),
                lines.next().unwrap_or_default().trim()
            ));
            for more in lines {
                out.push_str("  ");
                out.push_str(more.trim_end());
                out.push('\n');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_derived_uuid_is_a_v7_and_depends_only_on_its_inputs() {
        let a = derived_uuid("beads:bd-a1", "2026-04-18T16:19:12Z");
        assert_eq!(a, derived_uuid("beads:bd-a1", "2026-04-18T16:19:12Z"));
        assert_ne!(a, derived_uuid("beads:bd-a2", "2026-04-18T16:19:12Z"));
        let parsed: openwarrant_core::WarUuid = a.parse().expect("a UUIDv7");
        assert_eq!(parsed.to_string(), a);
        // The timestamp bits are the creation time.
        assert!(a.starts_with("019da163-9800-7"), "{a}");
    }
}
