// SPDX-License-Identifier: Apache-2.0
//! Compact, read-only view over the same live records as `war status`.
use openwarrant_core::status::{CorpusStatus, WarrantRung};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Row {
    alias: String,
    title: String,
    phase: String,
    assessment: WarrantRung,
    outcome: String,
    unmet: Vec<String>,
    blocking_unknowns: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    schema: &'static str,
    status_model: &'static str,
    qualification_assessed: bool,
    total: usize,
    recorded_resolutions: usize,
    remaining: usize,
    includes_resolved: bool,
    note: &'static str,
    warrants: Vec<Row>,
}

pub fn build(corpus: CorpusStatus, all: bool) -> Overview {
    let total = corpus.warrants.len();
    let recorded_resolutions = corpus
        .warrants
        .iter()
        .filter(|w| w.rung == WarrantRung::Resolved)
        .count();
    let warrants = corpus
        .warrants
        .into_iter()
        .filter(|w| all || w.rung != WarrantRung::Resolved)
        .map(|w| Row {
            alias: w.alias,
            title: w
                .title
                .unwrap_or_else(|| "(invalid or untitled record)".into()),
            phase: w
                .state
                .as_ref()
                .map_or("unknown", |s| s.phase.as_str())
                .into(),
            assessment: w.rung,
            outcome: w
                .state
                .as_ref()
                .map_or("unknown", |s| s.outcome.as_str())
                .into(),
            unmet: w.unmet,
            blocking_unknowns: w.blocking_unknowns,
        })
        .collect();
    Overview {
        schema: "oh.war/overview/v1",
        status_model: "legacy-records",
        qualification_assessed: false,
        total,
        recorded_resolutions,
        remaining: total - recorded_resolutions,
        includes_resolved: all,
        note: "Live legacy records. Remaining means unresolved, not necessarily unfinished implementation. Resolution is not a new Verified mark. No execution permission is inferred. Inspect a Warrant's current scope and input prerequisites before work.",
        warrants,
    }
}

fn cell(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() || c == '|' { ' ' } else { c })
        .collect()
}

pub fn render(view: &Overview) -> String {
    let mut text = format!(
        "# OpenWarrant overview\n\n{} remaining legacy records | {} recorded resolutions | {} total\n\n{}\n\n",
        view.remaining, view.recorded_resolutions, view.total, view.note
    );
    if view.warrants.is_empty() {
        text.push_str("No records in this view.\n");
    } else {
        text.push_str("| Warrant | Title | Recorded phase | Assessment | Unmet / unknowns |\n| --- | --- | --- | --- | --- |\n");
        for row in &view.warrants {
            let rung = match row.assessment {
                WarrantRung::Invalid => "invalid",
                WarrantRung::Draft => "draft",
                WarrantRung::ReadyToResolve => "ready_to_resolve",
                WarrantRung::WouldSatisfy => "would_satisfy",
                WarrantRung::Resolved => "resolved",
            };
            text.push_str(&format!(
                "| {} | {} | {} | {} | {} / {} |\n",
                cell(&row.alias),
                cell(&row.title),
                cell(&row.phase),
                rung,
                row.unmet.len(),
                row.blocking_unknowns.len()
            ));
        }
    }
    text.push_str("\nDetails: `war status <alias> --json`. Human acts: `war next --json`. Questions: `war questions --open --json`. Stages: `war frontier --json` (legacy stage readiness only).\n");
    text
}
