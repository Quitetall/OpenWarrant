// SPDX-License-Identifier: Apache-2.0
//! One compiled corpus, built once per process (OW-WAR-0148 M1).
//!
//! Before this, every corpus-level builder loaded the corpus for itself:
//! `war admin compile` built the corpus status about six times, `resolve::assess`
//! ran once in status and again in the frontier for every Warrant, and the
//! web UI, TUI and MCP server each rebuilt all of it per view. A [`Corpus`]
//! holds what they share:
//!
//! - every Warrant directory, in [`Repository::warrant_dirs`] order, with its
//!   [`Loaded`] (or the error loading it gave);
//! - each Warrant's `lower` result and contract digest, computed once;
//! - `relations::currencies` over the corpus;
//! - one `resolve::assess` per Warrant;
//! - the corpus status, the sign queue, the frontier, the ownership index;
//! - the roadmap record, the tickets, the ADRs and the SAS revisions;
//! - the record atoms and the relations documents author (OW-WAR-0148 M3);
//! - the folders `[[adapters]]` reads in place (OW-WAR-0148 M10).
//!
//! Every derived value is computed on first use and kept for the life of the
//! corpus: a command that never asks for the frontier never pays for it, and
//! one that asks twice pays once. Nothing here is a new reading of a record.
//! Each value is exactly what the plain builder computes from the same files;
//! the plain entry points (`status::build`, `sign::pending`, …) build a
//! corpus and delegate, so the two cannot disagree.
//!
//! A corpus is a snapshot. It is right for the life of one read-only command,
//! or for one watch fingerprint in a long-running client (the web UI, TUI and
//! MCP server rebuild it when the fingerprint moves). A command that writes
//! records and then reads them again must build a new one.
//!
//! Errors are kept as their message and returned as [`RepoError::Message`]
//! with the same text: a consumer that propagated a load error before
//! propagates the same words now, and one that skipped it still skips it.
//! The exception is an observation that could not be made
//! ([`RepoError::ObservationUnavailable`]), which comes back as itself so it
//! still reads UNKNOWN rather than as an error.

use std::sync::OnceLock;

use camino::{Utf8Path, Utf8PathBuf};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{AdrCorpus, Loaded, RepoError, Repository};
use crate::resolve::Assessment;

fn kept<T>(
    cell: &OnceLock<Result<T, Kept>>,
    f: impl FnOnce() -> Result<T, RepoError>,
) -> Result<&T, RepoError> {
    cell.get_or_init(|| f().map_err(Kept::of))
        .as_ref()
        .map_err(Kept::error)
}

/// An error as the corpus keeps it. An observation that could not be made
/// stays one (the caller reports it UNKNOWN, exit 2, as an uncached read
/// would); every other error is kept as its message.
#[derive(Debug, Clone)]
pub(crate) enum Kept {
    Unavailable { rule: &'static str, message: String },
    Message(String),
}

impl Kept {
    fn of(error: RepoError) -> Self {
        match error {
            RepoError::ObservationUnavailable { rule, message } => {
                Self::Unavailable { rule, message }
            }
            other => Self::Message(other.to_string()),
        }
    }

    fn error(&self) -> RepoError {
        match self {
            Self::Unavailable { rule, message } => RepoError::ObservationUnavailable {
                rule,
                message: message.clone(),
            },
            Self::Message(message) => RepoError::Message(message.clone()),
        }
    }
}

/// Every readable ticket, and a diagnostic per unreadable one.
pub type Tickets = (Vec<crate::ticket::Ticket>, Vec<Diagnostic>);

/// One Warrant directory of the corpus.
#[derive(Debug)]
pub struct Entry {
    pub dir: Utf8PathBuf,
    loaded: Result<Loaded, Kept>,
    ir: OnceLock<Result<openwarrant_compiler::WarIr, Kept>>,
    contract: OnceLock<Option<String>>,
    assessment: OnceLock<Result<Assessment, Kept>>,
}

impl Entry {
    /// The Warrant as `Repository::load_warrant` read it, or its error.
    pub fn loaded(&self) -> Result<&Loaded, RepoError> {
        self.loaded.as_ref().map_err(Kept::error)
    }

