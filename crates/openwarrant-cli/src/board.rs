// SPDX-License-Identifier: Apache-2.0
//! Read-only master board. Existing evaluators retain ownership of status and admission.
use crate::{
    console,
    diagnostic::Report,
    frontier,
    repo::{RepoError, Repository},
};
use openwarrant_core::status::CorpusStatus;
use serde::Serialize;
use std::fmt::Write as _;

#[derive(Serialize)]
pub struct Board {
    pub schema: &'static str,
    pub program: String,
    pub corpus: CorpusStatus,
    pub frontier: frontier::Frontier,
    pub approvals: Vec<console::Act>,
    pub questions: Vec<console::Question>,
}

pub fn build(repo: &Repository) -> Result<(Report, Board), RepoError> {
    let (report, frontier) = frontier::run(repo, None)?;
    let queue = console::board(repo)?;
    Ok((
        report,
        Board {
            schema: "oh.war/board-draft/v1",
            program: repo.config.project.name.clone(),
            corpus: crate::status::build(repo)?,
            frontier,
            approvals: queue.acts,
            questions: queue.questions,
        },
    ))
}

pub fn render(board: &Board) -> String {
    let mut out = format!(
        "{} — read-only project board\n\nObjectives\n",
        board.program
    );
    for objective in &board.corpus.objectives {
        let _ = writeln!(
            out,
            "- {}: {:?} ({} Warrants)",
            objective.title,
            objective.achieved,
            objective.warrants.len()
        );
    }
    out.push_str("\nWarrants (legacy record state; not implementation completion)\n");
    for w in &board.corpus.warrants {
        let _ = writeln!(
            out,
            "- {}: {} — {:?}; {:?}",
            w.alias,
            w.title.as_deref().unwrap_or("untitled"),
            w.rung,
            w.validity
        );
    }
    out.push_str("\nStages\n");
    for stage in &board.frontier.rows {
        let _ = writeln!(
            out,
            "- {} / {}: {} — {:?}; waiting on {:?}",
            stage.warrant, stage.stage, stage.title, stage.state, stage.waiting_on
        );
    }
    out.push_str("\nQuestions\n");
    for q in &board.questions {
        let _ = writeln!(
            out,
            "- {} / {}{}: {}\n  Recommendation: {}",
            q.warrant,
            q.id,
            if q.blocking { " [blocking]" } else { "" },
            q.question,
            q.recommended
        );
    }
    out.push_str("\nApprovals (commands only; this board never signs)\n");
    for act in &board.approvals {
        let _ = writeln!(out, "{}. {}\n   {}", act.n, act.line, act.command);
    }
    for caveat in &board.corpus.caveats {
        let _ = writeln!(out, "Note: {caveat}");
    }
    out
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn html(board: &Board, report: &Report) -> String {
    let text = escape(&render(board));
    let diagnostics = escape(&format!(
        "{}\n{:?}",
        report.verdict_line(),
        report.diagnostics
    ));
    format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; base-uri 'none'; form-action 'none'\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>OpenWarrant board</title><body><h1>OpenWarrant board</h1><p>Offline, read-only snapshot. No signing or network actions.</p><pre>{text}</pre><h2>Diagnostics</h2><pre>{diagnostics}</pre></body></html>\n"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn untrusted_record_text_cannot_become_html() {
        let escaped = super::escape("<script src=\"https://example.invalid\">'&</script>");
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(!escaped.contains('\''));
        assert!(escaped.contains("&lt;script"));
        assert!(escaped.contains("&quot;"));
    }
}
