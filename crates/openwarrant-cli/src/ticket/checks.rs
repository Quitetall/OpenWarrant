// SPDX-License-Identifier: Apache-2.0
//! Checks and triggers (FORMAT-1.0 §12) over Format 1.0 tickets.
//!
//! One runner, three halves kept apart:
//!
//! - [`run_check`] runs one `check` or `kpi` record (`sh -c` in the
//!   repository root, or `gate://<id>@<version>` through the gate runner),
//!   and records what happened: `trigger.fired`, then `check.run` with its
//!   verdict, the check's revision, the `[id, revision]` of each item it
//!   evaluates at the run, and the sha256 of its output, which is kept beside
//!   the document (§4.3). A run that could not complete (timeout, missing
//!   tool, no number from a kpi) is `UNKNOWN`: never a pass, never a fail.
//! - [`evaluate`] applies the recorded runs and nothing else, so any number
//!   of evaluators reading one journal reach one answer: the last run on a
//!   check's current revision, when it passed, ticks each open item it
//!   evaluates at `observed` and raises a done item below `observed`; when it
//!   failed, the check is `failing` and, with `reopen: true`, each done item
//!   it evaluates is reopened; when it is `UNKNOWN` nothing is ticked and the
//!   check reads `check.unknown`. A run counts only for the revisions it
//!   names: an item whose line changed since the run is not ticked by it.
//! - The triggers ([`fire`], [`on_done`]) decide which checks to run, and
//!   then call the runner and the evaluator. `war done` fires `done`; `war
//!   check fire` is what a git hook and a CI step call; `war view watch
//!   --checks` fires `commit` and `schedule:<cron>` as it sees them.
//!
//! A check never sets a disposition (Appendix C's verdict), and nothing here
//! signs or verifies anything: `observed` is the level a passing run backs.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::format::{self, FieldValue};
use openwarrant_core::ticks::{self, Level};
use serde::Serialize;

use super::{Outcome, Store, Ticket, event, ladder};
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// `check.unknown` (§12, §13.2): a run that could not complete.
pub const UNKNOWN_RULE: &str = "check.unknown";
/// A `war done` whose `done` check failed: nothing was ticked.
pub const FAILED_RULE: &str = "check.failed";
/// A trigger or target that names no check.
pub const NOTHING_RULE: &str = "check.nothing";
/// The default `timeout` (§12), in seconds.
pub const DEFAULT_TIMEOUT: u64 = 600;
/// What `war check install-hook` marks its hook with, so a second install
/// replaces its own hook and never someone else's.
pub const HOOK_MARK: &str = "# openwarrant: FORMAT-1.0 §12 commit trigger";

/// One `check` or `kpi` record of a work document, as §12 reads it.
#[derive(Debug, Clone, Serialize)]
pub struct CheckRecord {
    /// The local id (`CHK-reset`).
    pub id: String,
    /// `<alias>/<id>`.
    pub global: String,
    /// `check` or `kpi`.
    pub kind: String,
    /// The sha256 of the record's span: what a run names as `revision`.
    pub revision: String,
    /// `run`: a command, or `gate://<id>@<version>`. `None` is refused when
    /// it is asked to run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// `on`, default `done`.
    pub on: Vec<String>,
    pub timeout: u64,
    pub gating: bool,
    pub reopen: bool,
    /// The global ids of the items it evaluates in its own document.
    pub evaluates: Vec<String>,
    /// What it evaluates elsewhere: carried, never ticked from here.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evaluates_elsewhere: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// 1-based line of its heading.
    pub line: usize,
}

