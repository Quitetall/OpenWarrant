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
use openwarrant_compiler::{DispatchInputs, compile_dispatch, dispatch_json, lower, sha256_hex};
use openwarrant_core::context::{
    ContextItem, ContextManifest, ContextRole, Holder, Omission, Precedence, TrustClass,
};
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

    // §33 — the context manifest, from the atoms.
    let commit = git_head(&repo.root);
    let mut included = Vec::new();
    let mut omitted = Vec::new();
    for atom in &basis.atoms {
        if atom.required {
            included.push(ContextItem {
                id: atom.source.clone(),
                role: role_for(&atom.role),
                required: true,
                holder: Holder {
                    kind: "git".to_owned(),
                    repository: repo.config.project.name.clone(),
                    commit_sha: commit.clone().unwrap_or_default(),
                    path: repo.relative(&dir.join(&atom.source)),
                },
                content_digest: format!("sha256:{}", sha256_hex(&atom.bytes)),
                selector_sections: vec![],
                classification: "internal".to_owned(),
                trust: TrustClass::AuthoritativeInternal,
                taints: vec![],
                precedence: Some(Precedence::AuthorizedWarContract),
            });
        } else {
            omitted.push(Omission {
                id: atom.source.clone(),
                reason: format!(
                    "optional atom of role {:?}; not stage-relevant to {stage_id} (§47.2 \"select \
                     only stage-relevant context\")",
                    atom.role
                ),
                required: false,
            });
        }
    }
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
        dispatch_id: openwarrant_core::WarUuid::mint().to_string(),
    })
    .map_err(|e| RepoError::Message(format!("{alias}/{stage_id}: {e}")))?;

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

/// §33.2 — which role each atom plays as context.
fn role_for(atom_role: &str) -> ContextRole {
    match atom_role {
        "basis" => ContextRole::Governing,
        "intent" | "work_order" | "milestones" | "assurance" => ContextRole::Normative,
        _ => ContextRole::Informative,
    }
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
