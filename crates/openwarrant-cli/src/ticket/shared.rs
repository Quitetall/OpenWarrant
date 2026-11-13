// SPDX-License-Identifier: Apache-2.0
//! Shared work (R2): a ticket made in one worktree is every worktree's at
//! once, and no two agents make the same one.
//!
//! Tickets stay files in git. What this adds is an announcement beside
//! M11's shared claims, under git's common directory:
//! `<common>/openwarrant/tickets/<id>`, one JSON [`Entry`] per ticket that is
//! open and not yet on the default branch. Outside a git checkout (or with
//! `[tickets] claims_dir` set) the register is `.openwarrant/state/tickets/`
//! under the root.
//!
//! - **An id is one ticket's across the clone.** `war create` and
//!   `war plan new` take the id's name in the register as a claim takes a
//!   lock ([`take_file`]: a whole temporary file hard-linked to the name,
//!   which fails if the name exists). The id is taken only when the local
//!   directory and the register both gave it.
//! - **Seen everywhere.** [`add_elsewhere`] adds to what a store loads every
//!   registered ticket this checkout has no file for, marked with where it
//!   lives ([`Place`]). A local file with the same UUID wins; a different
//!   UUID under one id is `ticket.id-collision`, naming both. Only a store
//!   a command of the ticket loop opened asks for them ([`Store::sharing`]):
//!   what is compiled, exported or judged reads this checkout's files alone.
//! - **Worked anywhere.** [`bring_here`] writes an elsewhere ticket's
//!   document into this worktree at the same path; the two branches then
//!   merge item by item (`merge.rs`).
//! - **Kept current, then retired.** Every journalled act on a registered
//!   ticket refreshes its entry ([`sync`]); the entry is dropped when the
//!   ticket is done, or once its file is on the default branch ([`sweep`]),
//!   where everyone has it through git.
//! - **Other clones.** With `[claims] remote`, the entry is also a ref,
//!   `refs/openwarrant/tickets/<id>`: a commit whose message is the record
//!   and whose tree holds the document. The push that creates it expects no
//!   ref, so the remote is the compare-and-set for the id. It is read back at
//!   most once per `remote_ttl`. With no remote, nothing asks the network; a
//!   remote that cannot be read is `UNKNOWN`, never "nothing".
//! - **No duplicates.** `war create` holds a short lock named by its
//!   title's words ([`TitleLock`]), then refuses a title that is, or reads
//!   as, an open ticket's ([`alike`]): `ticket.duplicate`, naming it.
//!   `--force` creates anyway; a done ticket never blocks.
//!
//! What a reader pays: one listing of the register. A registered id this
//! checkout holds is settled from the listing alone (its UUID is in a marker
//! name, `<id>~<uuid>`); only entries of tickets held elsewhere are read, as
//! every claim lock is.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::{Outcome, Store, Target, Ticket, remote};
use crate::diagnostic::Diagnostic;
use crate::repo::RepoError;

/// Under git's common directory: the clone's register.
pub const SUBDIR: &str = "openwarrant/tickets";
/// The register outside a git checkout, under the root.
pub const LOCAL_DIR: &str = ".openwarrant/state/tickets";
pub const ENTRY_SCHEMA: &str = "oh.war/ticket-entry/v1";
/// Under git's common directory: the remote's tickets as last read.
const SEEN_FILE: &str = "openwarrant/remote-tickets.json";
/// In the register: the default branch's tip when it was last swept.
const SWEPT: &str = ".swept";
/// How long a title lock holds before it reads as left behind by a creator
/// that died.
const TITLE_LOCK_SECS: u64 = 30;
/// The share of the longer title's words the shorter must cover for two
/// titles to read as one piece of work ([`alike`]).
pub const CLOSE: f64 = 0.6;

/// Where a ticket this checkout lacks lives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    /// The root of the worktree that registered it.
    pub worktree: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// The claims remote it was read from, when no worktree of this clone
    /// registered it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

impl Place {
    /// `worktree /w (branch b)`, or `remote origin (branch b)`.
    #[must_use]
    pub fn describe(&self) -> String {
        let branch = self
            .branch
            .as_ref()
            .map(|b| format!(" (branch {b})"))
            .unwrap_or_default();
        match &self.remote {
            Some(r) => format!("remote {r}{branch}"),
            None => format!("worktree {}{branch}", self.worktree),
        }
    }
}

/// What this clone last published for an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Published {
    pub remote: String,
    pub commit: String,
}

/// One registered ticket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub schema: String,
    pub id: String,
    pub uuid: String,
    pub title: String,
    #[serde(default)]
    pub labels: Vec<String>,
    /// `Ticket`, `Warrant` or `Order` as the registrant read it.
    pub level: String,
    /// `creating` until its document is written, then `open`. A done ticket
    /// has no entry.
    pub state: String,
    pub created_by: String,
    /// The repository root of the worktree that registered it.
    pub root: String,
    /// That root below its worktree's top level (empty at the top): entries
    /// of another root in the same clone are not this store's.
    #[serde(default)]
    pub below: String,
    #[serde(default)]
    pub branch: Option<String>,
    /// Its document, relative to the root.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_of: Option<String>,
    /// Its files as last written, by path relative to the root: the one
    /// document, or a directory ticket's manifest and atoms.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published: Option<Published>,
    /// Set only as read from a claims remote: its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