fn field<'a>(fields: &'a BTreeMap<String, FieldValue>, key: &str) -> Option<&'a str> {
    fields
        .get(key)
        .and_then(FieldValue::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn list(fields: &BTreeMap<String, FieldValue>, key: &str) -> Vec<String> {
    let raw: Vec<String> = match fields.get(key) {
        Some(FieldValue::One(s)) => vec![s.clone()],
        Some(FieldValue::Many(v)) => v.clone(),
        None => Vec::new(),
    };
    raw.iter()
        .flat_map(|s| s.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The check and kpi records of the document `bytes` (alias `alias`).
#[must_use]
pub fn records_in(alias: &str, bytes: &[u8]) -> Vec<CheckRecord> {
    let doc = format::parse(bytes);
    let items: std::collections::BTreeSet<String> = doc
        .units
        .iter()
        .flat_map(|u| u.items.iter())
        .filter_map(|i| i.id.clone())
        .collect();
    doc.units
        .iter()
        .filter(|u| matches!(u.record_type.as_str(), "check" | "kpi"))
        .map(|u| {
            let mut evaluates = Vec::new();
            let mut elsewhere = Vec::new();
            for r in u.relations.iter().filter(|r| r.kind == "evaluates") {
                let id = r.target.id.as_str();
                let local = id
                    .strip_prefix(&format!("{alias}/"))
                    .unwrap_or(id)
                    .to_owned();
                if id == alias {
                    evaluates.push(alias.to_owned());
                } else if items.contains(&local) {
                    evaluates.push(format!("{alias}/{local}"));
                } else {
                    elsewhere.push(id.to_owned());
                }
            }
            let on = list(&u.fields, "on");
            let reopen = field(&u.fields, "reopen").map_or_else(
                || {
                    on.iter().any(|trigger| {
                        matches!(trigger.as_str(), "commit" | "pr.merged")
                            || trigger.starts_with("paths:")
                            || trigger.starts_with("ci:")
                            || trigger.starts_with("schedule:")
                    })
                },
                |value| value == "true",
            );
            CheckRecord {
                id: u.id.clone(),
                global: format!("{alias}/{}", u.id),
                kind: u.record_type.clone(),
                revision: format::sha256(bytes.get(u.start..u.end).unwrap_or_default()),
                run: field(&u.fields, "run").map(str::to_owned),
                on: if on.is_empty() {
                    vec!["done".to_owned()]
                } else {
                    on
                },
                timeout: field(&u.fields, "timeout")
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(DEFAULT_TIMEOUT),
                gating: field(&u.fields, "gating") != Some("false"),
                reopen,
                evaluates,
                evaluates_elsewhere: elsewhere,
                direction: field(&u.fields, "direction").map(str::to_owned),
                target: field(&u.fields, "target").and_then(|t| t.parse().ok()),
                mode: field(&u.fields, "mode").map(str::to_owned),
                line: u.line,
            }
        })
        .collect()
}

/// The document's own `paths` (§3.3): the globs a `commit` trigger watches
/// besides the document itself.
#[must_use]
pub fn paths_of(bytes: &[u8]) -> Vec<String> {
    list(&format::parse(bytes).fields, "paths")
}

/// The check records of `t`; none for a directory ticket.
#[must_use]
pub fn of_ticket(t: &Ticket) -> Vec<CheckRecord> {
    if !t.format {
        return Vec::new();
    }
    crate::vfs::read(&t.checklist_path)
        .map(|b| records_in(t.id(), &b))
        .unwrap_or_default()
}

// ---- the runner -------------------------------------------------------------

/// The subject of a firing about a time (`manual`, `done`): now, to the
/// microsecond, so that asking again is a new firing and two evaluators
/// answering one schedule minute name one (`schedule:` passes its minute).
#[must_use]
pub fn moment() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

/// What one run of one check came to.
#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    /// `<alias>/<id>`.
    pub check: String,
    pub trigger: String,
    pub subject: String,
    pub run: String,
    /// `PASS`, `FAIL` or `UNKNOWN`.
    pub verdict: String,
    pub gating: bool,
    /// Why a run is UNKNOWN (`timeout`, `not_askable`, `no-number`...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Where the output is kept, relative to the root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// `false`: this trigger already fired for this revision and subject, so
    /// nothing ran and nothing was recorded (§12: several evaluators firing
    /// the same trigger record it once).
    pub recorded: bool,
}

/// A run id that is a function of what fired it: the check, its revision,
/// the trigger and the subject. Two evaluators firing one trigger name one
/// run.
fn run_id(c: &CheckRecord, trigger: &str, subject: &str) -> String {
    let h = openwarrant_compiler::sha256_hex(
        format!("{}\n{}\n{trigger}\n{subject}", c.global, c.revision).as_bytes(),
    );
    format!("r-{}", &h[..16])
}

/// The `trigger.fired` payload (§11.2): the check and its revision, so the
/// idempotency key holds the revision (§12).
fn fired_payload(c: &CheckRecord, trigger: &str, subject: &str) -> serde_json::Value {
    serde_json::json!({
        "check": c.id,
        "revision": c.revision,
        "trigger": trigger,
        "subject": subject,
    })
}

/// Whether the journal in `dir` already holds `trigger.fired` with this
/// payload, by anyone.
fn already_fired(dir: &Utf8Path, payload: &serde_json::Value) -> Result<bool, RepoError> {
    let journal = crate::journal_cmd::load(dir)?;
    let want = crate::journal_cmd::canonical_payload(&payload.to_string());
    Ok(journal.events.iter().any(|e| {
        e.event_type == event::TRIGGER_FIRED
            && crate::journal_cmd::canonical_payload(&e.payload) == want
    }))
}

/// The current revision of each item `c` evaluates: `[global id, revision]`.
fn subjects(t: &Ticket, text: &str, c: &CheckRecord) -> Vec<(String, String)> {
    let list = super::checklist_in(t, text);
    c.evaluates
        .iter()
        .filter_map(|g| {
            if g == t.id() {
                let doc = format::parse(text.as_bytes());
                return Some((g.clone(), format::sha256(&text.as_bytes()[..doc.head_end])));
            }
            let local = g.rsplit('/').next()?;
            let it = list.item(local)?;
            Some((g.clone(), super::item_revision(text, it)))
        })
        .collect()
}

fn definition(c: &CheckRecord, cmd: &str) -> openwarrant_core::gate::GateDefinition {
    openwarrant_core::gate::GateDefinition {
        gate_id: c.global.replace('/', "."),
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
        timeout_secs: Some(c.timeout),
    }
}

/// The verdict of a kpi that printed `value` (§12): `threshold` and `best`
/// pass when it meets the target, `optimise` never fails. A kpi with no
/// target and a mode that needs one decides nothing: UNKNOWN.
fn kpi_verdict(c: &CheckRecord, value: f64) -> (&'static str, Option<String>) {
    let mode = c.mode.as_deref().unwrap_or("best");
    if mode == "optimise" {
        return ("PASS", None);
    }
    let Some(target) = c.target else {
        return ("UNKNOWN", Some("no-target".to_owned()));
    };
    let met = match c.direction.as_deref() {
        Some("max") => value >= target,
        Some("min") => value <= target,
        _ => return ("UNKNOWN", Some("no-direction".to_owned())),
    };
    (if met { "PASS" } else { "FAIL" }, None)
}

/// Run `c` of `t` because `trigger` fired for `subject`, and record it.
///
/// The trigger is recorded first (`trigger.fired`); a trigger already
/// recorded for this revision and subject runs nothing (`recorded: false`).
/// A check with no `run`, a `gate://` the registry does not hold, a timeout,
/// a tool that is not there, or a kpi that printed no number is `UNKNOWN`.
pub fn run_check(
    store: &Store,
    repo: &Repository,
    t: &Ticket,
    c: &CheckRecord,
    trigger: &str,
    subject: &str,
) -> Result<RunResult, RepoError> {
    let run = run_id(c, trigger, subject);
    let mut result = RunResult {
        check: c.global.clone(),
        trigger: trigger.to_owned(),
        subject: subject.to_owned(),
        run: run.clone(),
        verdict: "UNKNOWN".to_owned(),
        gating: c.gating,
        reason: None,
        value: None,
        output: None,
        recorded: false,
    };
    let fired = fired_payload(c, trigger, subject);
    if already_fired(&t.dir, &fired)? {
        // The run that firing recorded says what it came to.
        let journal = crate::journal_cmd::load(&t.dir)?;
        let recorded = crate::states::last_runs_where(&journal, &|r| r.run == run);
        match recorded.into_values().next() {
            Some(r) => {
                let head = ladder::head_commit(&repo.root);
                let current = r.source_basis.as_ref().is_none_or(|recorded| {
                    super::source_basis::capture(&repo.root, &t.dir).as_ref() == Ok(recorded)
                });
                if r.at_commit(head.as_deref()) && current {
                    result.verdict = r.verdict.to_ascii_uppercase();
                    result.value = r.value;
                } else {
                    result.reason = Some("stale-source-basis".to_owned());
                }
            }
            None => result.reason = Some("not-recorded".to_owned()),
        }
        return Ok(result);
    }
    store.journal(t, event::TRIGGER_FIRED, &fired)?;
    result.recorded = true;
    let text = crate::vfs::read_to_string(&t.checklist_path).unwrap_or_default();
    let subjects = subjects(t, &text, c);
    let commit = ladder::head_commit(&repo.root);
    let mut output_sha256 = None;
    let basis = super::source_basis::capture(&repo.root, &t.dir);
    match if basis.is_ok() {
        c.run.as_deref()
    } else {
        None
    } {
        None => result.reason = Some("no-run".to_owned()),
        Some(cmd) => {
            let def = match cmd.strip_prefix("gate://") {
                Some(key) => {
                    let mut scratch = Report::default();
                    let registry = crate::check::load_gate_registry(repo, &mut scratch);
                    registry
                        .definitions
                        .iter()
                        .find(|d| d.key() == key)
                        .cloned()
                }
                None => Some(definition(c, cmd)),
            };
            match def {
                None => result.reason = Some("gate-unknown".to_owned()),
                Some(def) => {
                    // Kept beside the document (§4.3): the evidence directory.
                    let dir = t.dir.join("checks").join(&run);
                    let gate = crate::gate_cmd::run_gate(&def, repo, &dir);
                    store.renew_held(None);
                    let slug = def.key().replace(['/', '@', '.'], "_");
                    let out_path = dir.join(format!("{slug}.stdout.txt"));
                    let stdout = crate::vfs::read(&out_path).unwrap_or_default();
                    if out_path.is_file() {
                        output_sha256 = Some(openwarrant_compiler::sha256_hex(&stdout));
                        result.output = Some(repo.relative(&out_path));
                    }
                    let passed = gate.verdict == openwarrant_core::Verdict::Pass
                        && gate.satisfies_required_pass();
                    let word = |v: serde_json::Value| v.as_str().map(str::to_owned);
                    let why = gate
                        .reason_code
                        .and_then(|r| word(serde_json::to_value(r).unwrap_or_default()))
                        .or_else(|| {
                            word(serde_json::to_value(gate.execution_status).unwrap_or_default())
                        });
                    if c.kind == "kpi" {
                        let value = passed
                            .then(|| ticks::parse_number(&String::from_utf8_lossy(&stdout)))
                            .flatten();
                        result.value = value;
                        match value {
                            Some(v) => {
                                let (verdict, reason) = kpi_verdict(c, v);
                                result.verdict = verdict.to_owned();
                                result.reason = reason;
                            }
                            None if passed => result.reason = Some("no-number".to_owned()),
                            None => result.reason = why,
                        }
                    } else if passed {
                        result.verdict = "PASS".to_owned();
                    } else if gate.verdict == openwarrant_core::Verdict::Fail {
                        result.verdict = "FAIL".to_owned();
                    } else {
                        result.reason = why;
                    }
                }
            }
        }
    }
    match &basis {
        Err(error) => {
            result.verdict = "UNKNOWN".into();
            result.reason = Some(format!("source-basis-unavailable: {error}"));
        }
        Ok(before) => {
            if super::source_basis::capture(&repo.root, &t.dir).as_ref() != Ok(before) {
                result.verdict = "UNKNOWN".into();
                result.reason = Some("source-changed-during-check".into());
            }
        }
    }
    let mut payload = serde_json::json!({
        "check": c.id,
        "run": run,
        "verdict": result.verdict,
        "revision": c.revision,
        "subjects": subjects,
    });
    if let Ok(basis) = basis {
        payload["source_basis"] = serde_json::json!(basis);
    }
    if let Some(commit) = commit {
        payload["commit"] = serde_json::json!(commit);
    }
    if let Some(d) = output_sha256 {
        payload["output_sha256"] = serde_json::json!(d);
    }
    if let Some(v) = result.value {
        payload["value"] = serde_json::json!(v);
    }
    store.journal(t, event::CHECK_RUN, &payload)?;
    Ok(result)
}

// ---- the evaluator ----------------------------------------------------------

/// One thing an evaluation did, or declined to do.
#[derive(Debug, Clone, Serialize)]
pub struct Effect {
    /// `<alias>/<item>`.
    pub target: String,
    pub check: String,
    pub run: String,
    /// `ticked`, `raised`, `reopened`, or (nothing written) `stale` (the run
    /// names another revision of the item or the check) or `unknown`.
    pub effect: &'static str,
}

/// What [`evaluate`] found and did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Evaluation {
    pub effects: Vec<Effect>,
    /// Checks whose last run on their current revision failed.
    pub failing: Vec<String>,
    /// Checks whose last run on their current revision is UNKNOWN.
    pub unknown: Vec<String>,
}

/// The last `check.run` of `c` on its current revision, from `journal`.
fn last_run(
    journal: &openwarrant_core::journal::Journal,
    c: &CheckRecord,
    head: Option<&str>,
    basis: Option<&super::source_basis::Basis>,
) -> Option<crate::states::CheckRun> {
    let current = |r: &crate::states::CheckRun| {
        (r.check == c.id || r.check == c.global)
            && r.revision == c.revision
            && r.at_commit(head)
            && r.source_basis
                .as_ref()
                .is_none_or(|recorded| Some(recorded) == basis)
    };
    crate::states::last_runs_where(journal, &current)
        .into_values()
        .next()
}

/// `episode` (§11.2): how many events of `event_type` for `target` the
/// journal already holds, so a fact that happens again is a new event.
fn episode(journal: &openwarrant_core::journal::Journal, event_type: &str, target: &str) -> usize {
    journal
        .events
        .iter()
        .filter(|e| crate::journal_cmd::work_type(&e.event_type) == event_type)
        .filter(|e| {
            serde_json::from_str::<serde_json::Value>(&e.payload)
                .ok()
                .and_then(|p| p.get("target").and_then(|v| v.as_str()).map(str::to_owned))
                .as_deref()
                == Some(target)
        })
        .count()
}

/// One planned write to a line, decided from what the run names.
struct Plan {
    item: String,
    target: String,
    check: String,
    run: String,
    runs: Vec<String>,
    /// The revision the run names for the item: the compare-and-set's base.
    base: String,
    action: Action,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Tick,
    Raise(Level),
    Reopen,
}

/// Apply the recorded runs of every Format 1.0 ticket (`only`: of that one
/// ticket) as §12 says, writing through the compare-and-set of §11.3.
pub fn evaluate(store: &Store, only: Option<&str>) -> Result<(Evaluation, Report), RepoError> {
    let (tickets, _) = store.load_all()?;
    let mut out = Evaluation::default();
    let mut report = Report::default();
    for t in tickets.iter().filter(|t| t.format) {
        if only.is_some_and(|a| a != t.id()) {
            continue;
        }
        evaluate_one(store, t, &mut out, &mut report)?;
    }
    Ok((out, report))
}

fn evaluate_one(
    store: &Store,
    t: &Ticket,
    out: &mut Evaluation,
    report: &mut Report,
) -> Result<(), RepoError> {
    let checks = of_ticket(t);
    if checks.is_empty() {
        return Ok(());
    }
    let journal = crate::journal_cmd::load(&t.dir)?;
    let reader = ladder::Reader::new(store);
    let parts = ladder::checks_of(t);
    let backing = ladder::backing(t);
    let file = store.rel(&t.checklist_path);
    let mut plans: Vec<Plan> = Vec::new();
    let head = ladder::head_commit(store.check_root.as_deref().unwrap_or(&store.root));
    let basis =
        super::source_basis::capture(store.check_root.as_deref().unwrap_or(&store.root), &t.dir)
            .ok();
    for c in &checks {
        let Some(run) = last_run(&journal, c, head.as_deref(), basis.as_ref()) else {
            let old = crate::states::last_runs_where(&journal, &|r| {
                (r.check == c.id || r.check == c.global) && r.revision == c.revision
            })
            .into_values()
            .next();
            if let Some(old) = old {
                for target in &c.evaluates {
                    out.effects.push(Effect {
                        target: target.clone(),
                        check: c.global.clone(),
                        run: old.run.clone(),
                        effect: "stale",
                    });
                }
                report.push(Diagnostic::warn("check.source-stale", &file, format!("{} has no observation for the current local source; rerun `war check run {}`", c.global, c.global)));
            }
            continue;
        };
        let verdict = run.verdict.to_ascii_uppercase();
        let named: BTreeMap<String, String> = run
            .subjects
            .iter()
            .map(|(id, rev)| {
                let id = if id.contains('/') {
                    id.clone()
                } else {
                    format!("{}/{id}", t.id())
                };
                (id, rev.clone())
            })
            .collect();
        match verdict.as_str() {
            "PASS" | "FAIL" => {}
            _ => {
                out.unknown.push(c.global.clone());
                report.push(Diagnostic::unknown(
                    UNKNOWN_RULE,
                    format!("{file}:{}", c.line),
                    format!(
                        "{}: its last run ({}) could not complete; UNKNOWN is neither a pass \
                         nor a fail, and nothing it evaluates was ticked",
                        c.global, run.run
                    ),
                ));
                continue;
            }
        }
        if verdict == "FAIL" {
            out.failing.push(c.global.clone());
            if !(c.reopen && c.gating) {
                continue;
            }
        }
        if verdict == "PASS" && !c.gating {
            // Informational checks supply observations, never completion.
            continue;
        }
        for g in &c.evaluates {
            let local = g.rsplit('/').next().unwrap_or(g);
            let Some(it) = t.item(local) else { continue };
            let current = super::item_revision(&t.checklist_text, it);
            let effect = |e: &'static str| Effect {
                target: g.clone(),
                check: c.global.clone(),
                run: run.run.clone(),
                effect: e,
            };
            let level = || ladder::view_of(&reader, t, it, &parts, &backing).level;
            if verdict == "PASS" && it.done && level() >= Level::Observed {
                // Already where a pass puts it.
                continue;
            }
            if named.get(g) != Some(&current) {
                // §12: a run counts only for the revisions it names.
                if verdict == "PASS" || it.done {
                    out.effects.push(effect("stale"));
                }
                continue;
            }
            let action = match (verdict.as_str(), it.done) {
                ("PASS", false) => Action::Tick,
                ("PASS", true) => Action::Raise(level()),
                ("FAIL", true) => Action::Reopen,
                _ => continue,
            };
            if action == Action::Reopen
                && let Some(signed) = backing.signoff(local)
            {
                let verified = level() == Level::Signed;
                match super::repair::propose(store, t, local, signed, verified, c, &run.run) {
                    Ok(repair) => {
                        out.effects.push(effect("repair-proposed"));
                        report.push(Diagnostic::warn("check.repair-proposed", &file, format!("{g}: prior sign-off record retained; proposed repair {repair}. New work is unverified")));
                        if !verified { report.push(Diagnostic::unknown("check.acceptance-unestablished", &file, format!("{g}: prior sign-off evidence cannot currently be established; history was retained and no acceptance was awarded"))); }
                    }
                    Err(error) => report.push(Diagnostic::unknown("check.repair-unavailable", &file, format!("{g}: prior sign-off record retained; repair proposal unavailable: {error}"))),
                }
                continue;
            }
            let mut runs = vec![run.run.clone()];
            if matches!(action, Action::Tick | Action::Raise(_)) {
                let required: Vec<_> = checks
                    .iter()
                    .filter(|check| check.gating && check.evaluates.contains(g))
                    .collect();
                let mut current_runs = Vec::new();
                let all_pass = required.iter().all(|check| {
                    let Some(other) = last_run(&journal, check, head.as_deref(), basis.as_ref())
                    else {
                        return false;
                    };
                    if !other.verdict.eq_ignore_ascii_case("PASS")
                        || !other.subjects.iter().any(|(id, revision)| {
                            (id == g || format!("{}/{id}", t.id()) == *g) && revision == &current
                        })
                    {
                        return false;
                    }
                    current_runs.push(other.run);
                    true
                });
                if !all_pass || required.is_empty() {
                    out.effects.push(effect("required-results-pending"));
                    continue;
                }
                let (minimum, _) = ladder::minimum(store, t, &parts, Some(local));
                if minimum > Level::Observed {
                    report.push(Diagnostic::warn(
                        "check.minimum-pending",
                        &file,
                        format!(
                            "{g}: checks passed; completion requires {}: {}",
                            minimum.as_str(),
                            ladder::command_for(minimum, g)
                        ),
                    ));
                    continue;
                }
                runs = current_runs;
            }
            if plans.iter().any(|p| p.target == *g) {
                continue;
            }
            plans.push(Plan {
                item: local.to_owned(),
                target: g.clone(),
                check: c.global.clone(),
                run: run.run.clone(),
                runs,
                base: current,
                action,
            });
        }
    }
    if t.checklist.items.is_empty() {
        evaluate_document(store, t, &checks, out, report)?;
    }
    if plans.is_empty() {
        return Ok(());
    }
    let actor = store.actor.clone();
    let today = super::date_of(super::now_secs());
    let written = super::rewrite(&t.checklist_path, |text| {
        let list = super::checklist_in(t, text);
        let mut text = text.to_owned();
        let mut landed = Vec::new();
        for p in &plans {
            let Some(it) = list.item(&p.item) else {
                continue;
            };
            // The compare-and-set: the line as the run saw it, or nothing.
            if super::item_revision(&text, it) != p.base {
                continue;
            }
            let line = match p.action {
                Action::Tick => it
                    .ticked(&actor, &ticks::with_level(&today, Level::Observed), None)
                    .render(),
                Action::Raise(_) => {
                    let date = it
                        .done_on
                        .as_deref()
                        .map_or_else(|| today.clone(), |d| ticks::split_level(d).0.to_owned());
                    let by = it.done_by.clone().unwrap_or_else(|| actor.clone());
                    it.ticked(
                        &by,
                        &ticks::with_level(&date, Level::Observed),
                        it.note.as_deref(),
                    )
                    .render()
                }
                Action::Reopen => {
                    let old = text
                        .split_inclusive('\n')
                        .nth(it.line)
                        .unwrap_or_default()
                        .trim_end_matches(['\n', '\r']);
                    super::format::reopened_line(old)
                }
            };
            text = openwarrant_core::ticket::replace_line(&text, it.line, &line);
            landed.push(p.target.clone());
        }
        Ok((text, landed))
    })?;
    let landed = match written {
        Ok(l) => l,
        Err(refusal) => {
            for d in refusal.report.diagnostics {
                report.push(d);
            }
            return Ok(());
        }
    };
    let on = today;
    for p in plans.iter() {
        if !landed.contains(&p.target) {
            out.effects.push(Effect {
                target: p.target.clone(),
                check: p.check.clone(),
                run: p.run.clone(),
                effect: "stale",
            });
            continue;
        }
        let journal = crate::journal_cmd::load(&t.dir)?;
        let (kind, payload, effect) = match p.action {
            Action::Tick => (
                event::ITEM_DONE,
                serde_json::json!({
                    "item": p.item, "target": p.target, "on": on, "level": "observed",
                    "runs": p.runs,
                    "episode": episode(&journal, event::ITEM_DONE, &p.target),
                }),
                "ticked",
            ),
            Action::Raise(from) => (
                ladder::event::TICK_RAISED,
                serde_json::json!({
                    "item": p.item, "target": p.target, "from": from.as_str(),
                    "to": "observed", "runs": p.runs,
                }),
                "raised",
            ),
            Action::Reopen => (
                event::ITEM_REOPENED,
                serde_json::json!({
                    "item": p.item, "target": p.target,
                    "reason": format!("{} failed (run {})", p.check, p.run),
                    "episode": episode(&journal, event::ITEM_REOPENED, &p.target),
                }),
                "reopened",
            ),
        };
        store.journal(t, kind, &payload)?;
        out.effects.push(Effect {
            target: p.target.clone(),
            check: p.check.clone(),
            run: p.run.clone(),
            effect,
        });
    }
    Ok(())
}

