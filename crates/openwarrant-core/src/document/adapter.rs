// SPDX-License-Identifier: Apache-2.0
//! Explicit in-process compiler boundary. No provider discovery or fallback.
use super::{
    Diagnostic,
    packet::{self, CheckedPackage, CompileBasis, DigestDomain, PackageLimits},
    source::check_sources,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "oh.war/compiler-adapter/1";
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    TaskPackage,
    OfflineSemanticCheck,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderInfo {
    pub profile: String,
    pub compiler: String,
    pub capabilities: BTreeSet<Capability>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    UnsupportedProfile,
    UnsupportedCapability,
    Unavailable,
    MalformedResponse,
    BasisMismatch,
    InvalidRequest,
    ProviderRefusal,
}
#[derive(Debug)]
pub struct AdapterError {
    pub kind: ErrorKind,
    pub message: String,
    pub diagnostic: Option<Diagnostic>,
}
impl AdapterError {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Unavailable,
            message: message.into(),
            diagnostic: None,
        }
    }
    pub fn refused(diagnostic: Diagnostic) -> Self {
        Self {
            kind: ErrorKind::ProviderRefusal,
            message: diagnostic.message.clone(),
            diagnostic: Some(diagnostic),
        }
    }
}
fn error(kind: ErrorKind, message: &str) -> AdapterError {
    AdapterError {
        kind,
        message: message.into(),
        diagnostic: None,
    }
}
fn mapped(kind: ErrorKind, d: Diagnostic) -> AdapterError {
    AdapterError {
        kind,
        message: d.message.clone(),
        diagnostic: Some(d),
    }
}
/// Construct through `prepare`; fields cannot be replaced after validation.
pub struct CompileRequest {
    basis: CompileBasis,
    value: serde_json::Value,
    blobs: BTreeMap<String, Vec<u8>>,
    basis_digest: String,
    provider: ProviderInfo,
    limits: PackageLimits,
    semantic_required: bool,
}
impl CompileRequest {
    pub fn basis(&self) -> &CompileBasis {
        &self.basis
    }
    pub fn value(&self) -> &serde_json::Value {
        &self.value
    }
    pub fn blobs(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.blobs
    }
    pub fn basis_digest(&self) -> &str {
        &self.basis_digest
    }
    pub fn provider(&self) -> &ProviderInfo {
        &self.provider
    }
    pub fn limits(&self) -> PackageLimits {
        self.limits
    }
}
/// Provider statement, not an SDK finding, signature or grant of trust.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticEvidence {
    pub compiler: String,
    pub basis_digest: String,
    pub package_root: String,
    pub method: String,
    pub context_complete: bool,
}
pub struct CompileResponse {
    pub provider: ProviderInfo,
    pub basis_digest: String,
    pub files: BTreeMap<String, Vec<u8>>,
    pub semantic_evidence: Option<SemanticEvidence>,
}
pub struct ValidatedResponse {
    pub package: CheckedPackage,
    pub provider: ProviderInfo,
    pub provider_semantic_evidence: Option<SemanticEvidence>,
}
impl ValidatedResponse {
    pub fn local_semantic_coverage_established(&self) -> bool {
        false
    }
}
pub trait CompilerProvider {
    fn info(&self) -> Result<ProviderInfo, AdapterError>;
    fn compile(&self, request: &CompileRequest) -> Result<CompileResponse, AdapterError>;
}
pub fn prepare(
    basis: &CompileBasis,
    blobs: BTreeMap<String, Vec<u8>>,
    provider: ProviderInfo,
    required: &BTreeSet<Capability>,
    limits: PackageLimits,
) -> Result<CompileRequest, AdapterError> {
    if limits.packet.json_bytes == 0
        || limits.packet.references == 0
        || limits.packet.sources == 0
        || limits.total_bytes == 0
        || limits.entry_bytes == 0
    {
        return Err(error(
            ErrorKind::InvalidRequest,
            "Adapter limits must be positive",
        ));
    }
    if provider.profile != PROFILE {
        return Err(error(
            ErrorKind::UnsupportedProfile,
            "Unsupported compiler adapter profile",
        ));
    }
    if !required.is_subset(&provider.capabilities)
        || !provider.capabilities.contains(&Capability::TaskPackage)
    {
        return Err(error(
            ErrorKind::UnsupportedCapability,
            "Required compiler capability absent",
        ));
    }
    if provider.compiler.is_empty()
        || provider.compiler.len() > 4096
        || provider.compiler != basis.compiler
    {
        return Err(error(
            ErrorKind::InvalidRequest,
            "Explicit compiler identity disagrees with provider",
        ));
    }
    packet::validate_basis_shape(basis).map_err(|d| mapped(ErrorKind::InvalidRequest, d))?;
    if basis.mode != "task" {
        return Err(error(
            ErrorKind::UnsupportedCapability,
            "This adapter profile requires task packages",
        ));
    }
    let task = basis.task.as_ref().expect("validated task mode");
    if !matches!(
        task.role.as_str(),
        "preparation" | "implementation" | "verification" | "human-review"
    ) || task.stage.as_ref().is_some_and(String::is_empty)
        || !basis.sources.iter().any(|s| {
            s.source_digest == task.source_digest
                && s.document.as_ref().is_some_and(|d| d.kind == "warrant")
        })
    {
        return Err(error(
            ErrorKind::InvalidRequest,
            "Task must name a captured Warrant and supported role",
        ));
    }
    // Borrow hostile typed data until bounded validation succeeds, avoiding recursive
    // destruction of an owned, arbitrarily deep JSON value on the refusal path.
    let mut nodes = limits.packet.json_bytes;
    for record in &basis.records {
        for value in record.values() {
            packet::check_json_depth(value, 3, &mut nodes)
                .map_err(|d| mapped(ErrorKind::InvalidRequest, d))?;
        }
    }
    check_sources(&basis.sources, &blobs, limits.sources)
        .map_err(|d| mapped(ErrorKind::InvalidRequest, d))?;
    // Bound serialization before creating the owned generic basis used by F9.
    let mut counter = Counter(limits.packet.json_bytes);
    serde_json::to_writer(&mut counter, &basis).map_err(|_| {
        error(
            ErrorKind::InvalidRequest,
            "Compile basis exceeds JSON quota",
        )
    })?;
    let value = serde_json::to_value(basis)
        .map_err(|_| error(ErrorKind::InvalidRequest, "Invalid compile basis"))?;
    let basis_digest =
        packet::structured_digest(DigestDomain::Basis, &value, limits.packet.json_bytes)
            .map_err(|d| mapped(ErrorKind::InvalidRequest, d))?;
    Ok(CompileRequest {
        basis: basis.clone(),
        value,
        blobs,
        basis_digest,
        provider,
        limits,
        semantic_required: required.contains(&Capability::OfflineSemanticCheck),
    })
}
pub fn validate_response(
    request: &CompileRequest,
    response: CompileResponse,
) -> Result<ValidatedResponse, AdapterError> {
    if response.provider.profile != request.provider.profile
        || response.provider.compiler != request.provider.compiler
        || response.provider.capabilities != request.provider.capabilities
    {
        return Err(error(
            ErrorKind::MalformedResponse,
            "Provider identity or capabilities changed during call",
        ));
    }
    if response.basis_digest != request.basis_digest {
        return Err(error(
            ErrorKind::BasisMismatch,
            "Provider returned another compilation basis",
        ));
    }
    let package = packet::check_package(response.files, request.limits)
        .map_err(|d| mapped(ErrorKind::MalformedResponse, d))?;
    if package.manifest.basis != request.value
        || package.packet.basis_digest != request.basis_digest
    {
        return Err(error(
            ErrorKind::BasisMismatch,
            "Package basis differs from request",
        ));
    }
    if request.semantic_required && response.semantic_evidence.is_none() {
        return Err(error(
            ErrorKind::MalformedResponse,
            "Requested semantic evidence absent",
        ));
    }
    if let Some(evidence) = &response.semantic_evidence
        && (!response
            .provider
            .capabilities
            .contains(&Capability::OfflineSemanticCheck)
            || evidence.compiler != response.provider.compiler
            || evidence.basis_digest != request.basis_digest
            || evidence.package_root != package.manifest.root_digest
            || evidence.method.is_empty()
            || evidence.method.len() > 4096)
    {
        return Err(error(
            ErrorKind::MalformedResponse,
            "Invalid provider semantic evidence binding",
        ));
    }
    Ok(ValidatedResponse {
        package,
        provider: response.provider,
        provider_semantic_evidence: response.semantic_evidence,
    })
}
/// Caller chooses one provider. Errors never trigger an implicit alternate backend.
pub fn compile_with(
    provider: &impl CompilerProvider,
    basis: &CompileBasis,
    blobs: BTreeMap<String, Vec<u8>>,
    required: &BTreeSet<Capability>,
    limits: PackageLimits,
) -> Result<ValidatedResponse, AdapterError> {
    let request = prepare(basis, blobs, provider.info()?, required, limits)?;
    let response = provider.compile(&request)?;
    validate_response(&request, response)
}
struct Counter(usize);
impl std::io::Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("JSON quota exhausted"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