impl Entry {
    fn place(&self) -> Place {
        Place {
            worktree: self.root.clone(),
            branch: self.branch.clone(),
            remote: self.remote.clone(),
        }
    }
}

/// The register of one store.
#[derive(Debug, Clone)]
pub struct Registry {
    pub dir: Utf8PathBuf,
    root: Utf8PathBuf,
    below: String,
    toplevel: Option<Utf8PathBuf>,
    common: Option<Utf8PathBuf>,
}

/// Link a whole file to `path`, failing if the name exists: `claim::take`'s
/// pattern. `Ok(false)`: someone holds the name.
fn take_file(path: &Utf8Path, bytes: &[u8]) -> std::io::Result<bool> {
    let dir = path.parent().unwrap_or(Utf8Path::new("."));
    std::fs::create_dir_all(dir)?;
    // Runtime state, never a source: the directory ignores itself.
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        let _ = std::fs::write(
            &ignore,
            "# war: the tickets this clone announces; runtime state, never committed\n*\n",
        );
    }
    let temp = dir.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name().unwrap_or("entry"),
        std::process::id(),
        openwarrant_core::WarUuid::mint()
    ));
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    let linked = std::fs::hard_link(&temp, path);
    let _ = std::fs::remove_file(&temp);
    match linked {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        // No hard links here: O_EXCL create, then write.
        Err(_) => match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(mut f) => {
                f.write_all(bytes)?;
                f.sync_all()?;
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(e) => Err(e),
        },
    }
}

impl Store {
    /// This store's register; `None` in a hosted or recording run, which
    /// reads only its basis and writes nothing.
    #[must_use]
    pub fn registry(&self) -> Option<Registry> {
        if !crate::vfs::is_disk() {
            return None;
        }
        Some(match &self.layout {
            Some(layout) => Registry {
                dir: layout.common_dir.join(SUBDIR),
                root: self.root.clone(),
                below: self
                    .root
                    .strip_prefix(&layout.toplevel)
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                toplevel: Some(layout.toplevel.clone()),
                common: Some(layout.common_dir.clone()),
            },
            None => Registry {
                dir: self.root.join(LOCAL_DIR),
                root: self.root.clone(),
                below: String::new(),
                toplevel: None,
                common: None,
            },
        })
    }

    /// With `[claims] remote`: the tickets other clones announced there,
    /// read at most once per `remote_ttl`. `None` with no remote, and then
    /// nothing is asked of the network. `Err` is `UNKNOWN`.
    pub fn remote_tickets(&self) -> Option<Result<Vec<Entry>, String>> {
        let name = self.remote.as_deref()?;
        if !crate::vfs::is_disk() {
            return Some(Err("not read in a hosted or recording run".to_owned()));
        }
        let Some(layout) = &self.layout else {
            return Some(Err("this is not a git checkout".to_owned()));
        };
        Some(seen_remote(
            &layout.toplevel,
            &layout.common_dir,
            name,
            self.remote_ttl_secs,
            super::now_secs(),
        ))
    }
}

impl Registry {
    fn entry_path(&self, id: &str) -> Utf8PathBuf {
        self.dir.join(id)
    }

    fn marker_path(&self, id: &str, uuid: &str) -> Utf8PathBuf {
        self.dir.join(format!("{id}~{uuid}"))
    }

    /// A new entry for this store's checkout, with no files yet.
    #[must_use]
    pub fn stub(&self, id: &str, uuid: &str, title: &str, actor: &str, path: &str) -> Entry {
        Entry {
            schema: ENTRY_SCHEMA.to_owned(),
            id: id.to_owned(),
            uuid: uuid.to_owned(),
            title: title.to_owned(),
            labels: Vec::new(),
            level: crate::work::Level::Ticket.as_str().to_owned(),
            state: "creating".to_owned(),
            created_by: actor.to_owned(),
            root: self.root.to_string(),
            below: self.below.clone(),
            branch: self.toplevel.as_deref().and_then(super::live::checked_out),
            path: path.to_owned(),
            part_of: None,
            files: BTreeMap::new(),
            published: None,
            remote: None,
        }
    }

    /// Take `entry.id` for it. `Ok(false)`: the id is another ticket's.
    pub fn reserve(&self, entry: &Entry) -> std::io::Result<bool> {
        let bytes = serde_json::to_vec_pretty(entry).map_err(std::io::Error::other)?;
        if !take_file(&self.entry_path(&entry.id), &bytes)? {
            return Ok(false);
        }
        // The listing then answers whose id it is without a read.
        let _ = std::fs::write(self.marker_path(&entry.id, &entry.uuid), b"");
        Ok(true)
    }

    /// A new work order's entry (`war plan new`), once its document is
    /// written: open, with its bytes.
    pub fn fill(&self, entry: &mut Entry, delivery: bool, text: &str) {
        // A delivery ends with a person's signature; a decision's level is
        // what its own text comes to say.
        entry.level = if delivery {
            crate::work::Level::Order
        } else {
            crate::work::Level::Unknown
        }
        .as_str()
        .to_owned();
        entry.state = "open".to_owned();
        entry.files.insert(entry.path.clone(), text.to_owned());
        self.write(entry);
    }

