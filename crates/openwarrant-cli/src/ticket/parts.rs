// SPDX-License-Identifier: Apache-2.0
//! `war plan add <id> --test/--kpi/--milestone/--min` (OW-WAR-0148 M13): the
//! optional parts any Warrant may carry, a title-only one included.
//!
//! A ticket's parts live in its optional checks atom, `atoms/30-checks.md`
//! ([`openwarrant_core::ticks`]): written here on the first part, and absent
//! from every ticket that has none, so those read exactly as before. Each
//! part added is one line appended to its section, and one
//! `ticket.part_added` in the journal.
//!
//! - a **test** is a command whose exit code decides pass or fail;
//! - a **KPI** is a command that prints one number, a direction (`max` or
//!   `min`), an optional target and a mode (`best`, the default: pass or
//!   fail against the target and the best value kept; `threshold`: pass or
//!   fail only; `optimise`: a signal only, never deciding a tick);
//! - a **milestone** is an item with a minimum level: `--milestone "<text>"`
//!   adds the item, `--min` on an existing item makes it one.
//!
//! On a ticket (`t-x`) a test or KPI is the Warrant's own and applies to
//! every item; on an item (`t-x/i-y`), or given with the text of a new item
//! in the same command, it is that item's.

use openwarrant_core::ticks::{self, Direction, Kpi, KpiMode, Level, Milestone, Section};

use super::{Outcome, Store, Target, rewrite_or_create};
use crate::repo::RepoError;

/// A KPI as `war plan add --kpi` gives it.
#[derive(Debug, Clone, Default)]
pub struct KpiSpec {
    pub name: String,
    pub cmd: Option<String>,
    pub direction: Option<String>,
    pub target: Option<f64>,
    pub mode: Option<String>,
}

/// What `war plan add` is asked for beside an item's text.
#[derive(Debug, Clone, Default)]
pub struct PartsArgs {
    pub tests: Vec<String>,
    /// The name of the one test given; `test-N` otherwise.
    pub name: Option<String>,
    pub kpi: Option<KpiSpec>,
    /// The text of a new item that is a milestone.
    pub milestone: Option<String>,
    /// The milestone's minimum level, or (on an existing item, alone) the
    /// minimum that makes it a milestone.
    pub min: Option<String>,
}

impl PartsArgs {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tests.is_empty()
            && self.kpi.is_none()
            && self.milestone.is_none()
            && self.min.is_none()
    }
}

fn refused(rule: &str, message: impl Into<String>) -> Outcome {
    Outcome::refused(rule, String::new(), message)
}

/// Why tests cannot be written as given: `war create --test` asks this before
/// anything exists, so a refused test leaves no half-made Warrant behind.
pub(super) fn refuse_tests(tests: &[String]) -> Option<Outcome> {
    if tests.iter().any(|c| c.trim().is_empty()) {
        return Some(refused("ticket.test-empty", "--test needs a command"));
    }
    if tests.iter().any(|c| c.trim().contains(['\r', '\n'])) {
        return Some(refused(
            "ticket.part-malformed",
            "a run is one command line; put a multiline program in a script file",
        ));
    }
    None
}

/// `war create --sign`: `created`, raised to an Order in place. A refusal of
/// either step is the answer.
pub fn signed(store: &Store, created: Outcome) -> Result<Outcome, RepoError> {
    let Some(id) = created.result["id"].as_str().map(str::to_owned) else {
        return Ok(created);
    };
    if created.is_refused() {
        return Ok(created);
    }
    let raised = promote(store, &id, &[])?;
    if raised.is_refused() {
        return Ok(raised);
    }
    let mut outcome = created;
    outcome.human = format!("{}\n{}", outcome.human, raised.human);
    outcome.result["level"] = raised.result["level"].clone();
    Ok(outcome)
}

/// The item a promotion adds: what a person signs off at the end.
pub const SIGN_OFF_ITEM: &str = "Signed off by a person";