/// A work document without checklist items is itself the completion target.
/// Its progress stays in the journal; no synthetic checklist or new ID is needed.
fn evaluate_document(
    store: &Store,
    t: &Ticket,
    checks: &[CheckRecord],
    out: &mut Evaluation,
    report: &mut Report,
) -> Result<(), RepoError> {
    let required: Vec<_> = checks
        .iter()
        .filter(|c| c.gating && c.evaluates.iter().any(|g| g == t.id()))
        .collect();
    if required.is_empty() {
        return Ok(());
    }
    let journal = crate::journal_cmd::load(&t.dir)?;
    let root = store.check_root.as_deref().unwrap_or(&store.root);
    let head = ladder::head_commit(root);
    let basis = super::source_basis::capture(root, &t.dir).ok();
    let doc = format::parse(t.checklist_text.as_bytes());
    let revision = format::sha256(&t.checklist_text.as_bytes()[..doc.head_end]);
    let observations: Vec<_> = required
        .iter()
        .filter_map(|c| {
            let run = last_run(&journal, c, head.as_deref(), basis.as_ref())?;
            run.subjects
                .iter()
                .any(|(target, seen)| target == t.id() && seen == &revision)
                .then_some((*c, run))
        })
        .collect();
    let failure = observations
        .iter()
        .find(|(c, run)| c.reopen && run.verdict.eq_ignore_ascii_case("FAIL"));
    let (kind, mut payload, check, run, effect) = if t.is_done() {
        let Some((check, run)) = failure else {
            return Ok(());
        };
        (
            event::ITEM_REOPENED,
            serde_json::json!({"item":t.id(),"target":t.id(),"reason":format!("{} failed (run {})",check.global,run.run),"episode":episode(&journal,event::ITEM_REOPENED,t.id())}),
            check.global.clone(),
            run.run.clone(),
            "reopened",
        )
    } else {
        if observations.len() != required.len()
            || observations
                .iter()
                .any(|(_, r)| !r.verdict.eq_ignore_ascii_case("PASS"))
        {
            return Ok(());
        }
        let parts = ladder::checks_of(t);
        let (minimum, _) = ladder::minimum(store, t, &parts, None);
        if minimum > Level::Observed {
            report.push(Diagnostic::warn(
                "check.minimum-pending",
                store.rel(&t.intent_path),
                format!(
                    "{} requires {}; passing checks do not satisfy that act",
                    t.id(),
                    minimum.as_str()
                ),
            ));
            return Ok(());
        }
        let runs: Vec<_> = observations.iter().map(|(_, r)| r.run.clone()).collect();
        (
            event::ITEM_DONE,
            serde_json::json!({"item":t.id(),"target":t.id(),"on":super::date_of(super::now_secs()),"level":"observed","runs":runs,"episode":episode(&journal,event::ITEM_DONE,t.id())}),
            required[0].global.clone(),
            runs.join(","),
            "ticked",
        )
    };
    // Re-read the contract while holding the same document write mechanism as
    // checklist progress. A concurrent contract edit cannot inherit the result.
    let written = super::rewrite(&t.checklist_path, |text| {
        if text != t.checklist_text || super::format::done_whole(&t.dir, t.id()) != t.is_done() {
            return Ok((text.to_owned(), false));
        }
        let latest = crate::journal_cmd::load(&t.dir).map_err(|e| {
            Box::new(Outcome::refused(
                "check.progress-write",
                store.rel(&t.intent_path),
                e.to_string(),
            ))
        })?;
        payload["episode"] = serde_json::json!(episode(&latest, kind, t.id()));
        store.journal(t, kind, &payload).map_err(|e| {
            Box::new(Outcome::refused(
                "check.progress-write",
                store.rel(&t.intent_path),
                e.to_string(),
            ))
        })?;
        Ok((text.to_owned(), true))
    })?;
    if matches!(written, Ok(true)) {
        out.effects.push(Effect {
            target: t.id().into(),
            check,
            run,
            effect,
        });
    }
    Ok(())
}