    /// Replace an entry already taken.
    fn write(&self, entry: &Entry) {
        if let Ok(bytes) = serde_json::to_vec_pretty(entry) {
            let _ = crate::compile::atomic::write(&self.entry_path(&entry.id), bytes);
        }
    }

    #[must_use]
    pub fn read(&self, id: &str) -> Option<Entry> {
        std::fs::read(self.entry_path(id))
            .ok()
            .and_then(|b| serde_json::from_slice::<Entry>(&b).ok())
            .filter(|e| e.id == id)
    }

    /// Drop `id` if it is still `uuid`'s.
    pub fn release(&self, id: &str, uuid: &str) {
        if self.read(id).is_some_and(|e| e.uuid == uuid) {
            let _ = std::fs::remove_file(self.entry_path(id));
            let _ = std::fs::remove_file(self.marker_path(id, uuid));
        }
    }

    /// The registered ids, and the UUID each marker names.
    fn listing(&self) -> (BTreeSet<String>, BTreeMap<String, String>) {
        let mut ids = BTreeSet::new();
        let mut uuids = BTreeMap::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return (ids, uuids);
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            match name.split_once('~') {
                Some((id, uuid)) => {
                    uuids.insert(id.to_owned(), uuid.to_owned());
                }
                None => {
                    ids.insert(name);
                }
            }
        }
        (ids, uuids)
    }

    /// Every entry of this store's root.
    #[must_use]
    pub fn entries(&self) -> Vec<Entry> {
        self.listing()
            .0
            .iter()
            .filter_map(|id| self.read(id))
            .filter(|e| e.below == self.below)
            .collect()
    }
}

/// The files of a ticket this checkout holds, by path relative to the root.
fn files_of(store: &Store, t: &Ticket) -> BTreeMap<String, String> {
    let mut paths = vec![t.intent_path.clone()];
    if !t.format {
        paths.push(t.dir.join("manifest.toml"));
        paths.push(t.checklist_path.clone());
        paths.push(t.dir.join(openwarrant_core::ticks::CHECKS_FILE));
    }
    paths
        .into_iter()
        .filter_map(|p| Some((store.rel(&p), std::fs::read_to_string(&p).ok()?)))
        .collect()
}

/// The ticket an entry describes, as this store would read its files.
/// `None`: nothing to list (no document yet, or a master that is no ticket).
fn ticket_of(store: &Store, e: &Entry) -> Option<Result<Ticket, Diagnostic>> {
    let text = e.files.get(&e.path)?;
    let path = store.root.join(&e.path);
    let mut read = if path.file_name() == Some("manifest.toml") {
        let dir = path.parent()?.to_owned();
        store.load_read(&dir, &mut |p| {
            e.files
                .get(&store.rel(p))
                .cloned()
                .ok_or_else(|| "the register holds no copy of it".to_owned())
        })
    } else {
        if Utf8Path::new(&e.path).starts_with(crate::subject::WORK_ROOT)
            && crate::format_doc::frontmatter_profile(text) != Some("ticket")
        {
            return None;
        }
        store.format_ticket(&path, &e.id, text.clone().into_bytes())
    };
    if let Ok(t) = &mut read {
        t.elsewhere = Some(e.place());
    }
    Some(read)
}

fn collision(store: &Store, here: &Ticket, there: &Entry) -> Diagnostic {
    Diagnostic::error(
        "ticket.id-collision",
        store.rel(&here.intent_path),
        format!(
            "two tickets are {}: \"{}\" here (uuid {}) and \"{}\" in {} (uuid {}). Ids are \
             unique per clone, so one came in by a merge or another clone: recreate one of them \
             (`war create --force`) and close the other",
            here.id(),
            here.manifest.title,
            here.manifest.uuid,
            there.title,
            there.place().describe(),
            there.uuid
        ),
    )
}

/// Add to `tickets` every registered or remote-announced ticket this
/// checkout has no file for, marked [`Ticket::elsewhere`]; a registered id
/// held here under another UUID is a fault, and stays listed. `skip`: ids
/// this checkout holds but does not list (two documents of one id).
pub(super) fn add_elsewhere(
    store: &Store,
    tickets: &mut Vec<Ticket>,
    faults: &mut Vec<Diagnostic>,
    skip: &BTreeSet<String>,
) {
    let Some(reg) = store.registry() else {
        return;
    };
    let (ids, uuids) = reg.listing();
    if ids.is_empty() && store.remote.is_none() {
        return;
    }
    let mut added: Vec<Ticket> = Vec::new();
    {
        let here: BTreeMap<&str, &Ticket> = tickets.iter().map(|t| (t.id(), t)).collect();
        let mut consider = |e: Entry, added: &mut Vec<Ticket>| {
            // An entry this very worktree registered is never "elsewhere":
            // its file is here, or was removed or cannot be read here, and
            // the store has already said which.
            if e.below != reg.below
                || skip.contains(&e.id)
                || (e.remote.is_none() && e.root == reg.root.as_str())
            {
                return;
            }
            if let Some(local) = here.get(e.id.as_str()) {
                if !local.manifest.uuid.is_empty() && local.manifest.uuid != e.uuid {
                    faults.push(collision(store, local, &e));
                }
                return;
            }
            if let Some(first) = added.iter().find(|t| t.id() == e.id) {
                if first.manifest.uuid != e.uuid {
                    faults.push(collision(store, first, &e));
                }
                return;
            }
            match ticket_of(store, &e) {
                Some(Ok(t)) => added.push(t),
                Some(Err(d)) => faults.push(d),
                None => {}
            }
        };
        for id in &ids {
            // A registered id this checkout holds under the same UUID is
            // settled by the listing: nothing is read.
            if let (Some(local), Some(uuid)) = (here.get(id.as_str()), uuids.get(id))
                && &local.manifest.uuid == uuid
            {
                continue;
            }
            if let Some(e) = reg.read(id) {
                consider(e, &mut added);
            }
        }
        if let Some(Ok(announced)) = store.remote_tickets() {
            for e in announced {
                consider(e, &mut added);
            }
        }
    }
    if !added.is_empty() {
        tickets.extend(added);
        tickets.sort_by(|a, b| a.id().cmp(b.id()));
    }
}

