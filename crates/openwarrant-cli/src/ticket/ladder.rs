// SPDX-License-Identifier: Apache-2.0
//! The tick ladder on tickets (OW-WAR-0148 M13): what each tick shows, what
//! backs it, what `war done --check` runs, and `war kpi run`.
//!
//! # A tick's level is what the file says, as far as a record backs it
//!
//! `war done` writes a claimed tick as it always has; `war done --check`
//! writes `[observed]` after the date and journals the receipt; an ingested
//! verification writes `[independent]`, a verified signature `[signed]`. A
//! reader believes the marker only as far as a record backs it:
//!
//! - `observed`: a `ticket.item_done` or `ticket.tick_raised` in the
//!   ticket's journal at that level, carrying the runs that passed;
//! - `independent`: a `ticket.tick_verified` whose verifier is not the
//!   tick's performer, over a response file whose bytes still digest as
//!   recorded;
//! - `signed`: a `ticket.tick_signed` whose response verifies as a human's
//!   signature over the item's current text ([`crate::authority_check`]).
//!
//! A marker nothing backs reads one level down, to the highest that is
//! backed, and says why. `claimed` needs no record: it is the floor. So a
//! claimed tick never reads as checked, verified or signed, however its line
//! was edited.
//!
//! # A command a Warrant declares is run as written
//!
//! A test or KPI is a shell command (`sh -c`), run from the repository root
//! through [`crate::gate_cmd::run_gate`], with its deadline and its
//! askability: a command that could not be asked, timed out, or (for a KPI)
//! printed no number is UNKNOWN, never a pass and never a fail. The Warrant's
//! author is the trust boundary, as a gate definition's is.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use openwarrant_core::ticket::Item;
use openwarrant_core::ticks::{self, Checks, Kpi, Level, MinimumSource};
use serde::Serialize;

use super::{Outcome, Store, Target, Ticket, resolve};
use crate::diagnostic::Diagnostic;
use crate::repo::{RepoError, Repository};

/// The journal events the ladder records in a ticket's journal.
pub mod event {
    /// `war add --test/--kpi/--milestone`.
    pub const PART_ADDED: &str = "ticket.part_added";
    /// A `war done --check` that did not tick: what ran and why.
    pub const CHECK_RUN: &str = "ticket.check_run";
    /// One KPI measurement: value, verdict, commit.
    pub const KPI_RUN: &str = "ticket.kpi_run";
    /// A done item's tick raised to a higher level.
    pub const TICK_RAISED: &str = "ticket.tick_raised";
    /// An independent verification ingested for an item.
    pub const TICK_VERIFIED: &str = "ticket.tick_verified";
    /// A human's signature over an item's tick, verified and recorded.
    pub const TICK_SIGNED: &str = "ticket.tick_signed";
}

/// Where `war done --check` and `war kpi run` leave each command's output,
/// under the repository root: disposable state, never committed.
pub const OUTPUT_DIR: &str = ".openwarrant/state/checks";

/// The ticket's optional parts: its checks atom parsed, empty when it has
/// none (every ticket written before M13).
#[must_use]
pub fn checks_of(t: &Ticket) -> Checks {
    crate::vfs::read_to_string(t.dir.join(ticks::CHECKS_FILE))
        .map(|text| ticks::parse(&text))
        .unwrap_or_default()
}

/// The least `item` (or, `None`, the whole ticket) must reach, and which
/// declaration says so.
#[must_use]
pub fn minimum(
    store: &Store,
    t: &Ticket,
    checks: &Checks,
    item: Option<&str>,
) -> (Level, MinimumSource) {
    let milestone = item.and_then(|i| checks.milestone(i)).map(|m| m.min);
    store
        .definition
        .ticks
        .minimum(t.manifest.kind.as_deref(), milestone)
}

/// The command that ticks an item at `level`, for a refusal to name.
#[must_use]
pub fn command_for(level: Level, target: &str) -> String {
    match level {
        Level::Claimed => format!("`war done {target}`"),
        Level::Observed => format!("`war done {target} --check`"),
        Level::Independent => format!(
            "an independent verification: `war verify {target}` writes the request, and \
             someone other than you answers it (`war verify {target} --response <file>`)"
        ),
        Level::Signed => format!("a human's sign-off: `war sign {target} --ssh-sign`"),
    }
}

// ---- what backs a tick -----------------------------------------------------------

/// An ingested verification, as the journal recorded it.
#[derive(Debug, Clone, Serialize)]
pub struct Verified {
    pub verifier: String,
    pub performer: String,
    pub disposition: String,
    pub response: String,
    pub response_sha256: String,
    pub at: String,
}

/// A recorded sign-off.
#[derive(Debug, Clone, Serialize)]
pub struct SignedOff {
    pub signer: String,
    pub response: String,
    pub digest: String,
    pub at: String,
}

