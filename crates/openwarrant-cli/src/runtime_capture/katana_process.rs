// SPDX-License-Identifier: Apache-2.0
//! Native Katana verifier over an explicit process boundary. The caller must
//! authenticate and protect the verifier, original event log and trusted head.
//! The head must not come from the receipt being checked. No AGPL linkage,
//! auto-discovery, dollar accounting, sandbox witness or assurance is implied.
use openwarrant_core::{
    document::{records::raw_digest, runtime::*},
    seam::KatanaReceipt,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command, time::Duration};

use crate::repo::Repository;
pub const RESPONSE_SCHEMA: &str = "katana/openwarrant-verification/v1";

pub struct KatanaProcessConfig {
    pub provider: ProviderInterface,
    pub executable: PathBuf,
    pub event_log: PathBuf,
    /// Independently retained native BLAKE3 head, not a receipt declaration.
    pub trusted_log_head: String,
    pub scratch_root: PathBuf,
    pub timeout: Duration,
    pub max_response_bytes: usize,
}
pub struct KatanaProcessVerifier {
    config: KatanaProcessConfig,
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Native {
    schema: String,
    binding: Binding,
    session_id: String,
    outcome: String,
    prompt_ir_digest: Option<String>,
    provider_model_identity: Vec<String>,
    event_log_head: String,
    receipt_digest: String,
    realized_capabilities: Vec<String>,
    capability_observation: String,
    confinement: String,
    input_tokens: u64,
    output_tokens: u64,
    usage_estimated: bool,
    cost_usd: Option<f64>,
    artifact_event_refs: Vec<String>,
    taint_event_refs: Vec<String>,
    assurance_granted: bool,
}
fn rejected(e: impl ToString) -> ProviderFailure {
    ProviderFailure::Rejected(e.to_string())
}
fn digest(s: &str) -> bool {
    s.strip_prefix("b3:").is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
impl KatanaProcessVerifier {
    /// Resolve exact binding from retained Dispatch and compile event. The
    /// caller still owns current-basis assessment and protected source custody.
    pub fn for_recorded_dispatch(
        repo: &Repository,
        alias: &str,
        dispatch_id: &str,
        config: KatanaProcessConfig,
    ) -> Result<Self, ProviderFailure> {
        if config.provider.kind != ProviderKind::Katana
            || config.provider.identity.trim().is_empty()
            || config.provider.version != RESPONSE_SCHEMA
            || !digest(&config.trusted_log_head)
            || config.timeout.is_zero()
            || config.timeout > Duration::from_secs(60)
            || config.max_response_bytes == 0
            || config.max_response_bytes > 4 * 1024 * 1024
            || [&config.executable, &config.event_log, &config.scratch_root]
                .iter()
                .any(|p| !p.is_absolute())
        {
            return Err(rejected(
                "explicit absolute paths, protected native head and bounded process configuration required",
            ));
        }
        let recorded = super::recorded::load(repo, alias, dispatch_id).map_err(|f| {
            if f.unknown {
                ProviderFailure::Unavailable(f.message)
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
    fn map(&self, raw: &[u8], value: Value) -> Result<VerifiedReceipt, ProviderFailure> {
        // This native protocol returns its checked receipt unchanged. Require
        // that property in addition to trusting the caller-selected verifier.
        if crate::sdk::wire::decode_value(raw).map_err(rejected)? != value {
            return Err(rejected(
                "native response describes different receipt bytes",
            ));
        }
        let n: Native = serde_json::from_value(value).map_err(rejected)?;
        let binding = RuntimeBinding {
            warrant_ref: n.binding.warrant_ref,
            contract_digest: n.binding.contract_digest,
            dispatch_digest: n.binding.dispatch_digest,
            stage_id: n.binding.stage_id,
            attempt_id: n.binding.attempt_id,
        };
        if n.schema != "katana/openwarrant-receipt/v1"
            || binding != self.binding
            || n.event_log_head != self.config.trusted_log_head
            || n.receipt_digest != n.event_log_head
            || n.session_id.trim().is_empty()
            || n.confinement.trim().is_empty()
            || n.capability_observation != "dispatch-permission-upper-bound"
            || n.assurance_granted
            || n.cost_usd.is_some()
            || n.realized_capabilities.len() > 4096
            || n.artifact_event_refs.len() > 100_000
            || n.taint_event_refs.len() > 100_000
            || n.provider_model_identity.len() > 100_000
            || n.prompt_ir_digest.as_deref().is_some_and(|p| !digest(p))
        {
            return Err(rejected(
                "native binding, head, observation or field budget differs",
            ));
        }
        let outcome = match n.outcome.as_str() {
            "completed" => RuntimeOutcome::Completed,
            "failed" => RuntimeOutcome::Failed,
            "halted" => RuntimeOutcome::Halted,
            "cancelled" => RuntimeOutcome::Cancelled,
            _ => return Err(rejected("unsupported native terminal outcome")),
        };
        if outcome == RuntimeOutcome::Completed
            && (n.prompt_ir_digest.is_none() || n.provider_model_identity.is_empty())
        {
            return Err(rejected("completed native turn lacks inference identity"));
        }
        Ok(VerifiedReceipt {
            interface: self.config.provider.clone(), raw_digest: raw_digest(raw), binding,
            receipt: NativeReceipt::Katana(KatanaReceipt {
                session_id: n.session_id, dispatch_digest: self.binding.dispatch_digest.clone(),
                prompt_ir_digest: n.prompt_ir_digest.unwrap_or_default(),
                provider_model_identity: serde_json::to_string(&n.provider_model_identity).map_err(rejected)?,
                runtime_event_log_head: n.event_log_head, realized_capabilities: n.realized_capabilities,
                confinement: n.confinement,
                usage: json!({"input_tokens":n.input_tokens,"output_tokens":n.output_tokens,"estimated":n.usage_estimated,"cost_usd":null}).to_string(),
                // Provider-owned event references, not invented output files.
                artifact_refs: n.artifact_event_refs, terminal_runtime_status: n.outcome,
                receipt_digest: n.receipt_digest, taint_label_refs: n.taint_event_refs,
            }),
            outcome, execution: Observation::Established,
            confinement: Observation::Unknown, metered_cost: Observation::Unknown,
            spend_cap: Observation::Unknown, registry_digest: None,
        })
    }
}
impl ReceiptVerifier for KatanaProcessVerifier {
    fn interface(&self) -> &ProviderInterface {
        &self.config.provider
    }
    fn verify(&self, raw: &[u8]) -> Result<VerifiedReceipt, ProviderFailure> {
        if !cfg!(unix) {
            return Err(ProviderFailure::Unsupported(
                "bounded process supervision requires Unix process groups".into(),
            ));
        }
        if raw.is_empty() || raw.len() > 4 * 1024 * 1024 {
            return Err(rejected("native receipt byte budget"));
        }
        let scratch = super::process::Scratch::create(&self.config.scratch_root, "katana")?;
        let receipt = scratch.write("receipt.json", raw)?;
        let b = &self.binding;
        let binding = scratch.write("binding.json", &serde_json::to_vec(&json!({
            "warrant_ref":b.warrant_ref,"contract_digest":b.contract_digest,"dispatch_digest":b.dispatch_digest,
            "stage_id":b.stage_id,"attempt_id":b.attempt_id
        })).map_err(rejected)?)?;
        let mut command = Command::new(&self.config.executable);
        command
            .env_clear()
            .arg("--receipt")
            .arg(receipt)
            .arg("--log")
            .arg(&self.config.event_log)
            .arg("--binding")
            .arg(binding)
            .arg("--trusted-head")
            .arg(&self.config.trusted_log_head);
        let value = super::process::verify_response(
            command,
            self.config.timeout,
            self.config.max_response_bytes,
            RESPONSE_SCHEMA,
        )?;
        self.map(raw, value)
    }
}