/// What announcing a new ticket came to.
pub(super) enum Announced {
    Done,
    /// Another clone took the id on the remote first: whose it is.
    Taken(String),
    /// The remote could not be asked: the ticket is this clone's only.
    Unannounced(String),
}

/// Fill a new ticket's entry with its files and, with `[claims] remote`,
/// publish it: the push expects no ref, so of two clones minting one id,
/// one wins.
pub(super) fn announce(store: &Store, t: &Ticket) -> Announced {
    let Some(reg) = store.registry() else {
        return Announced::Done;
    };
    let Some(mut e) = reg.read(t.id()).filter(|e| e.uuid == t.manifest.uuid) else {
        return Announced::Done;
    };
    refresh(store, &mut e, t);
    let said = match publish(store, &reg, &mut e) {
        Ok(true) => Announced::Done,
        Ok(false) => Announced::Taken(
            remote_holder(store, &reg, &e.id).unwrap_or_else(|| "another clone".to_owned()),
        ),
        Err(why) => Announced::Unannounced(why),
    };
    reg.write(&e);
    said
}

fn refresh(store: &Store, e: &mut Entry, t: &Ticket) {
    e.title = t.manifest.title.clone();
    e.labels = t.manifest.labels.clone();
    e.part_of = t.manifest.part_of.clone();
    e.level = crate::work::ticket_level(store, t).as_str().to_owned();
    e.state = "open".to_owned();
    e.files = files_of(store, t);
}

/// The record a ref's commit carries: the entry without its files, and the
/// blob each file is in the commit's tree.
#[derive(Serialize, Deserialize)]
struct Record {
    #[serde(flatten)]
    entry: Entry,
    /// Path relative to the root, to the blob's name in the tree.
    #[serde(default)]
    blobs: BTreeMap<String, String>,
}

/// Publish `e` to the claims remote, if there is one. `Ok(false)`: the ref
/// is not what this clone last published (nothing, for a new entry).
fn publish(store: &Store, reg: &Registry, e: &mut Entry) -> Result<bool, String> {
    let (Some(name), Some(top)) = (store.remote.as_deref(), reg.toplevel.as_deref()) else {
        return Ok(true);
    };
    let mut blobs = BTreeMap::new();
    let mut tree = Vec::new();
    for (n, (path, text)) in e.files.iter().enumerate() {
        let base = Utf8Path::new(path).file_name().unwrap_or("document");
        let blob = if tree.iter().any(|(b, _)| b == base) {
            format!("{n}-{base}")
        } else {
            base.to_owned()
        };
        blobs.insert(path.clone(), blob.clone());
        tree.push((blob, text.clone()));
    }
    let mut entry = e.clone();
    entry.files.clear();
    entry.published = None;
    let message = serde_json::to_string(&Record { entry, blobs }).map_err(|x| x.to_string())?;
    let commit = remote::commit_with_blobs(top, &message, &tree)?;
    let expect = e
        .published
        .as_ref()
        .filter(|p| p.remote == name)
        .map(|p| p.commit.clone());
    let refname = format!("{}{}", remote::TICKET_PREFIX, e.id);
    match remote::push(top, name, &refname, Some(&commit), expect.as_deref()) {
        remote::Push::Won => {
            e.published = Some(Published {
                remote: name.to_owned(),
                commit,
            });
            forget_seen(reg);
            Ok(true)
        }
        remote::Push::Lost => Ok(false),
        remote::Push::Failed(why) | remote::Push::Unsettled(why) => Err(why),
    }
}

/// What was read of the remote is stale: read it again.
fn forget_seen(reg: &Registry) {
    if let Some(common) = &reg.common {
        let _ = std::fs::remove_file(common.join(SEEN_FILE));
    }
}

fn remote_holder(store: &Store, reg: &Registry, id: &str) -> Option<String> {
    forget_seen(reg);
    let e = store
        .remote_tickets()?
        .ok()?
        .into_iter()
        .find(|e| e.id == id)?;
    Some(format!(
        "\"{}\" by {} on {}",
        e.title,
        e.created_by,
        e.place().describe()
    ))
}

