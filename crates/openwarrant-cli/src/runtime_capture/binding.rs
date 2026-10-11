// SPDX-License-Identifier: Apache-2.0
//! Source-checked native binding projection; never a permission or receipt.
use super::*;

/// Export existing native field names from the actual current recorded Dispatch.
/// Read-only source checks do not authenticate local history or grant permission.
pub fn binding(repo: &Repository, alias: &str, dispatch_id: &str) -> Result<Value, Fault> {
    let recorded = recorded::load(repo, alias, dispatch_id)?;
    if !recorded.latest_for_stage {
        return Err(fault(
            "runtime.binding-superseded",
            "a newer recorded attempt supersedes this Dispatch",
            false,
        ));
    }
    let kind = openwarrant_compiler::runtime_basis::check_runtime_dispatch_basis(
        &recorded.basis,
        &recorded.validated,
        &recorded.dispatch,
    )
    .map_err(|e| fault(e.code, e.detail, e.standing == ReceiptStanding::Unknown))?;
    let b = RuntimeBinding::from_dispatch(&recorded.dispatch);
    let provider = match kind {
        ProviderKind::Katana => "katana",
        ProviderKind::Blut => "blut",
    };
    // Both current native interfaces use these exact five fields. Do not
    // shorten or rename them merely because an external summary did so.
    let native = json!({"warrant_ref":b.warrant_ref,"contract_digest":b.contract_digest,"dispatch_digest":b.dispatch_digest,"stage_id":b.stage_id,"attempt_id":b.attempt_id});
    // Detect observed source/attempt changes before returning. Callers still own
    // writer isolation and admission at actual execution; this is not a lock.
    let current = recorded::load(repo, alias, dispatch_id)?;
    if !current.latest_for_stage
        || current.basis != recorded.basis
        || current.dispatch_bytes != recorded.dispatch_bytes
        || current.event_bytes != recorded.event_bytes
    {
        return Err(fault(
            "runtime.binding-source-changed",
            "current source or attempt changed during export",
            false,
        ));
    }
    Ok(json!({
        "schema":"oh.war/runtime-binding/v1-draft.1",
        "provider_kind":provider, "binding":native,
        "dispatch_ref":recorded.dispatch_ref, "compile_event_ref":recorded.event_ref,
        "execution_eligibility":"not-assessed", "assurance_granted":false
    }))
}
