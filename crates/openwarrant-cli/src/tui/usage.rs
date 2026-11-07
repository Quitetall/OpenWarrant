// SPDX-License-Identifier: Apache-2.0
//! What the records say was spent: `war evidence go` node tokens, runtime
//! capture receipts and `work.usage` session events, and nothing else
//! (`docs/design/tui-usage.md`). Reads only. Anything not recorded stays
//! UNKNOWN, and zero is never shown for "nothing recorded".

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;

use super::model::Count;
use crate::repo::Repository;

/// A price for one model, dollars per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    /// Unset, a run with cache reads is not priced.
    pub cache_read_per_mtok: Option<f64>,
    /// Unset, a run with cache writes is not priced.
    pub cache_write_per_mtok: Option<f64>,
}

/// A price table older than this many days is shown as stale.
pub const STALE_DAYS: i64 = 90;

/// `[usage]` in `openwarrant.toml`: a dated table, or none.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Prices {
    pub as_of: String,
    pub models: BTreeMap<String, Price>,
}

/// What `[usage]` says. An undated table is refused: a price with no date
/// reads as a fact it is not.
pub fn prices(text: &str) -> Result<Option<Prices>, String> {
    let doc: toml::Table = toml::from_str(text).map_err(|e| format!("openwarrant.toml: {e}"))?;
    let Some(usage) = doc.get("usage") else {
        return Ok(None);
    };
    let Some(table) = usage.get("prices").and_then(toml::Value::as_table) else {
        return Ok(None);
    };
    let as_of = usage
        .get("prices_as_of")
        .and_then(toml::Value::as_str)
        .filter(|d| d.len() == 10 && chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok())
        .ok_or("[usage] prices need prices_as_of = \"YYYY-MM-DD\"; no dollars are shown")?;
    let mut models = BTreeMap::new();
    for (model, price) in table {
        let rate = |key: &str| {
            price
                .get(key)
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .filter(|r| r.is_finite() && *r >= 0.0)
                .ok_or_else(|| format!("[usage.prices.\"{model}\"] needs a number {key}"))
        };
        let optional = |key: &str| price.get(key).map(|_| rate(key)).transpose();
        models.insert(
            model.clone(),
            Price {
                input_per_mtok: rate("input_per_mtok")?,
                output_per_mtok: rate("output_per_mtok")?,
                cache_read_per_mtok: optional("cache_read_per_mtok")?,
                cache_write_per_mtok: optional("cache_write_per_mtok")?,
            },
        );
    }
    Ok(Some(Prices {
        as_of: as_of.to_owned(),
        models,
    }))
}

/// One run the records mention.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Run {
    /// The node or Warrant it was for.
    pub item: String,
    /// Binds a capture to a node's run, so one run is not counted twice.
    pub dispatch: Option<String>,
    /// `None`: it ran, and no number was recorded.
    pub tokens: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub model: Option<String>,
    /// The provider marked these counts as its own estimate.
    pub estimated: bool,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    /// Who recorded it, as the journal names them; empty for a capture.
    pub actor: String,
    /// When it was recorded (RFC 3339), when known; its first ten
    /// characters are the day.
    pub at: String,
    /// Which record: `go`, `capture`, or a `work.usage` source.
    pub source: String,
}

impl Run {
    /// The day it was recorded, or `UNKNOWN`.
    #[must_use]
    pub fn day(&self) -> &str {
        self.at.get(..10).unwrap_or("UNKNOWN")
    }
}

/// A run in the total, with its dollar estimate when one could be made.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Counted {
    pub run: Run,
    pub dollars: Option<f64>,
}

/// One row of a breakdown.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Group {
    pub key: String,
    pub tokens: u64,
    pub runs: usize,
    /// The estimate over the runs that could be priced.
    pub dollars: Option<f64>,
    /// Tokens of runs that could not be priced.
    pub unpriced: u64,
}