/// `war plan promote <id>`: raise an item's level by adding the field that
/// makes it, in place (t-3c16c, the lean model). Plain, a required signature:
/// a milestone with `min: signed`, so it is an Order. With `tests`, a trigger,
/// so it is a Warrant. No document is written and the id does not change; an
/// item already at the level it asks for is refused, not added to.
pub fn promote(store: &Store, query: &str, tests: &[String]) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let index = match super::resolve(&tickets, query) {
        Ok(Target::Ticket(n) | Target::Item(n, _)) => n,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let record = store.rel(if t.format { &t.intent_path } else { &t.dir });
    if let Some(w) = &t.manifest.promoted_to {
        return Ok(Outcome::refused(
            "ticket.already-promoted",
            record,
            format!(
                "{} was promoted into {w} already; work the contract there",
                t.id()
            ),
        ));
    }
    let id = t.id().to_owned();
    let before = crate::work::ticket_level(store, t);
    if tests.is_empty() && before == crate::work::Level::Order {
        return Ok(Outcome::refused(
            "ticket.already-promoted",
            record,
            format!("{id} already needs a person's signature"),
        ));
    }
    let args = if tests.is_empty() {
        PartsArgs {
            milestone: Some(SIGN_OFF_ITEM.to_owned()),
            min: Some(Level::Signed.as_str().to_owned()),
            ..PartsArgs::default()
        }
    } else {
        PartsArgs {
            tests: tests.to_vec(),
            ..PartsArgs::default()
        }
    };
    let added = add(store, &id, None, &[], &args, None)?;
    if added.is_refused() {
        return Ok(added);
    }
    let (reread, _) = store.load_all()?;
    let Some(t) = reread.iter().find(|t| t.id() == id) else {
        return Err(RepoError::Message(format!(
            "{id} could not be read back after its promotion"
        )));
    };
    let level = crate::work::ticket_level(store, t);
    store.journal(
        t,
        super::event::PROMOTED,
        &serde_json::json!({"warrant": id, "level": level, "parts": added.result["parts"]}),
    )?;
    let then = if tests.is_empty() {
        format!(
            "a person signs it off at the end: `war sign {id}/{} --ssh-sign`",
            added.result["item"].as_str().unwrap_or("<item>")
        )
    } else {
        "`war done` finishes it only when its test passes".to_owned()
    };
    Ok(Outcome::ok(
        format!("{id} is now {} level, in place: {then}", level.as_str()),
        serde_json::json!({"schema": "oh.war/ticket-promoted/v1", "ticket": id, "warrant": id,
            "dir": record, "level": level, "parts": added.result["parts"]}),
    ))
}