    /// The Warrant, when it loaded.
    #[must_use]
    pub fn ok(&self) -> Option<&Loaded> {
        self.loaded.as_ref().ok()
    }

    /// The directory's name: the alias every builder keys on.
    #[must_use]
    pub fn name(&self) -> &str {
        self.dir.file_name().unwrap_or_default()
    }

    /// `lower(basis, validated)`, once. `Err` names why it did not compile,
    /// or that the manifest did not validate.
    pub fn ir(&self) -> Result<&openwarrant_compiler::WarIr, RepoError> {
        kept(&self.ir, || {
            let one = self.loaded()?;
            let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
                return Err(RepoError::Message(format!(
                    "{}: the manifest did not validate",
                    self.name()
                )));
            };
            openwarrant_compiler::lower(basis, validated)
                .map_err(|e| RepoError::Message(e.to_string()))
        })
    }

    /// The contract digest the Warrant compiles to NOW — `None` when it will
    /// not compile. Not the digest an authorization recorded: that is the
    /// authorized revision's (`authorization.toml`), and the two differ
    /// exactly when the contract moved after signing.
    #[must_use]
    pub fn contract_digest(&self) -> Option<&str> {
        self.contract
            .get_or_init(|| self.ir().ok().and_then(|ir| ir.contract_digest().ok()))
            .as_deref()
    }

    /// `resolve::assess` for this Warrant, once.
    pub fn assessment(&self, repo: &Repository) -> Result<&Assessment, RepoError> {
        kept(&self.assessment, || {
            let one = self.loaded()?;
            let evidence = crate::evidence::load(repo, &one.dir)?;
            crate::resolve::assess_with_digest(
                repo,
                one,
                &evidence,
                self.contract_digest().map(str::to_owned),
            )
        })
    }
}

/// The corpus: every Warrant and the corpus-level values built from them.
pub struct Corpus {
    repo: Repository,
    entries: OnceLock<Result<Vec<Entry>, Kept>>,
    currencies: OnceLock<crate::relations::Currencies>,
    status: OnceLock<Result<openwarrant_core::status::CorpusStatus, Kept>>,
    pending: OnceLock<Result<Vec<crate::sign::Pending>, Kept>>,
    frontier: OnceLock<Result<(Report, crate::frontier::Frontier), Kept>>,
    ownership: OnceLock<Result<crate::ownership::Ownership, Kept>>,
    roadmap: OnceLock<Result<Option<crate::roadmap_cmd::Loaded>, Kept>>,
    tickets: OnceLock<Result<Tickets, Kept>>,
    adrs: OnceLock<Result<AdrCorpus, Kept>>,
    sas_revisions: OnceLock<Result<Vec<openwarrant_core::SasRevision>, Kept>>,
    records: OnceLock<crate::records::Records>,
    adapters: OnceLock<crate::interop::adapters::Adapted>,
}

impl std::fmt::Debug for Corpus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Corpus")
            .field("root", &self.repo.root)
            .field(
                "warrants",
                &self.entries.get().map(|e| e.as_ref().map(Vec::len)),
            )
            .finish_non_exhaustive()
    }
}