// ---- triggers ---------------------------------------------------------------

/// Whether `declared` (one of a check's `on`) fires for `trigger` given the
/// paths a commit changed. `commit` and `paths:<glob>` read `changed`;
/// `schedule:<cron>` fires for `schedule:<cron>` itself (the watcher
/// matches the cron); a dotted extension is carried and never fired.
fn fires(
    declared: &str,
    trigger: &str,
    changed: Option<&[String]>,
    doc: &str,
    paths: &[String],
) -> bool {
    let changes = |pred: &dyn Fn(&str) -> bool| changed.is_some_and(|c| c.iter().any(|p| pred(p)));
    match (declared, trigger) {
        ("commit", "commit") => {
            changes(&|p| p == doc || paths.iter().any(|g| crate::context::glob_match(g, p)))
        }
        (d, "commit") if d.starts_with("paths:") => {
            let glob = &d["paths:".len()..];
            changes(&|p| crate::context::glob_match(glob, p))
        }
        (d, t) => {
            d == t
                && (matches!(d, "manual" | "done" | "pr.merged")
                    || d.starts_with("ci:")
                    || d.starts_with("schedule:"))
        }
    }
}

/// The paths commit `sha` changed, from git; `None` when git cannot say.
#[must_use]
pub fn changed_paths(root: &Utf8Path, sha: &str) -> Option<Vec<String>> {
    let out = std::process::Command::new("git")
        .args([
            "-c",
            "core.quotePath=false",
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-only",
            "-r",
            "-m",
            sha,
        ])
        .current_dir(root)
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status.success().then(|| {
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_owned)
            .filter(|l| !l.is_empty())
            .collect()
    })
}

