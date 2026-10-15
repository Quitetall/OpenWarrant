// SPDX-License-Identifier: Apache-2.0
//! Experimental model view beside a lossless v2 archive (proposed ADR0030).
//! This is deliberately not a verifier input or an ingestion protocol.
use super::*;
use serde_json::{Value, json};

const SCHEMA: &str = "oh.war/verification-view-preview/v1";

fn encoded(value: &Value) -> Result<String, RepoError> {
    serde_jcs::to_string(value).map_err(|e| RepoError::Message(e.to_string()))
}

/// Estimate the actual final preview, including its accounting fields.
fn account(mut value: Value, budget: u64) -> Result<Value, RepoError> {
    value["estimated_tokens"] = json!(0);
    value["over_budget"] = json!(false);
    for _ in 0..16 {
        let estimate = openwarrant_core::tokens::estimate(encoded(&value)?.len() as u64);
        if value["estimated_tokens"] == estimate && value["over_budget"] == (estimate > budget) {
            return Ok(value);
        }
        value["estimated_tokens"] = json!(estimate);
        value["over_budget"] = json!(estimate > budget);
    }
    Err(RepoError::Message(
        "verify.preview-accounting: token estimate did not converge".into(),
    ))
}

struct Preview {
    archive_name: String,
    archive_bytes: Vec<u8>,
    view: Value,
}

fn project(bundle: &Bundle) -> Result<Preview, RepoError> {
    let (archive, digest) = canonical(bundle)?;
    let archive_name = format!("bundle-{}.json", short(&digest));
    let archive_bytes = (archive + "\n").into_bytes();
    let mut view = serde_json::to_value(bundle).map_err(|e| RepoError::Message(e.to_string()))?;
    view["schema"] = json!(SCHEMA);
    view["dispatch_eligible"] = json!(false);
    view["limitations"] = json!([
        "Preview only: current verifier adapters and ingestion do not support this view.",
        "Archived bytes are available offline, but have not been shown in this view.",
        "Excerpts cannot establish claims requiring observations outside their coverage."
    ]);
    view["archive"] = json!({
        "path": archive_name,
        "digest": digest,
        "sha256": sha256_hex(&archive_bytes),
        "bytes": archive_bytes.len()
    });
    let request = view["request"].as_object_mut().ok_or_else(|| {
        RepoError::Message("verify.preview-request: expected request object".into())
    })?;
    request.remove("reviewed_subject");
    request.insert("reviewed_subject_archive".into(), json!({
        "pointer": "/request/reviewed_subject",
        "gate_input_count": bundle.request.reviewed_subject.gate_inputs.len(),
        "meaning": "Exact workspace bindings are retained in the accompanying archive; this is not a source coverage claim."
    }));
    // Raw evidence remains lossless in the archive. The existing gate_runs
    // projection carries explicitly numbered excerpts under its own bounds.
    for source in view["required_sources"]
        .as_array_mut()
        .ok_or_else(|| RepoError::Message("verify.preview-sources: expected source array".into()))?
    {
        if source["kind"] == "gate-evidence" {
            let source = source.as_object_mut().ok_or_else(|| {
                RepoError::Message("verify.preview-source: expected source object".into())
            })?;
            source.remove("text");
            source.remove("bytes");
            source.insert("archive_only".into(), json!(true));
        }
    }
    Ok(Preview {
        archive_name,
        archive_bytes,
        view: account(view, bundle.budget_tokens)?,
    })
}

fn obligation(src: &Sources, requested: &RequestedObligation) -> Result<Preview, RepoError> {
    let fixed = project(&obligation_bundle_at(src, requested, 0)?)?;
    let budget = src.budget.saturating_mul(4);
    let fixed_bytes = encoded(&fixed.view)?.len() as u64;
    if fixed_bytes >= budget {
        return Ok(fixed);
    }
    let mut allowance = usize::try_from(budget - fixed_bytes).unwrap_or(usize::MAX);
    for _ in 0..16 {
        let preview = project(&obligation_bundle_at(src, requested, allowance)?)?;
        if preview.view["over_budget"] == false || allowance == 0 {
            return Ok(preview);
        }
        let over = (encoded(&preview.view)?.len() as u64).saturating_sub(budget);
        let over = usize::try_from(over).unwrap_or(usize::MAX);
        allowance = allowance.saturating_sub(over.saturating_add(over / 4).saturating_add(1024));
    }
    Ok(fixed)
}

pub(crate) fn emit(
    repo: &Repository,
    alias: &str,
    performer: &str,
    directory: &Utf8Path,
) -> Result<(Report, Value), RepoError> {
    if !directory.is_absolute()
        || directory.starts_with(&repo.root)
        || directory.as_std_path().components().any(|c| {
            !matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
    {
        return Err(RepoError::Message("verify.preview-path: an absolute directory outside the reviewed repository, without dot components, is required".into()));
    }
    // Anchor every output component without following symlinks. Keep package
    // outputs out of the workspace: emitting a preview must not change its subject.
    let relative = directory
        .strip_prefix("/")
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let retained = store::Directory::open(Utf8Path::new("/"), relative)?;
    let src = load(repo, alias, performer)?;
    let previews = if src.request.obligations.is_empty() {
        vec![project(&warrant_bundle(&src)?)?]
    } else {
        src.request
            .obligations
            .iter()
            .map(|o| obligation(&src, o))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut report = Report::default();
    let mut index = Vec::new();
    for preview in previews {
        let text = encoded(&preview.view)?;
        let hash = sha256_hex(text.as_bytes());
        let name = format!("preview-{}.json", hash.trim_start_matches("sha256:"));
        retained.retain(&preview.archive_name, &preview.archive_bytes)?;
        retained.retain(&name, text.as_bytes())?;
        index.push(
            json!({"path": directory.join(&name).as_str(), "sha256": hash,
            "estimated_tokens": preview.view["estimated_tokens"],
            "budget_tokens": src.budget, "over_budget": preview.view["over_budget"],
            "dispatch_eligible": false}),
        );
        report.push(Diagnostic::pass("verify.preview", format!("{alias}: review preview and full archive retained at {directory}/{name}; no verification or dispatch performed")));
        if preview.view["over_budget"] == true {
            report.push(Diagnostic::warn("verify.preview-over-budget", directory.to_string(), "Exact required context exceeds the preview budget; it was retained without removing rules or fixtures"));
        }
    }
    Ok((
        report,
        json!({"schema": "oh.war/review-previews/v1", "dispatch_eligible": false, "previews": index}),
    ))
}