/// The sum of the records.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub total: Count,
    /// Runs that ran and recorded no number.
    pub unknown_runs: usize,
    /// Tokens a provider flagged as estimated, kept out of the total.
    pub estimated: u64,
    /// Recorded tokens by node or Warrant.
    pub by_item: BTreeMap<String, u64>,
    pub dollars: Option<Dollars>,
    /// The runs in the total, each with its estimate.
    pub counted: Vec<Counted>,
}

/// An estimate from a configured price table, and what it left out.
#[derive(Debug, Clone, PartialEq)]
pub struct Dollars {
    pub estimate: f64,
    pub as_of: String,
    /// Recorded tokens with no model or no price, so in no dollar figure.
    pub unpriced_tokens: u64,
    /// Days since `as_of`, when that is more than [`STALE_DAYS`].
    pub stale_days: Option<i64>,
}

/// Days from `as_of` to `today` (`YYYY-MM-DD` both), when over [`STALE_DAYS`].
#[must_use]
pub fn stale(as_of: &str, today: &str) -> Option<i64> {
    let d = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    let days = (d(today)? - d(as_of)?).num_days();
    (days > STALE_DAYS).then_some(days)
}

/// A run's estimate under `p`: only with both counts, and with a cache rate
/// for every cache count it has. Anything less is no estimate at all.
fn price_of(r: &Run, p: &Price) -> Option<f64> {
    let (i, o) = (r.input?, r.output?);
    let cache = |n: Option<u64>, rate: Option<f64>| match n.unwrap_or(0) {
        0 => Some(0.0),
        n => rate.map(|rate| n as f64 * rate),
    };
    let extra =
        cache(r.cache_read, p.cache_read_per_mtok)? + cache(r.cache_write, p.cache_write_per_mtok)?;
    Some(((i as f64).mul_add(p.input_per_mtok, o as f64 * p.output_per_mtok) + extra) / 1_000_000.0)
}

impl Usage {
    /// The runs in the total, grouped by `key`, largest first.
    #[must_use]
    pub fn group(&self, key: impl Fn(&Run) -> String) -> Vec<Group> {
        let mut groups: BTreeMap<String, Group> = BTreeMap::new();
        for c in &self.counted {
            let k = key(&c.run);
            let g = groups.entry(k.clone()).or_insert_with(|| Group {
                key: k,
                ..Group::default()
            });
            let n = c.run.tokens.unwrap_or(0);
            g.tokens = g.tokens.saturating_add(n);
            g.runs += 1;
            match c.dollars {
                Some(d) => g.dollars = Some(g.dollars.unwrap_or(0.0) + d),
                None => g.unpriced = g.unpriced.saturating_add(n),
            }
        }
        let mut out: Vec<Group> = groups.into_values().collect();
        out.sort_by(|a, b| b.tokens.cmp(&a.tokens).then_with(|| a.key.cmp(&b.key)));
        out
    }

    /// The recorded tokens of `id` and everything under it.
    #[must_use]
    pub fn of(&self, id: &str) -> Count {
        let prefix = format!("{id}/");
        let mut known = None;
        for (item, n) in &self.by_item {
            if item == id || item.starts_with(&prefix) {
                known = Some(known.unwrap_or(0) + n);
            }
        }
        known.map_or(Count::Unknown, Count::Known)
    }
}

