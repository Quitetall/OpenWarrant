// SPDX-License-Identifier: Apache-2.0
//! Reference SDK/CLI capture store (OW-WAR-0149), not a native receipt protocol.
//! Source reads are bounded and descriptor-relative; publication never replaces
//! a retained object. Local checksums establish byte identity, not authenticity.
pub mod blut_process;
pub mod collector_loading;
pub mod collector_signature;
pub mod katana_process;
mod process;
pub mod protected_executable;
mod recorded;
mod selection;
pub use selection::{
    REQUEST_SCHEMA as SELECTION_REQUEST_SCHEMA, Selection, SelectionRequest, assess_selected,
};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{
    runtime_basis::{RuntimeStageEvidence, assess_runtime_basis},
    sha256_hex,
};
use openwarrant_core::{
    attestation::{base64_decode, base64_encode},
    document::runtime::*,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    diagnostic::{Diagnostic, Report},
    repo::Repository,
};

pub const SCHEMA: &str = "oh.war/runtime-capture/v1-draft.1";
pub const REQUEST_SCHEMA: &str = "oh.war/runtime-capture-request/v1-draft.1";
const RECORD_LIMIT: usize = 16 * 1024 * 1024;
const RECEIPT_LIMIT: usize = 4 * 1024 * 1024;
const REQUEST_LIMIT: usize = 16 * 1024;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Capture exact bytes against a retained dispatch and its compile event; native eligibility remains unknown without a provider adapter.
    Import {
        alias: String,
        /// Repository-relative JSON capture request supplied by the collector.
        #[arg(long)]
        request: Utf8PathBuf,
    },
    /// Inspect a retained content-addressed capture, including original bytes. Does not trust historical verdicts or establish execution.
    Show { alias: String, digest: String },
    /// Reassess explicitly selected captures against current recorded attempts. No native adapter is configured by this CLI.
    Assess {
        alias: String,
        #[arg(long)]
        selection: Utf8PathBuf,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub kind: String,
    pub identity: String,
    pub version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub observation_id: String,
    pub observed_at: String,
    pub original_receipt_ref: String,
    /// Collector declarations. A checksum or a name does not authenticate a build.
    pub binary_identity: Option<String>,
    pub source_identity: Option<String>,
    pub transport: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    /// Preserve the provider's vocabulary; do not infer completed execution here.
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub dispatch_id: String,
    /// Repository-relative file read once; the original native reference is separate.
    pub receipt: String,
    pub provider: Provider,
    pub metadata: Metadata,
}

/// Trusted, caller-resolved native verification and exact-dispatch policy.
/// The adapter must verify provider seals and provenance, not decode assertions.
pub struct Verification<'a> {
    pub verifier: &'a dyn ReceiptVerifier,
    pub registry_digest: Option<&'a str>,
    pub authorized_capabilities: Option<&'a [String]>,
    pub confinement_required: bool,
    pub hard_spend_cap_required: bool,
}

#[derive(Debug)]
pub struct Fault {
    pub code: &'static str,
    pub message: String,
    pub unknown: bool,
}
pub(super) fn fault(code: &'static str, message: impl ToString, unknown: bool) -> Fault {
    Fault {
        code,
        message: message.to_string(),
        unknown,
    }
}
pub(super) fn read(repo: &Repository, path: &Utf8Path, limit: usize) -> Result<Vec<u8>, Fault> {
    if path
        .components()
        .any(|p| !matches!(p, camino::Utf8Component::Normal(_)))
        || path.as_str().is_empty()
    {
        return Err(fault(
            "runtime.capture-path",
            "repository-relative path without traversal required",
            false,
        ));
    }
    crate::progress_viewer::source::read(repo.root.as_std_path(), path.as_std_path(), limit)
        .map_err(|e| fault("runtime.capture-source-unavailable", e, true))
}
pub(super) fn decode(bytes: &[u8]) -> Result<Value, Fault> {
    crate::sdk::wire::decode_value(bytes).map_err(|e| fault("runtime.capture-syntax", e, false))
}

