// SPDX-License-Identifier: Apache-2.0
//! Explicit native BLUT process adapter. The host must authenticate and protect
//! the configured executable, key, expected plan and native job. Paths/labels
//! do not establish that trust. No discovery, execution retry or assurance.
use openwarrant_core::{
    document::{records::raw_digest, runtime::*},
    seam::BlutLineageReceipt,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command, time::Duration};

use crate::repo::Repository;

pub const RESPONSE_SCHEMA: &str = "blut/openwarrant-verification/v1";

/// Caller-trusted configuration, never read from a receipt or auto-discovered.
/// The receipt verifier establishes no isolation of this configuration itself.
pub struct BlutProcessConfig {
    pub provider: ProviderInterface,
    pub executable: PathBuf,
    pub public_key: PathBuf,
    pub plan: PathBuf,
    pub job: PathBuf,
    pub producer_executable: PathBuf,
    pub scratch_root: PathBuf,
    pub timeout: Duration,
    pub max_response_bytes: usize,
}

pub struct BlutProcessVerifier {
    config: BlutProcessConfig,
    binding: RuntimeBinding,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    warrant_ref: String,
    contract_digest: String,
    dispatch_digest: String,
    stage_id: String,
    attempt_id: String,
}
#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct NativeFile {
    path: String,
    bytes: u64,
    digest: String,
    mode: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Native {
    binding: Binding,
    outcome: String,
    run_id: String,
    receipt_digest: String,
    registry_digest: String,
    native_files: Vec<NativeFile>,
    lineage_reference: Option<NativeFile>,
}

fn native_digest(digest: &str) -> bool {
    digest.strip_prefix("blake3:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn file_reference(job: &std::path::Path, file: &NativeFile) -> Result<String, ProviderFailure> {
    let path = std::path::Path::new(&file.path);
    let parts: Vec<_> = path.components().collect();
    if parts.is_empty()
        || parts
            .iter()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
        || file.path.contains('\\')
        || file.path.chars().any(char::is_control)
        || parts
            .iter()
            .map(|p| p.as_os_str().to_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("/")
            != file.path
        || !native_digest(&file.digest)
    {
        return Err(rejected("invalid native file reference"));
    }
    let joined = job.join(path);
    let absolute = joined
        .to_str()
        .ok_or_else(|| rejected("native job reference is not UTF-8"))?;
    let mut reference = String::from("file://");
    for b in absolute.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            reference.push(char::from(b));
        } else {
            use std::fmt::Write;
            write!(reference, "%{b:02X}").expect("String write");
        }
    }
    Ok(reference)
}

impl BlutProcessVerifier {
    fn map(&self, raw: &[u8], native: Value) -> Result<VerifiedReceipt, ProviderFailure> {
        let native: Native = serde_json::from_value(native).map_err(rejected)?;
        let binding = RuntimeBinding {
            warrant_ref: native.binding.warrant_ref,
            contract_digest: native.binding.contract_digest,
            dispatch_digest: native.binding.dispatch_digest,
            stage_id: native.binding.stage_id,
            attempt_id: native.binding.attempt_id,
        };
        if binding != self.binding
            || native.run_id.trim().is_empty()
            || !native_digest(&native.receipt_digest)
            || !native_digest(&native.registry_digest)
            || native.native_files.len() > 4096
        {
            return Err(rejected(
                "native binding, identity or reference budget differs",
            ));
        }
        let outcome = match native.outcome.as_str() {
            "completed" | "completed_with_warnings" => RuntimeOutcome::Completed,
            "failed" => RuntimeOutcome::Failed,
            "cancelled" => RuntimeOutcome::Cancelled,
            "deadline_exceeded" => RuntimeOutcome::Halted,
            _ => return Err(rejected("unsupported native outcome")),
        };
        let mut names = std::collections::BTreeSet::new();
        let mut references = Vec::new();
        for file in &native.native_files {
            if !names.insert(&file.path) {
                return Err(rejected("duplicate native file reference"));
            }
            references.push(file_reference(&self.config.job, file)?);
        }
        let lineage = match native.lineage_reference {
            Some(file)
                if file.path == "status.jsonl"
                    && file.bytes > 0
                    && native.native_files.contains(&file) =>
            {
                file_reference(&self.config.job, &file)?
            }
            Some(_) => {
                return Err(rejected(
                    "native lifecycle reference differs from checked file table",
                ));
            }
            None => String::new(),
        };
        if outcome == RuntimeOutcome::Completed && lineage.is_empty() {
            return Err(rejected(
                "completed native execution has no retained lifecycle",
            ));
        }
        Ok(VerifiedReceipt {
            interface: self.config.provider.clone(),
            raw_digest: raw_digest(raw),
            binding,
            receipt: NativeReceipt::Blut(BlutLineageReceipt {
                status: native.outcome,
                artifact_refs: references,
                lineage_ref: lineage,
                receipt_digest: native.receipt_digest,
            }),
            outcome,
            execution: if native.native_files.is_empty() {
                Observation::Unknown
            } else {
                Observation::Established
            },
            confinement: Observation::Unknown,
            metered_cost: Observation::Unknown,
            spend_cap: Observation::Unknown,
            // This is the provider-established native catalog identity. It is
            // never relabeled as a SHA-256 digest or copied from a lowering hint.
            registry_digest: Some(native.registry_digest),
        })
    }
}