impl Corpus {
    /// A corpus over `repo`. Nothing is read until something is asked for:
    /// the Warrant directories are listed and loaded on the first
    /// [`Self::entries`], so a builder that listed them only after another
    /// check still fails at that check first.
    #[must_use]
    pub fn new(repo: &Repository) -> Self {
        Self {
            repo: repo.clone(),
            entries: OnceLock::new(),
            currencies: OnceLock::new(),
            status: OnceLock::new(),
            pending: OnceLock::new(),
            frontier: OnceLock::new(),
            ownership: OnceLock::new(),
            roadmap: OnceLock::new(),
            tickets: OnceLock::new(),
            adrs: OnceLock::new(),
            sas_revisions: OnceLock::new(),
            records: OnceLock::new(),
            adapters: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn repo(&self) -> &Repository {
        &self.repo
    }

    /// Every Warrant directory, in `warrant_dirs` order (sorted by path),
    /// each loaded once. Fails exactly where `Repository::warrant_dirs`
    /// fails; a Warrant that does not load is kept with its error, for each
    /// consumer to treat as it always has.
    pub fn entries(&self) -> Result<&[Entry], RepoError> {
        kept(&self.entries, || {
            Ok(self
                .repo
                .warrant_dirs()?
                .into_iter()
                .map(|dir| Entry {
                    loaded: self.repo.load_warrant(&dir).map_err(Kept::of),
                    dir,
                    ir: OnceLock::new(),
                    contract: OnceLock::new(),
                    assessment: OnceLock::new(),
                })
                .collect())
        })
        .map(Vec::as_slice)
    }

    /// The entry for the directory named `alias`, if the corpus lists one.
    #[must_use]
    pub fn entry(&self, alias: &str) -> Option<&Entry> {
        self.entries().ok()?.iter().find(|e| e.name() == alias)
    }

    /// The entry for `dir`, if the corpus lists it.
    #[must_use]
    pub fn entry_at(&self, dir: &Utf8Path) -> Option<&Entry> {
        self.entries().ok()?.iter().find(|e| e.dir == dir)
    }

    /// Every Warrant that loaded, in directory order — what the
    /// filter-on-error builders (`relations::currencies`, `children_of`)
    /// read. Empty when the directories cannot be listed.
    #[must_use]
    pub fn loaded(&self) -> Vec<Loaded> {
        self.entries()
            .map(|es| es.iter().filter_map(|e| e.ok().cloned()).collect())
            .unwrap_or_default()
    }

    /// Every Warrant, failing on the first that did not load — what
    /// `status::build` and `war check` did with `?`.
    pub fn loaded_all(&self) -> Result<Vec<&Loaded>, RepoError> {
        self.entries()?.iter().map(Entry::loaded).collect()
    }

    /// OW-ADR-0022 currency, derived once over every Warrant that loaded.
    pub fn currencies(&self) -> &crate::relations::Currencies {
        self.currencies
            .get_or_init(|| crate::relations::currencies(&self.loaded()))
    }

    /// The corpus status (`status::build`), once.
    pub fn status(&self) -> Result<&openwarrant_core::status::CorpusStatus, RepoError> {
        kept(&self.status, || crate::status::build_with(self))
    }

    /// Everything awaiting a signature (`sign::pending`), once.
    pub fn pending(&self) -> Result<&[crate::sign::Pending], RepoError> {
        kept(&self.pending, || crate::sign::pending_with(self)).map(Vec::as_slice)
    }

    /// Every unresolved Warrant's frontier (`frontier::run(repo, None)`), once.
    pub fn frontier(&self) -> Result<&(Report, crate::frontier::Frontier), RepoError> {
        kept(&self.frontier, || crate::frontier::run_with(self, None))
    }

    /// OW-ADR-0021's ownership index over this corpus's currencies, once.
    pub fn ownership(&self) -> Result<&crate::ownership::Ownership, RepoError> {
        kept(&self.ownership, || {
            crate::ownership::Ownership::index_with(&self.repo, self.currencies())
        })
    }

    /// The roadmap record (`roadmap_cmd::load`), once. `Ok(None)` for a
    /// program without one.
    pub fn roadmap(&self) -> Result<Option<&crate::roadmap_cmd::Loaded>, RepoError> {
        kept(&self.roadmap, || {
            crate::roadmap_cmd::load(&self.repo).map_err(|e| RepoError::Message(e.to_string()))
        })
        .map(Option::as_ref)
    }

    /// Every readable ticket and a diagnostic per unreadable one
    /// (`ticket::Store::load_all`), once.
    pub fn tickets(&self) -> Result<&Tickets, RepoError> {
        kept(&self.tickets, || {
            crate::ticket::Store::open(&self.repo, None)?.load_all()
        })
    }

    /// The ADR corpus, once.
    pub fn adrs(&self) -> Result<&AdrCorpus, RepoError> {
        kept(&self.adrs, || self.repo.load_adrs())
    }

    /// The record atoms and every authored relation (OW-WAR-0148 M3), once.
    /// Infallible: what could not be read is a fault inside, by rule.
    pub fn records(&self) -> &crate::records::Records {
        self.records.get_or_init(|| crate::records::load(self))
    }

    /// The folders `[[adapters]]` reads in place (OW-WAR-0148 M10), once.
    /// Infallible: what could not be read is a fault inside, by rule.
    pub fn adapters(&self) -> &crate::interop::adapters::Adapted {
        self.adapters
            .get_or_init(|| crate::interop::adapters::load(&self.repo))
    }

    /// The recorded SAS revisions, once.
    pub fn sas_revisions(&self) -> Result<&[openwarrant_core::SasRevision], RepoError> {
        kept(&self.sas_revisions, || self.repo.load_sas_revisions()).map(Vec::as_slice)
    }
}

/// The corpus a long-running client (the MCP server) reads for `repo` now:
/// the one held for this process while [`working_tree_fingerprint`] has not
/// moved, else a new one, held in its place. With no fingerprint (not a git
/// checkout) a new one every time, as before there was a corpus.
#[must_use]
pub fn held(repo: &Repository) -> std::sync::Arc<Corpus> {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    type Held = Mutex<BTreeMap<Utf8PathBuf, (u64, Arc<Corpus>)>>;
    static HELD: OnceLock<Held> = OnceLock::new();
    let Some(fp) = working_tree_fingerprint(repo) else {
        return Arc::new(Corpus::new(repo));
    };
    let held = HELD.get_or_init(Default::default);
    if let Ok(m) = held.lock()
        && let Some((f, c)) = m.get(&repo.root)
        && *f == fp
    {
        return Arc::clone(c);
    }
    let fresh = Arc::new(Corpus::new(repo));
    if let Ok(mut m) = held.lock() {
        m.insert(repo.root.clone(), (fp, Arc::clone(&fresh)));
    }
    fresh
}

/// The key a long-running client (the MCP server) reuses a corpus under:
/// `None` when it cannot be formed, and the corpus is then rebuilt per
/// request, as before.
///
/// It covers every input a corpus reads. The record trees the web UI's
/// watch covers (`docs/warrants`, `docs/authority`, the SAS revisions, the
/// roadmap, the tickets) by path, length and mtime; the configuration; and
/// the working tree as git sees it — `HEAD`, and the path, length and mtime
/// of every changed or untracked file — because a deliverable's bytes and a
/// receipt's reuse (`evidence::reuse_of`) are read from outside the record
/// trees. An edit anywhere a corpus reads moves the key.
#[must_use]
pub fn working_tree_fingerprint(repo: &Repository) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut dirs = crate::watch::watched_dirs(repo);
    dirs.push(crate::roadmap_cmd::dir(repo));
    dirs.push(repo.root.join(crate::records::DIR));
    dirs.extend(crate::ticket::watched(repo));
    dirs.extend(crate::interop::adapters::watched(repo));
    crate::watch::fingerprint(&dirs).hash(&mut h);
    let meta = |p: &Utf8Path| {
        std::fs::metadata(p).ok().map(|m| {
            (
                m.len(),
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos()),
            )
        })
    };
    meta(&repo.root.join(crate::init::CONFIG_FILE)).hash(&mut h);
    let git = |args: &[&str]| -> Option<Vec<u8>> {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(repo.root.as_str())
            .args(args)
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    };
    git(&["rev-parse", "--verify", "-q", "HEAD"])?.hash(&mut h);
    let status = git(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])?;
    status.hash(&mut h);
    for entry in status.split(|b| *b == 0).filter(|e| e.len() > 3) {
        let path = String::from_utf8_lossy(&entry[3..]);
        meta(&repo.root.join(path.as_ref())).hash(&mut h);
    }
    Some(h.finish())
}