/// Read one bounded repository-relative selection request. Unknown schema
/// versions cannot be silently interpreted as the current request.
pub fn read_selection(repo: &Repository, path: &Utf8Path) -> Result<SelectionRequest, Fault> {
    let value = decode(&read(repo, path, REQUEST_LIMIT)?)?;
    if value["schema"] != selection::REQUEST_SCHEMA {
        return Err(fault(
            "runtime.selection-unsupported-schema",
            "unsupported selection request",
            true,
        ));
    }
    serde_json::from_value(value).map_err(|e| fault("runtime.selection-request", e, false))
}
fn blob(reference: &str, bytes: &[u8]) -> Value {
    json!({"reference":reference,"digest":format!("sha256:{}",sha256_hex(bytes)),"base64":base64_encode(bytes)})
}
fn native(receipt: &ReceiptAssessment) -> Value {
    json!({"standing":match receipt.standing {ReceiptStanding::Matches=>"matches", ReceiptStanding::Refused=>"refused", ReceiptStanding::Unknown=>"unknown"},"code":receipt.code,"detail":receipt.detail,"raw_digest":receipt.raw_digest,"provider_receipt_digest":receipt.provider_receipt_digest,"cost_observation":match receipt.cost_observation {Observation::Established=>"established",Observation::Refuted=>"refuted",Observation::Unknown=>"unknown"}})
}

fn provider(request: &Request) -> Result<ProviderInterface, Fault> {
    Ok(ProviderInterface {
        kind: match request.provider.kind.as_str() {
            "katana" => ProviderKind::Katana,
            "blut" => ProviderKind::Blut,
            _ => {
                return Err(fault(
                    "runtime.capture-unsupported-provider",
                    "provider kind not supported by this candidate",
                    true,
                ));
            }
        },
        identity: request.provider.identity.clone(),
        version: request.provider.version.clone(),
    })
}
fn expectation<'a>(
    dispatch: &'a openwarrant_core::execution::StageDispatch,
    provider: &'a ProviderInterface,
    verification: Option<&Verification<'a>>,
) -> RuntimeExpectation<'a> {
    RuntimeExpectation {
        dispatch,
        provider,
        registry_digest: verification.and_then(|v| v.registry_digest),
        authorized_capabilities: verification
            .as_ref()
            .and_then(|v| v.authorized_capabilities),
        confinement_required: verification.is_none_or(|v| v.confinement_required),
        hard_spend_cap_required: verification
            .as_ref()
            .is_some_and(|v| v.hard_spend_cap_required),
        max_receipt_bytes: RECEIPT_LIMIT,
    }
}