/// Add up `runs`. A capture naming a dispatch a node already counted is that
/// node's run, not a second one.
#[must_use]
pub fn sum(runs: &[Run], prices: Option<&Prices>) -> Usage {
    let counted: BTreeSet<&str> = runs
        .iter()
        .filter(|r| r.input.is_none() && r.tokens.is_some())
        .filter_map(|r| r.dispatch.as_deref())
        .collect();
    let mut usage = Usage::default();
    let (mut total, mut any) = (0u64, false);
    let (mut dollars, mut unpriced, mut priced_any) = (0.0, 0u64, false);
    for r in runs {
        let again = r.input.is_some() && r.dispatch.as_deref().is_some_and(|d| counted.contains(d));
        if again {
            continue;
        }
        let Some(n) = r.tokens else {
            usage.unknown_runs += 1;
            continue;
        };
        if r.estimated {
            usage.estimated = usage.estimated.saturating_add(n);
            continue;
        }
        any = true;
        total = total.saturating_add(n);
        *usage.by_item.entry(r.item.clone()).or_insert(0) += n;
        let estimate = prices
            .and_then(|p| p.models.get(r.model.as_deref()?))
            .and_then(|p| price_of(r, p));
        match estimate {
            Some(d) => {
                priced_any = true;
                dollars += d;
            }
            None => unpriced = unpriced.saturating_add(n),
        }
        usage.counted.push(Counted {
            run: r.clone(),
            dollars: estimate,
        });
    }
    if any {
        usage.total = Count::Known(total);
    }
    if let Some(p) = prices.filter(|_| priced_any) {
        usage.dollars = Some(Dollars {
            estimate: dollars,
            as_of: p.as_of.clone(),
            unpriced_tokens: unpriced,
            stale_days: None,
        });
    }
    usage
}

/// The runs a `go.ended` journal payload records. Anything else is `None`.
#[must_use]
pub fn run_of_node(payload: &str) -> Option<Run> {
    let p: serde_json::Value = serde_json::from_str(payload).ok()?;
    Some(Run {
        item: p.get("node")?.as_str()?.to_owned(),
        dispatch: p
            .get("dispatch_id")
            .and_then(|d| d.as_str())
            .map(str::to_owned),
        // A harness that reported nothing wrote `"unknown"`.
        tokens: p.get("tokens").and_then(serde_json::Value::as_u64),
        source: "go".to_owned(),
        ..Run::default()
    })
}

/// The runs a `work.usage` payload records: one per model it names, or one
/// with no number when it says `"unknown"`. Anything else is none.
#[must_use]
pub fn runs_of_session(payload: &str, actor: &str, occurred_at: &str) -> Vec<Run> {
    let Ok(p) = serde_json::from_str::<serde_json::Value>(payload) else {
        return Vec::new();
    };
    let Some(item) = p.get("target").and_then(|v| v.as_str()) else {
        return Vec::new();
    };
    let base = Run {
        item: item.to_owned(),
        actor: actor.to_owned(),
        at: occurred_at.to_owned(),
        source: p
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("session")
            .to_owned(),
        ..Run::default()
    };
    let models = p.get("models").and_then(|m| m.as_array());
    match (p.get("tokens").and_then(serde_json::Value::as_u64), models) {
        (Some(_), Some(models)) if !models.is_empty() => models
            .iter()
            .map(|m| {
                let n = |k: &str| m.get(k).and_then(serde_json::Value::as_u64);
                Run {
                    tokens: n("tokens"),
                    input: n("input"),
                    output: n("output"),
                    cache_read: n("cache_read"),
                    cache_write: n("cache_write"),
                    model: m.get("model").and_then(|v| v.as_str()).map(str::to_owned),
                    ..base.clone()
                }
            })
            .collect(),
        (Some(n), _) => vec![Run {
            tokens: Some(n),
            ..base
        }],
        (None, _) => vec![base],
    }
}

/// The run a capture's raw provider receipt records.
#[must_use]
pub fn run_of_receipt(alias: &str, dispatch: Option<String>, raw: &serde_json::Value) -> Run {
    let input = raw.get("input_tokens").and_then(serde_json::Value::as_u64);
    let output = raw.get("output_tokens").and_then(serde_json::Value::as_u64);
    Run {
        item: alias.to_owned(),
        dispatch,
        tokens: input.zip(output).map(|(i, o)| i.saturating_add(o)),
        input,
        output,
        model: raw
            .get("provider_model_identity")
            .and_then(|m| m.as_array())
            .and_then(|m| m.iter().find_map(|v| v.as_str()))
            .map(str::to_owned),
        estimated: raw
            .get("usage_estimated")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        source: "capture".to_owned(),
        ..Run::default()
    }
}

