// SPDX-License-Identifier: Apache-2.0
//! The tick ladder and a Warrant's optional parts (OW-WAR-0148 M13).
//!
//! # The ladder
//!
//! Every tick says how it was earned, and the four ways are ordered:
//!
//! | level | earned by |
//! |---|---|
//! | `claimed` | `war done`: the performer says so |
//! | `observed` | `war done --check`: the item's tests and KPIs ran and passed, and the receipt is journalled |
//! | `independent` | a verification response by someone other than the performer, ingested |
//! | `signed` | a human's signature over the tick, verified |
//!
//! A claimed tick is written exactly as it always was, so a checklist written
//! before the ladder existed reads, byte for byte, as claimed ticks. A higher
//! level rides in the done suffix as a marker after the date
//! (`— done by claude, 2026-10-07 [observed]: note`), which a parser that
//! predates the ladder reads as part of the date: the line keeps its id, its
//! box and its note everywhere.
//!
//! The marker is what the file says. Whether a record backs it (a receipt in
//! the journal, an admissible verification, a signature that verifies) is the
//! reader's to establish; a marker nothing backs reads as `claimed`, never
//! higher.
//!
//! # Optional parts
//!
//! A Warrant may carry tests (a command whose exit code decides), KPIs (a
//! command that prints one number, a direction, an optional target) and
//! milestones (an item with a minimum level). For a ticket-encoded Warrant
//! they live in one optional atom, `atoms/30-checks.md`, plain Markdown a
//! person edits as readily as the tool:
//!
//! ```markdown
//! # Checks
//!
//! ## Tests
//!
//! - unit: `cargo test -p parser`
//! - smoke: `./smoke.sh` (for i-3f2a)
//!
//! ## KPIs
//!
//! - p95_ms: `./bench.sh p95` · min · target 120 · best
//!
//! ## Milestones
//!
//! - i-77be · min observed
//! ```
//!
//! A ticket without the file has no parts and reads exactly as before.
//!
//! This module is pure (§79.1): it parses, renders and judges. Running a
//! command, reading the journal and writing files are the CLI's.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ticket::{Fault, is_field_word, is_item_id};

/// Where a ticket keeps its optional parts, relative to its directory.
pub const CHECKS_FILE: &str = "atoms/30-checks.md";

/// What `war add --test` writes into a ticket that has no checks atom yet.
pub const CHECKS_STUB: &str = "# Checks\n\nWhat must say \"pass\" before an item ticks as observed: `war done <item> --check`\nruns the Warrant's tests and KPIs (and the item's own) and ticks only if they pass.\n";

// ---- the ladder --------------------------------------------------------------

/// How a tick was earned, weakest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// The performer said so: `war done`.
    Claimed,
    /// The item's tests and KPIs ran and passed: `war done --check`.
    Observed,
    /// Someone other than the performer verified it, with evidence.
    Independent,
    /// A human signed it off, and the signature verifies.
    Signed,
}

impl Level {
    /// The ladder, weakest first.
    pub const ALL: [Self; 4] = [
        Self::Claimed,
        Self::Observed,
        Self::Independent,
        Self::Signed,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claimed => "claimed",
            Self::Observed => "observed",
            Self::Independent => "independent",
            Self::Signed => "signed",
        }
    }

    /// A level by name, or `None`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.as_str() == name.trim())
    }

    /// What a view puts beside a tick of this level. Distinct for every
    /// level, and none of the four reads as another: a claimed tick is
    /// marked as a claim, never as checked.
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Claimed => "(claimed)",
            Self::Observed => "(observed)",
            Self::Independent => "(independent)",
            Self::Signed => "(signed)",
        }
    }

    /// How a reader is told what the level means, in plain words.
    #[must_use]
    pub const fn meaning(self) -> &'static str {
        match self {
            Self::Claimed => "the performer says so; nothing was checked",
            Self::Observed => "its tests and KPIs ran and passed",
            Self::Independent => "someone other than the performer verified it",
            Self::Signed => "a human signed it off",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A done line's date part, split from its level marker:
/// `"2026-10-07 [observed]"` is `("2026-10-07", Some(Observed))`. A date with
/// no marker, or a bracket that names no level above claimed, is returned
/// whole with `None` (claimed).
#[must_use]
pub fn split_level(done_on: &str) -> (&str, Option<Level>) {
    let t = done_on.trim_end();
    if let Some(inner) = t.strip_suffix(']')
        && let Some(open) = inner.rfind(" [")
        && let Some(level) = Level::parse(&inner[open + 2..])
        && level > Level::Claimed
    {
        return (inner[..open].trim_end(), Some(level));
    }
    (done_on, None)
}

/// The date part a tick at `level` writes: the date alone for a claimed
/// tick (the bytes a tick always had), the date and its marker otherwise.
#[must_use]
pub fn with_level(date: &str, level: Level) -> String {
    match level {
        Level::Claimed => date.to_owned(),
        other => format!("{date} [{}]", other.as_str()),
    }
}

/// The minimum a profile's `[ticks]` sets (OW-WAR-0148 M13): what every item
/// of the type must reach, what its milestones must reach, and, by the
/// record's `type` field, what that type's milestones must reach. Absent
/// everywhere a profile declares none: every minimum is `claimed`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TicksDecl {
    pub item: Option<Level>,
    pub milestone: Option<Level>,
    /// `[ticks.types] bug = "observed"`: a ticket of type `bug` holds its
    /// milestones to `observed`.
    pub types: BTreeMap<String, Level>,
}

