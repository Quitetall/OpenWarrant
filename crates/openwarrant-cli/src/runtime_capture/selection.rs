// SPDX-License-Identifier: Apache-2.0
//! Explicit current-attempt selection. Never pick a successful historical receipt
//! or reuse a saved verdict. Journal append order defines the last compiled
//! attempt locally; it does not authenticate execution or grant authority.
use super::*;
use openwarrant_compiler::runtime_basis::RuntimeBasisAssessment;
use openwarrant_core::execution::StageDispatch;
use std::collections::BTreeSet;

pub const REQUEST_SCHEMA: &str = "oh.war/runtime-selection-request/v1-draft.1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub stage_id: String,
    pub capture_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionRequest {
    pub schema: String,
    pub selections: Vec<Selection>,
}

/// Assess all runtime stages using explicit retained captures, actual current
/// source records and freshly resolved provider verification/policy.
///
/// The callback must supply trusted, dispatch-specific policy and a native
/// verifier; collector metadata and stored verdicts are not policy inputs.
/// Missing adapters or stages remain UNKNOWN. Superseded attempts refuse:
/// compiling a later attempt cannot silently fall back to an older success.
/// This is read-only and grants neither authorization nor assurance. It does
/// not implement an execution scheduler or authenticate the local journal.
pub fn assess_selected<'a>(
    repo: &Repository,
    alias: &str,
    selections: &[Selection],
    resolve: impl Fn(&StageDispatch, &ProviderInterface) -> Option<Verification<'a>>,
) -> Result<RuntimeBasisAssessment, Fault> {
    if selections.len() > 256 {
        return Err(fault(
            "runtime.selection-limit",
            "too many selected captures",
            false,
        ));
    }
    let directory = repo
        .warrant_dir(alias)
        .map_err(|e| fault("runtime.capture-warrant", e, false))?;
    let current = repo
        .load_warrant(&directory)
        .map_err(|e| fault("runtime.capture-basis", e, true))?;
    let basis = current.basis.ok_or_else(|| {
        fault(
            "runtime.capture-basis",
            "current source basis unavailable",
            true,
        )
    })?;
    let validated = current.validated.ok_or_else(|| {
        fault(
            "runtime.capture-basis",
            "validated manifest unavailable",
            true,
        )
    })?;
    let mut seen = BTreeSet::new();
    let mut loaded = vec![];
    let mut total_bytes = 0usize;
    for selected in selections {
        if selected.stage_id.trim().is_empty() || !seen.insert(&selected.stage_id) {
            return Err(fault(
                "runtime.selection-ambiguous",
                "one explicit capture per nonempty stage required",
                false,
            ));
        }
        let shown = show(repo, alias, &selected.capture_digest)?;
        let capture = &shown["record"];
        if capture["declared_capture"]["schema"] != super::REQUEST_SCHEMA {
            return Err(fault(
                "runtime.capture-unsupported-schema",
                "unsupported captured request",
                true,
            ));
        }
        let request: Request = serde_json::from_value(capture["declared_capture"].clone())
            .map_err(|e| fault("runtime.selection-capture", e, false))?;
        let recorded = recorded::load(repo, alias, &request.dispatch_id)?;
        if recorded.dispatch.stage_id != selected.stage_id {
            return Err(fault(
                "runtime.selection-stage",
                "capture belongs to another stage",
                false,
            ));
        }
        if !recorded.latest_for_stage {
            return Err(fault(
                "runtime.selection-superseded-attempt",
                "a later attempt was compiled for this stage; historical receipt cannot clear it",
                false,
            ));
        }
        for (name, bytes, reference) in [
            ("dispatch", &recorded.dispatch_bytes, &recorded.dispatch_ref),
            ("compile_event", &recorded.event_bytes, &recorded.event_ref),
        ] {
            if capture[name]["digest"] != format!("sha256:{}", sha256_hex(bytes))
                || capture[name]["reference"] != *reference
            {
                return Err(fault(
                    "runtime.selection-recorded-binding",
                    "capture differs from actual retained dispatch or compile event",
                    false,
                ));
            }
        }
        // Detect an observed source change during this selection pass. This is
        // not a transaction or a same-UID isolation claim.
        if recorded.basis != basis {
            return Err(fault(
                "runtime.selection-basis-changed",
                "source basis changed while selecting captures",
                true,
            ));
        }
        let provider = provider(&request)?;
        let raw = base64_decode(
            capture["receipt"]["base64"]
                .as_str()
                .ok_or_else(|| fault("runtime.capture-blob", "missing receipt bytes", false))?,
        )
        .map_err(|e| fault("runtime.capture-blob", e, false))?;
        if raw.len() > RECEIPT_LIMIT {
            return Err(fault(
                "runtime.capture-limit",
                "retained receipt exceeds native limit",
                false,
            ));
        }
        total_bytes += raw.len() + recorded.dispatch_bytes.len() + recorded.event_bytes.len();
        if total_bytes > 32 * 1024 * 1024 {
            return Err(fault(
                "runtime.selection-limit",
                "selected source bytes exceed aggregate limit",
                false,
            ));
        }
        loaded.push((recorded.dispatch, provider, raw));
    }
    let source_evidence: Vec<_> = loaded
        .iter()
        .map(|(dispatch, provider, raw)| RuntimeStageEvidence {
            expectation: expectation(dispatch, provider, None),
            raw_receipt: raw,
            verifier: None,
        })
        .collect();
    let source_assessment = assess_runtime_basis(&basis, &validated, &source_evidence);
    if source_assessment.standing == ReceiptStanding::Refused
        || (source_assessment.standing == ReceiptStanding::Unknown
            && source_assessment.stages.is_empty())
    {
        return Ok(source_assessment);
    }
    // Known stale/invalid sources refuse before provider or policy callbacks.
    let verification: Vec<_> = loaded
        .iter()
        .map(|(dispatch, provider, _)| resolve(dispatch, provider))
        .collect();
    let evidence: Vec<_> = loaded
        .iter()
        .zip(&verification)
        .map(
            |((dispatch, provider, raw), verification)| RuntimeStageEvidence {
                expectation: expectation(dispatch, provider, verification.as_ref()),
                raw_receipt: raw,
                verifier: verification.as_ref().map(|v| v.verifier),
            },
        )
        .collect();
    let assessed = assess_runtime_basis(&basis, &validated, &evidence);
    // Native callbacks can take time. Refuse observed source/attempt changes
    // before returning a previously current verdict. Callers still own locking.
    for (dispatch, _, _) in &loaded {
        let current = recorded::load(repo, alias, &dispatch.dispatch_id)?;
        if !current.latest_for_stage || current.basis != basis || current.dispatch != *dispatch {
            return Err(fault(
                "runtime.selection-changed",
                "basis or recorded attempt changed during assessment",
                true,
            ));
        }
    }
    Ok(assessed)
}

pub(super) fn render(value: &RuntimeBasisAssessment) -> Value {
    fn label(standing: ReceiptStanding) -> &'static str {
        match standing {
            ReceiptStanding::Matches => "matches",
            ReceiptStanding::Refused => "refused",
            ReceiptStanding::Unknown => "unknown",
        }
    }
    json!({"standing":label(value.standing),"code":value.code,"detail":value.detail,
        "stages":value.stages.iter().map(|stage| json!({"stage_id":stage.stage_id,"receipt":native(&stage.receipt)})).collect::<Vec<_>>(),
        "saved_native_observations_used":false,"assurance_granted":false})
}
