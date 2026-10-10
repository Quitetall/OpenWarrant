// SPDX-License-Identifier: Apache-2.0
//! Explicit operator-protected native assessment. Configuration is local host
//! policy, not a document-standard authority act or a receipt declaration.
use super::{
    Fault, Provider, Request, Selection, Verification,
    blut_process::{BlutProcessConfig, BlutProcessVerifier, BlutProtectedInputs},
    collector_loading::{self, LoadedEnrollment},
    collector_signature::OpenSshSignatureCheck,
    katana_process::{KatanaProcessConfig, KatanaProcessVerifier},
    protected_input::ProtectedInput,
    protected_job::{JobFile, ProtectedJob},
};
use crate::repo::Repository;
use openwarrant_core::{
    document::runtime::{ProviderInterface, ProviderKind, ReceiptVerifier, RuntimeBinding},
    execution::StageDispatch,
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema: String,
    entries: Vec<Entry>,
    max_input_bytes: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    dispatch_digest: String,
    provider: Provider,
    executable: PathBuf,
    event_log: Option<Input>,
    trusted_log_head: Option<String>,
    blut_inputs: Option<BlutInputs>,
    authorized_capabilities: Vec<String>,
    confinement_required: bool,
    hard_spend_cap_required: bool,
    timeout_ms: u64,
    max_response_bytes: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    path: PathBuf,
    sha256: String,
    max_bytes: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BlutInputs {
    public_key: Input,
    plan: Input,
    producer: Input,
    job: Job,
    registry_digest: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    path: PathBuf,
    files: Vec<JobFile>,
    max_bytes: usize,
}
struct Adapter {
    binding: RuntimeBinding,
    provider: ProviderInterface,
    verifier: Box<dyn ReceiptVerifier>,
    registry_digest: Option<String>,
    capabilities: Vec<String>,
    confinement_required: bool,
    hard_spend_cap_required: bool,
}
pub struct NativeAssessment {
    adapters: Vec<Adapter>,
}
fn host_fault(fault: openwarrant_core::runtime_collector::Fault) -> Fault {
    use openwarrant_core::runtime_collector::Fault::*;
    match fault {
        Rejected(reason) => super::fault("runtime.native-host", reason, false),
        Unavailable(reason) => super::fault("runtime.native-host", reason, true),
    }
}
fn provider_fault(fault: openwarrant_core::document::runtime::ProviderFailure) -> Fault {
    use openwarrant_core::document::runtime::ProviderFailure::*;
    match fault {
        Rejected(reason) => super::fault("runtime.native-host", reason, false),
        Unavailable(reason) | Unsupported(reason) => {
            super::fault("runtime.native-host", reason, true)
        }
    }
}
impl NativeAssessment {
    /// The store path is explicit. Its actual ownership, access, authenticated
    /// authority and host UID/repository binding are checked before use.
    /// No active configuration is written, and no archived producer is run.
    pub fn load(
        repo: &Repository,
        alias: &str,
        selections: &[Selection],
        store: &Path,
    ) -> Result<Self, Fault> {
        if selections.len() > 256 {
            return Err(super::fault(
                "runtime.selection-limit",
                "too many selected captures",
                false,
            ));
        }
        let signatures =
            OpenSshSignatureCheck::new(repo.root.as_std_path().to_owned(), Duration::from_secs(10))
                .map_err(host_fault)?;
        // Authenticate the execution account even when the selection is empty.
        LoadedEnrollment::load_host(store, repo.root.as_std_path(), &signatures)
            .map_err(host_fault)?;
        let bytes = collector_loading::configuration_bytes(store, "runtime-native.json")
            .map_err(host_fault)?;
        let value = crate::sdk::wire::decode_value(&bytes)
            .map_err(|e| super::fault("runtime.native-host", e, false))?;
        if ![
            "oh.war/runtime-native-config/v1-draft.1",
            "oh.war/runtime-native-config/v2-draft.1",
        ]
        .iter()
        .any(|v| value["schema"] == *v)
        {
            return Err(super::fault(
                "runtime.native-host",
                "native host schema unsupported",
                true,
            ));
        }
        let config: Configuration = serde_json::from_value(value)
            .map_err(|e| super::fault("runtime.native-host", e, false))?;
        let legacy = config.schema == "oh.war/runtime-native-config/v1-draft.1";
        let input_limit = if legacy {
            if config.max_input_bytes.is_some() {
                return Err(super::fault(
                    "runtime.native-host",
                    "v1 native policy cannot expand its input budget",
                    false,
                ));
            }
            128 * 1024 * 1024
        } else {
            config
                .max_input_bytes
                .filter(|n| *n > 0 && *n <= 2 * 1024 * 1024 * 1024)
                .ok_or_else(|| {
                    super::fault(
                        "runtime.native-host",
                        "v2 requires explicit positive native input budget at most 2 GiB",
                        false,
                    )
                })?
        };
        if config.entries.len() > 256 {
            return Err(super::fault(
                "runtime.native-host",
                "native configuration entry budget",
                false,
            ));
        }
        let mut configured = BTreeSet::new();
        for e in &config.entries {
            if !configured.insert(&e.dispatch_digest)
                || e.dispatch_digest.trim().is_empty()
                || e.authorized_capabilities.len() > 4096
                || e.authorized_capabilities
                    .iter()
                    .any(|c| c.trim().is_empty() || c.len() > 4096)
                || e.timeout_ms == 0
                || e.timeout_ms > 60_000
                || e.max_response_bytes == 0
                || e.max_response_bytes > 4 * 1024 * 1024
            {
                return Err(super::fault(
                    "runtime.native-host",
                    "ambiguous or over-budget native configuration",
                    false,
                ));
            }
        }
        let mut adapters = vec![];
        let mut seen = BTreeSet::new();
        let mut input_bytes = 0usize;
        for selection in selections {
            if !seen.insert(&selection.stage_id) || selection.stage_id.trim().is_empty() {
                return Err(super::fault(
                    "runtime.selection-ambiguous",
                    "one explicit capture per nonempty stage required",
                    false,
                ));
            }
            let shown = super::show(repo, alias, &selection.capture_digest)?;
            let request: Request =
                serde_json::from_value(shown["record"]["declared_capture"].clone())
                    .map_err(|e| super::fault("runtime.native-host", e, false))?;
            if request.schema != super::REQUEST_SCHEMA {
                return Err(super::fault(
                    "runtime.capture-unsupported-schema",
                    "unsupported captured request",
                    true,
                ));
            }
            let recorded = super::recorded::load(repo, alias, &request.dispatch_id)?;
            if recorded.dispatch.stage_id != selection.stage_id || !recorded.latest_for_stage {
                return Err(super::fault(
                    "runtime.native-host",
                    "selected native attempt is superseded or belongs to another stage",
                    false,
                ));
            }
            let binding = RuntimeBinding::from_dispatch(&recorded.dispatch);
            let Some(entry) = config
                .entries
                .iter()
                .find(|e| e.dispatch_digest == binding.dispatch_digest)
            else {
                // Missing explicit host policy must not be inferred from the receipt.
                continue;
            };
            let provider = super::provider(&request)?;
            if entry.provider.kind != request.provider.kind
                || entry.provider.identity != provider.identity
                || entry.provider.version != provider.version
            {
                return Err(super::fault(
                    "runtime.native-host",
                    "protected provider differs from selected capture",
                    false,
                ));
            }
            let host = LoadedEnrollment::load_host(store, repo.root.as_std_path(), &signatures)
                .map_err(host_fault)?
                .with_native_configuration(bytes.clone())
                .map_err(host_fault)?;
            let (verifier, registry_digest): (Box<dyn ReceiptVerifier>, Option<String>) =
                match provider.kind {
                    ProviderKind::Katana => {
                        if entry.blut_inputs.is_some() {
                            return Err(super::fault(
                                "runtime.native-host",
                                "mixed native provider input policy",
                                false,
                            ));
                        }
                        let log = entry.event_log.as_ref().ok_or_else(|| {
                            super::fault("runtime.native-host", "Katana log policy required", false)
                        })?;
                        let head = entry.trusted_log_head.as_ref().ok_or_else(|| {
                            super::fault(
                                "runtime.native-host",
                                "independent Katana head required",
                                false,
                            )
                        })?;
                        input_bytes = input_bytes.checked_add(log.max_bytes).ok_or_else(|| {
                            super::fault(
                                "runtime.native-host",
                                "native input budget overflow",
                                false,
                            )
                        })?;
                        if input_bytes > input_limit {
                            return Err(super::fault(
                                "runtime.native-host",
                                "aggregate native input budget",
                                false,
                            ));
                        }
                        let input = ProtectedInput::acquire(&log.path, &log.sha256, log.max_bytes)
                            .map_err(provider_fault)?;
                        let verifier = KatanaProcessVerifier::for_protected_host(
                            repo,
                            alias,
                            &request.dispatch_id,
                            KatanaProcessConfig {
                                provider: provider.clone(),
                                executable: entry.executable.clone(),
                                event_log: log.path.clone(),
                                trusted_log_head: head.clone(),
                                scratch_root: repo.root.as_std_path().to_owned(),
                                timeout: Duration::from_millis(entry.timeout_ms),
                                max_response_bytes: entry.max_response_bytes,
                            },
                            host,
                            input,
                        )
                        .map_err(provider_fault)?;
                        (Box::new(verifier), None)
                    }
                    ProviderKind::Blut => {
                        if legacy {
                            return Err(super::fault(
                                "runtime.native-host",
                                "protected BLUT inputs require explicit v2 host policy",
                                true,
                            ));
                        }
                        if entry.event_log.is_some() || entry.trusted_log_head.is_some() {
                            return Err(super::fault(
                                "runtime.native-host",
                                "mixed native provider input policy",
                                false,
                            ));
                        }
                        let b = entry.blut_inputs.as_ref().ok_or_else(|| {
                            super::fault(
                                "runtime.native-host",
                                "independent BLUT key, plan, producer, job and catalog required",
                                false,
                            )
                        })?;
                        if !b.registry_digest.strip_prefix("blake3:").is_some_and(|h| {
                            h.len() == 64
                                && h.bytes()
                                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                        }) {
                            return Err(super::fault(
                                "runtime.native-host",
                                "canonical independent BLUT catalog digest required",
                                false,
                            ));
                        }
                        for n in [
                            b.public_key.max_bytes,
                            b.plan.max_bytes,
                            b.producer.max_bytes,
                            b.job.max_bytes,
                        ] {
                            input_bytes = input_bytes.checked_add(n).ok_or_else(|| {
                                super::fault(
                                    "runtime.native-host",
                                    "native input budget overflow",
                                    false,
                                )
                            })?;
                        }
                        if input_bytes > input_limit {
                            return Err(super::fault(
                                "runtime.native-host",
                                "aggregate native input budget",
                                false,
                            ));
                        }
                        let inputs = BlutProtectedInputs {
                            public_key: ProtectedInput::acquire(
                                &b.public_key.path,
                                &b.public_key.sha256,
                                b.public_key.max_bytes,
                            )
                            .map_err(provider_fault)?,
                            plan: ProtectedInput::acquire(
                                &b.plan.path,
                                &b.plan.sha256,
                                b.plan.max_bytes,
                            )
                            .map_err(provider_fault)?,
                            producer: ProtectedInput::acquire(
                                &b.producer.path,
                                &b.producer.sha256,
                                b.producer.max_bytes,
                            )
                            .map_err(provider_fault)?,
                            job: ProtectedJob::acquire(&b.job.path, &b.job.files, b.job.max_bytes)
                                .map_err(provider_fault)?,
                        };
                        let verifier = BlutProcessVerifier::for_protected_host(
                            repo,
                            alias,
                            &request.dispatch_id,
                            BlutProcessConfig {
                                provider: provider.clone(),
                                executable: entry.executable.clone(),
                                public_key: b.public_key.path.clone(),
                                plan: b.plan.path.clone(),
                                job: b.job.path.clone(),
                                producer_executable: b.producer.path.clone(),
                                scratch_root: repo.root.as_std_path().to_owned(),
                                timeout: Duration::from_millis(entry.timeout_ms),
                                max_response_bytes: entry.max_response_bytes,
                            },
                            host,
                            inputs,
                        )
                        .map_err(provider_fault)?;
                        (Box::new(verifier), Some(b.registry_digest.clone()))
                    }
                };
            adapters.push(Adapter {
                binding,
                provider,
                verifier,
                registry_digest,
                capabilities: entry.authorized_capabilities.clone(),
                confinement_required: entry.confinement_required,
                hard_spend_cap_required: entry.hard_spend_cap_required,
            });
        }
        Ok(Self { adapters })
    }
    /// Only an exact retained dispatch and provider receive this host's policy.
    pub fn resolve(
        &self,
        dispatch: &StageDispatch,
        provider: &ProviderInterface,
    ) -> Option<Verification<'_>> {
        let binding = RuntimeBinding::from_dispatch(dispatch);
        self.adapters
            .iter()
            .find(|a| a.binding == binding && &a.provider == provider)
            .map(|a| Verification {
                verifier: a.verifier.as_ref(),
                registry_digest: a.registry_digest.as_deref(),
                authorized_capabilities: Some(&a.capabilities),
                confinement_required: a.confinement_required,
                hard_spend_cap_required: a.hard_spend_cap_required,
            })
    }
}