impl TicksDecl {
    /// The least an item must reach: the profile's `item` minimum, raised
    /// for a milestone by the profile's `milestone` minimum, the type's, and
    /// the milestone's own. With the source of the deciding minimum, for the
    /// message that names it.
    #[must_use]
    pub fn minimum(
        &self,
        kind: Option<&str>,
        milestone: Option<Option<Level>>,
    ) -> (Level, MinimumSource) {
        let mut best = (self.item.unwrap_or(Level::Claimed), MinimumSource::Default);
        if self.item.is_some_and(|l| l > Level::Claimed) {
            best.1 = MinimumSource::ProfileItem;
        }
        let mut raise = |level: Level, source: MinimumSource| {
            if level > best.0 {
                best = (level, source);
            }
        };
        if let Some(own) = milestone {
            if let Some(l) = self.milestone {
                raise(l, MinimumSource::ProfileMilestone);
            }
            if let Some(l) = kind.and_then(|k| self.types.get(k)) {
                raise(*l, MinimumSource::Type);
            }
            if let Some(l) = own {
                raise(l, MinimumSource::Milestone);
            }
        }
        best
    }
}

/// Which declaration set an item's minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MinimumSource {
    /// Nothing raised it: `claimed`.
    Default,
    /// The profile's `[ticks] item`.
    ProfileItem,
    /// The profile's `[ticks] milestone`.
    ProfileMilestone,
    /// The profile's `[ticks.types] <type>`.
    Type,
    /// The milestone's own `min` (`war add --milestone ... --min`).
    Milestone,
}

impl MinimumSource {
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Default => "nothing raises it",
            Self::ProfileItem => "the profile's [ticks] item minimum",
            Self::ProfileMilestone => "the profile's [ticks] milestone minimum",
            Self::Type => "the minimum the ticket's type sets ([ticks.types])",
            Self::Milestone => "the milestone's own minimum",
        }
    }
}

// ---- the parts ---------------------------------------------------------------

/// Which way a KPI improves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Max,
    Min,
}

impl Direction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Max => "max",
            Self::Min => "min",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "max" => Some(Self::Max),
            "min" => Some(Self::Min),
            _ => None,
        }
    }

    /// Whether `a` is better than `b`.
    #[must_use]
    pub fn better(self, a: f64, b: f64) -> bool {
        match self {
            Self::Max => a > b,
            Self::Min => a < b,
        }
    }
}

/// What a KPI is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KpiMode {
    /// The default: pass or fail against the target, and the best value kept.
    #[default]
    Best,
    /// Pass or fail against the target only.
    Threshold,
    /// A signal only: never decides a tick; the best value is kept.
    Optimise,
}

impl KpiMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Best => "best",
            Self::Threshold => "threshold",
            Self::Optimise => "optimise",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "best" => Some(Self::Best),
            "threshold" => Some(Self::Threshold),
            "optimise" | "optimize" => Some(Self::Optimise),
            _ => None,
        }
    }

    /// Whether the best value is tracked.
    #[must_use]
    pub const fn tracks_best(self) -> bool {
        matches!(self, Self::Best | Self::Optimise)
    }
}