/// Drop an entry, and its ref on the remote if this clone published it.
fn retire(store: &Store, reg: &Registry, e: &Entry) {
    let _ = std::fs::remove_file(reg.entry_path(&e.id));
    let _ = std::fs::remove_file(reg.marker_path(&e.id, &e.uuid));
    if let (Some(p), Some(name), Some(top)) = (
        &e.published,
        store.remote.as_deref(),
        reg.toplevel.as_deref(),
    ) && p.remote == name
    {
        // With a lease: a ref another clone moved meanwhile is left alone.
        let refname = format!("{}{}", remote::TICKET_PREFIX, e.id);
        let _ = remote::push(top, name, &refname, None, Some(&p.commit));
        forget_seen(reg);
    }
}

/// After a journalled act on `t`: its entry, if it has one, says what its
/// files say now, or is dropped when it is done. Best effort: the entry is
/// an announcement, and the ticket's own files are the record.
pub(super) fn sync(store: &Store, t: &Ticket) {
    if t.elsewhere.is_some() {
        return;
    }
    let Some(reg) = store.registry() else {
        return;
    };
    if !reg.entry_path(t.id()).exists() {
        return;
    }
    let Some(mut e) = reg.read(t.id()).filter(|e| e.uuid == t.manifest.uuid) else {
        return;
    };
    let Ok(now) = store.load(&t.dir) else {
        return;
    };
    if now.is_done() {
        retire(store, &reg, &e);
    } else {
        let before = e.clone();
        refresh(store, &mut e, &now);
        if e != before {
            // A push that fails leaves the entry unpublished at its old
            // commit; the document still travels by git.
            let _ = publish(store, &reg, &mut e);
            reg.write(&e);
        }
    }
    sweep(store, &reg);
}

/// Drop every entry whose file is on the default branch: everyone has it
/// through git. Runs only when that branch's tip moved since the last sweep.
pub(super) fn sweep(store: &Store, reg: &Registry) {
    let Some(common) = reg.common.as_deref() else {
        return;
    };
    let Some((_, tip)) = super::live::default_branch(common, &store.root) else {
        return;
    };
    let mark = reg.dir.join(SWEPT);
    if std::fs::read_to_string(&mark).is_ok_and(|t| t.trim() == tip) {
        return;
    }
    for e in reg.entries() {
        if e.state == "creating" {
            continue;
        }
        let there = super::live::blob(&store.root, &tip, &format!("./{}", e.path));
        if let Some(Some(bytes)) = there
            && String::from_utf8_lossy(&bytes).contains(&e.uuid)
        {
            retire(store, reg, &e);
        }
    }
    if reg.dir.is_dir() {
        let _ = std::fs::write(&mark, format!("{tip}\n"));
    }
}

/// `war start <id>` (and a claim, `war done`, `war plan add`, `war plan
/// note`) on a ticket held elsewhere: write its document into this worktree
/// at the same path, so the act that follows finds it. Nothing happens for a
/// ticket this checkout holds, or a query that names none.
pub(super) fn bring_here(store: &Store, query: &str) -> Result<(), RepoError> {
    let Some(reg) = store.registry() else {
        return Ok(());
    };
    let held_here = |id: &str| {
        store.dir.join(format!("{id}.md")).exists()
            || store.dir.join(id).exists()
            || store.root.join(crate::subject::WORK_ROOT).join(id).exists()
    };
    // The usual case costs a stat: the query is the id of a ticket here.
    let named = query.trim().split('/').next().unwrap_or_default();
    if !named.is_empty() && held_here(named) {
        return Ok(());
    }
    let (ids, _) = reg.listing();
    if store.remote.is_none() && ids.iter().all(|id| held_here(id)) {
        return Ok(());
    }
    let (tickets, _) = store.load_all_shared()?;
    let Ok(Target::Ticket(n) | Target::Item(n, _)) = super::resolve(&tickets, query) else {
        return Ok(());
    };
    let t = &tickets[n];
    if t.elsewhere.is_none() {
        return Ok(());
    }
    let local = reg.read(t.id()).filter(|e| e.uuid == t.manifest.uuid);
    let announced = || {
        store
            .remote_tickets()
            .and_then(Result::ok)
            .and_then(|all| all.into_iter().find(|e| e.uuid == t.manifest.uuid))
    };
    let from_remote = local.is_none();
    let Some(e) = local.or_else(announced) else {
        return Ok(());
    };
    for (path, text) in &e.files {
        let to = store.root.join(path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(super::io(format!("could not create {parent}")))?;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&to)
        {
            Ok(mut f) => f
                .write_all(text.as_bytes())
                .map_err(super::io(format!("could not write {to}")))?,
            // Someone brought it here meanwhile: theirs stands.
            Err(x) if x.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not create {to}"),
                    source,
                });
            }
        }
    }
    if from_remote {
        // This clone's other worktrees see it too, and its closing here
        // retires the ref.
        let mut mine = e.clone();
        mine.remote = None;
        mine.root = reg.root.to_string();
        mine.below = reg.below.clone();
        mine.branch = reg.toplevel.as_deref().and_then(super::live::checked_out);
        let _ = reg.reserve(&mine);
    }
    Ok(())
}

