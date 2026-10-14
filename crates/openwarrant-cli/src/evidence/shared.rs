// SPDX-License-Identifier: Apache-2.0
//! One actual observation, bound to all selected contracts BEFORE execution.
//! Attachments preserve its seal and immutable stream references. They are not
//! additional executions, verifier judgments, or signatures.

use super::{GATE_RUNS_DIR, attached_payload, contract_subject};
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use camino::Utf8PathBuf;

fn definition(repo: &Repository, key: &str) -> Option<openwarrant_core::GateDefinition> {
    let mut report = Report::default();
    let registry = crate::check::load_gate_registry(repo, &mut report);
    report
        .is_ready()
        .then(|| registry.definitions.into_iter().find(|d| d.key() == key))
        .flatten()
}

pub(crate) fn eligible(repo: &Repository, key: &str) -> bool {
    !crate::gate_cmd::is_bonsai_gate(key)
        && !key.starts_with("document.")
        && definition(repo, key)
            .is_some_and(|d| !d.argv.iter().any(|a| a == crate::gate_cmd::WARRANT_ARG))
}

struct Member {
    alias: String,
    dir: Utf8PathBuf,
    uuid: String,
    binding: [String; 2],
}

fn current_binding(repo: &Repository, dir: &camino::Utf8Path) -> Result<[String; 2], RepoError> {
    let one = repo.load_warrant(dir)?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message("contract no longer compiles".into()));
    };
    let ir = openwarrant_compiler::lower(basis, validated)
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let digest = ir
        .contract_digest()
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let set = repo
        .load_deliverables(dir)?
        .records
        .iter()
        .map(|d| (d.id.clone(), d.target_ref.clone()))
        .collect::<Vec<_>>();
    Ok([
        contract_subject(&digest),
        crate::gate_cmd::source::deliverables_digest(&repo.root, &set),
    ])
}

