// SPDX-License-Identifier: Apache-2.0
//! Explicit operator-protected native assessment. Configuration is local host
//! policy, not a document-standard authority act or a receipt declaration.
use super::{
    Fault, Provider, Request, Selection, Verification,
    collector_loading::{self, LoadedEnrollment},
    collector_signature::OpenSshSignatureCheck,
    katana_process::{KatanaProcessConfig, KatanaProcessVerifier},
    protected_input::ProtectedInput,
};
use crate::repo::Repository;
use openwarrant_core::{
    document::runtime::{ProviderInterface, ProviderKind, RuntimeBinding},
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
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    dispatch_digest: String,
    provider: Provider,
    executable: PathBuf,
    event_log: Input,
    trusted_log_head: String,
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
struct Adapter {
    binding: RuntimeBinding,
    provider: ProviderInterface,
    verifier: KatanaProcessVerifier,
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
        if value["schema"] != "oh.war/runtime-native-config/v1-draft.1" {
            return Err(super::fault(
                "runtime.native-host",
                "native host schema unsupported",
                true,
            ));
        }
        let config: Configuration = serde_json::from_value(value)
            .map_err(|e| super::fault("runtime.native-host", e, false))?;
        let _schema = config.schema;
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
            if provider.kind != ProviderKind::Katana {
                return Err(super::fault(
                    "runtime.native-host",
                    "protected BLUT job-directory custody is not yet available",
                    true,
                ));
            }
            input_bytes = input_bytes
                .checked_add(entry.event_log.max_bytes)
                .ok_or_else(|| {
                    super::fault("runtime.native-host", "native input budget overflow", false)
                })?;
            if input_bytes > 128 * 1024 * 1024 {
                return Err(super::fault(
                    "runtime.native-host",
                    "aggregate native input budget",
                    false,
                ));
            }
            let host = LoadedEnrollment::load_host(store, repo.root.as_std_path(), &signatures)
                .map_err(host_fault)?
                .with_native_configuration(bytes.clone())
                .map_err(host_fault)?;
            let input = ProtectedInput::acquire(
                &entry.event_log.path,
                &entry.event_log.sha256,
                entry.event_log.max_bytes,
            )
            .map_err(provider_fault)?;
            let verifier = KatanaProcessVerifier::for_protected_host(
                repo,
                alias,
                &request.dispatch_id,
                KatanaProcessConfig {
                    provider: provider.clone(),
                    executable: entry.executable.clone(),
                    event_log: entry.event_log.path.clone(),
                    trusted_log_head: entry.trusted_log_head.clone(),
                    scratch_root: repo.root.as_std_path().to_owned(),
                    timeout: Duration::from_millis(entry.timeout_ms),
                    max_response_bytes: entry.max_response_bytes,
                },
                host,
                input,
            )
            .map_err(provider_fault)?;
            adapters.push(Adapter {
                binding,
                provider,
                verifier,
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
                verifier: &a.verifier,
                registry_digest: None,
                authorized_capabilities: Some(&a.capabilities),
                confinement_required: a.confinement_required,
                hard_spend_cap_required: a.hard_spend_cap_required,
            })
    }
}
