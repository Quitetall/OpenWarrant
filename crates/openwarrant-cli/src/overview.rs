// SPDX-License-Identifier: Apache-2.0
//! Compact, read-only view over the same live records as `war status`.
use openwarrant_core::status::{CorpusStatus, WarrantRung};
use serde::Serialize;

/// Checklist completion is a work record, never an assurance assessment.
#[derive(Debug, Serialize)]
pub struct WorkingForm {
    pub id: String,
    pub title: String,
    pub state: crate::ticket::TicketState,
    pub done: usize,
    pub total: usize,
    pub sources: Vec<String>,
    pub source_digest: String,
    pub ticks: crate::ticket::ladder::TickReport,
    pub milestones: Vec<crate::ticket::ladder::MilestoneView>,
    pub proof_faults: Vec<openwarrant_core::ticket::Fault>,
}

#[derive(Debug, Serialize)]
pub struct WorkingFormDiagnostic {
    severity: String,
    rule: String,
    file: Option<String>,
    message: String,
}

#[derive(Debug, Serialize)]
pub struct WorkingForms {
    pub total: usize,
    pub completed: usize,
    pub ticks_below_minimum: usize,
    pub proof_incomplete: usize,
    pub remaining: usize,
    pub includes_completed: bool,
    pub qualification_assessed: bool,
    pub coverage_complete: bool,
    pub diagnostics: Vec<WorkingFormDiagnostic>,
    pub records: Vec<WorkingForm>,
}