fn unavailable(e: impl ToString) -> ProviderFailure {
    ProviderFailure::Unavailable(e.to_string())
}
fn rejected(e: impl ToString) -> ProviderFailure {
    ProviderFailure::Rejected(e.to_string())
}

impl BlutProcessVerifier {
    /// Bind to a retained Dispatch and its matching compile event. This is
    /// local record integrity, not authentication, permission or current-basis
    /// eligibility. The capture/basis assessor must still check current sources.
    pub fn for_recorded_dispatch(
        repo: &Repository,
        alias: &str,
        dispatch_id: &str,
        config: BlutProcessConfig,
    ) -> Result<Self, ProviderFailure> {
        if config.provider.kind != ProviderKind::Blut
            || config.provider.identity.trim().is_empty()
            || config.provider.version.trim().is_empty()
            || config.timeout.is_zero()
            || config.timeout > Duration::from_secs(60)
            || config.max_response_bytes == 0
            || config.max_response_bytes > 4 * 1024 * 1024
            || [
                &config.executable,
                &config.public_key,
                &config.plan,
                &config.job,
                &config.producer_executable,
                &config.scratch_root,
            ]
            .iter()
            .any(|p| !p.is_absolute())
        {
            return Err(rejected(
                "explicit absolute paths and positive bounded process configuration required",
            ));
        }
        let recorded = super::recorded::load(repo, alias, dispatch_id).map_err(|f| {
            if f.unknown {
                unavailable(f.message)
            } else {
                rejected(f.message)
            }
        })?;
        if !recorded.latest_for_stage {
            return Err(rejected(
                "a newer recorded attempt supersedes this Dispatch",
            ));
        }
        Ok(Self {
            config,
            binding: RuntimeBinding::from_dispatch(&recorded.dispatch),
        })
    }
}

impl ReceiptVerifier for BlutProcessVerifier {
    fn interface(&self) -> &ProviderInterface {
        &self.config.provider
    }
    fn verify(&self, raw_receipt: &[u8]) -> Result<VerifiedReceipt, ProviderFailure> {
        if !cfg!(unix) {
            return Err(ProviderFailure::Unsupported(
                "bounded process supervision requires Unix process groups".into(),
            ));
        }
        if raw_receipt.is_empty() || raw_receipt.len() > 4 * 1024 * 1024 {
            return Err(rejected("native receipt byte budget"));
        }
        let scratch = super::process::Scratch::create(&self.config.scratch_root, "blut")?;
        let receipt = scratch.write("receipt.json", raw_receipt)?;
        let b = &self.binding;
        let binding = scratch.write("binding.json", &serde_json::to_vec(&json!({
            "warrant_ref":b.warrant_ref,"contract_digest":b.contract_digest,"dispatch_digest":b.dispatch_digest,
            "stage_id":b.stage_id,"attempt_id":b.attempt_id
        })).map_err(unavailable)?)?;
        let mut command = Command::new(&self.config.executable);
        // Verification uses explicit public inputs, not ambient credentials or
        // caller routing. The configured executable must work without them.
        command.env_clear();
        command
            .args(["openwarrant", "verify", "--receipt"])
            .arg(receipt)
            .arg("--public-key")
            .arg(&self.config.public_key)
            .arg("--binding")
            .arg(binding)
            .arg("--plan")
            .arg(&self.config.plan)
            .arg("--job")
            .arg(&self.config.job)
            .arg("--producer-executable")
            .arg(&self.config.producer_executable);
        let native = super::process::verify_response(
            command,
            self.config.timeout,
            self.config.max_response_bytes,
            RESPONSE_SCHEMA,
        )?;
        self.map(raw_receipt, native)
    }
}
