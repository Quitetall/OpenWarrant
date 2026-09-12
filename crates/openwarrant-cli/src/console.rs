// SPDX-License-Identifier: Apache-2.0

//! `war console` (OW-WAR-0069): one screen for everything a human owes.
//!
//! The queue was never the friction. Typing was. Ten acts meant ten commands
//! and ten sentences about changes the repository had already described in ten
//! commit messages, so the tool asked the signer to restate what it knew. This
//! screen asks for two things instead: which rows, and which reason from a
//! list the signer wrote once. Everything else is drafted, and every signature
//! is still the signer's own ssh confirmation.
//!
//! What it shows, in one pass over the records:
//!
//! - **acts** awaiting a signature, as a numbered checklist;
//! - **questions** a performing agent asked, which only a human can answer;
//! - **stages** on the frontier, which an agent can start the moment its
//!   Warrant is authorized.
//!
//! What it does NOT do: sign. `s` runs the same `war sign --ssh-sign` a hand
//! would, once per checked row, so the agent's confirm dialog is still the act.
//! A console that signed for you would be a console that forged your name.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;

use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use crate::sign::{self, Pending};

pub const SCHEMA: &str = "oh.war/console/v1";

/// What the screen is made of, and what `--json` returns.
#[derive(Debug, Clone, Serialize)]
pub struct Board {
    pub schema: String,
    /// Acts awaiting a signature, in the order the screen numbers them.
    pub acts: Vec<Act>,
    /// Open questions, blocking first.
    pub questions: Vec<Question>,
    /// Stages an agent could start now.
    pub stages: Vec<Stage>,
    pub presets: Vec<Preset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Act {
    /// The number pressed to check it.
    pub n: usize,
    /// `authorize`, `resolve`, `accept` or `correct`.
    pub act: String,
    /// What `war sign <target>` takes.
    pub target: String,
    pub line: String,
    /// The command this row becomes.
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Question {
    pub warrant: String,
    pub id: String,
    pub blocking: bool,
    pub question: String,
    pub recommended: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stage {
    pub warrant: String,
    pub stage: String,
    pub title: String,
    pub executor_kind: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Preset {
    pub key: String,
    pub label: String,
    pub acts: Vec<String>,
}

#[must_use]
pub fn act_of(p: &Pending) -> &'static str {
    match p {
        Pending::Authorize { .. } => "authorize",
        Pending::Resolve { .. } => "resolve",
        Pending::Accept { .. } => "accept",
        Pending::Correct { .. } => "correct",
    }
}

#[must_use]
pub fn target_of(p: &Pending) -> String {
    match p {
        Pending::Authorize { alias, .. } | Pending::Resolve { alias, .. } => alias.clone(),
        Pending::Accept { version, .. } => version.clone(),
        Pending::Correct {
            alias,
            deliverable_id,
            ..
        } => format!("{alias}/{deliverable_id}"),
    }
}

/// Read the whole board from the records. No writes, no prompts.
pub fn board(repo: &Repository) -> Result<Board, RepoError> {
    let acts: Vec<Act> = sign::pending(repo)?
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let target = target_of(p);
            let act = act_of(p).to_owned();
            Act {
                n: i + 1,
                command: format!("war sign {target} --ssh-sign"),
                act,
                target,
                line: sign::line(p).trim().to_owned(),
            }
        })
        .collect();
    let questions: Vec<Question> = crate::questions::list(repo, None, true)
        .map(|(_, l)| {
            l.questions
                .into_iter()
                .map(|q| Question {
                    command: format!("war answer {} {} \"…\" --as <you>", q.warrant, q.id),
                    warrant: q.warrant,
                    id: q.id,
                    blocking: q.blocking,
                    question: q.question,
                    recommended: q.recommended,
                })
                .collect()
        })
        .unwrap_or_default();
    let stages: Vec<Stage> = crate::frontier::run(repo, None)
        .map(|(_, f)| {
            f.rows
                .into_iter()
                .filter(|r| r.state == crate::frontier::StageState::Open)
                .filter(|r| r.executor_kind != "human")
                .map(|r| Stage {
                    command: format!("war dispatch {} {}", r.warrant, r.stage),
                    warrant: r.warrant,
                    stage: r.stage,
                    title: r.title,
                    executor_kind: r.executor_kind,
                })
                .collect()
        })
        .unwrap_or_default();
    let presets = repo
        .config
        .sign
        .presets
        .iter()
        .map(|p| Preset {
            key: p.key.clone(),
            label: p.label.clone(),
            acts: p.acts.clone(),
        })
        .collect();
    Ok(Board {
        schema: SCHEMA.to_owned(),
        acts,
        questions,
        stages,
        presets,
    })
}

#[must_use]
pub fn render(b: &Board, checked: &BTreeSet<usize>) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "\n  OpenWarrant console");
    if b.acts.is_empty() {
        let _ = writeln!(s, "\n  nothing awaits a signature.");
    } else {
        let _ = writeln!(
            s,
            "\n  acts awaiting your signature ({} checked of {})",
            checked.len(),
            b.acts.len()
        );
        for a in &b.acts {
            let mark = if checked.contains(&a.n) { "x" } else { " " };
            let _ = writeln!(s, "   [{mark}] {:>2}  {}", a.n, a.line);
        }
    }
    if !b.questions.is_empty() {
        let blocking = b.questions.iter().filter(|q| q.blocking).count();
        let _ = writeln!(
            s,
            "\n  questions an agent asked you ({} open, {blocking} blocking)",
            b.questions.len()
        );
        for q in &b.questions {
            let _ = writeln!(
                s,
                "   {} {} {}{}",
                if q.blocking { "!" } else { "·" },
                q.warrant,
                q.id,
                if q.recommended.is_empty() {
                    String::new()
                } else {
                    "  (a recommendation is offered)".to_owned()
                }
            );
            let _ = writeln!(s, "        {}", q.question);
        }
    }
    if !b.stages.is_empty() {
        let _ = writeln!(s, "\n  stages an agent can start now ({})", b.stages.len());
        for st in &b.stages {
            let _ = writeln!(
                s,
                "   · {} {} [{}] {}",
                st.warrant, st.stage, st.executor_kind, st.title
            );
        }
    }
    let _ = writeln!(
        s,
        "\n  1-{}  check a row      a  all      n  none\
         \n  s     sign the checked rows (one ssh dialog each)\
         \n  q     answer the questions      r  start the stages\
         \n  c     write the commit message      <Enter>  refresh      x  exit",
        b.acts.len().max(1)
    );
    if !b.presets.is_empty() {
        let _ = writeln!(s, "\n  presets: {}", preset_line(&b.presets));
    }
    s
}