/// Use the same cached documents and state derivation as the ticket commands.
pub fn working_forms(
    corpus: &crate::corpus::Corpus,
    all: bool,
) -> Result<WorkingForms, crate::repo::RepoError> {
    let (tickets, diagnostics) = corpus.tickets()?;
    let store = crate::ticket::Store::open(corpus.repo(), None)?;
    let claims = store.claims()?;
    let reader = crate::ticket::ladder::Reader::new(&store);
    let mut records = Vec::new();
    let mut completed = 0;
    let mut ticks_below_minimum = 0;
    let mut proof_incomplete = 0;
    for ticket in tickets {
        let state = crate::ticket::state_of(ticket, &claims);
        let ticks = crate::ticket::ladder::ticks_of(&reader, ticket);
        ticks_below_minimum += ticks.counts.below_minimum;
        if ticks.unavailable.is_some() || !ticks.checks.faults.is_empty() {
            proof_incomplete += 1;
        }
        if state == crate::ticket::TicketState::Done {
            completed += 1;
            if !all {
                continue;
            }
        }
        let (done, total) = ticket.checklist.progress();
        // Include observed document contents, not just the manifest revision:
        // an intent or checklist edit must invalidate a viewer snapshot too.
        let bytes = serde_json::to_vec(&(&ticket.revision, &ticket.intent, &ticket.checklist_text))
            .map_err(|e| crate::repo::RepoError::Message(e.to_string()))?;
        records.push(WorkingForm {
            id: ticket.id().into(),
            title: ticket.manifest.title.clone(),
            state,
            done,
            total,
            sources: if ticket.format {
                vec![ticket.intent_path.clone()]
            } else {
                vec![
                    ticket.dir.join("manifest.toml"),
                    ticket.intent_path.clone(),
                    ticket.checklist_path.clone(),
                ]
            }
            .iter()
            .map(|path| corpus.repo().relative(path))
            .collect(),
            source_digest: openwarrant_compiler::sha256_hex(&bytes),
            milestones: crate::ticket::ladder::milestones_of(&reader, ticket, &ticks),
            proof_faults: ticks.checks.faults.clone(),
            ticks,
        });
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(WorkingForms {
        total: tickets.len(),
        completed,
        ticks_below_minimum,
        proof_incomplete,
        remaining: tickets.len() - completed,
        includes_completed: all,
        qualification_assessed: false,
        coverage_complete: diagnostics.is_empty(),
        diagnostics: diagnostics
            .iter()
            .map(|d| WorkingFormDiagnostic {
                severity: d.severity.to_string(),
                rule: d.rule.clone(),
                file: d.file.clone(),
                message: d.message.clone(),
            })
            .collect(),
        records,
    })
}

#[derive(Debug, Serialize)]
pub struct UnifiedOverview {
    #[serde(flatten)]
    legacy: Overview,
    working_forms: WorkingForms,
}

pub fn capture(
    corpus: &crate::corpus::Corpus,
    all: bool,
) -> Result<UnifiedOverview, crate::repo::RepoError> {
    Ok(UnifiedOverview {
        legacy: build(corpus.status()?.clone(), all),
        working_forms: working_forms(corpus, all)?,
    })
}

pub fn render_unified(view: &UnifiedOverview) -> String {
    let mut text = render(&view.legacy);
    let forms = &view.working_forms;
    text.push_str(&format!(
        "\n## Working-form Warrants\n\n{} remaining | {} completed | {} readable records. Completion is unverified; no execution permission is inferred.\n\n",
        forms.remaining, forms.completed, forms.total
    ));
    if !forms.coverage_complete {
        text.push_str(
            "Coverage is incomplete; unreadable records are excluded from these counts.\n",
        );
    }
    if forms.ticks_below_minimum > 0 {
        text.push_str(&format!(
            "{} completed tick(s) below their configured proof minimum. Completion remains unverified.\n",
            forms.ticks_below_minimum
        ));
    }
    if forms.proof_incomplete > 0 {
        text.push_str(&format!(
            "Proof assessment incomplete for {} record(s); use --all for details.\n",
            forms.proof_incomplete
        ));
    }
    for d in &forms.diagnostics {
        text.push_str(&format!("- {}: {}\n", cell(&d.rule), cell(&d.message)));
    }
    for row in &forms.records {
        text.push_str(&format!(
            "- {} — {}: {} ({}/{})\n",
            cell(&row.id),
            cell(&row.title),
            row.state.as_str(),
            row.done,
            row.total
        ));
        if let Some(reason) = &row.ticks.unavailable {
            text.push_str(&format!("  - Proof UNKNOWN: {}\n", cell(reason)));
        }
        for m in &row.milestones {
            text.push_str(&format!(
                "  - {}: {} · minimum {}\n",
                cell(&m.target),
                cell(&m.marker),
                m.minimum.as_str()
            ));
        }
        for fault in &row.proof_faults {
            text.push_str(&format!(
                "  - {}: {}\n",
                cell(fault.rule),
                cell(&fault.message)
            ));
        }
    }
    text
}

#[derive(Debug, Serialize)]
pub struct Row {
    alias: String,
    title: String,
    phase: String,
    currency: String,
    standing: String,
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
    retired_records: usize,
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
    let retired_records = corpus
        .warrants
        .iter()
        .filter(|w| crate::status::is_retired(w))
        .count();
    let remaining = corpus
        .warrants
        .iter()
        .filter(|w| !crate::status::is_retired(w) && w.rung != WarrantRung::Resolved)
        .count();
    let warrants = corpus
        .warrants
        .into_iter()
        .filter(|w| all || (!crate::status::is_retired(w) && w.rung != WarrantRung::Resolved))
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
            currency: w
                .state
                .as_ref()
                .map_or("unknown", |s| s.currency.as_str())
                .into(),
            standing: w
                .state
                .as_ref()
                .map_or("unknown", |s| s.standing.as_str())
                .into(),
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
        retired_records,
        remaining,
        includes_resolved: all,
        note: "Live legacy records. Remaining means current and unresolved, not necessarily unfinished implementation. Retired records remain available with --all. Resolution is not a new Verified mark. No execution permission is inferred. Inspect a Warrant's current scope and input prerequisites before work.",
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
        "# OpenWarrant overview\n\n{} remaining current legacy records | {} recorded resolutions | {} retired records | {} total\n\n{}\n\n",
        view.remaining, view.recorded_resolutions, view.retired_records, view.total, view.note
    );
    if view.warrants.is_empty() {
        text.push_str("No records in this view.\n");
    } else {
        text.push_str("| Warrant | Title | Recorded phase | Currency / standing | Assessment | Unmet / unknowns |\n| --- | --- | --- | --- | --- | --- |\n");
        for row in &view.warrants {
            let rung = match row.assessment {
                WarrantRung::Invalid => "invalid",
                WarrantRung::Draft => "draft",
                WarrantRung::ReadyToResolve => "ready_to_resolve",
                WarrantRung::WouldSatisfy => "would_satisfy",
                WarrantRung::Resolved => "resolved",
            };
            text.push_str(&format!(
                "| {} | {} | {} | {} / {} | {} | {} / {} |\n",
                cell(&row.alias),
                cell(&row.title),
                cell(&row.phase),
                cell(&row.currency),
                cell(&row.standing),
                rung,
                row.unmet.len(),
                row.blocking_unknowns.len()
            ));
        }
    }
    text.push_str("\nDetails: `war status <alias> --json`. Human acts: `war view next --json`. Questions: `war plan questions --open --json`. Stages: `war plan frontier --json` (legacy stage readiness only).\n");
    text
}