/// One KPI measurement from the journal.
#[derive(Debug, Clone, Serialize)]
pub struct KpiRun {
    pub kpi: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    pub value: Option<f64>,
    pub verdict: String,
    pub at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

/// What a ticket's journal records for the ladder, read once.
#[derive(Debug, Clone, Default)]
pub struct Backing {
    observed: BTreeSet<String>,
    verified: BTreeMap<String, Verified>,
    refuted: BTreeMap<String, Verified>,
    signed: BTreeMap<String, SignedOff>,
    pub kpi_runs: Vec<KpiRun>,
}

/// The journal's ladder records for `t`. Cheap for a ticket that has none:
/// the file is scanned for the event names before it is parsed.
#[must_use]
pub fn backing(t: &Ticket) -> Backing {
    let mut out = Backing::default();
    let Ok(text) = crate::vfs::read_to_string(t.dir.join(crate::journal_cmd::FILE)) else {
        return out;
    };
    if !(text.contains("level") || text.contains("ticket.kpi_run") || text.contains("ticket.tick_"))
    {
        return out;
    }
    let Ok(journal) = crate::journal_cmd::parse(&text) else {
        return out;
    };
    for e in &journal.events {
        let Ok(p) = serde_json::from_str::<serde_json::Value>(&e.payload) else {
            continue;
        };
        let s = |k: &str| p.get(k).and_then(|v| v.as_str()).map(str::to_owned);
        match e.event_type.as_str() {
            super::event::ITEM_DONE if s("level").as_deref() == Some("observed") => {
                if let Some(i) = s("item") {
                    out.observed.insert(i);
                }
            }
            event::TICK_RAISED if s("to").as_deref() == Some("observed") => {
                if let Some(i) = s("item") {
                    out.observed.insert(i);
                }
            }
            event::TICK_VERIFIED => {
                let (Some(item), Some(verifier), Some(performer)) =
                    (s("item"), s("verifier"), s("performer"))
                else {
                    continue;
                };
                let v = Verified {
                    verifier,
                    performer,
                    disposition: s("disposition").unwrap_or_default(),
                    response: s("response").unwrap_or_default(),
                    response_sha256: s("response_sha256").unwrap_or_default(),
                    at: e.occurred_at.clone(),
                };
                if v.disposition == "established" {
                    out.refuted.remove(&item);
                    out.verified.insert(item, v);
                } else {
                    out.refuted.insert(item, v);
                }
            }
            event::TICK_SIGNED => {
                let (Some(item), Some(signer), Some(response), Some(digest)) =
                    (s("item"), s("signer"), s("response"), s("digest"))
                else {
                    continue;
                };
                out.signed.insert(
                    item,
                    SignedOff {
                        signer,
                        response,
                        digest,
                        at: e.occurred_at.clone(),
                    },
                );
            }
            event::KPI_RUN => {
                let Some(kpi) = s("kpi") else { continue };
                out.kpi_runs.push(KpiRun {
                    kpi,
                    item: s("item"),
                    value: p.get("value").and_then(serde_json::Value::as_f64),
                    verdict: s("verdict").unwrap_or_default(),
                    at: e.occurred_at.clone(),
                    commit: s("commit"),
                });
            }
            _ => {}
        }
    }
    out
}

/// What the statement a sign-off binds digests to: the ticket, the item and
/// its text. A sign-off is over these words; edit them and it no longer
/// covers the item.
#[must_use]
pub fn statement_digest(t: &Ticket, item: &str, text: &str) -> String {
    let statement = format!(
        "oh.war/tick-signoff/v1\n{}\n{}/{item}\n{}\n",
        t.manifest.uuid,
        t.id(),
        ticks_text(text)
    );
    format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(statement.as_bytes())
    )
}

fn ticks_text(text: &str) -> String {
    openwarrant_core::ticket::one_line(text)
}

/// The name a sign-off response for an item is filed under: the claim
/// lock's spelling, one flat name per item.
#[must_use]
pub fn signoff_subject(t: &Ticket, item: &str) -> String {
    format!("{}--{item}", t.id())
}

/// Opens the repository once, only when a signature has to be checked.
pub struct Reader<'a> {
    store: &'a Store,
    repo: std::cell::OnceCell<Option<Repository>>,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(store: &'a Store) -> Self {
        Self {
            store,
            repo: std::cell::OnceCell::new(),
        }
    }

    fn repo(&self) -> Option<&Repository> {
        self.repo
            .get_or_init(|| Repository::discover(Some(self.store.root.clone())).ok())
            .as_ref()
    }
}

/// One tick as every view shows it.
#[derive(Debug, Clone, Serialize)]
pub struct TickView {
    /// How it was earned, as far as a record backs it.
    pub level: Level,
    /// The least it must show.
    pub minimum: Level,
    pub meets_minimum: bool,
    /// The marker the line carries, when it claims more than a record backs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<Level>,
    /// Why the written marker was not believed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unbacked: Option<String>,
    /// Who verified or signed it, for those levels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// A verification on record that refuted it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refuted_by: Option<String>,
}

impl TickView {
    /// The marker a view puts beside the tick: the level, and the minimum
    /// when the tick is below it. `(claimed)` is never confused with a
    /// checked level.
    #[must_use]
    pub fn marker(&self) -> String {
        if self.meets_minimum {
            self.level.marker().to_owned()
        } else {
            format!("({}; needs {})", self.level.as_str(), self.minimum.as_str())
        }
    }
}

/// How many ticks of a ticket stand at each level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct TickCounts {
    pub claimed: usize,
    pub observed: usize,
    pub independent: usize,
    pub signed: usize,
    /// Ticks below the minimum their item must reach.
    pub below_minimum: usize,
}

impl TickCounts {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.claimed + self.observed + self.independent + self.signed
    }

    fn add(&mut self, v: &TickView) {
        match v.level {
            Level::Claimed => self.claimed += 1,
            Level::Observed => self.observed += 1,
            Level::Independent => self.independent += 1,
            Level::Signed => self.signed += 1,
        }
        if !v.meets_minimum {
            self.below_minimum += 1;
        }
    }

    /// `2 claimed, 1 observed (1 below its minimum)`: each level that holds
    /// a tick, in ladder order.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        for (n, l) in [
            (self.claimed, Level::Claimed),
            (self.observed, Level::Observed),
            (self.independent, Level::Independent),
            (self.signed, Level::Signed),
        ] {
            if n > 0 {
                parts.push(format!("{n} {}", l.as_str()));
            }
        }
        let mut out = parts.join(", ");
        if self.below_minimum > 0 {
            out.push_str(&format!(" ({} below its minimum)", self.below_minimum));
        }
        out
    }
}

