// SPDX-License-Identifier: Apache-2.0
//! Explicit collector attribution, separate from native receipt verification.
use super::*;
use openwarrant_core::runtime_collector::attribution::{
    AuthenticatedAttribution, SignedAttribution,
};

/// Trusted host adapter. Authenticate against independently observed current
/// authority/enrollment and these checked sources, never submitted identities.
/// A returned signature claim does not establish execution or assurance.
pub trait CaptureAuthenticator {
    fn authenticate(
        &self,
        signed: &SignedAttribution,
        binding: &RuntimeBinding,
        provider: &Provider,
        payload: &[u8],
    ) -> Result<AuthenticatedAttribution, Fault>;
}

pub struct Authentication<'a> {
    pub signed: &'a SignedAttribution,
    pub host: &'a dyn CaptureAuthenticator,
}

/// Reference protected local host. Holds the observed executable image alive;
/// collecting attribution does not execute it or validate a native receipt.
pub struct HostAuthenticator {
    host: collector_loading::HostEnrollment,
    image: protected_executable::ProtectedExecutable,
    signatures: collector_signature::OpenSshSignatureCheck,
}
fn host_fault(error: openwarrant_core::runtime_collector::Fault) -> Fault {
    use openwarrant_core::runtime_collector::Fault::*;
    match error {
        Rejected(reason) => fault("runtime.capture-attribution", reason, false),
        Unavailable(reason) => fault("runtime.capture-attribution", reason, true),
    }
}
impl HostAuthenticator {
    pub fn load(
        repo: &Repository,
        store: &std::path::Path,
        executable: &std::path::Path,
    ) -> Result<Self, Fault> {
        let signatures = collector_signature::OpenSshSignatureCheck::new(
            repo.root.as_std_path().to_owned(),
            std::time::Duration::from_secs(10),
        )
        .map_err(host_fault)?;
        let host = collector_loading::LoadedEnrollment::load_host(
            store,
            repo.root.as_std_path(),
            &signatures,
        )
        .map_err(host_fault)?;
        let expected = host.enrolled_verifier_digest();
        let image = protected_executable::ProtectedExecutable::acquire(
            executable,
            expected.strip_prefix("sha256:").unwrap_or(expected),
        )
        .map_err(|error| match error {
            ProviderFailure::Rejected(reason) => {
                fault("runtime.capture-attribution", reason, false)
            }
            ProviderFailure::Unavailable(reason) | ProviderFailure::Unsupported(reason) => {
                fault("runtime.capture-attribution", reason, true)
            }
        })?;
        Ok(Self {
            host,
            image,
            signatures,
        })
    }
}
impl CaptureAuthenticator for HostAuthenticator {
    fn authenticate(
        &self,
        signed: &SignedAttribution,
        binding: &RuntimeBinding,
        provider: &Provider,
        payload: &[u8],
    ) -> Result<AuthenticatedAttribution, Fault> {
        let provider = openwarrant_core::runtime_collector::Provider {
            kind: provider.kind.clone(),
            identity: provider.identity.clone(),
            version: provider.version.clone(),
        };
        self.host
            .authenticate_capture(
                signed,
                binding,
                &provider,
                &format!("sha256:{}", self.image.digest()),
                payload,
                &self.signatures,
            )
            .map_err(|error| match error {
                openwarrant_core::runtime_collector::attribution::Fault::Rejected(reason) => {
                    fault("runtime.capture-attribution", reason, false)
                }
                openwarrant_core::runtime_collector::attribution::Fault::Unavailable(reason) => {
                    fault("runtime.capture-attribution", reason, true)
                }
            })
    }
}

/// Exact canonical inline envelope signed by the collector: all metadata and
/// original receipt bytes. This prepares bytes, signs nothing and writes nothing.
pub fn payload(request: &Request, receipt: &[u8]) -> Result<Vec<u8>, Fault> {
    if receipt.len() > RECEIPT_LIMIT
        || serde_json::to_vec(request)
            .map_err(|e| fault("runtime.capture-request", e, false))?
            .len()
            > REQUEST_LIMIT
    {
        return Err(fault(
            "runtime.capture-limit",
            "attribution input budget",
            false,
        ));
    }
    if request.schema != REQUEST_SCHEMA {
        return Err(fault(
            "runtime.capture-unsupported-schema",
            "unsupported capture request",
            true,
        ));
    }
    if request.receipt != format!("receipt://sha256:{}", sha256_hex(receipt)) {
        return Err(fault(
            "runtime.capture-inline-reference",
            "attributed receipt must name exact bytes",
            false,
        ));
    }
    let bytes = openwarrant_compiler::to_canonical_bytes(&json!({
        "schema":INLINE_SCHEMA,"request":request,"receipt_base64":base64_encode(receipt)
    }))
    .map_err(|e| fault("runtime.capture-encoding", e, false))?;
    if bytes.len() > INLINE_LIMIT {
        return Err(fault(
            "runtime.capture-limit",
            "attribution payload exceeds 6 MiB",
            false,
        ));
    }
    Ok(bytes)
}

pub(super) fn check(
    authentication: &Authentication<'_>,
    request: &Request,
    recorded: &recorded::Recorded,
    receipt: &[u8],
) -> Result<Value, Fault> {
    if !recorded.latest_for_stage {
        return Err(fault(
            "runtime.capture-attribution",
            "superseded capture attempt",
            false,
        ));
    }
    let payload = payload(request, receipt)?;
    let binding = RuntimeBinding::from_dispatch(&recorded.dispatch);
    let checked = authentication.host.authenticate(
        authentication.signed,
        &binding,
        &request.provider,
        &payload,
    )?;
    let proof = checked.proof();
    if proof != &authentication.signed.proof
        || proof.warrant_ref != binding.warrant_ref
        || proof.contract_digest != binding.contract_digest
        || proof.dispatch_digest != binding.dispatch_digest
        || proof.stage_id != binding.stage_id
        || proof.attempt_id != binding.attempt_id
        || proof.payload_digest != format!("sha256:{}", sha256_hex(&payload))
    {
        return Err(fault(
            "runtime.capture-attribution",
            "authenticated proof has a different subject",
            false,
        ));
    }
    Ok(
        json!({"signed":authentication.signed,"payload_digest":proof.payload_digest,
        "authenticated_on_import":true,"trusted_on_read":false,"assurance_granted":false}),
    )
}

/// Reusable attributed SDK import. The trusted host is checked twice, including
/// after provider verification. Snapshot comparisons are not atomic fencing.
pub fn import_inline(
    repo: &Repository,
    alias: &str,
    bytes: &[u8],
    authentication: Authentication<'_>,
    verification: Option<Verification<'_>>,
) -> Result<Value, Fault> {
    let (request, receipt) = super::decode_inline(bytes)?;
    super::import_receipt_source(
        repo,
        alias,
        &request,
        &receipt,
        verification,
        Some(authentication),
    )
}