/// A test: a command whose exit code decides pass or fail.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Test {
    pub name: String,
    pub cmd: String,
    /// The item it belongs to; `None` for the whole Warrant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// 0-based line in the atom.
    #[serde(skip)]
    pub line: usize,
}

/// A KPI: a command that prints one number.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Kpi {
    pub name: String,
    pub cmd: String,
    pub direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
    pub mode: KpiMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip)]
    pub line: usize,
}

/// A milestone: an item with a minimum level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Milestone {
    pub item: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<Level>,
    #[serde(skip)]
    pub line: usize,
}

/// What one KPI value means against its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Judged {
    /// At or beyond the target.
    Pass,
    /// Short of the target.
    Fail,
    /// Kept, deciding nothing: an `optimise` KPI, or one with no target.
    Recorded,
}

impl Judged {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Recorded => "recorded",
        }
    }
}

impl Kpi {
    /// Whether this KPI can decide a tick: it has a target and is not
    /// `optimise`.
    #[must_use]
    pub fn gates(&self) -> bool {
        self.mode != KpiMode::Optimise && self.target.is_some()
    }

    /// `value` against the target.
    #[must_use]
    pub fn judge(&self, value: f64) -> Judged {
        match (self.gates(), self.target) {
            (true, Some(t)) => {
                let ok = match self.direction {
                    Direction::Max => value >= t,
                    Direction::Min => value <= t,
                };
                if ok { Judged::Pass } else { Judged::Fail }
            }
            _ => Judged::Recorded,
        }
    }

    /// The best of `values` in this KPI's direction, or `None` when there
    /// are none (or the mode keeps no best).
    #[must_use]
    pub fn best(&self, values: &[f64]) -> Option<f64> {
        if !self.mode.tracks_best() {
            return None;
        }
        values.iter().copied().fold(None, |acc, v| match acc {
            Some(b) if !self.direction.better(v, b) => Some(b),
            _ => Some(v),
        })
    }

    /// The line `war add --kpi` writes (no terminator).
    #[must_use]
    pub fn render(&self) -> String {
        let mut line = format!(
            "- {}: {} · {}",
            self.name,
            code_span(&self.cmd),
            self.direction.as_str()
        );
        if let Some(t) = self.target {
            line.push_str(&format!(" · target {}", number(t)));
        }
        line.push_str(&format!(" · {}", self.mode.as_str()));
        if let Some(i) = &self.item {
            line.push_str(&format!(" (for {i})"));
        }
        line
    }
}

impl Test {
    /// The line `war add --test` writes (no terminator).
    #[must_use]
    pub fn render(&self) -> String {
        let mut line = format!("- {}: {}", self.name, code_span(&self.cmd));
        if let Some(i) = &self.item {
            line.push_str(&format!(" (for {i})"));
        }
        line
    }
}

impl Milestone {
    /// The line `war add --milestone` writes (no terminator).
    #[must_use]
    pub fn render(&self) -> String {
        match self.min {
            Some(l) => format!("- {} · min {}", self.item, l.as_str()),
            None => format!("- {}", self.item),
        }
    }
}

/// A number as a person writes it: `120`, `0.95`.
#[must_use]
pub fn number(v: f64) -> String {
    format!("{v}")
}

/// The number a KPI command printed: the whole output when it is one number,
/// else the last non-empty line when that line is one number. Anything else
/// (no output, words, two numbers, NaN, an infinity) is `None`: the run is
/// UNKNOWN, never a pass and never a fail.
#[must_use]
pub fn parse_number(stdout: &str) -> Option<f64> {
    let one = |s: &str| -> Option<f64> {
        let s = s.trim();
        if s.is_empty() || s.split_whitespace().count() != 1 {
            return None;
        }
        s.parse::<f64>().ok().filter(|v| v.is_finite())
    };
    one(stdout).or_else(|| {
        stdout
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .and_then(one)
    })
}

/// `cmd` as a Markdown code span that reads back as exactly `cmd`.
#[must_use]
pub fn code_span(cmd: &str) -> String {
    let longest = cmd.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    let both_spaced = cmd.len() > 1 && cmd.starts_with(' ') && cmd.ends_with(' ');
    if cmd.starts_with('`') || cmd.ends_with('`') || both_spaced {
        format!("{fence} {cmd} {fence}")
    } else {
        format!("{fence}{cmd}{fence}")
    }
}