/// The remote's announced tickets as a view sees them: `seen_claims`'s
/// pattern. One `ls-remote` per `ttl`; a commit already read is not read
/// again. A remote that could not be read is kept as such for as long.
fn seen_remote(
    root: &Utf8Path,
    common: &Utf8Path,
    name: &str,
    ttl: u64,
    now: u64,
) -> Result<Vec<Entry>, String> {
    #[derive(Serialize, Deserialize, Default)]
    struct Seen {
        remote: String,
        read_unix: u64,
        #[serde(default)]
        tickets: Vec<(String, Entry)>,
        #[serde(default)]
        unknown: Option<String>,
    }
    let file = common.join(SEEN_FILE);
    let was = std::fs::read(&file)
        .ok()
        .and_then(|b| serde_json::from_slice::<Seen>(&b).ok())
        .filter(|s| s.remote == name);
    let answer = |s: Seen| match s.unknown {
        Some(why) => Err(why),
        None => Ok(s.tickets.into_iter().map(|(_, e)| e).collect()),
    };
    if let Some(s) = was
        .as_ref()
        .filter(|s| now >= s.read_unix && now - s.read_unix < ttl)
    {
        return answer(Seen {
            remote: s.remote.clone(),
            read_unix: s.read_unix,
            tickets: s.tickets.clone(),
            unknown: s.unknown.clone(),
        });
    }
    let known: BTreeMap<String, Entry> = was
        .map(|s| s.tickets)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let read = || -> Result<Vec<(String, Entry)>, String> {
        let refs = remote::list_refs(root, name, remote::TICKET_PREFIX)?;
        let missing: Vec<&str> = refs
            .iter()
            .filter(|(commit, _)| !known.contains_key(commit))
            .map(|(_, r)| r.as_str())
            .collect();
        remote::fetch_refs(root, name, &missing)?;
        let mut out = Vec::new();
        for (commit, _) in &refs {
            if let Some(e) = known.get(commit) {
                out.push((commit.clone(), e.clone()));
                continue;
            }
            // A ref that is no announcement lists nothing.
            let Some(record) = remote::message_of(root, commit)?
                .and_then(|m| serde_json::from_str::<Record>(&m).ok())
            else {
                continue;
            };
            let mut e = record.entry;
            for (path, blob) in &record.blobs {
                e.files
                    .insert(path.clone(), remote::blob_of(root, commit, blob)?);
            }
            e.remote = Some(name.to_owned());
            e.published = Some(Published {
                remote: name.to_owned(),
                commit: commit.clone(),
            });
            out.push((commit.clone(), e));
        }
        Ok(out)
    };
    let read = read();
    let seen = Seen {
        remote: name.to_owned(),
        read_unix: now,
        unknown: read.as_ref().err().cloned(),
        // What was read before is kept beside an UNKNOWN only so its commits
        // need no second read; it is not answered.
        tickets: read.unwrap_or_else(|_| known.into_iter().collect()),
    };
    if let Ok(bytes) = serde_json::to_vec(&seen) {
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = crate::compile::atomic::write(&file, bytes);
    }
    answer(seen)
}

// ---- no duplicates ---------------------------------------------------------

/// Words that say nothing about what a piece of work is.
const STOP: [&str; 26] = [
    "a", "an", "and", "as", "at", "be", "by", "for", "from", "in", "into", "is", "it", "its", "of",
    "on", "or", "so", "that", "the", "this", "to", "when", "with", "we", "our",
];

/// A word without the endings that do not change what it names: `exports`,
/// `exporting`, `exported` and `export` are one word, as are `cache` and
/// `caching`.
fn stem(word: &str) -> String {
    let mut w = word;
    let n = w.chars().count();
    if n >= 6 && w.ends_with("ing") {
        w = &w[..w.len() - 3];
    } else if n >= 5
        && ["ed", "ses", "xes", "zes", "ches", "shes"]
            .iter()
            .any(|s| w.ends_with(s))
    {
        w = &w[..w.len() - 2];
    } else if n >= 4 && w.ends_with('s') && !w.ends_with("ss") {
        w = &w[..w.len() - 1];
    }
    if w.chars().count() >= 4 && w.ends_with('e') {
        w = &w[..w.len() - 1];
    }
    w.to_owned()
}

/// The words of a title that say what the work is, stemmed, each with how
/// often it occurs: "round 0 of worker 0" and "round 0 of worker 1" differ.
#[must_use]
pub fn content_words(title: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for w in super::words_of(title) {
        if !STOP.contains(&w.as_str()) {
            *out.entry(stem(&w)).or_insert(0) += 1;
        }
    }
    out
}

/// How two titles read as one piece of work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alike {
    /// The same words.
    Same,
    /// One title's words are all in the other's, covering this share of it.
    Close(f64),
}