fn preset_line(presets: &[Preset]) -> String {
    presets
        .iter()
        .map(|p| format!("{}={}", p.key, p.label))
        .collect::<Vec<_>>()
        .join("  ")
}

fn prompt(text: &str) -> Result<String, RepoError> {
    let mut out = std::io::stdout();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|source| RepoError::Io {
            context: "could not write the prompt".to_owned(),
            source,
        })?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|source| RepoError::Io {
            context: "could not read the answer".to_owned(),
            source,
        })?;
    Ok(line.trim().to_owned())
}

/// Which preset the signer picked, for one act kind.
fn choose_preset(
    repo: &Repository,
    act: &str,
) -> Result<Option<openwarrant_core::config::SignPreset>, RepoError> {
    let available: Vec<openwarrant_core::config::SignPreset> =
        repo.config.sign.for_act(act).into_iter().cloned().collect();
    if available.is_empty() {
        return Ok(None);
    }
    println!("\n  reasons for a {act} (the words go on the record):");
    for p in &available {
        println!("   {}  {}", p.key, p.label);
        println!("        {}", p.meaning);
    }
    println!("   -  none: the tool's drafted reason only");
    loop {
        let answer = prompt("  reason: ")?;
        if answer == "-" || answer.is_empty() {
            return Ok(None);
        }
        if let Some(p) = available.iter().find(|p| p.key == answer) {
            return Ok(Some(p.clone()));
        }
        println!("  no preset with that key.");
    }
}

/// The reason a batch of one act kind carries: a preset, a line the signer
/// adds, or both. The description is optional by design — the whole point of
/// the presets is that a batch need not be narrated — but when this act is the
/// one that needs a sentence, typing it here is cheaper than `--meaning`.
fn choose_reason(
    repo: &Repository,
    act: &str,
) -> Result<(Option<openwarrant_core::config::SignPreset>, Option<String>), RepoError> {
    let preset = choose_preset(repo, act)?;
    let extra = prompt("  anything to add (Enter for nothing): ")?;
    let extra = (!extra.is_empty()).then_some(extra);
    Ok((preset, extra))
}

/// The words that reach the record: the preset's, the signer's addition, both,
/// or neither — and neither means `war sign` falls back to its own draft.
fn compose_meaning(
    preset: Option<&openwarrant_core::config::SignPreset>,
    extra: Option<&str>,
) -> Option<String> {
    match (preset, extra) {
        (Some(p), Some(e)) => Some(format!("{} {e}", p.meaning)),
        (Some(p), None) => Some(p.meaning.clone()),
        (None, Some(e)) => Some(e.to_owned()),
        (None, None) => None,
    }
}