/// Capture and atomically retain one source-bound observation. An unavailable
/// native verifier permits explicitly unverified retention, never qualification.
/// Known binding/native mismatches refuse before publication. Exact replay has
/// identical bytes and name; changing collector metadata is a distinct observation.
pub fn import(
    repo: &Repository,
    alias: &str,
    request: &Request,
    verification: Option<Verification<'_>>,
) -> Result<Value, Fault> {
    let encoded =
        serde_json::to_vec(request).map_err(|e| fault("runtime.capture-request", e, false))?;
    if encoded.len() > REQUEST_LIMIT {
        return Err(fault(
            "runtime.capture-limit",
            "capture metadata exceeds limit",
            false,
        ));
    }
    if request.schema != REQUEST_SCHEMA {
        return Err(fault(
            "runtime.capture-unsupported-schema",
            "unsupported capture request version",
            true,
        ));
    }
    let metadata = &request.metadata;
    if [
        &request.provider.identity,
        &request.provider.version,
        &metadata.observation_id,
        &metadata.original_receipt_ref,
        &metadata.transport,
        &metadata.status,
    ]
    .iter()
    .any(|s| s.trim().is_empty())
        || chrono::DateTime::parse_from_rfc3339(&metadata.observed_at).is_err()
        || [&metadata.binary_identity, &metadata.source_identity]
            .iter()
            .any(|s| s.as_ref().is_some_and(|s| s.trim().is_empty()))
    {
        return Err(fault(
            "runtime.capture-metadata",
            "complete declared identity, observation, time and transport required",
            false,
        ));
    }
    let provider = provider(request)?;
    let recorded = recorded::load(repo, alias, &request.dispatch_id)?;
    let raw = read(repo, Utf8Path::new(&request.receipt), RECEIPT_LIMIT)?;
    let expectation = expectation(&recorded.dispatch, &provider, verification.as_ref());
    let assessment = assess_runtime_basis(
        &recorded.basis,
        &recorded.validated,
        &[RuntimeStageEvidence {
            expectation,
            raw_receipt: &raw,
            verifier: verification.as_ref().map(|v| v.verifier),
        }],
    );
    let Some(stage) = assessment
        .stages
        .iter()
        .find(|s| s.stage_id == recorded.dispatch.stage_id)
    else {
        return Err(fault(
            assessment.code,
            assessment.detail,
            assessment.standing == ReceiptStanding::Unknown,
        ));
    };
    if stage.receipt.standing == ReceiptStanding::Refused
        || assessment.standing == ReceiptStanding::Refused
    {
        return Err(fault(stage.receipt.code, &stage.receipt.detail, false));
    }
    let actor = format!("agent://{}", repo.performer());
    // Capture is a pre-result source observation, not an RC.2 assurance
    // Record (which requires an exact result digest). Do not invent a result.
    let observation = json!({
        "schema":"oh.war/runtime-source-observation/v1-draft.1",
        "id":metadata.observation_id,"actor":{"id":actor,"kind":"agent","role":"collector"},
        "binding":{"warrant":recorded.dispatch.warrant_ref,"contract_digest":recorded.dispatch.contract_digest,"dispatch_digest":recorded.dispatch.dispatch_digest,"stage":recorded.dispatch.stage_id,"attempt":recorded.dispatch.attempt_id},
        "source_refs":[recorded.dispatch_ref,recorded.event_ref,request.receipt],
        "check":{"id":"runtime.capture-bound-source","source_digest":format!("sha256:{}",sha256_hex(include_bytes!("runtime_capture.rs"))),"source_binding":"matched","scope":"Exact readable receipt bytes and the current recorded dispatch/compile-event snapshot only. Native execution and authenticity are not established."},
        "provenance":{"source":recorded.dispatch_ref,"authenticity":"unverified"},
        "result_digest":null,"assurance_granted":false
    });
    let record = json!({"schema":SCHEMA,"alias":alias,"declared_capture":request,"observation":observation,
        "dispatch":blob(&recorded.dispatch_ref,&recorded.dispatch_bytes),
        "compile_event":blob(&recorded.event_ref,&recorded.event_bytes),
        "receipt":blob(&request.receipt,&raw),"native_observation":native(&stage.receipt),
        "native_observation_is_trusted_on_read":false,"assurance_granted":false});
    let bytes = openwarrant_compiler::to_canonical_bytes(&record)
        .map_err(|e| fault("runtime.capture-encoding", e, false))?;
    if bytes.len() > RECORD_LIMIT {
        return Err(fault(
            "runtime.capture-limit",
            "capture exceeds retained byte limit",
            false,
        ));
    }
    let digest = sha256_hex(&bytes);
    let directory = recorded.directory.join("runtime-receipts");
    let retained = crate::bundle::store::Directory::open(&repo.root, &directory)
        .map_err(|e| fault("runtime.capture-storage", e, true))?;
    retained
        .retain(&format!("capture-{digest}.json"), &bytes)
        .map_err(|e| fault("runtime.capture-storage", e, false))?;
    Ok(
        json!({"retained":true,"digest":format!("sha256:{digest}"),"reference":directory.join(format!("capture-{digest}.json")).as_str(),"native_observation":native(&stage.receipt),"assurance_granted":false}),
    )
}

/// Inspect an immutable capture by local content identity. Recheck native seals,
/// dispatch currency and current policy before using it for receipt eligibility.
/// Neither saved verdicts nor local checksums grant trust on read.
pub fn show(repo: &Repository, alias: &str, digest: &str) -> Result<Value, Fault> {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fault(
            "runtime.capture-digest",
            "lowercase SHA-256 content identity required",
            false,
        ));
    }
    let dir = repo
        .warrant_dir(alias)
        .map_err(|e| fault("runtime.capture-warrant", e, false))?;
    let relative = dir
        .strip_prefix(&repo.root)
        .map_err(|e| fault("runtime.capture-path", e, false))?
        .join(format!("runtime-receipts/capture-{hex}.json"));
    let bytes = read(repo, &relative, RECORD_LIMIT)?;
    let record = inspect_retained(&bytes, alias, digest)?;
    Ok(
        json!({"reference":relative.as_str(),"digest":format!("sha256:{hex}"),"record":record,"native_observation_is_trusted":false,"assurance_granted":false}),
    )
}