/// Whether `a` and `b` read as one piece of work, by word overlap.
///
/// The same content words is [`Alike::Same`]. Otherwise the two are close
/// when one title's words are all among the other's and cover at least
/// [`CLOSE`] of them: a rewording adds or drops words. Two titles that each
/// hold a word the other lacks are distinct work however much else they
/// share ("Add CSV export" and "Add JSON export" differ in the one word
/// that matters), so a plain Jaccard score, which reads those as 0.5 and
/// their longer forms as 0.67 and more, is not used. A synonym swap
/// ("refuse" for "reject") is therefore not caught: nothing in the words
/// tells it from CSV for JSON.
#[must_use]
pub fn alike(a: &str, b: &str) -> Option<Alike> {
    let (x, y) = (content_words(a), content_words(b));
    if x.is_empty() || y.is_empty() {
        let plain = |s: &str| super::words_of(s).join(" ");
        return (plain(a) == plain(b) && !plain(a).is_empty()).then_some(Alike::Same);
    }
    if x == y {
        return Some(Alike::Same);
    }
    let size = |m: &BTreeMap<String, usize>| m.values().sum::<usize>();
    let (small, large) = if size(&x) <= size(&y) {
        (&x, &y)
    } else {
        (&y, &x)
    };
    let within = small
        .iter()
        .all(|(w, n)| large.get(w).is_some_and(|m| m >= n));
    #[allow(clippy::cast_precision_loss)]
    let share = size(small) as f64 / size(large) as f64;
    (within && share >= CLOSE).then_some(Alike::Close(share))
}

/// One creator per title at a time: a lock in the register named by a hash
/// of the title's words, taken as a claim is and released when dropped. Of
/// two agents creating one title at the same instant, the second waits, then
/// finds the first's ticket and is refused by [`duplicate`].
pub(super) struct TitleLock(Option<Utf8PathBuf>);