/// `war console`: the loop. Every act still goes through `war sign`.
pub fn run(repo: &Repository) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let mut checked: BTreeSet<usize> = BTreeSet::new();
    loop {
        let b = board(repo)?;
        checked.retain(|n| b.acts.iter().any(|a| a.n == *n));
        print!("{}", render(&b, &checked));
        let answer = prompt("\n  > ")?;
        match answer.as_str() {
            "x" | "exit" => break,
            "" => continue,
            "a" => {
                checked = b.acts.iter().map(|a| a.n).collect();
            }
            "n" => checked.clear(),
            "s" => {
                if checked.is_empty() {
                    println!("  nothing checked.");
                    continue;
                }
                sign_checked(repo, &b, &checked, &mut report)?;
                checked.clear();
            }
            "q" => answer_questions(repo, &b, &mut report)?,
            "r" => start_stages(repo, &b, &mut report)?,
            "c" => {
                let message = crate::commit::message(repo)?;
                println!("\n{message}\n");
                println!("  git commit -F - <<'MSG'  (or `war commit --write`)");
            }
            other => {
                for part in other.split(|c: char| !c.is_ascii_digit()) {
                    if let Ok(n) = part.parse::<usize>()
                        && b.acts.iter().any(|a| a.n == n)
                    {
                        if checked.contains(&n) {
                            checked.remove(&n);
                        } else {
                            checked.insert(n);
                        }
                    }
                }
            }
        }
    }
    Ok(report)
}

fn sign_checked(
    repo: &Repository,
    b: &Board,
    checked: &BTreeSet<usize>,
    report: &mut Report,
) -> Result<(), RepoError> {
    // One preset question per act KIND, not per row: the reason a batch of
    // corrections shares is one reason, and typing it ten times was the
    // friction this screen exists to remove.
    let rows: Vec<&Act> = b.acts.iter().filter(|a| checked.contains(&a.n)).collect();
    let mut kinds: Vec<String> = rows.iter().map(|a| a.act.clone()).collect();
    kinds.sort();
    kinds.dedup();
    type Reason = (Option<openwarrant_core::config::SignPreset>, Option<String>);
    let mut chosen: Vec<(String, Reason)> = Vec::new();
    for kind in &kinds {
        chosen.push((kind.clone(), choose_reason(repo, kind)?));
    }
    println!(
        "\n  {} act(s) to sign. Each one puts the agent's confirm dialog in front of you.",
        rows.len()
    );
    for a in rows {
        let reason = chosen.iter().find(|(k, _)| k == &a.act).map(|(_, r)| r);
        let preset = reason.and_then(|(p, _)| p.clone());
        let extra = reason.and_then(|(_, e)| e.clone());
        let mut opts = sign::Options {
            actor: None,
            meaning: compose_meaning(preset.as_ref(), extra.as_deref()),
            outcome: None,
            adr_ref: None,
            independence: openwarrant_core::contract::Independence::SeparateRole,
            edit: false,
            all: false,
            show: false,
            ssh_sign: true,
            verify: false,
            kind: None,
        };
        if a.act == "correct" {
            let word = preset
                .as_ref()
                .map(|p| p.kind.clone())
                .filter(|k| !k.is_empty())
                .unwrap_or_else(|| "behaviour-change".to_owned());
            match word.parse() {
                Ok(k) => opts.kind = Some(k),
                Err(e) => {
                    report.push(Diagnostic::error(
                        "console.preset-kind",
                        a.target.clone(),
                        format!("the preset's kind {word:?} is not a correction kind: {e}"),
                    ));
                    continue;
                }
            }
        }
        println!("\n  ── {}", a.line);
        match sign::run(repo, Some(&a.target), &opts) {
            Ok(r) => {
                let signed = r.diagnostics.iter().any(|d| d.rule.contains("recorded"));
                for d in r.diagnostics {
                    report.push(d);
                }
                if !signed {
                    println!("  refused; it stays in the queue.");
                }
            }
            Err(e) => report.push(Diagnostic::error(
                "console.sign",
                a.target.clone(),
                e.to_string(),
            )),
        }
    }
    Ok(())
}

fn answer_questions(repo: &Repository, b: &Board, report: &mut Report) -> Result<(), RepoError> {
    if b.questions.is_empty() {
        println!("  no open questions.");
        return Ok(());
    }
    let actor = prompt("  answering as (your name in roles.toml): ")?;
    if actor.is_empty() {
        println!("  an answer needs an actor; nothing written.");
        return Ok(());
    }
    for q in &b.questions {
        println!(
            "\n  ── {} {}{}\n     {}",
            q.warrant,
            q.id,
            if q.blocking { "  (blocking)" } else { "" },
            q.question
        );
        if !q.recommended.is_empty() {
            println!("     recommended: {}", q.recommended);
        }
        let answer = prompt("     your answer (Enter to take the recommendation, s to skip): ")?;
        let text = match answer.as_str() {
            "s" => continue,
            "" if !q.recommended.is_empty() => q.recommended.clone(),
            "" => {
                println!("     no recommendation to take; skipped.");
                continue;
            }
            other => other.to_owned(),
        };
        match crate::questions::answer(repo, &q.warrant, &q.id, &text, &actor) {
            Ok(r) => {
                for d in r.diagnostics {
                    report.push(d);
                }
            }
            Err(e) => report.push(Diagnostic::error(
                "console.answer",
                format!("{}/{}", q.warrant, q.id),
                e.to_string(),
            )),
        }
    }
    Ok(())
}