/// Every tick of a ticket, by item key (the id, or `line N` for a line a
/// person ticked without one), its counts, and its parts.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TickReport {
    pub items: BTreeMap<String, TickView>,
    pub counts: TickCounts,
    #[serde(skip)]
    pub checks: Checks,
    #[serde(skip)]
    pub backing: Backing,
}

/// The key a tick is listed under.
#[must_use]
pub fn item_key(item: &Item) -> String {
    item.id
        .clone()
        .unwrap_or_else(|| format!("line {}", item.line + 1))
}

/// Every tick of `t` as a view shows it.
#[must_use]
pub fn ticks_of(reader: &Reader<'_>, t: &Ticket) -> TickReport {
    let checks = checks_of(t);
    // Only a marker above claimed needs a record behind it; a ticket of
    // claimed ticks reads no journal here.
    let marked = t.checklist.items.iter().any(|i| {
        i.done
            && i.done_on
                .as_deref()
                .and_then(|d| ticks::split_level(d).1)
                .is_some()
    });
    let backing = if marked {
        backing(t)
    } else {
        Backing::default()
    };
    let mut report = TickReport {
        checks,
        backing,
        ..TickReport::default()
    };
    for item in t.checklist.items.iter().filter(|i| i.done) {
        let view = view_of(reader, t, item, &report.checks, &report.backing);
        report.counts.add(&view);
        report.items.insert(item_key(item), view);
    }
    report
}

/// One done item's tick.
#[must_use]
pub fn view_of(
    reader: &Reader<'_>,
    t: &Ticket,
    item: &Item,
    checks: &Checks,
    backing: &Backing,
) -> TickView {
    let (minimum, _) = minimum(reader.store, t, checks, item.id.as_deref());
    let written = item
        .done_on
        .as_deref()
        .and_then(|d| ticks::split_level(d).1)
        .unwrap_or(Level::Claimed);
    let mut level = Level::Claimed;
    let mut by = None;
    let mut doubts: Vec<String> = Vec::new();
    if let Some(id) = &item.id {
        for candidate in [Level::Signed, Level::Independent, Level::Observed] {
            if candidate > written {
                continue;
            }
            match backed(reader, t, item, id, candidate, backing) {
                Ok(who) => {
                    level = candidate;
                    by = who;
                    break;
                }
                // Why the written level is not believed; the levels below it
                // are only where the reading stopped.
                Err(why) if candidate == written => doubts.push(why),
                Err(_) => {}
            }
        }
    } else if written > Level::Claimed {
        doubts.push("the line has no id, so no record can name it".to_owned());
    }
    let refuted_by = item
        .id
        .as_ref()
        .and_then(|id| backing.refuted.get(id))
        .map(|v| v.verifier.clone());
    TickView {
        level,
        minimum,
        meets_minimum: level >= minimum,
        written: (written > level).then_some(written),
        unbacked: (written > level).then(|| doubts.join("; ")),
        by,
        refuted_by,
    }
}

/// Whether a record backs `item` at `level`: `Ok(who)` (the verifier or
/// signer), or why not.
fn backed(
    reader: &Reader<'_>,
    t: &Ticket,
    item: &Item,
    id: &str,
    level: Level,
    backing: &Backing,
) -> Result<Option<String>, String> {
    match level {
        Level::Claimed => Ok(None),
        Level::Observed => {
            if backing.observed.contains(id) {
                Ok(None)
            } else {
                Err(format!(
                    "[observed]: no passing check of {id} is in the journal"
                ))
            }
        }
        Level::Independent => {
            let Some(v) = backing.verified.get(id) else {
                return Err(format!(
                    "[independent]: no verification of {id} is in the journal"
                ));
            };
            let performer = item.done_by.as_deref().unwrap_or(v.performer.as_str());
            if v.verifier == performer {
                return Err(format!(
                    "[independent]: the verification on record is by {performer}, who ticked it"
                ));
            }
            Ok(Some(v.verifier.clone()))
        }
        Level::Signed => {
            let Some(s) = backing.signed.get(id) else {
                return Err(format!("[signed]: no sign-off of {id} is in the journal"));
            };
            if s.digest != statement_digest(t, id, &item.text) {
                return Err(format!(
                    "[signed]: {id}'s text changed since {} signed it off",
                    s.signer
                ));
            }
            let Some(repo) = reader.repo() else {
                return Err(
                    "[signed]: UNKNOWN, the repository could not be opened to check the \
                            signature"
                        .to_owned(),
                );
            };
            let verdict = crate::authority_check::verify(
                repo,
                crate::authority_check::Act::SignOff,
                &signoff_subject(t, id),
                &s.signer,
                Some(&s.digest),
            );
            if verdict.is_signed() {
                Ok(Some(s.signer.clone()))
            } else {
                Err(format!("[signed]: {}", verdict.why()))
            }
        }
    }
}

// ---- running a Warrant's checks ------------------------------------------------

/// One test or KPI run.
#[derive(Debug, Clone, Serialize)]
pub struct Run {
    /// `test` or `kpi`.
    pub kind: &'static str,
    pub name: String,
    pub cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// `pass`, `fail`, `unknown`, or (a KPI that decides nothing) `recorded`.
    pub verdict: &'static str,
    /// Whether it decides the tick.
    pub gating: bool,
    /// The run as the gate runner saw it: `completed`, `timeout`, `not_run`...
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<&'static str>,
    pub duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_sha256: Option<String>,
    /// Where its output is, relative to the root.
    pub output: String,
}