/// A code span at the start of `s`: (its content, the rest).
fn read_code_span(s: &str) -> Option<(String, &str)> {
    let n = s.chars().take_while(|c| *c == '`').count();
    if n == 0 {
        return None;
    }
    let body = &s[n..];
    let mut at = 0;
    while at < body.len() {
        let rest = &body[at..];
        let run = rest.chars().take_while(|c| *c == '`').count();
        if run == 0 {
            at += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        }
        if run == n {
            let mut content = body[..at].to_owned();
            if content.len() >= 2
                && content.starts_with(' ')
                && content.ends_with(' ')
                && !content.trim().is_empty()
            {
                content = content[1..content.len() - 1].to_owned();
            }
            return Some((content, &body[at + n..]));
        }
        at += run;
    }
    None
}

// ---- the atom ----------------------------------------------------------------

/// Which section of the checks atom a line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Tests,
    Kpis,
    Milestones,
}

impl Section {
    #[must_use]
    pub const fn heading(self) -> &'static str {
        match self {
            Self::Tests => "## Tests",
            Self::Kpis => "## KPIs",
            Self::Milestones => "## Milestones",
        }
    }

    fn of(line: &str) -> Option<Option<Self>> {
        let t = line.trim_end();
        if !t.starts_with("## ") {
            return None;
        }
        Some(match t.to_ascii_lowercase().as_str() {
            "## tests" => Some(Self::Tests),
            "## kpis" => Some(Self::Kpis),
            "## milestones" => Some(Self::Milestones),
            _ => None,
        })
    }
}

/// A parsed checks atom: every part in file order, and the faults found.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Checks {
    pub tests: Vec<Test>,
    pub kpis: Vec<Kpi>,
    pub milestones: Vec<Milestone>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub faults: Vec<Fault>,
}

impl Checks {
    /// Whether nothing is declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tests.is_empty() && self.kpis.is_empty() && self.milestones.is_empty()
    }

    /// The tests and KPIs that apply to `item` (`None`: the whole Warrant):
    /// the Warrant's own, then the item's.
    #[must_use]
    pub fn for_item(&self, item: Option<&str>) -> (Vec<&Test>, Vec<&Kpi>) {
        let applies = |of: &Option<String>| of.is_none() || of.as_deref() == item;
        (
            self.tests.iter().filter(|t| applies(&t.item)).collect(),
            self.kpis.iter().filter(|k| applies(&k.item)).collect(),
        )
    }

    /// The milestone entry for `item`, if it is one.
    #[must_use]
    pub fn milestone(&self, item: &str) -> Option<&Milestone> {
        self.milestones.iter().find(|m| m.item == item)
    }

    /// Every name a test or KPI already uses.
    #[must_use]
    pub fn names(&self) -> BTreeSet<String> {
        self.tests
            .iter()
            .map(|t| t.name.clone())
            .chain(self.kpis.iter().map(|k| k.name.clone()))
            .collect()
    }

    /// The first `test-N` no part uses.
    #[must_use]
    pub fn next_test_name(&self) -> String {
        let names = self.names();
        (1..)
            .map(|n| format!("test-{n}"))
            .find(|n| !names.contains(n))
            .unwrap_or_else(|| "test".to_owned())
    }
}

/// `- <name>: <code span><rest>` → (name, cmd, rest).
fn named_command(body: &str) -> Result<(String, String, &str), String> {
    let Some((name, rest)) = body.split_once(": ") else {
        return Err("a part is `- <name>: `<command>`...`".to_owned());
    };
    let name = name.trim();
    if !is_field_word(name) {
        return Err(format!(
            "name {name:?} is not a word (lowercase, [a-z0-9_-], 1 to 40)"
        ));
    }
    let Some((cmd, rest)) = read_code_span(rest.trim_start()) else {
        return Err(format!(
            "{name}: the command is a code span, `like this`, and none closes here"
        ));
    };
    if cmd.trim().is_empty() {
        return Err(format!("{name}: an empty command"));
    }
    Ok((name.to_owned(), cmd, rest))
}