fn captures(dir: &Utf8Path, alias: &str, runs: &mut Vec<Run>, faults: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir.join("runtime-receipts")) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(hex) = name
            .strip_prefix("capture-")
            .and_then(|n| n.strip_suffix(".json"))
        else {
            continue;
        };
        let record = std::fs::read(entry.path())
            .map_err(|e| e.to_string())
            .and_then(|b| {
                crate::runtime_capture::inspect_retained(&b, alias, hex)
                    .map_err(|e| format!("{e:?}"))
            });
        let Ok(record) = record else {
            faults.push(format!("the capture {alias}/{name} could not be read"));
            continue;
        };
        let raw = record["receipt"]["base64"]
            .as_str()
            .and_then(|b| openwarrant_core::attestation::base64_decode(b).ok())
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
        let dispatch = record["declared_capture"]["dispatch_id"]
            .as_str()
            .map(str::to_owned);
        runs.push(raw.map_or_else(
            || Run {
                item: alias.to_owned(),
                dispatch: dispatch.clone(),
                source: "capture".to_owned(),
                ..Run::default()
            },
            |raw| run_of_receipt(alias, dispatch.clone(), &raw),
        ));
    }
}

/// Every run this repository's records mention.
pub fn read(repo: &Repository, tickets: &[crate::ticket::Ticket]) -> (Usage, Vec<String>) {
    let mut faults = Vec::new();
    let table = std::fs::read_to_string(repo.root.join(crate::init::CONFIG_FILE))
        .map_err(|e| e.to_string())
        .and_then(|text| prices(&text))
        .unwrap_or_else(|why| {
            faults.push(why);
            None
        });
    let mut dirs: BTreeSet<camino::Utf8PathBuf> = tickets.iter().map(|t| t.dir.clone()).collect();
    dirs.extend(repo.warrant_dirs().unwrap_or_default());
    let mut runs = Vec::new();
    for dir in &dirs {
        if let Ok(journal) = crate::journal_cmd::load(dir) {
            for e in &journal.events {
                if e.event_type == crate::go::event::ENDED {
                    runs.extend(run_of_node(&e.payload));
                } else if crate::journal_cmd::work_type(&e.event_type)
                    == crate::session_usage::EVENT
                {
                    runs.extend(runs_of_session(&e.payload, &e.actor_ref, &e.occurred_at));
                }
            }
        }
        captures(dir, dir.file_name().unwrap_or("?"), &mut runs, &mut faults);
    }
    let mut usage = sum(&runs, table.as_ref());
    if let Some(d) = usage.dollars.as_mut() {
        let today = crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs());
        d.stale_days = stale(&d.as_of, &today[..10]);
    }
    (usage, faults)
}

/// Dollars to the cent, or to a hundredth of one below a cent, so a small
/// estimate does not read as nothing.
#[must_use]
pub fn money(d: f64) -> String {
    if d > 0.0 && d < 0.01 {
        format!("{d:.4}")
    } else {
        format!("{d:.2}")
    }
}

/// One breakdown row: `t-1a2b  12345 tok  ~$0.12 est.` Dollars only from
/// the dated table, and the tokens it could not price said beside them.
fn group_line(kind: &str, g: &Group, priced: bool) -> String {
    let mut out = format!("{kind} {}  {} tok", g.key, g.tokens);
    if priced {
        match g.dollars {
            Some(d) => out.push_str(&format!("  ~${} est.", money(d))),
            None => out.push_str("  no price"),
        }
        if g.dollars.is_some() && g.unpriced > 0 {
            out.push_str(&format!(" ({} tok unpriced)", g.unpriced));
        }
    }
    out
}