/// Inspect retained bytes without repository or provider access. Content and
/// embedded blob identities are checked; declarations and saved native verdicts
/// are not authenticated, current-attempt eligibility or assurance.
pub fn inspect_retained(bytes: &[u8], alias: &str, digest: &str) -> Result<Value, Fault> {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    if bytes.len() > RECORD_LIMIT {
        return Err(fault(
            "runtime.capture-limit",
            "capture exceeds retained byte limit",
            false,
        ));
    }
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fault(
            "runtime.capture-digest",
            "lowercase SHA-256 content identity required",
            false,
        ));
    }
    if sha256_hex(bytes) != hex {
        return Err(fault(
            "runtime.capture-altered",
            "retained bytes do not match the referenced content identity",
            false,
        ));
    }
    let record = decode(bytes)?;
    if record["schema"] != SCHEMA {
        return Err(fault(
            "runtime.capture-unsupported-schema",
            "unsupported retained capture version",
            true,
        ));
    }
    if record["alias"] != alias {
        return Err(fault(
            "runtime.capture-subject",
            "unsupported or unrelated capture",
            false,
        ));
    }
    for name in ["dispatch", "receipt", "compile_event"] {
        let item = &record[name];
        let encoded = item["base64"]
            .as_str()
            .ok_or_else(|| fault("runtime.capture-blob", "missing source bytes", false))?;
        let decoded =
            base64_decode(encoded).map_err(|e| fault("runtime.capture-blob", e, false))?;
        if item["digest"] != format!("sha256:{}", sha256_hex(&decoded)) {
            return Err(fault(
                "runtime.capture-blob",
                "embedded source bytes have the wrong identity",
                false,
            ));
        }
    }
    Ok(record)
}

pub fn run(repo: &Repository, command: Command) -> (Report, Value) {
    let result = match command {
        Command::Import { alias, request } => (|| {
            let bytes = read(repo, &request, REQUEST_LIMIT)?;
            let value = decode(&bytes)?;
            if value["schema"] != REQUEST_SCHEMA {
                return Err(fault(
                    "runtime.capture-unsupported-schema",
                    "unsupported capture request version",
                    true,
                ));
            }
            let request: Request = serde_json::from_value(value)
                .map_err(|e| fault("runtime.capture-request", e, false))?;
            import(repo, &alias, &request, None)
        })(),
        Command::Show { alias, digest } => show(repo, &alias, &digest),
        Command::Assess { alias, selection } => (|| {
            let selected = read_selection(repo, &selection)?;
            let assessed = assess_selected(repo, &alias, &selected.selections, |_, _| None)?;
            Ok(selection::render(&assessed))
        })(),
    };
    let mut report = Report::default();
    report.notes.push("Capture retention and byte integrity only. Collector declarations and saved native verdicts do not establish authentication, execution, current-basis eligibility or assurance.".into());
    match result {
        Ok(value) => {
            let diagnostic = match value["standing"].as_str() {
                Some("unknown") => Diagnostic::unknown(
                    "runtime.selection-incomplete",
                    "",
                    "current receipt eligibility is unknown; see stage observations",
                ),
                Some("refused") => Diagnostic::error(
                    "runtime.selection-refused",
                    "",
                    "current receipt eligibility refused; see stage observations",
                ),
                Some("matches") => Diagnostic::pass(
                    "runtime.selection-matches",
                    "current receipt bindings match; no assurance granted",
                ),
                _ => Diagnostic::pass(
                    "runtime.capture",
                    "source-bound capture operation completed; native standing is separate",
                ),
            };
            report.push(diagnostic);
            (report, value)
        }
        Err(e) => {
            report.push(if e.unknown {
                Diagnostic::unknown(e.code, "", e.message)
            } else {
                Diagnostic::error(e.code, "", e.message)
            });
            (report, json!({"retained":false,"assurance_granted":false}))
        }
    }
}