fn start_stages(repo: &Repository, b: &Board, report: &mut Report) -> Result<(), RepoError> {
    if b.stages.is_empty() {
        println!("  no stage is startable: every one is blocked, claimed, or a human's.");
        return Ok(());
    }
    println!(
        "\n  {} stage(s). Each gets its Dispatch compiled; a service stage also runs.",
        b.stages.len()
    );
    for st in &b.stages {
        println!("\n  ── {} {} [{}]", st.warrant, st.stage, st.executor_kind);
        let r = if st.executor_kind == "service" {
            crate::run_cmd::run(repo, &st.warrant, &st.stage)
        } else {
            crate::dispatch::run(
                repo,
                &st.warrant,
                &st.stage,
                openwarrant_core::execution::AttemptKind::Initial,
                &[],
                None,
                None,
            )
        };
        match r {
            Ok(rep) => {
                for d in rep.diagnostics {
                    report.push(d);
                }
            }
            Err(e) => report.push(Diagnostic::error(
                "console.start",
                format!("{}/{}", st.warrant, st.stage),
                e.to_string(),
            )),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(meaning: &str) -> openwarrant_core::config::SignPreset {
        openwarrant_core::config::SignPreset {
            key: "1".to_owned(),
            label: "a label".to_owned(),
            meaning: meaning.to_owned(),
            acts: vec!["correct".to_owned()],
            kind: String::new(),
        }
    }

    #[test]
    fn the_description_is_optional_and_adds_to_the_preset() {
        let p = preset("The bytes moved with the plan.");
        assert_eq!(
            compose_meaning(Some(&p), Some("The SPDX line moved too.")).as_deref(),
            Some("The bytes moved with the plan. The SPDX line moved too.")
        );
        assert_eq!(
            compose_meaning(Some(&p), None).as_deref(),
            Some("The bytes moved with the plan.")
        );
        // A signer who picks no preset but types a line gets their own words,
        // and one who does neither gets `war sign`'s drafted reason.
        assert_eq!(
            compose_meaning(None, Some("Only my own words.")).as_deref(),
            Some("Only my own words.")
        );
        assert_eq!(compose_meaning(None, None), None);
    }

    fn act(n: usize, act: &str, target: &str) -> Act {
        Act {
            n,
            act: act.to_owned(),
            target: target.to_owned(),
            line: format!("{target}  {act}  something"),
            command: format!("war sign {target} --ssh-sign"),
        }
    }

    fn board_of(acts: Vec<Act>) -> Board {
        Board {
            schema: SCHEMA.to_owned(),
            acts,
            questions: vec![],
            stages: vec![],
            presets: vec![],
        }
    }

    #[test]
    fn a_checked_row_is_marked_and_counted() {
        let b = board_of(vec![
            act(1, "correct", "OW-WAR-0005/D-001"),
            act(2, "authorize", "OW-WAR-0070"),
        ]);
        let checked: BTreeSet<usize> = [2].into_iter().collect();
        let screen = render(&b, &checked);
        assert!(screen.contains("1 checked of 2"), "{screen}");
        assert!(screen.contains("[ ]  1"), "{screen}");
        assert!(screen.contains("[x]  2"), "{screen}");
    }

    #[test]
    fn the_screen_offers_no_way_to_sign_without_the_key() {
        // A console that could sign without the agent's dialog would be the
        // one defect this whole seam exists to prevent, so the screen says so
        // and the only signing path is `sign::run` with ssh_sign set.
        let b = board_of(vec![act(1, "resolve", "OW-WAR-0001")]);
        let screen = render(&b, &BTreeSet::new());
        assert!(screen.contains("one ssh dialog each"), "{screen}");
        assert!(!screen.to_lowercase().contains("auto"), "{screen}");
    }

    #[test]
    fn presets_are_listed_by_key() {
        let p = vec![
            Preset {
                key: "1".to_owned(),
                label: "the relicense".to_owned(),
                acts: vec!["correct".to_owned()],
            },
            Preset {
                key: "2".to_owned(),
                label: "a slice of the 1.0 plan".to_owned(),
                acts: vec![],
            },
        ];
        assert_eq!(
            preset_line(&p),
            "1=the relicense  2=a slice of the 1.0 plan"
        );
    }
}
