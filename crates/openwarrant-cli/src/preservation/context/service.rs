// SPDX-License-Identifier: Apache-2.0
//! Retention checks for local gate service attempts. No execution qualification.
use openwarrant_core::{
    ExecutionStatus, GateReceipt, GateRun,
    execution::{StageDispatch, StageSubmission},
    milestones::Stage,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn collect(
    files: &BTreeMap<String, Vec<u8>>,
    root: &str,
    stage: &Stage,
) -> Result<BTreeSet<String>, String> {
    let gate = stage
        .executor_ref
        .as_deref()
        .and_then(|value| value.strip_prefix("gate://"))
        .filter(|value| !value.is_empty())
        .ok_or("service backend has no local gate resolver")?;
    let dispatch_root = format!("{root}/dispatches/");
    let mut paths = BTreeSet::new();
    for (path, bytes) in files
        .iter()
        .filter(|(path, _)| path.starts_with(&dispatch_root))
    {
        let dispatch: StageDispatch = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if dispatch.stage_id != stage.id {
            continue;
        }
        if path != &format!("{dispatch_root}{}.json", dispatch.dispatch_id)
            || dispatch.dispatch_id.is_empty()
            || !dispatch
                .dispatch_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err(format!("{path}: invalid local dispatch identity"));
        }
        let mut blank = dispatch.clone();
        blank.dispatch_digest.clear();
        let digest = openwarrant_compiler::canonical::sha256_digest(
            openwarrant_compiler::digest::DigestDomain::Dispatch,
            &blank,
        )
        .map_err(|e| e.to_string())?;
        if dispatch.api_version != openwarrant_core::execution::DISPATCH_API_VERSION
            || dispatch.dispatch_digest != digest
        {
            return Err(format!("{path}: invalid dispatch seal"));
        }
        let attempt = format!("{root}/gate-runs/{}", dispatch.dispatch_id);
        let slug = gate.replace(['/', ':', '@', '.'], "_");
        let run_path = format!("{attempt}/{slug}.run.toml");
        let run_bytes = files
            .get(&run_path)
            .ok_or_else(|| format!("missing attempt run: {run_path}"))?;
        let run: GateRun =
            toml::from_str(std::str::from_utf8(run_bytes).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        run.validate().map_err(|e| e.to_string())?;
        if run.id != format!("GR-{}", dispatch.dispatch_id) || run.gate != gate {
            return Err(format!(
                "{run_path}: run identity differs from dispatch or gate"
            ));
        }
        let submission_path = format!("{root}/submissions/{}.json", dispatch.dispatch_id);
        let submission: StageSubmission = serde_json::from_slice(
            files
                .get(&submission_path)
                .ok_or_else(|| format!("missing attempt submission: {submission_path}"))?,
        )
        .map_err(|e| e.to_string())?;
        submission.validate().map_err(|e| e.to_string())?;
        if submission.dispatch_id != dispatch.dispatch_id
            || submission.attempt_id != dispatch.attempt_id
            || submission.stage_id != dispatch.stage_id
            || submission.contract_digest != dispatch.contract_digest
        {
            return Err(format!(
                "{submission_path}: submission differs from dispatch"
            ));
        }
        let receipt_path = format!("{attempt}/{slug}.receipt.json");
        if run.execution_status == ExecutionStatus::Completed {
            let receipt: GateReceipt = serde_json::from_slice(
                files
                    .get(&receipt_path)
                    .ok_or_else(|| format!("missing completed attempt receipt: {receipt_path}"))?,
            )
            .map_err(|e| e.to_string())?;
            receipt.validate().map_err(|e| e.to_string())?;
            if !crate::evidence::receipt_digest_recomputes(&receipt)
                || receipt.run_id != run.id
                || receipt.verdict != run.verdict
                || receipt.subject_digests != vec![format!("dispatch:{digest}")]
            {
                return Err(format!(
                    "{receipt_path}: receipt differs from sealed dispatch/run"
                ));
            }
            paths.insert(receipt_path);
        } else if files.contains_key(&receipt_path) {
            return Err(format!(
                "{receipt_path}: incomplete attempt has completion receipt"
            ));
        }
        if matches!(
            run.execution_status,
            ExecutionStatus::Completed | ExecutionStatus::Timeout
        ) {
            for suffix in ["stdout.txt", "stderr.txt"] {
                let stream = format!("{attempt}/{slug}.{suffix}");
                if !files.contains_key(&stream) {
                    return Err(format!("missing attempt stream: {stream}"));
                }
            }
        }
        paths.extend([path.clone(), run_path, submission_path]);
        // Streams, including partial timeout output, belong to this exact attempt.
        paths.extend(
            files
                .keys()
                .filter(|p| p.starts_with(&format!("{attempt}/")))
                .cloned(),
        );
    }
    Ok(paths)
}