impl TitleLock {
    pub(super) fn take(store: &Store, title: &str) -> Self {
        let Some(reg) = store.registry() else {
            return Self(None);
        };
        let words: Vec<String> = content_words(title).into_keys().collect();
        let key = if words.is_empty() {
            super::words_of(title).join(" ")
        } else {
            words.join(" ")
        };
        let path = reg.dir.join(format!(
            ".title-{}.lock",
            &blake3::hash(key.as_bytes()).to_hex()[..16]
        ));
        let body = format!("{} {}\n", store.actor, std::process::id());
        let started = std::time::Instant::now();
        loop {
            match take_file(&path, body.as_bytes()) {
                Ok(true) => return Self(Some(path)),
                Ok(false) => {}
                // No register to lock in: the duplicate check still runs.
                Err(_) => return Self(None),
            }
            let age = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|m| m.elapsed().ok());
            if age.is_some_and(|a| a.as_secs() >= TITLE_LOCK_SECS) {
                let _ = std::fs::remove_file(&path);
                continue;
            }
            if started.elapsed().as_secs() >= TITLE_LOCK_SECS {
                return Self(None);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

impl Drop for TitleLock {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The refusal for a new ticket titled `title` that is, or reads as, an
/// open one: this checkout's, a registered one, or one another clone
/// announced. Tickets under two different parents are never compared: "Write
/// the tests" is a step of many plans.
pub(super) fn duplicate(
    store: &Store,
    title: &str,
    part_of: Option<&str>,
) -> Result<Option<Outcome>, RepoError> {
    struct Open {
        id: String,
        title: String,
        part_of: Option<String>,
        place: String,
    }
    let (tickets, _) = store.load_all_shared()?;
    let mut open: Vec<Open> = tickets
        .iter()
        .filter(|t| !t.is_done())
        .map(|t| Open {
            id: t.id().to_owned(),
            title: t.manifest.title.clone(),
            part_of: t.manifest.part_of.clone(),
            place: t.elsewhere.as_ref().map_or_else(
                || store.rel(&t.intent_path),
                |p| format!("{} ({})", p.describe(), store.rel(&t.intent_path)),
            ),
        })
        .collect();
    // What the register holds that is no listed ticket: one still being
    // created, or a work order `war plan new` made in another worktree.
    if let Some(reg) = store.registry() {
        for e in reg.entries() {
            if !tickets.iter().any(|t| t.id() == e.id) {
                open.push(Open {
                    place: format!("{} ({})", e.place().describe(), e.path),
                    id: e.id,
                    title: e.title,
                    part_of: e.part_of,
                });
            }
        }
    }
    let found = open
        .iter()
        .filter(|o| match (part_of, o.part_of.as_deref()) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        })
        .filter_map(|o| alike(title, &o.title).map(|how| (o, how)))
        .max_by(|a, b| {
            let rank = |h: &Alike| match h {
                Alike::Same => 2.0,
                Alike::Close(s) => *s,
            };
            rank(&a.1).total_cmp(&rank(&b.1))
        });
    let Some((o, how)) = found else {
        return Ok(None);
    };
    let holder = store
        .claims_seen()
        .unwrap_or_default()
        .values()
        .flatten()
        .find(|c| c.ticket == o.id)
        .map(|c| format!(", claimed by {}", c.actor))
        .unwrap_or_default();
    let reads = match how {
        Alike::Same => "is the same work as",
        Alike::Close(_) => "reads as the same work as",
    };
    let mut refusal = Outcome::refused(
        "ticket.duplicate",
        o.place.clone(),
        format!(
            "\"{title}\" {reads} {} (\"{}\"), open in {}{holder}. `war start {}` works on it; \
             `--force` creates this one anyway",
            o.id, o.title, o.place, o.id
        ),
    );
    refusal.result = serde_json::json!({
        "schema": "oh.war/ticket-duplicate/v1",
        "id": o.id,
        "title": o.title,
        "place": o.place,
    });
    Ok(Some(refusal))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table the threshold was tuned against. Each pair is two titles
    /// and whether the second is refused while the first is open.
    #[test]
    fn rewordings_match_and_distinct_work_does_not() {
        let table: &[(&str, &str, bool)] = &[
            // The same title, however it is cased or punctuated.
            ("Add CSV export", "Add CSV export", true),
            ("Add CSV export", "add csv export.", true),
            // Rewordings: words added, dropped, reordered or inflected.
            ("Add CSV export", "Add export to CSV", true),
            ("Add CSV export", "Add a CSV export", true),
            ("Add CSV export", "Adding CSV exports", true),
            ("Fix the login redirect bug", "Fix login redirect bug", true),
            ("Cache parsed tickets", "Caching of parsed tickets", true),
            (
                "Document the claims remote",
                "Document claims remote setup",
                true,
            ),
            (
                "Refuse duplicate tickets",
                "Refuse duplicate tickets at create time",
                true,
            ),
            ("Add CSV export", "Add CSV export to the reports page", true),
            // Distinct work that shares most of its words: one word differs
            // on each side, and that word is the work.
            ("Add CSV export", "Add JSON export", false),
            (
                "Add CSV export to the reports page",
                "Add JSON export to the reports page",
                false,
            ),
            ("Fix login timeout", "Fix login redirect", false),
            ("Add login tests", "Fix login tests", false),
            (
                "Plant 290 shared tickets",
                "Plant 291 shared tickets",
                false,
            ),
            ("Release alpha.7", "Release alpha.8", false),
            // A repeated word counts each time it occurs.
            ("worker-0 round 0", "worker-1 round 0", false),
            (
                "Migrate docs/tickets to docs/work",
                "Migrate docs/work state directories",
                false,
            ),
            // A title that only shares a little of a longer one.
            ("Add export", "Add CSV export to the reports page", false),
            (
                "Update docs",
                "Update docs for the claims remote and shared work",
                false,
            ),
            ("Refactor", "Refactor the parser", false),
            // Known miss: a synonym is a substitution like any other.
            (
                "Refuse duplicate tickets",
                "Reject duplicate tickets",
                false,
            ),
            ("Speed up create", "Make create faster", false),
        ];
        for (a, b, want) in table {
            assert_eq!(
                alike(a, b).is_some(),
                *want,
                "{a:?} vs {b:?}: {:?}",
                alike(a, b)
            );
            assert_eq!(alike(b, a).is_some(), *want, "{b:?} vs {a:?} (symmetry)");
        }
    }

    /// Plain Jaccard at 0.6, the brief's starting point, refuses distinct
    /// work once titles are long: the reason the rule is containment.
    #[test]
    fn plain_jaccard_would_refuse_the_longer_csv_and_json_pair() {
        let words = |t: &str| content_words(t).into_keys().collect::<BTreeSet<_>>();
        let (x, y) = (
            words("Add CSV export to the reports page"),
            words("Add JSON export to the reports page"),
        );
        #[allow(clippy::cast_precision_loss)]
        let jaccard = x.intersection(&y).count() as f64 / x.union(&y).count() as f64;
        assert!(jaccard >= CLOSE, "{jaccard}");
        assert_eq!(
            alike(
                "Add CSV export to the reports page",
                "Add JSON export to the reports page"
            ),
            None
        );
    }

    #[test]
    fn a_title_of_stop_words_only_matches_itself() {
        assert_eq!(alike("To be", "to be"), Some(Alike::Same));
        assert_eq!(alike("To be", "Or not"), None);
    }

    #[test]
    fn an_id_is_taken_once_and_released_only_by_its_own_uuid() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let reg = Registry {
            dir: root.join("register"),
            root: root.to_owned(),
            below: String::new(),
            toplevel: None,
            common: None,
        };
        let first = reg.stub("t-abcd", "uuid-1", "One", "a", "docs/tickets/t-abcd.md");
        let second = reg.stub("t-abcd", "uuid-2", "Two", "b", "docs/tickets/t-abcd.md");
        assert!(reg.reserve(&first).unwrap());
        assert!(!reg.reserve(&second).unwrap(), "the id is taken");
        assert_eq!(reg.read("t-abcd").unwrap().title, "One");
        let (ids, uuids) = reg.listing();
        assert_eq!(ids.into_iter().collect::<Vec<_>>(), ["t-abcd"]);
        assert_eq!(uuids["t-abcd"], "uuid-1");
        reg.release("t-abcd", "uuid-2");
        assert!(reg.read("t-abcd").is_some(), "not the loser's to release");
        reg.release("t-abcd", "uuid-1");
        assert!(reg.read("t-abcd").is_none());
        assert!(reg.reserve(&second).unwrap());
    }

    #[test]
    fn of_concurrent_reservations_of_one_id_exactly_one_wins() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap().to_owned();
        let won: usize = std::thread::scope(|s| {
            let handles: Vec<_> = (0..16)
                .map(|n| {
                    let root = root.clone();
                    s.spawn(move || {
                        let reg = Registry {
                            dir: root.join("register"),
                            root: root.clone(),
                            below: String::new(),
                            toplevel: None,
                            common: None,
                        };
                        let e = reg.stub("t-race", &format!("uuid-{n}"), "T", "a", "p.md");
                        usize::from(reg.reserve(&e).unwrap())
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).sum()
        });
        assert_eq!(won, 1);
    }
}