/// A trailing ` (for i-x)`: (the rest without it, the item).
fn split_for(rest: &str) -> Result<(&str, Option<String>), String> {
    let t = rest.trim_end();
    if let Some(inner) = t.strip_suffix(')')
        && let Some(open) = inner.rfind("(for ")
    {
        let item = inner[open + 5..].trim();
        if !is_item_id(item) {
            return Err(format!("`(for {item})` names no item (i-x)"));
        }
        return Ok((&inner[..open], Some(item.to_owned())));
    }
    Ok((t, None))
}

fn attributes(rest: &str) -> Vec<&str> {
    rest.split('·')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .collect()
}

/// Parse a checks atom. Never fails: faults are collected, and every
/// well-formed line is still a part.
#[must_use]
pub fn parse(text: &str) -> Checks {
    let mut out = Checks::default();
    let mut section: Option<Section> = None;
    let mut fence: Option<&str> = None;
    let mut seen_names: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen_milestones: BTreeMap<String, usize> = BTreeMap::new();
    for (index, raw) in text.split_inclusive('\n').enumerate() {
        let line = raw.trim_end_matches(['\n', '\r']);
        let t = line.trim_start();
        for marker in ["```", "~~~"] {
            if t.starts_with(marker) && line.len() - t.len() <= 3 {
                fence = match fence {
                    Some(open) if open == marker => None,
                    Some(open) => Some(open),
                    None => Some(marker),
                };
            }
        }
        if fence.is_some() || t.starts_with("```") || t.starts_with("~~~") {
            continue;
        }
        if let Some(s) = Section::of(line) {
            section = s;
            continue;
        }
        if line.starts_with("# ") {
            section = None;
            continue;
        }
        let Some(sec) = section else { continue };
        let Some(body) = line.strip_prefix("- ") else {
            continue;
        };
        let fault = |message: String| Fault {
            rule: "checks.malformed",
            line: index + 1,
            message,
        };
        let mut dup = |name: &str, out: &mut Checks| {
            if let Some(first) = seen_names.get(name) {
                out.faults.push(Fault {
                    rule: "checks.duplicate",
                    line: index + 1,
                    message: format!(
                        "{name} is also the name on line {}; a name says which part ran",
                        first + 1
                    ),
                });
                true
            } else {
                seen_names.insert(name.to_owned(), index);
                false
            }
        };
        match sec {
            Section::Tests => {
                let parsed = named_command(body).and_then(|(name, cmd, rest)| {
                    let (rest, item) = split_for(rest)?;
                    if !rest.trim().is_empty() {
                        return Err(format!(
                            "{name}: {:?} after the command; a test is `- <name>: `<command>`` \
                             and an optional `(for i-x)`",
                            rest.trim()
                        ));
                    }
                    Ok(Test {
                        name,
                        cmd,
                        item,
                        line: index,
                    })
                });
                match parsed {
                    Ok(test) => {
                        if !dup(&test.name, &mut out) {
                            out.tests.push(test);
                        }
                    }
                    Err(why) => out.faults.push(fault(why)),
                }
            }
            Section::Kpis => {
                let parsed = named_command(body).and_then(|(name, cmd, rest)| {
                    let (rest, item) = split_for(rest)?;
                    let mut direction = None;
                    let mut target = None;
                    let mut mode = None;
                    for a in attributes(rest) {
                        if let Some(d) = Direction::parse(a) {
                            direction = Some(d);
                        } else if let Some(n) = a.strip_prefix("target ") {
                            target = Some(
                                n.trim()
                                    .parse::<f64>()
                                    .ok()
                                    .filter(|v| v.is_finite())
                                    .ok_or_else(|| {
                                        format!("{name}: target {:?} is not a number", n.trim())
                                    })?,
                            );
                        } else if let Some(m) = KpiMode::parse(a) {
                            mode = Some(m);
                        } else {
                            return Err(format!(
                                "{name}: {a:?}; a KPI says `max` or `min`, optionally `target \
                                 <N>`, and `best`, `threshold` or `optimise`"
                            ));
                        }
                    }
                    let direction = direction.ok_or_else(|| {
                        format!("{name}: a KPI says which way is better: max or min")
                    })?;
                    let mode = mode.unwrap_or_default();
                    if mode == KpiMode::Threshold && target.is_none() {
                        return Err(format!(
                            "{name}: a threshold KPI needs a target to pass or fail against"
                        ));
                    }
                    Ok(Kpi {
                        name,
                        cmd,
                        direction,
                        target,
                        mode,
                        item,
                        line: index,
                    })
                });
                match parsed {
                    Ok(kpi) => {
                        if !dup(&kpi.name, &mut out) {
                            out.kpis.push(kpi);
                        }
                    }
                    Err(why) => out.faults.push(fault(why)),
                }
            }
            Section::Milestones => {
                let mut parts = body.split('·').map(str::trim);
                let item = parts.next().unwrap_or_default();
                if !is_item_id(item) {
                    out.faults.push(fault(format!(
                        "{item:?}: a milestone names an item of this ticket (i-x)"
                    )));
                    continue;
                }
                let mut min = None;
                let mut bad = None;
                for a in parts {
                    match a.strip_prefix("min ").and_then(Level::parse) {
                        Some(l) => min = Some(l),
                        None => {
                            bad = Some(format!(
                                "{item}: {a:?}; a milestone says `min <claimed|observed|independent|signed>`"
                            ));
                        }
                    }
                }
                if let Some(why) = bad {
                    out.faults.push(fault(why));
                    continue;
                }
                if let Some(first) = seen_milestones.get(item) {
                    out.faults.push(Fault {
                        rule: "checks.duplicate",
                        line: index + 1,
                        message: format!("{item} is a milestone on line {} already", first + 1),
                    });
                    continue;
                }
                seen_milestones.insert(item.to_owned(), index);
                out.milestones.push(Milestone {
                    item: item.to_owned(),
                    min,
                    line: index,
                });
            }
        }
    }
    out
}