/// Only preparation's pending members enter this path. No subject can be added
/// after execution; a later, unselected Warrant needs a new observation.
pub(crate) fn record(
    repo: &Repository,
    aliases: &[String],
    key: &str,
) -> Result<Report, RepoError> {
    if aliases.len() < 2 || !eligible(repo, key) {
        return Err(RepoError::Message(format!(
            "{key}: shared recording needs at least two Warrants and a registered gate \
             without Warrant-specific arguments or document/Bonsai bindings"
        )));
    }
    let Some(definition_before) = definition(repo, key) else {
        let mut report = Report::default();
        report.push(Diagnostic::unknown(
            "evidence.shared-source-moved",
            key,
            "the gate definition became unavailable before execution; nothing was run",
        ));
        return Ok(report);
    };
    let mut subjects = Vec::new();
    let mut members = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    // Complete preflight before creating an output directory or running a gate.
    for alias in aliases {
        if !seen.insert(alias) {
            return Err(RepoError::Message(format!(
                "duplicate shared member {alias}"
            )));
        }
        let dir = repo.warrant_dir(alias)?;
        if crate::vfs::is_file(dir.join("resolution.toml")) {
            return Err(RepoError::Message(format!(
                "{alias}: resolved records are history"
            )));
        }
        let one = repo.load_warrant(&dir)?;
        one.require(&repo.profiles, openwarrant_core::Capability::Evidence)?;
        if !crate::resolve::cited_gate_keys(&one)
            .iter()
            .any(|g| g == key)
        {
            return Err(RepoError::Message(format!(
                "{alias} does not cite gate://{key}"
            )));
        }
        let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
            return Err(RepoError::Message(format!(
                "{alias}: no valid compiled contract"
            )));
        };
        let ir = openwarrant_compiler::lower(basis, validated)
            .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;
        let digest = ir
            .contract_digest()
            .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;
        let set = repo
            .load_deliverables(&dir)?
            .records
            .iter()
            .map(|d| (d.id.clone(), d.target_ref.clone()))
            .collect::<Vec<_>>();
        let binding = [
            contract_subject(&digest),
            crate::gate_cmd::source::deliverables_digest(&repo.root, &set),
        ];
        subjects.extend(binding.iter().cloned());
        members.push(Member {
            alias: alias.clone(),
            dir,
            uuid: validated.uuid.to_string(),
            binding,
        });
    }
    subjects.sort();
    subjects.dedup();
    // A new immutable directory keeps later runs from overwriting streams that
    // another Warrant still cites. The directory is tracked evidence, excluded
    // from source-tree reuse just like every other gate-runs/ record.
    let store = members[0]
        .dir
        .join(GATE_RUNS_DIR)
        .join("shared")
        .join(openwarrant_core::WarUuid::mint().to_string());
    let mut report = crate::gate_cmd::run_for(
        repo,
        true,
        Some(key),
        true,
        &subjects,
        &[],
        Some(&store),
        None,
    )?;
    if !report
        .diagnostics
        .iter()
        .any(|d| d.rule == "gate-run.receipt")
    {
        return Ok(report);
    }
    let run_path = crate::gate_cmd::run_record_path(key, &store);
    let slug = key.replace(['/', ':', '@', '.'], "_");
    let receipt_path = store.join(format!("{slug}.receipt.json"));
    let read = |path: &camino::Utf8Path| {
        crate::vfs::read(path).map_err(|source| RepoError::Io {
            context: format!("could not read shared observation {path}"),
            source,
        })
    };
    let run_bytes = read(&run_path)?;
    let receipt_bytes = read(&receipt_path)?;
    let run: openwarrant_core::GateRun = toml::from_str(
        std::str::from_utf8(&run_bytes).map_err(|e| RepoError::Message(e.to_string()))?,
    )
    .map_err(|e| RepoError::Message(e.to_string()))?;
    let receipt: openwarrant_core::GateReceipt =
        serde_json::from_slice(&receipt_bytes).map_err(|e| RepoError::Message(e.to_string()))?;
    run.validate()
        .map_err(|e| RepoError::Message(e.to_string()))?;
    receipt
        .validate()
        .map_err(|e| RepoError::Message(e.to_string()))?;
    if !super::receipt_digest_recomputes(&receipt)
        || run.verdict != receipt.verdict
        || run.gate != key
        || !subjects.iter().all(|s| receipt.subject_digests.contains(s))
    {
        return Err(RepoError::Message(
            "shared observation failed seal or subject validation".into(),
        ));
    }
    if definition(repo, key).as_ref() != Some(&definition_before)
        || !matches!(super::reuse_of(repo, key, &receipt), super::Reuse::Holds(_))
    {
        report.push(Diagnostic::unknown(
            "evidence.shared-source-moved",
            repo.relative(&receipt_path),
            format!(
                "{key}: gate definition or source binding no longer holds after execution; the actual \
                observation remains at {}, but no current Warrant attachment was written",
                repo.relative(&receipt_path)
            ),
        ));
        return Ok(report);
    }
    let actor = format!("agent://{}", repo.performer());
    let payload = attached_payload(key, &receipt.verdict.to_string(), &receipt.receipt_digest);
    // A contract can change outside a gate's declared input set. Recheck each
    // member separately: pause the changed one, not unrelated selected work.
    let members = members
        .into_iter()
        .filter(|member| {
            if current_binding(repo, &member.dir).is_ok_and(|now| now == member.binding) {
                true
            } else {
                report.push(Diagnostic::unknown(
                    "evidence.shared-member-stale",
                    repo.relative(&member.dir),
                    format!(
                        "{}: contract or delivered bytes changed during {key}; \
                    the historical shared observation is retained without a current attachment",
                        member.alias
                    ),
                ));
                false
            }
        })
        .collect::<Vec<_>>();
    // Check every attribution conflict before writing any attachment.
    for member in &members {
        if let crate::journal_cmd::Prior::Conflict { recorded_by } =
            crate::journal_cmd::already_recorded(
                &member.dir,
                crate::journal_cmd::RECEIPT_ATTACHED,
                &payload,
                &actor,
            )?
        {
            report.push(crate::journal_cmd::conflict(
                repo.relative(&member.dir.join(crate::journal_cmd::FILE)),
                crate::journal_cmd::RECEIPT_ATTACHED,
                &recorded_by,
                &actor,
            ));
            return Ok(report);
        }
    }
    for member in members {
        let out = member.dir.join(GATE_RUNS_DIR);
        std::fs::create_dir_all(&out).map_err(|source| RepoError::Io {
            context: format!("could not create shared attachment directory {out}"),
            source,
        })?;
        crate::compile::atomic::write(&crate::gate_cmd::run_record_path(key, &out), &run_bytes)?;
        crate::compile::atomic::write(&out.join(format!("{slug}.receipt.json")), &receipt_bytes)?;
        crate::journal_cmd::record(
            &member.dir,
            &member.uuid,
            crate::journal_cmd::RECEIPT_ATTACHED,
            &actor,
            &payload,
        )?;
        report.push(Diagnostic::pass(
            "evidence.shared-attached",
            format!(
                "{}: {key} attached the unchanged shared observation {}; no additional execution",
                member.alias, receipt.receipt_digest,
            ),
        ));
    }
    Ok(report)
}