/// The work documents commit `sha`'s message cites: each word that is a
/// ticket id (`t-…`), for `pr.merged` (§12: a pull request citing the work
/// merges). Empty when git cannot say, so nothing fires.
#[must_use]
pub fn cited_work(root: &Utf8Path, sha: &str) -> Vec<String> {
    let Ok(out) = std::process::Command::new("git")
        .args(["log", "-1", "--format=%B", sha])
        .current_dir(root)
        .stderr(std::process::Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let mut cited: Vec<String> = Vec::new();
    for word in String::from_utf8_lossy(&out.stdout)
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|w| openwarrant_core::ticket::is_ticket_id(w))
    {
        if !cited.iter().any(|c| c == word) {
            cited.push(word.to_owned());
        }
    }
    // Nothing cited: an empty selection would fire every document's check,
    // so a sentinel no alias equals keeps the firing to none.
    if cited.is_empty() {
        cited.push(String::new());
    }
    cited
}

/// What one firing did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Firing {
    pub trigger: String,
    pub subject: String,
    pub runs: Vec<RunResult>,
    pub evaluation: Evaluation,
}

/// Which checks a trigger selects.
pub struct Select<'a> {
    /// `manual`, `done`, `commit`, `pr.merged`, `ci:<name>`, `schedule:<cron>`.
    pub trigger: &'a str,
    pub subject: &'a str,
    /// For `commit`: the paths the commit changed.
    pub changed: Option<Vec<String>>,
    /// Only these documents (aliases), when given.
    pub work: Vec<String>,
    /// Only this check (`<alias>/<id>` or `<id>`), for `manual`.
    pub check: Option<String>,
}