/// What one round of checks found.
#[derive(Debug, Clone, Serialize)]
pub struct CheckRound {
    pub run: String,
    pub runs: Vec<Run>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

impl CheckRound {
    /// The runs that decide the tick.
    pub fn gating(&self) -> impl Iterator<Item = &Run> {
        self.runs.iter().filter(|r| r.gating)
    }

    #[must_use]
    pub fn failed(&self) -> Vec<&Run> {
        self.gating().filter(|r| r.verdict == "fail").collect()
    }

    #[must_use]
    pub fn unknown(&self) -> Vec<&Run> {
        self.gating().filter(|r| r.verdict == "unknown").collect()
    }

    /// `unit (`cargo test`) failed; p95 printed 130, target 120`.
    #[must_use]
    pub fn describe(runs: &[&Run]) -> String {
        runs.iter()
            .map(|r| {
                let what = match (r.kind, r.verdict, r.value) {
                    ("kpi", "fail", Some(v)) => format!(
                        "printed {}, target {} {}",
                        ticks::number(v),
                        if r.direction == Some("max") {
                            ">="
                        } else {
                            "<="
                        },
                        r.target.map(ticks::number).unwrap_or_default()
                    ),
                    ("kpi", "unknown", None) if r.status == "completed" => {
                        "printed no number".to_owned()
                    }
                    (_, "fail", _) => "exited non-zero".to_owned(),
                    _ => format!(
                        "could not be established ({}{})",
                        r.status,
                        r.reason
                            .as_ref()
                            .map(|x| format!(", {x}"))
                            .unwrap_or_default()
                    ),
                };
                format!("{} (`{}`) {what}; output: {}", r.name, r.cmd, r.output)
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// HEAD's commit, when the root is a git checkout.
#[must_use]
pub fn head_commit(root: &camino::Utf8Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn definition(t: &Ticket, name: &str, cmd: &str) -> openwarrant_core::gate::GateDefinition {
    openwarrant_core::gate::GateDefinition {
        gate_id: format!("{}.{name}", t.id()),
        version: "check".to_owned(),
        digest: String::new(),
        lifecycle: openwarrant_core::gate::GateLifecycle::Active,
        implementation_ref: String::new(),
        input_kinds: Vec::new(),
        output_schema_ref: String::new(),
        fault_model: Vec::new(),
        known_blind_spots: Vec::new(),
        qualification: None,
        provenance: openwarrant_core::gate::GateProvenance::default(),
        argv: vec!["sh".to_owned(), "-c".to_owned(), cmd.to_owned()],
        mutating: false,
        timeout_secs: None,
    }
}

/// Run one command through the gate runner: (the run, its stdout).
fn run_one(
    repo: &Repository,
    t: &Ticket,
    name: &str,
    cmd: &str,
) -> (
    openwarrant_core::GateRun,
    String,
    String,
    u128,
    Option<String>,
) {
    let dir = repo.root.join(OUTPUT_DIR).join(t.id());
    let def = definition(t, name, cmd);
    let started = Instant::now();
    let run = crate::gate_cmd::run_gate(&def, repo, &dir);
    let ms = started.elapsed().as_millis();
    let slug = def.key().replace(['/', '@', '.'], "_");
    let out_path = dir.join(format!("{slug}.stdout.txt"));
    let stdout = crate::vfs::read(&out_path).unwrap_or_default();
    let digest = (!stdout.is_empty() || out_path.is_file())
        .then(|| format!("sha256:{}", openwarrant_compiler::sha256_hex(&stdout)));
    (
        run,
        String::from_utf8_lossy(&stdout).into_owned(),
        repo.relative(&out_path),
        ms,
        digest,
    )
}

fn status_of(run: &openwarrant_core::GateRun) -> (String, Option<String>) {
    let word = |v: serde_json::Value| v.as_str().map(str::to_owned).unwrap_or_default();
    (
        word(serde_json::to_value(run.execution_status).unwrap_or_default()),
        run.reason_code
            .map(|r| word(serde_json::to_value(r).unwrap_or_default())),
    )
}

/// Run the tests and KPIs that apply to `item` (`None`: the whole Warrant):
/// every one, in file order, each KPI run journalled as it lands.
pub fn run_checks(
    store: &Store,
    repo: &Repository,
    t: &Ticket,
    item: Option<&str>,
    checks: &Checks,
) -> Result<CheckRound, RepoError> {
    let (tests, kpis) = checks.for_item(item);
    let round = CheckRound {
        run: openwarrant_core::WarUuid::mint().to_string(),
        runs: Vec::new(),
        commit: head_commit(&repo.root),
    };
    let mut round = round;
    for test in tests {
        let (run, _, output, ms, digest) = run_one(repo, t, &test.name, &test.cmd);
        // M11: a long run keeps this agent's leases fresh, one touch per lock.
        store.renew_held(None);
        let (status, reason) = status_of(&run);
        let verdict = match run.verdict {
            openwarrant_core::Verdict::Pass if run.satisfies_required_pass() => "pass",
            openwarrant_core::Verdict::Fail => "fail",
            _ => "unknown",
        };
        round.runs.push(Run {
            kind: "test",
            name: test.name.clone(),
            cmd: test.cmd.clone(),
            item: test.item.clone(),
            verdict,
            gating: true,
            status,
            reason,
            value: None,
            target: None,
            direction: None,
            mode: None,
            duration_ms: ms,
            stdout_sha256: digest,
            output,
        });
    }
    for kpi in kpis {
        let r = measure(store, repo, t, kpi, round.commit.as_deref(), &round.run)?;
        store.renew_held(None);
        round.runs.push(r);
    }
    Ok(round)
}

/// Run one KPI, judge it, and journal the run.
fn measure(
    store: &Store,
    repo: &Repository,
    t: &Ticket,
    kpi: &Kpi,
    commit: Option<&str>,
    round: &str,
) -> Result<Run, RepoError> {
    let (run, stdout, output, ms, digest) = run_one(repo, t, &kpi.name, &kpi.cmd);
    let (status, reason) = status_of(&run);
    // A command that did not complete with exit 0 measured nothing: UNKNOWN,
    // never a value. One that did, and printed no number, is UNKNOWN too.
    let value = (run.verdict == openwarrant_core::Verdict::Pass && run.satisfies_required_pass())
        .then(|| ticks::parse_number(&stdout))
        .flatten();
    let verdict = match value {
        None => "unknown",
        Some(v) => kpi.judge(v).as_str(),
    };
    let reason = match (value, run.verdict) {
        (None, openwarrant_core::Verdict::Pass) => Some("no-number".to_owned()),
        (None, openwarrant_core::Verdict::Fail) => Some("exit-non-zero".to_owned()),
        _ => reason,
    };
    let r = Run {
        kind: "kpi",
        name: kpi.name.clone(),
        cmd: kpi.cmd.clone(),
        item: kpi.item.clone(),
        verdict,
        gating: kpi.gates(),
        status,
        reason: reason.clone(),
        value,
        target: kpi.target,
        direction: Some(kpi.direction.as_str()),
        mode: Some(kpi.mode.as_str()),
        duration_ms: ms,
        stdout_sha256: digest,
        output,
    };
    let mut payload = serde_json::json!({
        "kpi": kpi.name,
        "value": value,
        "verdict": verdict,
        "direction": kpi.direction.as_str(),
        "mode": kpi.mode.as_str(),
        "commit": commit,
        "run": format!("{round}/{}", kpi.name),
    });
    if let Some(i) = &kpi.item {
        payload["item"] = serde_json::json!(i);
    }
    if let Some(target) = kpi.target {
        payload["target"] = serde_json::json!(target);
    }
    if let Some(why) = reason {
        payload["reason"] = serde_json::json!(why);
    }
    store.journal(t, event::KPI_RUN, &payload)?;
    Ok(r)
}

// ---- war kpi run -----------------------------------------------------------------

/// A KPI's standing over its journalled runs.
#[derive(Debug, Clone, Serialize)]
pub struct KpiStanding {
    pub name: String,
    pub cmd: String,
    pub direction: &'static str,
    pub mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
    /// The best value measured, in the KPI's direction (`best` and
    /// `optimise` modes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best: Option<f64>,
    /// The latest run's value, `None` when it printed no number.
    pub latest: Option<f64>,
    /// The latest run's verdict.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_verdict: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_at: Option<String>,
    pub runs: usize,
}

/// Every KPI of `checks` with its best, latest and target from `runs`.
#[must_use]
pub fn standings(checks: &Checks, runs: &[KpiRun]) -> Vec<KpiStanding> {
    checks
        .kpis
        .iter()
        .map(|k| {
            let mine: Vec<&KpiRun> = runs
                .iter()
                .filter(|r| r.kpi == k.name && r.item == k.item)
                .collect();
            let values: Vec<f64> = mine.iter().filter_map(|r| r.value).collect();
            let last = mine.last();
            KpiStanding {
                name: k.name.clone(),
                cmd: k.cmd.clone(),
                direction: k.direction.as_str(),
                mode: k.mode.as_str(),
                item: k.item.clone(),
                target: k.target,
                best: k.best(&values),
                latest: last.and_then(|r| r.value),
                latest_verdict: last.map(|r| r.verdict.clone()),
                latest_at: last.map(|r| r.at.clone()),
                runs: mine.len(),
            }
        })
        .collect()
}

/// `p95_ms: latest 95 (pass), best 95, target <= 120, 3 runs`.
#[must_use]
pub fn describe_standing(s: &KpiStanding) -> String {
    let mut out = format!("{}: ", s.name);
    match (s.latest, s.latest_verdict.as_deref()) {
        (Some(v), Some(verdict)) => {
            out.push_str(&format!("latest {} ({verdict})", ticks::number(v)));
        }
        (None, Some(_)) => out.push_str("latest UNKNOWN (printed no number)"),
        _ => out.push_str("not run yet"),
    }
    if let Some(b) = s.best {
        out.push_str(&format!(", best {}", ticks::number(b)));
    }
    match s.target {
        Some(t) => out.push_str(&format!(
            ", target {} {}",
            if s.direction == "max" { ">=" } else { "<=" },
            ticks::number(t)
        )),
        None => out.push_str(&format!(", no target ({})", s.direction)),
    }
    out.push_str(&format!(
        ", {} run{}",
        s.runs,
        if s.runs == 1 { "" } else { "s" }
    ));
    if s.mode == "optimise" {
        out.push_str(", a signal only");
    }
    if let Some(i) = &s.item {
        out.push_str(&format!(" (for {i})"));
    }
    out
}

/// `war kpi run <ticket|item>`: run every KPI that applies, journal each
/// run, and say each one's latest, best and target. Ticks nothing.
pub fn kpi_run(store: &Store, query: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, item) = match resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => (n, None),
        Ok(Target::Item(n, i)) => (n, Some(i)),
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let checks = checks_of(t);
    let chosen: Vec<&Kpi> = match &item {
        None => checks.kpis.iter().collect(),
        Some(i) => checks.for_item(Some(i)).1,
    };
    let what = item
        .as_ref()
        .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
    if chosen.is_empty() {
        return Ok(Outcome::refused(
            "kpi.none",
            store.rel(&t.dir.join(ticks::CHECKS_FILE)),
            format!(
                "{what} has no KPI to run. Add one: `war add {} --kpi <name> --cmd \"<prints a \
                 number>\" --direction max` (or min), with `--target <N>` to pass or fail \
                 against",
                t.id()
            ),
        ));
    }
    let repo = Repository::discover(Some(store.root.clone()))?;
    let round = openwarrant_core::WarUuid::mint().to_string();
    let commit = head_commit(&repo.root);
    let mut runs = Vec::new();
    for k in &chosen {
        runs.push(measure(store, &repo, t, k, commit.as_deref(), &round)?);
    }
    let after = backing(t);
    let all = standings(&checks, &after.kpi_runs);
    let mut human = format!("{what}: {} KPI run(s)", runs.len());
    if let Some(c) = &commit {
        human.push_str(&format!(" at {}", &c[..c.len().min(12)]));
    }
    let mut report = crate::diagnostic::Report::default();
    for r in &runs {
        if let Some(s) = all.iter().find(|s| s.name == r.name && s.item == r.item) {
            human.push_str(&format!("\n  {}", describe_standing(s)));
        }
        match r.verdict {
            "unknown" => report.push(Diagnostic::unknown(
                "kpi.unknown",
                r.output.clone(),
                format!(
                    "{}: UNKNOWN, {}; recorded as no value, never a pass or a fail",
                    r.name,
                    match r.reason.as_deref() {
                        Some("no-number") => "the command printed no number".to_owned(),
                        Some("exit-non-zero") => "the command exited non-zero".to_owned(),
                        other => format!(
                            "the command could not be run ({}{})",
                            r.status,
                            other.map(|o| format!(", {o}")).unwrap_or_default()
                        ),
                    }
                ),
            )),
            // A measurement short of its target is said, and the run is
            // journalled like any other: `war kpi run` measures, it ticks
            // nothing, so a miss is not a refusal of the command.
            "fail" => report.push(Diagnostic::warn(
                "kpi.target-missed",
                r.output.clone(),
                CheckRound::describe(&[r]),
            )),
            _ => {}
        }
    }
    Ok(Outcome {
        report,
        human,
        result: serde_json::json!({
            "schema": "oh.war/kpi-run/v1",
            "target": what,
            "commit": commit,
            "runs": runs,
            "kpis": all,
        }),
    })
}

// ---- war done --check --------------------------------------------------------------

/// The refusal of a tick at `reach` when the item must reach more: by rule
/// `ticket.tick-below-minimum`, naming the minimum, what sets it, and the
/// command that reaches it. `None` when `reach` is enough.
#[must_use]
pub fn below_minimum(
    store: &Store,
    t: &Ticket,
    checks: &Checks,
    item: Option<&str>,
    what: &str,
    reach: Level,
) -> Option<Outcome> {
    let (minimum, source) = minimum(store, t, checks, item);
    if reach >= minimum {
        return None;
    }
    Some(Outcome::refused(
        "ticket.tick-below-minimum",
        store.rel(&t.dir.join(ticks::CHECKS_FILE)),
        format!(
            "{what} ticks at {} or above ({}), and this tick would be {}. Nothing was ticked; it \
             ticks with {}",
            minimum.as_str(),
            source.describe(),
            reach.as_str(),
            command_for(minimum, what)
        ),
    ))
}

/// A warning for each KPI that decides nothing and measured no number: said,
/// and never holding the tick.
#[must_use]
pub fn signal_warnings(round: &CheckRound) -> Vec<Diagnostic> {
    round
        .runs
        .iter()
        .filter(|r| !r.gating && r.verdict == "unknown")
        .map(|r| {
            Diagnostic::warn(
                "ticket.check-signal-unknown",
                r.output.clone(),
                format!(
                    "{} (a KPI that decides nothing) measured no number; recorded as UNKNOWN, \
                     and the tick is not held by it",
                    r.name
                ),
            )
        })
        .collect()
}

/// The names of the runs that passed, for the line `war done` prints.
#[must_use]
pub fn passed_names(round: &CheckRound) -> String {
    let names: Vec<String> = round
        .gating()
        .filter(|r| r.verdict == "pass")
        .map(|r| match r.value {
            Some(v) => format!("{} ({})", r.name, ticks::number(v)),
            None => r.name.clone(),
        })
        .collect();
    names.join(", ")
}

/// Run what decides `item`'s tick: `Ok(Ok(round))` when every test and
/// every targeted KPI passed; `Ok(Err(refusal))` when one failed (an error,
/// `ticket.check-failed`, naming each), could not be established (UNKNOWN,
/// `ticket.check-unknown`), or nothing could decide it
/// (`ticket.check-nothing`). A round that did not tick is journalled as
/// `ticket.check_run`.
pub fn check_for_tick(
    store: &Store,
    t: &Ticket,
    item: Option<&str>,
    checks: &Checks,
    what: &str,
) -> Result<Result<CheckRound, Box<Outcome>>, RepoError> {
    let (tests, kpis) = checks.for_item(item);
    let deciding = tests.len() + kpis.iter().filter(|k| k.gates()).count();
    if deciding == 0 {
        return Ok(Err(Box::new(Outcome::refused(
            "ticket.check-nothing",
            store.rel(&t.dir.join(ticks::CHECKS_FILE)),
            format!(
                "{what} has no test and no KPI with a target, so `--check` has nothing that \
                 could fail. Add one (`war add {what} --test \"<command>\"`), or tick it as \
                 claimed (`war done {what}`)"
            ),
        ))));
    }
    let repo = Repository::discover(Some(store.root.clone()))?;
    let round = run_checks(store, &repo, t, item, checks)?;
    let failed = round.failed();
    let unknown = round.unknown();
    let mut report = crate::diagnostic::Report::default();
    for w in signal_warnings(&round) {
        report.push(w);
    }
    if failed.is_empty() && unknown.is_empty() {
        return Ok(Ok(round));
    }
    let outcome = if failed.is_empty() { "unknown" } else { "fail" };
    let mut payload = serde_json::json!({
        "target": what,
        "outcome": outcome,
        "runs": round.runs,
        "commit": round.commit,
        "run": round.run,
    });
    if let Some(i) = item {
        payload["item"] = serde_json::json!(i);
    }
    store.journal(t, event::CHECK_RUN, &payload)?;
    let message = if failed.is_empty() {
        format!(
            "{what} was not ticked: {}. UNKNOWN is not a pass and not a fail; nothing was \
             ticked, and the claim is still yours",
            CheckRound::describe(&unknown)
        )
    } else {
        format!(
            "{what} was not ticked at observed: {}. Nothing was ticked, and the claim is still \
             yours",
            CheckRound::describe(&failed)
        )
    };
    let file = store.rel(&t.dir.join(ticks::CHECKS_FILE));
    report.push(if failed.is_empty() {
        Diagnostic::unknown("ticket.check-unknown", file, message.clone())
    } else {
        Diagnostic::error("ticket.check-failed", file, message.clone())
    });
    Ok(Err(Box::new(Outcome {
        report,
        human: message,
        result: serde_json::json!({
            "schema": "oh.war/ticket-check/v1",
            "target": what,
            "outcome": outcome,
            "checks": round,
        }),
    })))
}

/// `war done <item> --check` on an item already done: run its checks, and
/// raise its tick to `observed` when they pass. The line keeps who ticked
/// it, when, and its note; only the marker after the date changes.
pub fn raise_observed(
    store: &Store,
    t: &Ticket,
    id: &str,
    what: &str,
    if_rev: Option<&str>,
) -> Result<Outcome, RepoError> {
    let reader = Reader::new(store);
    let checks = checks_of(t);
    let Some(item) = t.item(id) else {
        return Ok(Outcome::refused(
            "ticket.unknown",
            String::new(),
            format!("{what} is not in the checklist"),
        ));
    };
    let before = view_of(&reader, t, item, &checks, &backing(t));
    if before.level >= Level::Observed {
        return Ok(Outcome::ok(
            format!(
                "{what} is already ticked at {}; nothing to raise",
                before.level.as_str()
            ),
            serde_json::json!({"schema": "oh.war/ticket-done/v1", "target": what,
                "already_done": true, "level": before.level}),
        ));
    }
    let round = match check_for_tick(store, t, Some(id), &checks, what)? {
        Ok(r) => r,
        Err(refusal) => return Ok(*refusal),
    };
    let actor = store.actor.clone();
    let today = super::date_of(super::now_secs());
    let written = super::rewrite(&t.checklist_path, |text| {
        let c = openwarrant_core::ticket::parse(text);
        let Some(it) = c.item(id).filter(|i| i.done) else {
            return Err(Box::new(Outcome::refused(
                "ticket.unknown",
                String::new(),
                format!("{what} is no longer a done line of the checklist; nothing was written"),
            )));
        };
        if let Some(given) = if_rev {
            let now = super::item_revision(text, it);
            if !super::same_revision(given, &now) {
                return Err(Box::new(super::stale_revision(
                    store,
                    t,
                    what,
                    &t.checklist_path,
                    given,
                    &now,
                )));
            }
        }
        let date = it
            .done_on
            .as_deref()
            .map_or_else(|| today.clone(), |d| ticks::split_level(d).0.to_owned());
        let by = it.done_by.clone().unwrap_or_else(|| actor.clone());
        let line = it
            .ticked(
                &by,
                &ticks::with_level(&date, Level::Observed),
                it.note.as_deref(),
            )
            .render();
        Ok((
            openwarrant_core::ticket::replace_line(text, it.line, &line),
            (),
        ))
    })?;
    if let Err(refusal) = written {
        return Ok(*refusal);
    }
    store.journal(
        t,
        event::TICK_RAISED,
        &serde_json::json!({
            "item": id,
            "target": what,
            "from": before.level,
            "to": "observed",
            "runs": round.runs,
            "commit": round.commit,
            "run": round.run,
        }),
    )?;
    Ok(Outcome::ok(
        format!(
            "raised {what} from {} to observed: {} passed",
            before.level.as_str(),
            passed_names(&round)
        ),
        serde_json::json!({
            "schema": "oh.war/ticket-done/v1",
            "ticket": t.id(),
            "item": id,
            "raised_from": before.level,
            "level": "observed",
            "checks": round,
        }),
    ))
}

// ---- the tracker: every view's ticks, across tickets ------------------------------

/// One milestone as the progress tracker shows it.
#[derive(Debug, Clone, Serialize)]
pub struct MilestoneView {
    /// `t-x/i-y`.
    pub target: String,
    pub ticket_title: String,
    pub text: String,
    pub minimum: Level,
    /// `None` while the item is open.
    pub level: Option<Level>,
    /// Ticked at or above its minimum: the marker on the tracker is set.
    pub met: bool,
    /// What the tracker prints beside it: `(observed)`, `(claimed; needs
    /// observed)`, or `open`.
    pub marker: String,
}

/// Every ticket's ticks and milestones, for the corpus views (`war status`,
/// the roadmap, the board, the web page).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Tracker {
    pub counts: TickCounts,
    /// Tickets with at least one tick.
    pub tickets: usize,
    pub milestones: Vec<MilestoneView>,
    /// Each ticket whose ticks are not all claimed, or that has a tick below
    /// its minimum or a milestone: `(id, title, counts)`.
    pub notable: Vec<(String, String, TickCounts)>,
}

/// Read every ticket once and gather its ticks and milestones.
pub fn tracker(store: &Store) -> Result<Tracker, RepoError> {
    let (tickets, _) = store.load_all()?;
    let reader = Reader::new(store);
    let mut out = Tracker::default();
    for t in &tickets {
        let report = ticks_of(&reader, t);
        let c = report.counts;
        if c.total() > 0 {
            out.tickets += 1;
            out.counts.claimed += c.claimed;
            out.counts.observed += c.observed;
            out.counts.independent += c.independent;
            out.counts.signed += c.signed;
            out.counts.below_minimum += c.below_minimum;
        }
        for m in &report.checks.milestones {
            let Some(item) = t.item(&m.item) else {
                continue;
            };
            let (minimum, _) = minimum(store, t, &report.checks, Some(&m.item));
            let view = report.items.get(&m.item);
            out.milestones.push(MilestoneView {
                target: format!("{}/{}", t.id(), m.item),
                ticket_title: t.manifest.title.clone(),
                text: item.text.clone(),
                minimum,
                level: view.map(|v| v.level),
                met: view.is_some_and(|v| v.meets_minimum),
                marker: view.map_or_else(|| "open".to_owned(), TickView::marker),
            });
        }
        if c.observed + c.independent + c.signed + c.below_minimum > 0
            || !report.checks.milestones.is_empty()
        {
            out.notable
                .push((t.id().to_owned(), t.manifest.title.clone(), c));
        }
    }
    Ok(out)
}

impl Tracker {
    /// The block `war status` (and the board) print after the corpus: the
    /// ladder's totals, each notable ticket, each milestone. `None` when
    /// nothing is ticked and there is no milestone.
    #[must_use]
    pub fn render(&self) -> Option<String> {
        if self.counts.total() == 0 && self.milestones.is_empty() {
            return None;
        }
        let mut blocks = vec!["## Ticks (tickets)\n".to_owned()];
        if self.counts.total() > 0 {
            blocks.push(format!(
                "{} across {} ticket(s). claimed < observed < independent < signed: a claimed \
                 tick is the performer's word, nothing checked it.\n",
                self.counts.describe(),
                self.tickets
            ));
        }
        if !self.notable.is_empty() {
            let mut b = String::new();
            for (id, title, c) in &self.notable {
                let ticks = if c.total() > 0 {
                    c.describe()
                } else {
                    "nothing ticked yet".to_owned()
                };
                b.push_str(&format!("- {id}  {title}: {ticks}\n"));
            }
            blocks.push(b);
        }
        if !self.milestones.is_empty() {
            let mut b = String::from("Milestones:\n\n");
            for m in &self.milestones {
                b.push_str(&format!("{}\n", milestone_line(m)));
            }
            blocks.push(b);
        }
        let s = blocks.join("\n");
        Some(s)
    }
}

/// `- [x] (observed) t-x/i-y  Ships — min observed, met` or `- [ ] ...`.
#[must_use]
pub fn milestone_line(m: &MilestoneView) -> String {
    format!(
        "- [{}] {}{}  {} ({}) — min {}{}",
        if m.met { 'x' } else { ' ' },
        if m.level.is_some() {
            format!("{} ", m.marker)
        } else {
            String::new()
        },
        m.target,
        m.text,
        m.ticket_title,
        m.minimum.as_str(),
        match (m.level, m.met) {
            (Some(_), true) => ", met".to_owned(),
            (Some(_), false) => format!(", not met: {}", command_for(m.minimum, &m.target)),
            (None, _) => String::new(),
        }
    )
}

/// `war check`'s rules for a ticket's optional parts: each malformed or
/// repeated line of its checks atom (`checks.malformed`,
/// `checks.duplicate`), and a part or milestone naming an item the
/// checklist does not have (`checks.item-unknown`). Structure only, like
/// every ticket rule: nothing here asks for a check to have run. Returns
/// the number of errors pushed.
pub fn check_parts(store: &Store, t: &Ticket, report: &mut crate::diagnostic::Report) -> usize {
    let path = t.dir.join(ticks::CHECKS_FILE);
    let Ok(text) = crate::vfs::read_to_string(&path) else {
        return 0;
    };
    let checks = ticks::parse(&text);
    let file = store.rel(&path);
    let mut errors = 0;
    for f in &checks.faults {
        errors += 1;
        report.push(Diagnostic::error(
            f.rule,
            format!("{file}:{}", f.line),
            format!("{}: line {}: {}", t.id(), f.line, f.message),
        ));
    }
    let named: Vec<(&str, &str, usize)> = checks
        .tests
        .iter()
        .filter_map(|x| x.item.as_deref().map(|i| ("test", i, x.line)))
        .chain(
            checks
                .kpis
                .iter()
                .filter_map(|x| x.item.as_deref().map(|i| ("KPI", i, x.line))),
        )
        .chain(
            checks
                .milestones
                .iter()
                .map(|m| ("milestone", m.item.as_str(), m.line)),
        )
        .collect();
    for (what, item, line) in named {
        if t.item(item).is_none() {
            errors += 1;
            report.push(Diagnostic::error(
                "checks.item-unknown",
                format!("{file}:{}", line + 1),
                format!(
                    "{}: line {}: the {what} names {item}, and the checklist has no such item",
                    t.id(),
                    line + 1
                ),
            ));
        }
    }
    errors
}