/// `war plan add <ticket|item> [text] --test/--kpi/--milestone/--min`: the parts
/// written to the checks atom, and the item (when there is text or a
/// milestone) to the checklist through `war plan add`'s own path.
pub fn add(
    store: &Store,
    query: &str,
    text: Option<&str>,
    after: &[String],
    args: &PartsArgs,
    if_rev: Option<&str>,
) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, target_item) = match super::resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => (n, None),
        Ok(Target::Item(n, i)) => (n, Some(i)),
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let what = target_item
        .as_ref()
        .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
    // M11: a stale `--if-rev` writes nothing, part or item.
    if let Some(refusal) = super::check_if_rev(store, t, target_item.as_deref(), &what, if_rev) {
        return Ok(refusal);
    }
    let checks_path = if t.format {
        t.intent_path.clone()
    } else {
        t.dir.join(ticks::CHECKS_FILE)
    };
    let existing = super::ladder::checks_of(t);

    // Refusals first: nothing is written unless every part is well-formed.
    if text.is_some() && args.milestone.is_some() {
        return Ok(refused(
            "ticket.add-two-items",
            "an item's text and --milestone each add an item; give one (a milestone is an item \
             with a minimum: `war plan add <ticket> --milestone \"<text>\" --min observed`)",
        ));
    }
    let min = match args.min.as_deref() {
        None => None,
        Some(m) => match Level::parse(m) {
            Some(l) => Some(l),
            None => {
                return Ok(refused(
                    "ticket.min-unknown",
                    format!("--min {m:?}: a level is claimed, observed, independent or signed"),
                ));
            }
        },
    };
    let marks_existing = min.is_some() && args.milestone.is_none() && text.is_none();
    if marks_existing && target_item.is_none() {
        return Ok(refused(
            "ticket.min-needs-item",
            format!(
                "--min sets a milestone's minimum: give it with --milestone \"<text>\", or name \
                 an existing item (`war plan add {}/<item> --min {}`)",
                t.id(),
                min.map_or("observed", Level::as_str)
            ),
        ));
    }
    if marks_existing
        && let Some(i) = &target_item
        && existing.milestone(i).is_some()
    {
        return Ok(refused(
            "ticket.milestone-exists",
            format!(
                "{}/{i} is a milestone already; edit its line in {} to change its minimum",
                t.id(),
                store.rel(&checks_path)
            ),
        ));
    }
    let tests: Vec<String> = args
        .tests
        .iter()
        .map(|c| c.trim().to_owned())
        .filter(|c| !c.is_empty())
        .collect();
    if tests.len() != args.tests.len() {
        return Ok(refused("ticket.test-empty", "--test needs a command"));
    }
    if args.name.is_some() && tests.len() != 1 {
        return Ok(refused(
            "ticket.test-name",
            "--name names one test; give exactly one --test with it",
        ));
    }
    let mut names = existing.names();
    if t.format {
        names.extend(
            super::checks::of_ticket(t)
                .iter()
                .map(|c| c.id.strip_prefix("CHK-").unwrap_or(&c.id).to_owned()),
        );
    }
    let mut test_names = Vec::new();
    for _ in &tests {
        let name = match &args.name {
            Some(n) => n.trim().to_owned(),
            None => (1..)
                .map(|n| format!("test-{n}"))
                .find(|n| !names.contains(n))
                .unwrap_or_else(|| "test".to_owned()),
        };
        if !openwarrant_core::ticket::is_field_word(&name) {
            return Ok(refused(
                "ticket.part-name",
                format!("--name {name:?} is not a word (lowercase, [a-z0-9_-], 1 to 40)"),
            ));
        }
        if !names.insert(name.clone()) {
            return Ok(refused(
                "ticket.part-exists",
                format!(
                    "{} already has a test or KPI named {name}; give another --name",
                    t.id()
                ),
            ));
        }
        test_names.push(name);
    }
    let kpi = match &args.kpi {
        None => None,
        Some(k) => match kpi_of(k, &names) {
            Ok(kpi) => Some(kpi),
            Err(refusal) => return Ok(*refusal),
        },
    };

    // The item, when the command adds one: through `war plan add`'s own path, so
    // it is named, journalled and placed exactly as any other item.
    let mut item_added = None;
    let mut human = Vec::new();
    let new_text = text.or(args.milestone.as_deref());
    if let Some(item_text) = new_text {
        let added = super::add(store, query, item_text, after, if_rev)?;
        if added.is_refused() {
            return Ok(added);
        }
        item_added = added.result["item"].as_str().map(str::to_owned);
        human.push(added.human.clone());
    }
    let scope = item_added.clone().or_else(|| target_item.clone());

    let mut lines: Vec<(Section, String, serde_json::Value)> = Vec::new();
    for (name, cmd) in test_names.iter().zip(&tests) {
        let part = ticks::Test {
            name: name.clone(),
            cmd: cmd.clone(),
            item: scope.clone(),
            line: 0,
        };
        lines.push((
            Section::Tests,
            part.render(),
            serde_json::json!({"kind": "test", "name": name, "cmd": cmd, "item": scope}),
        ));
    }
    if let Some(mut k) = kpi {
        k.item = scope.clone();
        lines.push((
            Section::Kpis,
            k.render(),
            serde_json::json!({"kind": "kpi", "name": k.name, "cmd": k.cmd,
                "direction": k.direction.as_str(), "target": k.target, "mode": k.mode.as_str(),
                "item": scope}),
        ));
    }
    let milestone_item = if args.milestone.is_some() {
        item_added.clone()
    } else if marks_existing {
        target_item.clone()
    } else {
        None
    };
    if let Some(i) = &milestone_item {
        let m = Milestone {
            item: i.clone(),
            min,
            line: 0,
        };
        lines.push((
            Section::Milestones,
            m.render(),
            serde_json::json!({"kind": "milestone", "item": i, "min": min}),
        ));
    }
    if lines.is_empty() {
        return Ok(refused(
            "ticket.add-nothing",
            "nothing to add: give an item's text, --test, --kpi or --milestone",
        ));
    }
    if t.format {
        let values: Vec<_> = lines
            .iter()
            .map(|(_, _, payload)| payload.clone())
            .collect();
        super::authoring::append_parts(store, t, &values)?;
        for value in &values {
            store.journal(t, super::ladder::event::PART_ADDED, value)?;
        }
        return Ok(Outcome::ok(
            format!("{}: added parts to {}", t.id(), store.rel(&checks_path)),
            serde_json::json!({"schema":"oh.war/ticket-parts/v1","ticket":t.id(),"item":item_added,"scope":scope,"parts":values,"file":store.rel(&checks_path)}),
        ));
    }
    let written = rewrite_or_create(&checks_path, ticks::CHECKS_STUB, |current| {
        let mut next = current.to_owned();
        for (section, line, _) in &lines {
            next = ticks::append(&next, *section, line);
        }
        let parsed = ticks::parse(&next);
        if parsed.faults.len() > ticks::parse(current).faults.len() {
            return Err(Box::new(Outcome::refused(
                "ticket.part-malformed",
                store.rel(&checks_path),
                format!(
                    "the part would not read back: {}; nothing was written",
                    parsed
                        .faults
                        .iter()
                        .map(|f| f.message.clone())
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            )));
        }
        Ok((next, ()))
    })?;
    if let Err(refusal) = written {
        return Ok(*refusal);
    }
    for (_, line, payload) in &lines {
        store.journal(t, super::ladder::event::PART_ADDED, payload)?;
        human.push(format!("added to {}: {line}", store.rel(&checks_path)));
    }
    let after = super::ladder::checks_of(t);
    let (min_now, source) = match &milestone_item {
        Some(i) => super::ladder::minimum(store, t, &after, Some(i)),
        None => (Level::Claimed, ticks::MinimumSource::Default),
    };
    if let Some(i) = &milestone_item {
        let target = format!("{}/{i}", t.id());
        human.push(format!(
            "{target} is a milestone: it ticks at {} or above ({}), with {}",
            min_now.as_str(),
            source.describe(),
            super::ladder::command_for(min_now, &target)
        ));
    }
    if !tests.is_empty() || args.kpi.is_some() {
        let target = scope
            .as_ref()
            .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
        human.push(format!(
            "`war done {target} --check` runs {} and ticks at observed only when they pass",
            if scope.is_some() {
                "the Warrant's checks and this item's"
            } else {
                "them"
            }
        ));
    }
    Ok(Outcome::ok(
        human.join("\n"),
        serde_json::json!({
            "schema": "oh.war/ticket-parts/v1",
            "ticket": t.id(),
            "item": item_added,
            "scope": scope,
            "parts": lines.iter().map(|(_, _, p)| p.clone()).collect::<Vec<_>>(),
            "file": store.rel(&checks_path),
        }),
    ))
}

