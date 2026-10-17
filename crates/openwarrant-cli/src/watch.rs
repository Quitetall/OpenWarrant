// SPDX-License-Identifier: Apache-2.0
//! `war view watch` — tell the human when an act is waiting for them.
//!
//! The pending set is `sign::pending` (§27.2's human acts: authorize,
//! resolve, accept, correct). This command polls the record trees every
//! `--interval` milliseconds (default 500), recomputes the set when a file's
//! size or modification time changed, and prints the difference: what
//! appeared, what was signed away. `--notify-send` also raises a desktop
//! notification through `notify-send` when it exists. `--once` prints the set
//! and exits, for scripts and tests.
//!
//! A poller, not an inotify client, on purpose: the three trees are small,
//! a 500 ms scan of a few hundred files is nothing, and it adds no crate.
//! It never reads outside the record trees and never writes.

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime};

use camino::Utf8Path;
use serde::Serialize;

use crate::repo::{RepoError, Repository};
use crate::sign;

pub const SCHEMA: &str = "oh.war/watch/v1";

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub schema: String,
    /// One line per pending act, as `war sign --list` prints them.
    pub pending: Vec<String>,
    pub count: usize,
    /// Open questions, blocking first (OW-WAR-0069): the other thing a human
    /// is the only one who can clear.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<String>,
}

/// The trees whose change can change the pending set. `pub(crate)`: the app
/// polls the same fingerprint (OW-WAR-0112), one poller and not two.
pub(crate) fn watched_dirs(repo: &Repository) -> Vec<camino::Utf8PathBuf> {
    vec![
        repo.root.join(&repo.config.paths.warrants),
        repo.root.join("docs/authority"),
        repo.root.join(&repo.config.paths.sas).join("revisions"),
    ]
}

/// A cheap fingerprint of the trees: every file's path, length and mtime,
/// skipping `generated/` (projections change without the pending set).
pub(crate) fn fingerprint(dirs: &[camino::Utf8PathBuf]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    fn walk(dir: &Utf8Path, h: &mut std::collections::hash_map::DefaultHasher) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = rd.filter_map(Result::ok).collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for e in entries {
            let path = e.path();
            let name = e.file_name();
            if name == "generated" {
                continue;
            }
            if path.is_dir() {
                if let Ok(p) = camino::Utf8PathBuf::from_path_buf(path) {
                    walk(&p, h);
                }
                continue;
            }
            if let Ok(meta) = e.metadata() {
                path.hash(h);
                meta.len().hash(h);
                meta.modified()
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .hash(h);
            }
        }
    }
    for d in dirs {
        walk(d, &mut h);
    }
    h.finish()
}

pub fn snapshot(repo: &Repository) -> Result<Snapshot, RepoError> {
    let pending: Vec<String> = sign::pending(repo)?.iter().map(sign::line).collect();
    // An open question costs the owner a sentence and an agent its whole
    // stage, so it waits in the same place a signature does (OW-WAR-0069).
    // An incomplete store cannot become an apparently complete signing queue.
    // `war questions` retains the individual diagnostics and readable records.
    let questions: Vec<String> = crate::questions::complete_list(repo, true).map(|l| {
        l.questions
            .iter()
            .map(|q| {
                format!(
                    "{} {}  {}{}",
                    q.warrant,
                    q.id,
                    if q.blocking { "BLOCKING  " } else { "" },
                    q.question
                )
            })
            .collect()
    })?;
    Ok(Snapshot {
        schema: SCHEMA.to_owned(),
        count: pending.len() + questions.len(),
        pending,
        questions,
    })
}

pub fn render(s: &Snapshot) -> String {
    if s.pending.is_empty() && s.questions.is_empty() {
        return "nothing awaits a human\n".to_owned();
    }
    let mut out = format!("{} thing(s) await a human:\n", s.count);
    for l in &s.pending {
        out.push_str("  ");
        out.push_str(l);
        out.push('\n');
    }
    for q in &s.questions {
        out.push_str("  question  ");
        out.push_str(q);
        out.push('\n');
    }
    out
}

/// A pipe is block-buffered; a watcher whose lines arrive at exit is useless.
/// A stdout nobody reads any more (a closed pipe) ends the watch rather than
/// leaving a loop that hashes for nobody.
fn flush() -> Result<(), RepoError> {
    use std::io::Write;
    std::io::stdout().flush().map_err(|source| RepoError::Io {
        context: "stdout is gone; stopping the watch".to_owned(),
        source,
    })
}