/// Fire `sel.trigger`: run every check that declares it (a `manual` firing
/// runs the checks named, whatever they declare), then evaluate the
/// documents they belong to.
pub fn fire(store: &Store, sel: &Select<'_>) -> Result<(Firing, Report), RepoError> {
    let repo = Repository::discover(Some(store.root.clone()))?;
    let (tickets, _) = store.load_all()?;
    let mut firing = Firing {
        trigger: sel.trigger.to_owned(),
        subject: sel.subject.to_owned(),
        ..Firing::default()
    };
    let mut report = Report::default();
    let mut touched: Vec<String> = Vec::new();
    for t in tickets.iter().filter(|t| t.format) {
        if !sel.work.is_empty() && !sel.work.iter().any(|w| w == t.id()) {
            continue;
        }
        let bytes = crate::vfs::read(&t.checklist_path).unwrap_or_default();
        let doc = store.rel(&t.checklist_path);
        let paths = paths_of(&bytes);
        for c in records_in(t.id(), &bytes) {
            let picked = match sel.trigger {
                "manual" => sel
                    .check
                    .as_deref()
                    .is_none_or(|q| q == c.global || q == c.id),
                trigger => {
                    c.on.iter()
                        .any(|d| fires(d, trigger, sel.changed.as_deref(), &doc, &paths))
                }
            };
            if !picked {
                continue;
            }
            let r = run_check(store, &repo, t, &c, sel.trigger, sel.subject)?;
            if r.verdict != "PASS" && r.verdict != "FAIL" {
                let at = format!("{doc}:{}", c.line);
                let message = format!(
                    "{} could not establish a current result (run {}): {}",
                    c.global,
                    r.run,
                    r.reason.as_deref().unwrap_or("unknown execution result")
                );
                report.push(if c.gating {
                    Diagnostic::unknown(UNKNOWN_RULE, at, message)
                } else {
                    Diagnostic::warn(UNKNOWN_RULE, at, message)
                });
            }
            if r.recorded && r.verdict == "FAIL" {
                let message = format!(
                    "{} failed (run {}); nothing it evaluates was ticked{}",
                    c.global,
                    r.run,
                    if c.gating {
                        ""
                    } else {
                        " (`gating: false`: recorded only)"
                    }
                );
                let at = format!("{doc}:{}", c.line);
                report.push(if c.gating {
                    Diagnostic::error(FAILED_RULE, at, message)
                } else {
                    Diagnostic::warn(FAILED_RULE, at, message)
                });
            }
            if !touched.iter().any(|a| a == t.id()) {
                touched.push(t.id().to_owned());
            }
            firing.runs.push(r);
        }
    }
    for alias in &touched {
        let (e, r) = evaluate(store, Some(alias))?;
        firing.evaluation.effects.extend(e.effects);
        firing.evaluation.failing.extend(e.failing);
        firing.evaluation.unknown.extend(e.unknown);
        for d in r.diagnostics {
            report.push(d);
        }
    }
    Ok((firing, report))
}