/// `text` with `line` added at the end of `section`, the section created at
/// the end of the file when it is absent. Every existing byte is kept.
#[must_use]
pub fn append(text: &str, section: Section, line: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let start = lines
        .iter()
        .position(|l| Section::of(l.trim_end_matches(['\n', '\r'])) == Some(Some(section)));
    let mut out = String::with_capacity(text.len() + line.len() + 32);
    let Some(start) = start else {
        out.push_str(text);
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        if !out.is_empty() && !out.ends_with(&format!("{newline}{newline}")) {
            out.push_str(newline);
        }
        out.push_str(section.heading());
        out.push_str(newline);
        out.push_str(newline);
        out.push_str(line);
        out.push_str(newline);
        return out;
    };
    // The section runs to the next heading; the new line goes after its last
    // list item, or after the heading's blank line when it has none.
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with('#'))
        .map_or(lines.len(), |p| start + 1 + p);
    let last_item = (start + 1..end).rev().find(|&i| lines[i].starts_with("- "));
    let insert_after = last_item.unwrap_or_else(|| {
        if start + 1 < end && lines[start + 1].trim().is_empty() {
            start + 1
        } else {
            start
        }
    });
    for (i, l) in lines.iter().enumerate() {
        out.push_str(l);
        if i == insert_after {
            if !l.ends_with('\n') {
                out.push_str(newline);
            }
            if last_item.is_none() && insert_after == start {
                out.push_str(newline);
            }
            out.push_str(line);
            out.push_str(newline);
            if last_item.is_none() && i + 1 < lines.len() && !lines[i + 1].trim().is_empty() {
                out.push_str(newline);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ATOM: &str = "# Checks\n\nProse a person wrote (with a `span`).\n\n## Tests\n\n- unit: `cargo test -p parser`\n- smoke: `` echo `x` `` (for i-3f2a)\n\n## KPIs\n\n- p95_ms: `./bench.sh p95` · min · target 120 · best\n- cov: `./cov.sh` · max · optimise (for i-3f2a)\n\n## Milestones\n\n- i-77be · min observed\n- i-0001\n\n```\n- fenced: `not a part`\n```\n";

    #[test]
    fn the_ladder_is_ordered_and_every_marker_is_distinct() {
        assert!(Level::Claimed < Level::Observed);
        assert!(Level::Observed < Level::Independent);
        assert!(Level::Independent < Level::Signed);
        let markers: BTreeSet<&str> = Level::ALL.iter().map(|l| l.marker()).collect();
        assert_eq!(markers.len(), 4);
        assert!(!Level::Claimed.marker().contains("observed"));
        assert!(!Level::Claimed.meaning().contains("verified"));
    }

    #[test]
    fn a_marker_rides_after_the_date_and_claimed_writes_none() {
        assert_eq!(with_level("2026-10-07", Level::Claimed), "2026-10-07");
        assert_eq!(
            with_level("2026-10-07", Level::Observed),
            "2026-10-07 [observed]"
        );
        assert_eq!(
            split_level("2026-10-07 [observed]"),
            ("2026-10-07", Some(Level::Observed))
        );
        assert_eq!(split_level("2026-10-07"), ("2026-10-07", None));
        // A bracket that names no level, or names claimed, is the date's.
        assert_eq!(
            split_level("2026-10-07 [later]"),
            ("2026-10-07 [later]", None)
        );
        assert_eq!(
            split_level("2026-10-07 [claimed]"),
            ("2026-10-07 [claimed]", None)
        );
    }

    #[test]
    fn a_ticked_line_with_a_marker_reads_back_through_the_checklist_parser() {
        let item = crate::ticket::Item::new("i-3f2a", "Parse it", Vec::new()).ticked(
            "claude",
            &with_level("2026-10-07", Level::Signed),
            Some("shipped"),
        );
        let line = item.render();
        assert_eq!(
            line,
            "- [x] Parse it (i-3f2a) — done by claude, 2026-10-07 [signed]: shipped"
        );
        let c = crate::ticket::parse(&format!("{line}\n"));
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        let back = &c.items[0];
        assert_eq!(back.id.as_deref(), Some("i-3f2a"));
        assert_eq!(back.note.as_deref(), Some("shipped"));
        let (date, level) = split_level(back.done_on.as_deref().unwrap_or_default());
        assert_eq!((date, level), ("2026-10-07", Some(Level::Signed)));
    }

    #[test]
    fn the_atom_reads_every_part_and_skips_prose_and_fences() {
        let c = parse(ATOM);
        assert!(c.faults.is_empty(), "{:?}", c.faults);
        assert_eq!(c.tests.len(), 2);
        assert_eq!(c.tests[0].cmd, "cargo test -p parser");
        assert_eq!(c.tests[1].cmd, "echo `x`");
        assert_eq!(c.tests[1].item.as_deref(), Some("i-3f2a"));
        assert_eq!(c.kpis.len(), 2);
        assert_eq!(c.kpis[0].direction, Direction::Min);
        assert_eq!(c.kpis[0].target, Some(120.0));
        assert_eq!(c.kpis[1].mode, KpiMode::Optimise);
        assert_eq!(c.milestones.len(), 2);
        assert_eq!(
            c.milestone("i-77be").and_then(|m| m.min),
            Some(Level::Observed)
        );
        // Rendering an unchanged part gives back its line.
        for t in &c.tests {
            assert_eq!(t.render(), ATOM.lines().nth(t.line).expect("line"));
        }
        for k in &c.kpis {
            assert_eq!(k.render(), ATOM.lines().nth(k.line).expect("line"));
        }
        for m in &c.milestones {
            assert_eq!(m.render(), ATOM.lines().nth(m.line).expect("line"));
        }
        let (tests, kpis) = c.for_item(None);
        assert_eq!((tests.len(), kpis.len()), (1, 1));
        let (tests, kpis) = c.for_item(Some("i-3f2a"));
        assert_eq!((tests.len(), kpis.len()), (2, 2));
    }

    #[test]
    fn every_fault_is_named_with_its_line() {
        let text = "## Tests\n\n- no command here\n- a: `x`\n- a: `y`\n## KPIs\n- k: `z` · up\n- t: `z` · max · threshold\n## Milestones\n- nope\n- i-0001 · min great\n";
        let c = parse(text);
        let rules: Vec<(&str, usize)> = c.faults.iter().map(|f| (f.rule, f.line)).collect();
        assert!(rules.contains(&("checks.malformed", 3)), "{rules:?}");
        assert!(rules.contains(&("checks.duplicate", 5)), "{rules:?}");
        assert!(rules.contains(&("checks.malformed", 7)), "{rules:?}");
        assert!(rules.contains(&("checks.malformed", 8)), "{rules:?}");
        assert!(rules.contains(&("checks.malformed", 10)), "{rules:?}");
        assert!(rules.contains(&("checks.malformed", 11)), "{rules:?}");
        assert_eq!(c.tests.len(), 1);
    }

    #[test]
    fn appending_keeps_every_byte_and_creates_a_missing_section() {
        let with_test = append(CHECKS_STUB, Section::Tests, "- unit: `cargo test`");
        assert!(with_test.starts_with(CHECKS_STUB));
        assert!(with_test.ends_with("\n\n## Tests\n\n- unit: `cargo test`\n"));
        let two = append(&with_test, Section::Tests, "- lint: `cargo clippy`");
        assert!(two.ends_with("- unit: `cargo test`\n- lint: `cargo clippy`\n"));
        let kpi = append(&two, Section::Kpis, "- k: `x` · max · best");
        assert!(kpi.starts_with(&two));
        // Into a section that sits between two others: after its last item,
        // and nothing after it moves.
        let mid = append(ATOM, Section::Tests, "- third: `true`");
        assert!(mid.contains("(for i-3f2a)\n- third: `true`\n\n## KPIs"));
        assert_eq!(mid.len(), ATOM.len() + "- third: `true`\n".len());
        assert_eq!(parse(&mid).tests.len(), 3);
    }

    #[test]
    fn a_code_span_round_trips_any_command() {
        for cmd in ["cargo test", "echo `x`", "`lead", "a``b", " spaced "] {
            let line = Test {
                name: "t".into(),
                cmd: cmd.into(),
                item: None,
                line: 0,
            }
            .render();
            let back = parse(&format!("## Tests\n{line}\n"));
            assert!(back.faults.is_empty(), "{cmd:?}: {:?}", back.faults);
            assert_eq!(back.tests[0].cmd, cmd, "{line}");
        }
    }

    #[test]
    fn a_number_is_one_number_or_unknown() {
        assert_eq!(parse_number("42\n"), Some(42.0));
        assert_eq!(parse_number("running...\n0.93\n"), Some(0.93));
        assert_eq!(parse_number("  -1.5e2  "), Some(-150.0));
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("fast"), None);
        assert_eq!(parse_number("12 ms"), None);
        assert_eq!(parse_number("NaN"), None);
        assert_eq!(parse_number("inf"), None);
    }

    #[test]
    fn a_kpi_judges_by_direction_and_keeps_its_best() {
        let k = Kpi {
            name: "p95".into(),
            cmd: "x".into(),
            direction: Direction::Min,
            target: Some(100.0),
            mode: KpiMode::Best,
            item: None,
            line: 0,
        };
        assert_eq!(k.judge(90.0), Judged::Pass);
        assert_eq!(k.judge(100.0), Judged::Pass);
        assert_eq!(k.judge(101.0), Judged::Fail);
        assert_eq!(k.best(&[120.0, 95.0, 110.0]), Some(95.0));
        let up = Kpi {
            direction: Direction::Max,
            ..k.clone()
        };
        assert_eq!(up.best(&[120.0, 95.0, 110.0]), Some(120.0));
        let only = Kpi {
            mode: KpiMode::Threshold,
            ..k.clone()
        };
        assert_eq!(only.best(&[1.0]), None);
        let signal = Kpi {
            mode: KpiMode::Optimise,
            ..k.clone()
        };
        assert!(!signal.gates());
        assert_eq!(signal.judge(1e9), Judged::Recorded);
        let untargeted = Kpi { target: None, ..k };
        assert_eq!(untargeted.judge(5.0), Judged::Recorded);
    }

    #[test]
    fn the_minimum_is_the_highest_declaration_that_applies() {
        let none = TicksDecl::default();
        assert_eq!(none.minimum(None, None).0, Level::Claimed);
        assert_eq!(
            none.minimum(None, Some(Some(Level::Observed))).0,
            Level::Observed
        );
        let mut d = TicksDecl {
            item: None,
            milestone: Some(Level::Observed),
            types: BTreeMap::new(),
        };
        d.types.insert("bug".into(), Level::Independent);
        // A plain item is not a milestone.
        assert_eq!(d.minimum(Some("bug"), None).0, Level::Claimed);
        assert_eq!(
            d.minimum(Some("task"), Some(None)),
            (Level::Observed, MinimumSource::ProfileMilestone)
        );
        assert_eq!(
            d.minimum(Some("bug"), Some(Some(Level::Observed))),
            (Level::Independent, MinimumSource::Type)
        );
        assert_eq!(
            d.minimum(Some("bug"), Some(Some(Level::Signed))),
            (Level::Signed, MinimumSource::Milestone)
        );
    }
}