fn notify_send(title: &str, body: &str) {
    let _ = std::process::Command::new("notify-send")
        .arg(title)
        .arg(body)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// `--checks`: the watcher as a FORMAT-1.0 §12 evaluator. Each poll fires
/// `commit` for every commit HEAD moved through since the last one it saw
/// (the first poll: HEAD itself), and `schedule:<cron>` once in each minute
/// a declared cron names. Each firing is the one runner's
/// ([`crate::ticket::checks::fire`]), so a firing another evaluator already
/// recorded runs nothing again.
#[derive(Debug, Default)]
pub struct Evaluator {
    head: Option<String>,
    minute: Option<u64>,
}

impl Evaluator {
    /// One poll: what it fired, one line each.
    pub fn poll(&mut self, repo: &Repository) -> Result<Vec<String>, RepoError> {
        use crate::ticket::checks;
        let store = crate::ticket::Store::open(repo, None)?;
        let mut lines = Vec::new();
        fn say(
            lines: &mut Vec<String>,
            trigger: &str,
            subject: &str,
            f: &checks::Firing,
            r: &crate::diagnostic::Report,
        ) {
            for run in f.runs.iter().filter(|x| x.recorded) {
                lines.push(format!(
                    "{trigger} {subject}: {} {} ({})",
                    run.check, run.verdict, run.run
                ));
            }
            for e in &f.evaluation.effects {
                lines.push(format!(
                    "{trigger} {subject}: {} {} by {}",
                    e.target, e.effect, e.check
                ));
            }
            for d in &r.diagnostics {
                lines.push(format!("{trigger} {subject}: {} ({})", d.message, d.rule));
            }
        }
        let head = crate::ticket::ladder::head_commit(&repo.root);
        if let Some(h) = head.as_ref().filter(|h| self.head.as_ref() != Some(*h)) {
            let commits = match &self.head {
                Some(old) => rev_list(&repo.root, old, h),
                None => vec![h.clone()],
            };
            for sha in commits {
                let Some(changed) = checks::changed_paths(&repo.root, &sha) else {
                    lines.push(format!(
                        "commit {sha}: UNKNOWN, git could not say what it changed"
                    ));
                    continue;
                };
                let sel = checks::Select {
                    trigger: "commit",
                    subject: &sha,
                    changed: Some(changed),
                    work: Vec::new(),
                    check: None,
                };
                let (f, r) = checks::fire(&store, &sel)?;
                say(&mut lines, "commit", &sha, &f, &r);
            }
            self.head = head;
        }
        let now = crate::ticket::now_secs();
        let minute = now / 60;
        if self.minute != Some(minute) {
            self.minute = Some(minute);
            let subject = crate::gate_cmd::receipt::rfc3339_from_secs(minute * 60);
            for trigger in checks::due_schedules(&store, minute * 60)? {
                let sel = checks::Select {
                    trigger: &trigger,
                    subject: &subject,
                    changed: None,
                    work: Vec::new(),
                    check: None,
                };
                let (f, r) = checks::fire(&store, &sel)?;
                say(&mut lines, &trigger, &subject, &f, &r);
            }
        }
        Ok(lines)
    }
}

/// The commits after `old` up to `new`, oldest first; `[new]` when git
/// cannot list them (a rewritten history).
fn rev_list(root: &Utf8Path, old: &str, new: &str) -> Vec<String> {
    let out = std::process::Command::new("git")
        .args(["rev-list", "--reverse", &format!("{old}..{new}")])
        .current_dir(root)
        .stderr(std::process::Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let list: Vec<String> = String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_owned)
                .collect();
            if list.is_empty() {
                vec![new.to_owned()]
            } else {
                list
            }
        }
        _ => vec![new.to_owned()],
    }
}

/// Watch until interrupted. `max_ticks` bounds the loop for tests. With
/// `checks`, each poll is also an [`Evaluator`] poll.
pub fn run(
    repo: &Repository,
    interval_ms: u64,
    desktop: bool,
    max_ticks: Option<u64>,
    checks: bool,
) -> Result<(), RepoError> {
    let mut evaluator = checks.then(Evaluator::default);
    let evaluate = |e: &mut Option<Evaluator>| -> Result<(), RepoError> {
        if let Some(ev) = e.as_mut() {
            match ev.poll(repo) {
                Ok(lines) => {
                    for l in lines {
                        println!("check  {l}");
                    }
                    flush()?;
                }
                Err(err) => eprintln!("watch: checks: {err}"),
            }
        }
        Ok(())
    };
    let dirs = watched_dirs(repo);
    // Fingerprint BEFORE the snapshot: computing the pending set builds every
    // request and takes a moment, and a change that lands meanwhile must be
    // seen on the next tick, not folded silently into the first set.
    let mut fp = fingerprint(&dirs);
    let mut last = snapshot(repo)?;
    print!("{}", render(&last));
    flush()?;
    if desktop && !last.pending.is_empty() {
        notify_send("OpenWarrant", &render(&last));
    }
    evaluate(&mut evaluator)?;
    let interval = Duration::from_millis(interval_ms.max(50));
    let mut ticks = 0u64;
    loop {
        std::thread::sleep(interval);
        ticks = ticks.saturating_add(1);
        if max_ticks.is_some_and(|m| ticks >= m) {
            return Ok(());
        }
        evaluate(&mut evaluator)?;
        let now_fp = fingerprint(&dirs);
        if now_fp == fp {
            continue;
        }
        fp = now_fp;
        // A register that cannot be read is said loudly, once per change,
        // and the loop goes on: the operator is exactly who should see it.
        let now = match snapshot(repo) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("watch: {e}");
                continue;
            }
        };
        let before: BTreeSet<&String> = last.pending.iter().collect();
        let after: BTreeSet<&String> = now.pending.iter().collect();
        let appeared: Vec<&&String> = after.difference(&before).collect();
        let gone: Vec<&&String> = before.difference(&after).collect();
        if appeared.is_empty() && gone.is_empty() {
            last = now;
            continue;
        }
        let mut msg = String::new();
        for a in &appeared {
            msg.push_str(&format!("+ {a}\n"));
        }
        for g in &gone {
            msg.push_str(&format!("- {g}\n"));
        }
        print!("{msg}");
        flush()?;
        if desktop && !appeared.is_empty() {
            notify_send(
                &format!("OpenWarrant: {} act(s) await you", now.count),
                &appeared
                    .iter()
                    .map(|a| a.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        last = now;
    }
}