fn kpi_of(k: &KpiSpec, names: &std::collections::BTreeSet<String>) -> Result<Kpi, Box<Outcome>> {
    let bad = |rule: &str, m: String| Box::new(refused(rule, m));
    let name = k.name.trim().to_owned();
    if !openwarrant_core::ticket::is_field_word(&name) {
        return Err(bad(
            "ticket.part-name",
            format!("--kpi {name:?} is not a word (lowercase, [a-z0-9_-], 1 to 40)"),
        ));
    }
    if names.contains(&name) {
        return Err(bad(
            "ticket.part-exists",
            format!("a test or KPI is already named {name}"),
        ));
    }
    let Some(cmd) = k.cmd.as_deref().map(str::trim).filter(|c| !c.is_empty()) else {
        return Err(bad(
            "ticket.kpi-cmd",
            format!("--kpi {name} needs --cmd \"<a command that prints one number>\""),
        ));
    };
    let Some(direction) = k.direction.as_deref().and_then(Direction::parse) else {
        return Err(bad(
            "ticket.kpi-direction",
            format!("--kpi {name} needs --direction max or --direction min: which way is better"),
        ));
    };
    let mode = match k.mode.as_deref() {
        None => KpiMode::default(),
        Some(m) => KpiMode::parse(m).ok_or_else(|| {
            bad(
                "ticket.kpi-mode",
                format!("--mode {m:?}: best (the default), threshold or optimise"),
            )
        })?,
    };
    if let Some(t) = k.target
        && !t.is_finite()
    {
        return Err(bad(
            "ticket.kpi-target",
            "--target is a finite number".to_owned(),
        ));
    }
    if mode == KpiMode::Threshold && k.target.is_none() {
        return Err(bad(
            "ticket.kpi-target",
            format!("--kpi {name} --mode threshold passes or fails against a --target; give one"),
        ));
    }
    Ok(Kpi {
        name,
        cmd: cmd.to_owned(),
        direction,
        target: k.target,
        mode,
        item: None,
        line: 0,
    })
}