/// Completion runs all required checks covering the item or title-only
/// Warrant, plus optional checks triggered by `done` (`all` runs every optional
/// check too). A failed or unknown required result refuses completion. Optional
/// observations alone do not raise the claimed completion level.
pub fn on_done(
    store: &Store,
    t: &Ticket,
    item: Option<&str>,
    all: bool,
) -> Result<Option<Result<ladder::CheckRound, Box<Outcome>>>, RepoError> {
    let target = item.map_or_else(|| t.id().to_owned(), |item| format!("{}/{item}", t.id()));
    let selected: Vec<CheckRecord> = of_ticket(t)
        .into_iter()
        .filter(|c| {
            c.evaluates.contains(&target) && (c.gating || all || c.on.iter().any(|o| o == "done"))
        })
        .collect();
    if selected.is_empty() {
        return Ok(None);
    }
    let repo = Repository::discover(Some(
        store
            .check_root
            .clone()
            .unwrap_or_else(|| store.root.clone()),
    ))?;
    let subject = moment();
    let mut results = Vec::new();
    for c in &selected {
        results.push((c, run_check(store, &repo, t, c, "done", &subject)?));
    }
    let file = store.rel(&t.checklist_path);
    let gating: Vec<&(&CheckRecord, RunResult)> =
        results.iter().filter(|(c, _)| c.gating).collect();
    let failed: Vec<String> = gating
        .iter()
        .filter(|(_, r)| r.verdict == "FAIL")
        .map(|(c, r)| format!("{} (run {})", c.global, r.run))
        .collect();
    let unknown: Vec<String> = gating
        .iter()
        .filter(|(_, r)| r.verdict != "PASS" && r.verdict != "FAIL")
        .map(|(c, r)| {
            format!(
                "{} (run {}, {})",
                c.global,
                r.run,
                r.reason.as_deref().unwrap_or("not run")
            )
        })
        .collect();
    let result = serde_json::json!({
        "schema": "oh.war/check-runs/v1",
        "target": target,
        "runs": results.iter().map(|(_, r)| r).collect::<Vec<_>>(),
    });
    if !failed.is_empty() {
        let mut o = Outcome::refused(
            FAILED_RULE,
            file,
            format!(
                "{target} was not ticked: {} failed. Nothing was ticked, and the claim is still \
                 yours",
                failed.join(", ")
            ),
        );
        o.result = result;
        return Ok(Some(Err(Box::new(o))));
    }
    if !unknown.is_empty() {
        let mut o = Outcome::unknown(
            UNKNOWN_RULE,
            file,
            format!(
                "{target} was not ticked: {} could not complete. UNKNOWN is not a pass and not \
                 a fail; nothing was ticked, and the claim is still yours",
                unknown.join(", ")
            ),
        );
        o.result = result;
        return Ok(Some(Err(Box::new(o))));
    }
    if gating.is_empty() {
        return Ok(None);
    }
    let runs = gating
        .iter()
        .map(|(c, r)| ladder::Run {
            kind: if c.kind == "kpi" { "kpi" } else { "test" },
            name: c.global.clone(),
            cmd: c.run.clone().unwrap_or_default(),
            item: item.map(str::to_owned),
            verdict: "pass",
            gating: true,
            status: "completed".to_owned(),
            reason: None,
            value: r.value,
            target: c.target,
            direction: None,
            mode: None,
            duration_ms: 0,
            stdout_sha256: None,
            output: r.output.clone().unwrap_or_default(),
        })
        .collect();
    Ok(Some(Ok(ladder::CheckRound {
        run: gating
            .iter()
            .map(|(_, r)| r.run.as_str())
            .collect::<Vec<_>>()
            .join(","),
        runs,
        commit: ladder::head_commit(&repo.root),
    })))
}

// ---- schedule:<cron> --------------------------------------------------------

/// Whether the five-field cron expression `expr` names the minute `secs`
/// (Unix seconds, UTC): minute, hour, day of month, month, day of week
/// (0 or 7 is Sunday). Each field is `*`, a number, a range `a-b`, a step
/// `*/n` or `a-b/n`, or a comma list of those. When both day fields are
/// restricted, either matching is enough (cron's own rule). An expression
/// that does not parse matches nothing.
#[must_use]
pub fn cron_matches(expr: &str, secs: u64) -> bool {
    use chrono::{Datelike, Timelike};
    let Some(t) = chrono::DateTime::from_timestamp(i64::try_from(secs).unwrap_or(0), 0) else {
        return false;
    };
    let fields: Vec<&str> = expr.split_whitespace().collect();
    let [min, hour, dom, mon, dow] = fields.as_slice() else {
        return false;
    };
    let one = |f: &str, v: u32, lo: u32, hi: u32| -> Option<bool> {
        let mut any = false;
        for part in f.split(',') {
            let (range, step) = match part.split_once('/') {
                Some((r, s)) => (r, s.parse::<u32>().ok().filter(|s| *s > 0)?),
                None => (part, 1),
            };
            let (a, b) = if range == "*" {
                (lo, hi)
            } else if let Some((a, b)) = range.split_once('-') {
                (a.parse().ok()?, b.parse().ok()?)
            } else {
                let n: u32 = range.parse().ok()?;
                (n, if part.contains('/') { hi } else { n })
            };
            if a < lo || b > hi || a > b {
                return None;
            }
            if v >= a && v <= b && (v - a).is_multiple_of(step) {
                any = true;
            }
        }
        Some(any)
    };
    let weekday = t.weekday().num_days_from_sunday();
    let (Some(m), Some(h), Some(d), Some(mo)) = (
        one(min, t.minute(), 0, 59),
        one(hour, t.hour(), 0, 23),
        one(dom, t.day(), 1, 31),
        one(mon, t.month(), 1, 12),
    ) else {
        return false;
    };
    let Some(w) =
        one(dow, weekday, 0, 7).map(|w| w || (weekday == 0 && one(dow, 7, 0, 7) == Some(true)))
    else {
        return false;
    };
    let day = match (*dom == "*", *dow == "*") {
        (true, true) => true,
        (false, true) => d,
        (true, false) => w,
        (false, false) => d || w,
    };
    m && h && mo && day
}

/// The `schedule:<cron>` triggers of every Format 1.0 ticket whose cron
/// names the minute `secs`.
pub fn due_schedules(store: &Store, secs: u64) -> Result<Vec<String>, RepoError> {
    let (tickets, _) = store.load_all()?;
    let mut out: Vec<String> = Vec::new();
    for t in tickets.iter().filter(|t| t.format) {
        for c in of_ticket(t) {
            for o in &c.on {
                if let Some(expr) = o.strip_prefix("schedule:")
                    && cron_matches(expr, secs)
                    && !out.contains(o)
                {
                    out.push(o.clone());
                }
            }
        }
    }
    Ok(out)
}

// ---- the version-control hook ------------------------------------------------