/// The breakdowns, each largest first except days, newest first.
fn breakdown(u: &Usage) -> [(&'static str, Vec<Group>); 4] {
    let or_unknown = |s: Option<&str>| s.filter(|s| !s.is_empty()).unwrap_or("UNKNOWN").to_owned();
    let mut days = u.group(|r| r.day().to_owned());
    days.sort_by(|a, b| b.key.cmp(&a.key));
    [
        (
            "ticket",
            u.group(|r| r.item.split('/').next().unwrap_or(&r.item).to_owned()),
        ),
        ("actor", u.group(|r| or_unknown(Some(&r.actor)))),
        ("model", u.group(|r| or_unknown(r.model.as_deref()))),
        ("day", days),
    ]
}

/// The Records Usage view: the total, then by ticket, actor, model and day.
#[must_use]
pub fn rows(u: &Usage) -> Vec<super::model::Row> {
    let row = |text: String| super::model::Row {
        text,
        command: "war status --usage".to_owned(),
        ..super::model::Row::default()
    };
    let mut out = vec![row(format!("TOTAL {}", super::watch::usage_line(u)))];
    if u.counted.is_empty() {
        out.push(row(
            "no usage recorded: UNKNOWN, not zero (`war done` records a session's)".to_owned(),
        ));
        return out;
    }
    let priced = u.dollars.is_some();
    for (kind, groups) in breakdown(u) {
        out.extend(groups.iter().map(|g| row(group_line(kind, g, priced))));
    }
    out
}

fn group_json(groups: &[Group]) -> serde_json::Value {
    serde_json::Value::Array(
        groups
            .iter()
            .map(|g| {
                serde_json::json!({
                    "key": g.key, "tokens": g.tokens, "runs": g.runs,
                    "dollars_estimate": g.dollars, "unpriced_tokens": g.unpriced,
                })
            })
            .collect(),
    )
}

/// `war status --usage`: a few lines, and `oh.war/usage/v1`.
#[must_use]
pub fn report(repo: &Repository, tickets: &[crate::ticket::Ticket]) -> (String, serde_json::Value) {
    let (u, faults) = read(repo, tickets);
    let total = match u.total {
        Count::Known(n) => serde_json::json!(n),
        Count::Unknown => serde_json::json!("unknown"),
    };
    let [tickets_g, actors, models, days] = breakdown(&u).map(|(_, g)| g);
    let value = serde_json::json!({
        "schema": "oh.war/usage/v1",
        "tokens": total,
        "unknown_runs": u.unknown_runs,
        "provider_estimated_tokens": u.estimated,
        "dollars": u.dollars.as_ref().map(|d| serde_json::json!({
            "estimate": d.estimate, "prices_as_of": d.as_of,
            "stale_days": d.stale_days, "unpriced_tokens": d.unpriced_tokens,
        })),
        "by_ticket": group_json(&tickets_g),
        "by_actor": group_json(&actors),
        "by_model": group_json(&models),
        "by_day": group_json(&days),
        "faults": faults,
        "note": "provider-reported counts; unverified, and they grant no assurance",
    });
    let mut human = vec![format!("tokens recorded: {}", super::watch::usage_line(&u))];
    let priced = u.dollars.is_some();
    let top = |kind: &str, groups: &[Group], human: &mut Vec<String>| {
        for g in groups.iter().take(5) {
            human.push(format!("  {}", group_line(kind, g, priced)));
        }
    };
    top("ticket", &tickets_g, &mut human);
    top("model", &models, &mut human);
    for f in &faults {
        human.push(format!("  UNKNOWN {f}"));
    }
    (human.join("\n"), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(item: &str, tokens: Option<u64>) -> Run {
        Run {
            item: item.to_owned(),
            dispatch: Some(format!("d-{item}")),
            tokens,
            ..Run::default()
        }
    }

    fn receipt(item: &str, model: &str, i: u64, o: u64) -> Run {
        Run {
            item: item.to_owned(),
            dispatch: Some(format!("d-{item}")),
            tokens: Some(i + o),
            input: Some(i),
            output: Some(o),
            model: Some(model.to_owned()),
            ..Run::default()
        }
    }

    fn table() -> Prices {
        prices(
            "[usage]\nprices_as_of = \"2026-11-01\"\n\
             [usage.prices.m]\ninput_per_mtok = 2\noutput_per_mtok = 10.0\n",
        )
        .expect("a table")
        .expect("present")
    }

    #[test]
    fn nothing_recorded_is_unknown_and_not_zero() {
        let usage = sum(&[], None);
        assert_eq!(usage.total, Count::Unknown);
        assert_eq!(usage.of("t-1"), Count::Unknown);
        assert_eq!(sum(&[node("t-1", None)], None).total, Count::Unknown);
    }

    #[test]
    fn a_total_counts_what_was_reported_and_says_what_was_not() {
        let usage = sum(&[node("t-1/i-1", Some(40)), node("t-1/i-2", None)], None);
        assert_eq!(usage.total, Count::Known(40));
        assert_eq!(usage.unknown_runs, 1);
        assert_eq!(usage.of("t-1"), Count::Known(40));
        assert_eq!(usage.of("t-1/i-2"), Count::Unknown);
        assert_eq!(
            usage.of("t-11"),
            Count::Unknown,
            "a prefix of an id is not it"
        );
    }

    #[test]
    fn a_node_says_unknown_as_the_journal_writes_it() {
        let run =
            run_of_node(r#"{"node":"t-1","dispatch_id":"d","tokens":"unknown"}"#).expect("a run");
        assert_eq!(run.tokens, None);
        let run = run_of_node(r#"{"node":"t-1","tokens":1234}"#).expect("a run");
        assert_eq!(run.tokens, Some(1234));
        assert!(run_of_node("not json").is_none());
    }

    #[test]
    fn a_receipt_is_read_for_counts_and_model() {
        let raw = serde_json::json!({
            "input_tokens": 10, "output_tokens": 5, "usage_estimated": false,
            "provider_model_identity": ["m", "rev"],
        });
        let run = run_of_receipt("OW-WAR-1", None, &raw);
        assert_eq!(run.tokens, Some(15));
        assert_eq!(run.model.as_deref(), Some("m"));
        let run = run_of_receipt("OW-WAR-1", None, &serde_json::json!({"outcome": "x"}));
        assert_eq!(run.tokens, None, "a receipt with no counts records none");
    }

    /// Refusal: a provider's estimate is not a measurement, so it stays out
    /// of the total, which is still UNKNOWN when nothing else was recorded.
    #[test]
    fn an_estimate_stays_out_of_the_total() {
        let mut run = receipt("t-1", "m", 10, 5);
        run.estimated = true;
        let usage = sum(&[run], None);
        assert_eq!(usage.total, Count::Unknown);
        assert_eq!(usage.estimated, 15);
    }

    /// Refusal: one run seen by both sources is counted once.
    #[test]
    fn a_capture_of_a_counted_node_is_not_counted_again() {
        let usage = sum(&[node("t-1", Some(15)), receipt("t-1", "m", 10, 5)], None);
        assert_eq!(usage.total, Count::Known(15));
        let usage = sum(&[node("t-1", None), receipt("t-1", "m", 10, 5)], None);
        assert_eq!(
            usage.total,
            Count::Known(15),
            "the receipt had the only count"
        );
    }

    #[test]
    fn dollars_come_only_from_a_dated_table_and_a_named_model() {
        let p = table();
        let usage = sum(&[receipt("t-1", "m", 1_000_000, 1_000_000)], Some(&p));
        let d = usage.dollars.expect("priced");
        assert!((d.estimate - 12.0).abs() < 1e-9);
        assert_eq!(d.as_of, "2026-11-01");
        assert_eq!(sum(&[receipt("t-1", "m", 1, 1)], None).dollars, None);
    }

    /// Refusal: tokens with no model, or a model with no price, get none.
    #[test]
    fn what_cannot_be_priced_is_listed_not_averaged() {
        let p = table();
        assert_eq!(sum(&[node("t-1", Some(9))], Some(&p)).dollars, None);
        assert_eq!(
            sum(&[receipt("t-1", "other", 1, 1)], Some(&p)).dollars,
            None
        );
        let usage = sum(
            &[receipt("t-1", "m", 1_000_000, 0), node("t-2", Some(9))],
            Some(&p),
        );
        assert_eq!(usage.dollars.expect("priced").unpriced_tokens, 9);
    }

    #[test]
    fn a_session_event_is_one_run_per_model_and_unknown_is_not_zero() {
        let p = r#"{"target":"t-1/i-2","source":"claude-code","tokens":30,
            "models":[{"model":"m","input":1,"output":2,"cache_read":7,"tokens":10},
                      {"model":"n","input":5,"output":15,"tokens":20}]}"#;
        let runs = runs_of_session(p, "alice", "2026-11-07T10:00:00Z");
        assert_eq!(runs.len(), 2);
        assert_eq!(
            (runs[0].actor.as_str(), runs[0].day()),
            ("alice", "2026-11-07")
        );
        let usage = sum(&runs, None);
        assert_eq!(usage.total, Count::Known(30));
        assert_eq!(usage.of("t-1"), Count::Known(30));
        let by_model = usage.group(|r| r.model.clone().unwrap_or_default());
        assert_eq!((by_model[0].key.as_str(), by_model[0].tokens), ("n", 20));
        let unknown = runs_of_session(
            r#"{"target":"t-1","tokens":"unknown","reason":"no transcript"}"#,
            "a",
            "2026-11-07T10:00:00Z",
        );
        let usage = sum(&unknown, None);
        assert_eq!(usage.total, Count::Unknown);
        assert_eq!(usage.unknown_runs, 1);
    }

    /// Refusal: cache tokens with no cache rate are not priced at a guess.
    #[test]
    fn cache_tokens_need_their_own_rate() {
        let mut run = receipt("t-1", "m", 1_000_000, 0);
        run.cache_read = Some(1_000_000);
        run.tokens = Some(2_000_000);
        let usage = sum(std::slice::from_ref(&run), Some(&table()));
        assert_eq!(usage.dollars, None);
        let rated = prices(
            "[usage]\nprices_as_of = \"2026-11-01\"\n\
             [usage.prices.m]\ninput_per_mtok = 2\noutput_per_mtok = 10\ncache_read_per_mtok = 0.5\n",
        )
        .expect("a table")
        .expect("present");
        let d = sum(&[run], Some(&rated)).dollars.expect("priced");
        assert!((d.estimate - 2.5).abs() < 1e-9);
    }

    #[test]
    fn an_old_price_table_is_stale() {
        assert_eq!(stale("2026-01-01", "2026-11-07"), Some(310));
        assert_eq!(stale("2026-11-01", "2026-11-07"), None);
        assert_eq!(stale("bad", "2026-11-07"), None);
    }

    /// Refusal: an undated table shows no dollars at all.
    #[test]
    fn an_undated_price_table_is_refused() {
        let undated = "[usage.prices.m]\ninput_per_mtok = 2\noutput_per_mtok = 10\n";
        assert!(prices(undated).is_err());
        let bad_date = "[usage]\nprices_as_of = \"soon\"\n[usage.prices.m]\ninput_per_mtok = 2\noutput_per_mtok = 10\n";
        assert!(prices(bad_date).is_err());
        assert_eq!(prices("[project]\nname = \"x\"\n"), Ok(None));
    }
}
