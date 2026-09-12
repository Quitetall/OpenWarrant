// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war dispatch` — build the inputs to a Stage Dispatch from a Warrant's own
//! records and emit the packet (SAS §47, §33, §52).
//!
//! # What the context manifest is built from
//!
//! The Warrant's atoms, and nothing else. Each atom becomes a [`ContextItem`]
//! pinned by the sha256 of its bytes, held by `git` at the current commit.
//! The five required atoms are included; optional atoms are recorded as
//! omitted WITH a reason (§33.7). Nothing outside the Warrant is reached for —
//! §33.6 forbids silently dropping a required item, and the cheapest way to
//! honour that is to start from a set whose required members are all known.
//!
//! # What is minted here and not in the compiler
//!
//! The `dispatch_id` and the attempt's id are UUIDv7s minted here, so that
//! [`openwarrant_compiler::compile_dispatch`] stays a pure function. Two calls
//! to this command produce two different packets, correctly: they are two
//! attempts. Two calls to the compiler with the same ids produce one.

use std::fs;
use std::process::Command;

use camino::Utf8Path;
use openwarrant_compiler::{DispatchInputs, compile_dispatch, dispatch_json, lower};
use openwarrant_core::context::ContextManifest;
use openwarrant_core::execution::{
    Attempt, AttemptKind, CapabilityAuthorization, ResourceEnvelope,
};
use openwarrant_core::milestones;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// Build and emit one dispatch.
pub fn run(
    repo: &Repository,
    alias: &str,
    stage_id: &str,
    attempt_kind: AttemptKind,
    prior_failure_evidence: &[String],
    emit_to: Option<&Utf8Path>,
    emit_context_to: Option<&Utf8Path>,
) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let mut report = Report::default();

    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!("{alias} could not be compiled")));
    };
    let ir = lower(basis, validated).map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;

    // The stage and the milestone that cites it, from the validated graph.
    let mut graph: Option<milestones::MilestoneGraph> = None;
    for atom in basis.atoms.iter().filter(|a| a.role == "milestones") {
        let text = std::str::from_utf8(&atom.bytes).map_err(|e| {
            RepoError::Message(format!("{alias}: milestones atom is not UTF-8: {e}"))
        })?;
        graph =
            Some(milestones::parse(text).map_err(|e| RepoError::Message(format!("{alias}: {e}")))?);
    }
    let Some(graph) = graph else {
        return Err(RepoError::Message(format!(
            "{alias}: declares no milestones atom"
        )));
    };
    let Some(stage) = graph.stages.iter().find(|s| s.id == stage_id) else {
        return Err(RepoError::Message(format!(
            "{alias}: no stage {stage_id:?}. Declared: {}",
            graph
                .stages
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    };
    let Some(milestone) = graph
        .milestones
        .iter()
        .find(|m| m.stage_refs.iter().any(|s| s == stage_id))
    else {
        return Err(RepoError::Message(format!(
            "{alias}: stage {stage_id:?} is cited by no milestone, so there is no milestone_id \
             to bind it to (§47.1)"
        )));
    };

    // §33 — the context manifest. Slice C1: stage-relevant selection —
    // required atoms, plus whatever the stage declares (atoms, sections,
    // artifacts, external refs); everything else omitted with a true reason.
    let commit = git_head(&repo.root);
    let selection = match crate::context_select::select(repo, &dir, basis, stage, commit.as_deref())
    {
        Ok(s) => s,
        Err(refusals) => {
            for r in refusals {
                report.push(Diagnostic::error(
                    r.rule,
                    repo.relative(&dir.join("atoms/45-milestones.yaml")),
                    r.message,
                ));
            }
            return Ok(report);
        }
    };
    // §33.7 (slice C2): estimate what the packet's context costs to read,
    // against the stage's budget or the repository's default. Over budget is
    // a refusal that names the three largest items, so the fix is a cut, not
    // a bigger number.
    let total_bytes: u64 = selection.bytes.iter().map(|(_, b)| *b).sum();
    let estimated_tokens = openwarrant_core::tokens::estimate(total_bytes);
    let budget_tokens = stage
        .budget_tokens
        .unwrap_or_else(|| repo.config.context.budget());
    if estimated_tokens > budget_tokens {
        let mut largest = selection.bytes.clone();
        largest.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let top: Vec<String> = largest
            .iter()
            .take(3)
            .map(|(id, b)| format!("{id} (~{} tokens)", openwarrant_core::tokens::estimate(*b)))
            .collect();
        report.push(Diagnostic::error(
            "dispatch.over-budget",
            repo.relative(&dir.join("atoms/45-milestones.yaml")),
            format!(
                "{alias}/{stage_id}: the selected context is ~{estimated_tokens} tokens against a \
                 budget of {budget_tokens} ({}); largest: {}. Narrow the stage's context_sections, \
                 or raise budget_tokens on the stage with a reason",
                openwarrant_core::tokens::METHOD,
                top.join(", ")
            ),
        ));
        return Ok(report);
    }
    let tokens = openwarrant_core::tokens::TokenAccount {
        estimated_tokens,
        budget_tokens,
        method: openwarrant_core::tokens::METHOD.to_owned(),
    };
    let (included, omitted) = (selection.included, selection.omitted);
    let context = ContextManifest {
        workspace_basis_ref: format!("basis://{}", basis.manifest_source),
        workspace_basis_digest: ir.integrity.workspace_basis_digest.clone(),
        included,
        omitted,
        unresolved: vec![],
        conflicts: vec![],
        effective_classification: "internal".to_owned(),
        policy_digest: String::new(),
        compiler_digest: format!("openwarrant-cli/{}", env!("CARGO_PKG_VERSION")),
    };
    if commit.is_none() {
        report.push(Diagnostic::warn(
            "dispatch.floating-holder",
            repo.relative(&dir.join("manifest.toml")),
            format!(
                "{alias}: could not read the git HEAD, so every context item's holder has no \
                 commit_sha and is floating (§33.5). A draft may float; an authorized \
                 dispatch may not."
            ),
        ));
    }

    // §52 — the attempt. Ids are minted here; the compiler stays pure.
    let attempt = Attempt {
        id: openwarrant_core::WarUuid::mint().to_string(),
        kind: attempt_kind,
        parent_attempt_ref: if attempt_kind.requires_parent() {
            "attempt://unspecified".to_owned()
        } else {
            String::new()
        },
        basis_change: if attempt_kind == AttemptKind::Replay {
            "none".to_owned()
        } else {
            String::new()
        },
        prior_failure_evidence_refs: prior_failure_evidence.to_vec(),
        prior_work_product_ref: String::new(),
        authorized_by: String::new(),
    };

    let dispatch = compile_dispatch(DispatchInputs {
        ir: &ir,
        basis,
        milestone,
        stage,
        attempt: &attempt,
        context: &context,
        resources: ResourceEnvelope {
            network_policy: "none".to_owned(),
            ..ResourceEnvelope::default()
        },
        capability: CapabilityAuthorization {
            // Not modelled in this repository. Said so, rather than a digest
            // of nothing.
            policy_ref: "policy://none-declared".to_owned(),
            digest: String::new(),
        },
        tokens: Some(tokens.clone()),
        dispatch_id: openwarrant_core::WarUuid::mint().to_string(),
    })
    .map_err(|e| RepoError::Message(format!("{alias}/{stage_id}: {e}")))?;

    if let Some(path) = emit_context_to {
        let text = serde_json::to_string_pretty(&context)
            .map_err(|e| RepoError::Message(e.to_string()))?;
        fs::write(path, format!("{text}\n")).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
        report.push(Diagnostic::pass(
            "dispatch.context-emitted",
            format!(
                "{alias}/{stage_id}: context manifest written to {path} ({} included, {} omitted)",
                context.included.len(),
                context.omitted.len()
            ),
        ));
    }
    // The compile is an event of the Warrant's history (§24): what was
    // dispatched, at what size, under what budget.
    crate::journal_cmd::record(
        &dir,
        &ir.identity.uuid.to_string(),
        "dispatch.compiled",
        &format!("agent://{}", repo.performer()),
        &serde_json::json!({
            "stage": stage_id,
            "dispatch_id": dispatch.dispatch_id,
            "dispatch_digest": dispatch.dispatch_digest,
            "estimated_tokens": tokens.estimated_tokens,
            "budget_tokens": tokens.budget_tokens,
            "method": tokens.method,
        })
        .to_string(),
    )?;
    report.push(Diagnostic::pass(
        "dispatch.tokens",
        format!(
            "{alias}/{stage_id}: ~{} tokens of context against a budget of {} ({})",
            tokens.estimated_tokens, tokens.budget_tokens, tokens.method
        ),
    ));
    let json = dispatch_json(&dispatch).map_err(|e| RepoError::Message(e.to_string()))?;
    match emit_to {
        Some(path) => {
            fs::write(path, format!("{json}\n")).map_err(|source| RepoError::Io {
                context: format!("could not write {path}"),
                source,
            })?;
            report.push(Diagnostic::pass(
                "dispatch.emitted",
                format!(
                    "{alias}/{stage_id}: dispatch {} written to {path} (digest {})",
                    dispatch.dispatch_id, dispatch.dispatch_digest
                ),
            ));
        }
        None => {
            println!("{json}");
            report.push(Diagnostic::pass(
                "dispatch.compiled",
                format!(
                    "{alias}/{stage_id}: dispatch {} (digest {})",
                    dispatch.dispatch_id, dispatch.dispatch_digest
                ),
            ));
        }
    }
    report.push(Diagnostic::pass(
        "dispatch.context",
        format!(
            "{alias}/{stage_id}: {} required atom(s) included, {} optional omitted with reason",
            context.included.len(),
            context.omitted.len()
        ),
    ));
    Ok(report)
}

/// The current commit, if this is a git checkout. `None` is reported, not
/// papered over with a placeholder that looks like a revision.
fn git_head(root: &Utf8Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())).then_some(sha)
}