/// Install the `commit` trigger's git hook: `post-commit` in the hooks
/// directory git uses (`git rev-parse --git-path hooks`, which honours
/// `core.hooksPath`). It runs `war check fire commit` for the new commit,
/// and a failing check never undoes a commit. A hook already there that is
/// not this one is refused (`check.hook-exists`), never overwritten.
pub fn install_hook(root: &Utf8Path, war: &str) -> Result<Outcome, RepoError> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .current_dir(root)
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(|source| RepoError::Io {
            context: "could not run git".to_owned(),
            source,
        })?;
    if !out.status.success() {
        return Ok(Outcome::refused(
            "check.hook-no-git",
            String::new(),
            format!("{root} is not a git checkout; there is no hook to install"),
        ));
    }
    let rel = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let dir = {
        let p = Utf8PathBuf::from(&rel);
        if p.is_absolute() { p } else { root.join(p) }
    };
    let path = dir.join("post-commit");
    let quoted = format!("'{}'", war.replace('\'', r"'\''"));
    let body = format!(
        "#!/bin/sh\n{HOOK_MARK}\n# Installed by `war check install-hook`. It runs the checks whose `on`\n\
         # names `commit` or `paths:<glob>` for the commit just made. A failing\n\
         # check never undoes the commit: the run is recorded in the journal.\n\
         {quoted} check fire commit --subject \"$(git rev-parse HEAD)\" || true\n"
    );
    match std::fs::read_to_string(&path) {
        Ok(existing) if existing == body => {
            return Ok(Outcome::ok(
                format!("{path} is already this hook; nothing was written"),
                serde_json::json!({"schema": "oh.war/check-hook/v1", "hook": path.to_string(), "written": false}),
            ));
        }
        Ok(existing) if !existing.contains(HOOK_MARK) => {
            return Ok(Outcome::refused(
                "check.hook-exists",
                path.to_string(),
                format!(
                    "{path} already exists and is not OpenWarrant's; nothing was written. Add \
                     `war check fire commit --subject \"$(git rev-parse HEAD)\"` to it yourself"
                ),
            ));
        }
        _ => {}
    }
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    crate::compile::atomic::write(&path, body).map_err(|r| RepoError::Message(r.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(
            |source| RepoError::Io {
                context: format!("could not make {path} executable"),
                source,
            },
        )?;
    }
    Ok(Outcome::ok(
        format!("installed {path}: each commit fires the `commit` and `paths:` checks"),
        serde_json::json!({"schema": "oh.war/check-hook/v1", "hook": path.to_string(), "written": true}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-0d61d\n\
        uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10\nsummary: Reset.\n---\n# Reset\n\n\
        ## Checklist\n- [ ] Reset endpoint (i-9c01)\n- [ ] Email template (i-4d11)\n\n\
        ## CHK-reset · check — reset tests pass\nrun: cargo test -p accounts reset::\n\
        evaluates i-9c01, i-4d11\n";

    #[test]
    fn a_check_reads_its_defaults() {
        let c = &records_in("t-0d61d", DOC.as_bytes())[0];
        assert_eq!(c.global, "t-0d61d/CHK-reset");
        assert_eq!(c.on, vec!["done"]);
        assert_eq!(c.timeout, DEFAULT_TIMEOUT);
        assert!(c.gating);
        assert!(!c.reopen);
        assert_eq!(c.evaluates, vec!["t-0d61d/i-9c01", "t-0d61d/i-4d11"]);
        // The revision is the span's: the case corpus's 408 names it.
        assert_eq!(
            c.revision,
            "sha256:4b51160ff4bc91fc9e2527a9b794582e14c4704659816b085419472848271d45"
        );
    }

    #[test]
    fn triggers_list_and_reopen_read() {
        let doc = DOC.replace(
            "evaluates i-9c01",
            "on: commit, paths:src/**, x.deploy\nreopen: true\ngating: false\ntimeout: 5\nevaluates i-9c01",
        );
        let c = &records_in("t-0d61d", doc.as_bytes())[0];
        assert_eq!(c.on, vec!["commit", "paths:src/**", "x.deploy"]);
        assert!(c.reopen);
        assert!(!c.gating);
        assert_eq!(c.timeout, 5);
    }

    #[test]
    fn a_trigger_fires_only_what_declares_it() {
        let changed = vec!["src/a.rs".to_owned()];
        let doc = "docs/tickets/t-0d61d.md";
        assert!(fires("commit", "commit", Some(&[doc.to_owned()]), doc, &[]));
        assert!(!fires("commit", "commit", Some(&changed), doc, &[]));
        assert!(fires(
            "commit",
            "commit",
            Some(&changed),
            doc,
            &["src/**".to_owned()]
        ));
        assert!(fires("paths:src/**", "commit", Some(&changed), doc, &[]));
        assert!(!fires("paths:lib/**", "commit", Some(&changed), doc, &[]));
        assert!(fires("ci:build", "ci:build", None, doc, &[]));
        assert!(!fires("ci:build", "ci:lint", None, doc, &[]));
        assert!(fires("pr.merged", "pr.merged", None, doc, &[]));
        // A dotted extension is carried, never fired.
        assert!(!fires("x.deploy", "x.deploy", None, doc, &[]));
        // `done` never fires on a commit.
        assert!(!fires("done", "commit", Some(&changed), doc, &[]));
    }

    #[test]
    fn a_kpi_passes_meets_or_never_fails() {
        let mut c = records_in("t-0d61d", DOC.as_bytes())[0].clone();
        c.kind = "kpi".to_owned();
        c.direction = Some("min".to_owned());
        c.target = Some(120.0);
        assert_eq!(kpi_verdict(&c, 100.0).0, "PASS");
        assert_eq!(kpi_verdict(&c, 130.0).0, "FAIL");
        c.mode = Some("optimise".to_owned());
        assert_eq!(kpi_verdict(&c, 130.0).0, "PASS");
        c.mode = Some("threshold".to_owned());
        c.target = None;
        assert_eq!(kpi_verdict(&c, 1.0).0, "UNKNOWN");
    }

    #[test]
    fn cron_names_its_minutes_in_utc() {
        // 2026-10-16T12:30:00Z, a Friday.
        let t = 1_792_153_800;
        assert!(cron_matches("30 12 * * *", t));
        assert!(cron_matches("*/15 * * * *", t));
        assert!(cron_matches("0-45/30 12 16 10 5", t));
        assert!(!cron_matches("31 12 * * *", t));
        assert!(!cron_matches("30 12 * * 1", t));
        // Either restricted day field is enough.
        assert!(cron_matches("30 12 1 * 5", t));
        assert!(!cron_matches("30 12 *", t));
        assert!(!cron_matches("61 * * * *", t));
    }
}
