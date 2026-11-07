// SPDX-License-Identifier: Apache-2.0
//! What the records say was spent: `war evidence go` node tokens and runtime
//! capture receipts, and nothing else (`docs/design/tui-usage.md`). Reads
//! only. Anything not recorded stays UNKNOWN, and zero is never shown for
//! "nothing recorded".

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;

use super::model::Count;
use crate::repo::Repository;

/// A price for one model, dollars per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

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
        models.insert(
            model.clone(),
            Price {
                input_per_mtok: rate("input_per_mtok")?,
                output_per_mtok: rate("output_per_mtok")?,
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
}

/// An estimate from a configured price table, and what it left out.
#[derive(Debug, Clone, PartialEq)]
pub struct Dollars {
    pub estimate: f64,
    pub as_of: String,
    /// Recorded tokens with no model or no price, so in no dollar figure.
    pub unpriced_tokens: u64,
}

impl Usage {
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
        let price = prices.and_then(|p| p.models.get(r.model.as_deref()?));
        match (price, r.input, r.output) {
            (Some(p), Some(i), Some(o)) => {
                priced_any = true;
                dollars += (i as f64).mul_add(p.input_per_mtok, o as f64 * p.output_per_mtok)
                    / 1_000_000.0;
            }
            _ => unpriced = unpriced.saturating_add(n),
        }
    }
    if any {
        usage.total = Count::Known(total);
    }
    if let Some(p) = prices.filter(|_| priced_any) {
        usage.dollars = Some(Dollars {
            estimate: dollars,
            as_of: p.as_of.clone(),
            unpriced_tokens: unpriced,
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
        ..Run::default()
    })
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
            runs.extend(
                journal
                    .events
                    .iter()
                    .filter(|e| e.event_type == crate::go::event::ENDED)
                    .filter_map(|e| run_of_node(&e.payload)),
            );
        }
        captures(dir, dir.file_name().unwrap_or("?"), &mut runs, &mut faults);
    }
    (sum(&runs, table.as_ref()), faults)
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
            estimated: false,
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
