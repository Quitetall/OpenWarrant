// SPDX-License-Identifier: Apache-2.0
//! Bounded F7 packet wire types and integrity checks. No semantic compilation.
use super::{
    Diagnostic,
    source::{BoundReference, SourceDescriptor},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub struct PacketLimits {
    pub json_bytes: usize,
    pub references: usize,
    pub sources: usize,
}
impl Default for PacketLimits {
    fn default() -> Self {
        Self {
            json_bytes: 64 * 1024 * 1024,
            references: 262144,
            sources: 4096,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub source_digest: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brief {
    pub outcome: Vec<BoundReference>,
    pub scope: Vec<BoundReference>,
    pub context: Vec<BoundReference>,
    pub constraints: Vec<BoundReference>,
    pub outputs: Vec<BoundReference>,
    pub stop: Vec<BoundReference>,
}
impl Brief {
    pub fn lists(&self) -> [(&'static str, &[BoundReference]); 6] {
        [
            ("outcome", &self.outcome),
            ("scope", &self.scope),
            ("context", &self.context),
            ("constraints", &self.constraints),
            ("outputs", &self.outputs),
            ("stop", &self.stop),
        ]
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionReason {
    pub kind: String,
    pub source_path: String,
    pub id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    #[serde(rename = "ref")]
    pub reference: BoundReference,
    pub kind: String,
    pub required: bool,
    pub reasons: Vec<InclusionReason>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Bound(BoundReference),
    Unresolved(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsystem: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<String>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    pub source: String,
    pub id: String,
    pub target: Target,
    pub condition: Option<Condition>,
    pub result: String,
    pub reasons: Vec<String>,
    pub supplied: bool,
    pub selected: bool,
    pub required: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionInputs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsystems: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origins: Option<BTreeMap<String, String>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PacketDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    pub details: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Readiness {
    pub value: String,
    pub basis: Vec<String>,
    pub limitations: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accounting {
    pub entry_bytes: usize,
    pub estimated_tokens: usize,
    pub method: String,
    pub budget: usize,
    pub package_payload_bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Packet {
    pub schema: String,
    pub compiler: String,
    pub basis_digest: String,
    pub task: Task,
    pub brief: Brief,
    pub binding_context: Vec<Binding>,
    pub reference_catalog: Vec<CatalogEntry>,
    pub sources: Vec<SourceDescriptor>,
    pub selection_inputs: SelectionInputs,
    pub diagnostics: Vec<PacketDiagnostic>,
    pub readiness: Readiness,
    pub coverage: String,
    pub accounting: Accounting,
}
fn error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(code, message, 0..0)
}
fn limits(limits: PacketLimits) -> Result<(), Diagnostic> {
    if [limits.json_bytes, limits.references, limits.sources].contains(&0) {
        return Err(error("resource-limit", "Packet limits must be positive"));
    }
    Ok(())
}
fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn invalid(message: &str) -> Diagnostic {
    error("package-invalid", message)
}
fn bound(r: &BoundReference) -> Result<(), Diagnostic> {
    if !digest(&r.source_digest)
        || (r.unit != "*" && !super::scan::unit_id(&r.unit))
        || r.start > r.end
        || r.end as u128 > 9_007_199_254_740_991
    {
        return Err(invalid("Invalid bound reference"));
    }
    Ok(())
}
fn shape(packet: &Packet, limit: PacketLimits) -> Result<(), Diagnostic> {
    if packet.schema != "oh.war/context-packet/1.0.0-rc.2" {
        return Err(error("schema-unsupported", "Unsupported packet schema"));
    }
    if !digest(&packet.basis_digest)
        || !digest(&packet.task.source_digest)
        || packet.compiler.is_empty()
        || !matches!(
            packet.task.role.as_str(),
            "preparation" | "implementation" | "verification" | "human-review"
        )
        || packet.coverage != "declared-inputs-and-dependencies"
    {
        return Err(invalid("Invalid packet header"));
    }
    if packet.sources.len() > limit.sources {
        return Err(error("resource-limit", "Too many packet sources"));
    }
    let mut count = packet
        .binding_context
        .len()
        .checked_add(packet.reference_catalog.len())
        .ok_or_else(|| error("resource-limit", "Too many references"))?;
    for (_, list) in packet.brief.lists() {
        count = count
            .checked_add(list.len())
            .ok_or_else(|| error("resource-limit", "Too many references"))?;
    }
    if count > limit.references {
        return Err(error("resource-limit", "Too many references"));
    }
    for (_, list) in packet.brief.lists() {
        for r in list {
            bound(r)?;
        }
    }
    for b in &packet.binding_context {
        bound(&b.reference)?;
        if !matches!(b.kind.as_str(), "binding" | "background" | "opaque") {
            return Err(invalid("Invalid binding kind"));
        }
    }
    for c in &packet.reference_catalog {
        if !matches!(c.result.as_str(), "TRUE" | "FALSE" | "UNKNOWN") {
            return Err(invalid("Invalid condition result"));
        }
        if let Target::Bound(r) = &c.target {
            bound(r)?;
        }
        if let Some(condition) = &c.condition {
            let mut entries = 0usize;
            let mut bytes = 0usize;
            for values in [&condition.stage, &condition.subsystem, &condition.path]
                .into_iter()
                .flatten()
            {
                entries = entries
                    .checked_add(values.len())
                    .filter(|n| *n <= 4096)
                    .ok_or_else(|| error("resource-limit", "Condition entry limit exceeded"))?;
                for value in values {
                    bytes = bytes
                        .checked_add(value.len())
                        .filter(|n| *n <= 1024 * 1024)
                        .ok_or_else(|| error("resource-limit", "Condition text limit exceeded"))?;
                }
            }
            let mut table = toml10::map::Map::new();
            for (key, values) in [
                ("stage", &condition.stage),
                ("subsystem", &condition.subsystem),
                ("path", &condition.path),
            ] {
                if let Some(values) = values {
                    table.insert(
                        key.into(),
                        super::MetadataValue::Array(
                            values
                                .iter()
                                .cloned()
                                .map(super::MetadataValue::String)
                                .collect(),
                        ),
                    );
                }
            }
            super::condition::validate_condition(
                &super::MetadataValue::Table(table),
                super::condition::ConditionLimits::default(),
            )?;
        }
    }
    if !matches!(
        packet.readiness.value.as_str(),
        "ready" | "blocked" | "not_evaluated"
    ) || packet
        .diagnostics
        .iter()
        .any(|d| !matches!(d.severity.as_str(), "error" | "warning" | "info"))
    {
        return Err(invalid("Invalid readiness or diagnostic severity"));
    }
    let a = &packet.accounting;
    if a.method != "utf8-bytes-div4-ceil"
        || [
            a.entry_bytes,
            a.estimated_tokens,
            a.budget,
            a.package_payload_bytes,
        ]
        .iter()
        .any(|n| *n as u128 > 9_007_199_254_740_991)
    {
        return Err(invalid("Invalid accounting"));
    }
    Ok(())
}
/// Decode canonical packet JSON. Does not validate source bytes or context coverage.
pub fn decode_packet(bytes: &[u8], limit: PacketLimits) -> Result<Packet, Diagnostic> {
    limits(limit)?;
    if bytes.len() > limit.json_bytes {
        return Err(error("resource-limit", "Packet JSON exceeds byte limit"));
    }
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| invalid(&e.to_string()))?;
    shape(&packet, limit)?;
    let canonical = serde_jcs::to_vec(&packet).map_err(|e| invalid(&e.to_string()))?;
    if canonical != bytes {
        return Err(invalid(
            "Packet JSON must be canonical, without duplicate keys or trailing bytes",
        ));
    }
    Ok(packet)
}

/// Checks wire claims against supplied bytes. Success does not establish selection
/// closure, access authorization, readiness or assurance; a compiler checks those.
pub fn check_packet_integrity(
    packet: &Packet,
    blobs: &BTreeMap<String, Vec<u8>>,
    entry: &[u8],
    limit: PacketLimits,
    source_limits: super::source::SourceLimits,
) -> Result<(), Diagnostic> {
    limits(limit)?;
    packet_depth(packet, limit.json_bytes)?;
    bounded_length(packet, limit.json_bytes)?;
    shape(packet, limit)?;
    let checked = super::source::check_packet_sources(&packet.sources, blobs, source_limits)?;
    let task = packet
        .sources
        .iter()
        .find(|s| s.source_digest == packet.task.source_digest)
        .ok_or_else(|| invalid("Task source absent"))?;
    if task.document.as_ref().is_none_or(|d| d.kind != "warrant") {
        return Err(invalid("Task source must be a Warrant"));
    }
    let mut selected = std::collections::BTreeSet::new();
    for binding in &packet.binding_context {
        checked.check_reference(&binding.reference)?;
        if !selected.insert((&binding.reference.source_digest, &binding.reference.unit)) {
            return Err(invalid("Duplicate binding reference"));
        }
    }
    for (_, list) in packet.brief.lists() {
        for reference in list {
            checked.check_reference(reference)?;
            if !selected.contains(&(&reference.source_digest, &reference.unit)) {
                return Err(invalid("Brief reference is absent from binding context"));
            }
        }
    }
    for catalog in &packet.reference_catalog {
        if let Target::Bound(reference) = &catalog.target {
            if catalog.supplied {
                checked.check_reference(reference)?;
            } else if blobs.contains_key(&reference.source_digest) {
                return Err(invalid("Catalog supplied flag disagrees with source bytes"));
            }
        } else if catalog.supplied || catalog.selected {
            return Err(invalid(
                "Unresolved catalog target claims supplied or selected bytes",
            ));
        }
    }
    let a = &packet.accounting;
    let payload = blobs
        .values()
        .try_fold(entry.len(), |total, b| total.checked_add(b.len()))
        .ok_or_else(|| error("resource-limit", "Payload size overflow"))?;
    if a.entry_bytes != entry.len()
        || a.estimated_tokens != entry.len().div_ceil(4)
        || a.estimated_tokens > a.budget
        || a.package_payload_bytes != payload
    {
        return Err(invalid(
            "Entry accounting disagrees with bytes or exceeds budget",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct PackageLimits {
    pub packet: PacketLimits,
    pub sources: super::source::SourceLimits,
    pub total_bytes: usize,
    pub entry_bytes: usize,
}
impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            packet: PacketLimits::default(),
            sources: super::source::SourceLimits::default(),
            total_bytes: 512 * 1024 * 1024,
            entry_bytes: 64 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub files: Vec<FileEntry>,
    pub basis: serde_json::Value,
    pub root_digest: String,
}
#[derive(Debug)]
pub struct CheckedPackage {
    pub packet: Packet,
    pub manifest: Manifest,
    pub entry: Vec<u8>,
    pub blobs: BTreeMap<String, Vec<u8>>,
}
impl CheckedPackage {
    /// SDK integrity never proves compiler selection or authority.
    pub fn semantic_coverage_established(&self) -> bool {
        false
    }
}
#[derive(Clone, Copy, Debug)]
pub enum DigestDomain {
    Basis,
    Contract,
    Package,
}
impl DigestDomain {
    fn name(self) -> &'static str {
        match self {
            Self::Basis => "oh.war/basis/1.0.0-rc.2",
            Self::Contract => "oh.war/contract/1.0.0-rc.2",
            Self::Package => "oh.war/package/1.0.0-rc.2",
        }
    }
}
use super::source::raw_digest;
/// F9 structured digest over existing domains; no replacement digest format.
pub fn structured_digest(
    domain: DigestDomain,
    payload: &serde_json::Value,
    max_bytes: usize,
) -> Result<String, Diagnostic> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        digest_domain: &'static str,
        payload: &'a serde_json::Value,
    }
    let mut remaining = max_bytes;
    check_json_depth(payload, 0, &mut remaining)?;
    let envelope = Envelope {
        digest_domain: domain.name(),
        payload,
    };
    bounded_length(&envelope, max_bytes)?;
    let bytes = serde_jcs::to_vec(&envelope).map_err(|e| invalid(&e.to_string()))?;
    Ok(raw_digest(&bytes))
}
fn bounded_length(value: &impl Serialize, limit: usize) -> Result<usize, Diagnostic> {
    struct Counter {
        remaining: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.remaining = self
                .remaining
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("resource-limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    if limit == 0 {
        return Err(error("resource-limit", "Zero byte limit"));
    }
    let mut counter = Counter { remaining: limit };
    serde_json::to_writer(&mut counter, value)
        .map_err(|_| error("resource-limit", "Serialized value exceeds byte limit"))?;
    let mut canonical = Counter { remaining: limit };
    serde_jcs::to_writer(&mut canonical, value)
        .map_err(|_| error("resource-limit", "Canonical value exceeds byte limit"))?;
    Ok(limit - canonical.remaining)
}
/// Validate a supplied regular-file map. Filesystem callers must reject symlinks
/// and special files before constructing this map. No filesystem access occurs.
/// Ownership permits blob reuse without cloning the package payload.
pub fn check_package(
    mut files: BTreeMap<String, Vec<u8>>,
    limit: PackageLimits,
) -> Result<CheckedPackage, Diagnostic> {
    limits(limit.packet)?;
    limit.sources.check()?;
    if limit.total_bytes == 0 || limit.entry_bytes == 0 {
        return Err(error("resource-limit", "Zero package limit"));
    }
    if files.len() > limit.packet.sources.saturating_add(3) {
        return Err(error("resource-limit", "Too many package files"));
    }
    let mut total = 0usize;
    for (path, bytes) in &files {
        let allowed = matches!(path.as_str(), "ENTRY.md" | "packet.json" | "manifest.json")
            || path
                .strip_prefix("blobs/")
                .and_then(|p| p.strip_suffix(".bin"))
                .is_some_and(|h| {
                    h.len() == 64
                        && h.bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                });
        if !allowed {
            return Err(invalid("Unexpected or unsafe package path"));
        }
        total = total
            .checked_add(bytes.len())
            .filter(|n| *n <= limit.total_bytes)
            .ok_or_else(|| error("resource-limit", "Package exceeds byte limit"))?;
    }
    let manifest_bytes = files
        .remove("manifest.json")
        .ok_or_else(|| invalid("Missing manifest"))?;
    if manifest_bytes.len() > limit.packet.json_bytes {
        return Err(error("resource-limit", "Manifest exceeds JSON limit"));
    }
    let manifest: Manifest =
        serde_json::from_slice(&manifest_bytes).map_err(|e| invalid(&e.to_string()))?;
    if manifest.basis.get("task").is_none() {
        return Err(invalid("F5 task field is required, including master null"));
    }
    let basis: CompileBasis = CompileBasis::deserialize(&manifest.basis)
        .map_err(|e| invalid(&format!("Invalid F5 basis: {e}")))?;
    validate_basis_shape(&basis)?;
    if manifest.schema != "oh.war/context-package/1.0.0-rc.2" {
        return Err(error("schema-unsupported", "Unsupported package schema"));
    }
    if serde_jcs::to_vec(&manifest).map_err(|e| invalid(&e.to_string()))? != manifest_bytes {
        return Err(invalid("Manifest must be canonical JSON"));
    }
    if manifest.files.len() != files.len()
        || manifest.files.windows(2).any(|p| p[0].path >= p[1].path)
    {
        return Err(invalid("Manifest file set must be unique and sorted"));
    }
    for entry in &manifest.files {
        let bytes = files
            .get(&entry.path)
            .ok_or_else(|| invalid("Manifest file absent"))?;
        if entry.bytes != bytes.len() || entry.sha256 != raw_digest(bytes) {
            return Err(error(
                "digest-mismatch",
                "Package file length or digest mismatch",
            ));
        }
    }
    let mut preimage = serde_json::to_value(&manifest).map_err(|e| invalid(&e.to_string()))?;
    preimage
        .as_object_mut()
        .expect("typed manifest")
        .remove("root_digest");
    if structured_digest(DigestDomain::Package, &preimage, limit.packet.json_bytes)?
        != manifest.root_digest
    {
        return Err(error("digest-mismatch", "Package root digest mismatch"));
    }
    let packet_bytes = files
        .remove("packet.json")
        .ok_or_else(|| invalid("Missing packet"))?;
    let packet = decode_packet(&packet_bytes, limit.packet)?;
    if structured_digest(
        DigestDomain::Basis,
        &manifest.basis,
        limit.packet.json_bytes,
    )? != packet.basis_digest
    {
        return Err(error("digest-mismatch", "Packet basis digest mismatch"));
    }
    if manifest
        .basis
        .get("schema")
        .and_then(serde_json::Value::as_str)
        != Some("oh.war/compile-request/1.0.0-rc.2")
        || manifest
            .basis
            .get("mode")
            .and_then(serde_json::Value::as_str)
            != Some("task")
    {
        return Err(invalid("Package requires task compilation basis"));
    }
    if manifest.basis.get("compiler") != Some(&serde_json::json!(packet.compiler))
        || manifest.basis.get("sources")
            != Some(&serde_json::to_value(&packet.sources).map_err(|e| invalid(&e.to_string()))?)
        || manifest.basis.get("task")
            != Some(&serde_json::to_value(&packet.task).map_err(|e| invalid(&e.to_string()))?)
    {
        return Err(invalid(
            "Packet source/task/compiler identity disagrees with basis",
        ));
    }
    let entry = files
        .remove("ENTRY.md")
        .ok_or_else(|| invalid("Missing entry"))?;
    if entry.len() > limit.entry_bytes {
        return Err(error("resource-limit", "Entry exceeds byte limit"));
    }
    let mut blobs = BTreeMap::new();
    for (path, bytes) in files {
        let hex = path
            .strip_prefix("blobs/")
            .and_then(|s| s.strip_suffix(".bin"))
            .ok_or_else(|| invalid("Unexpected payload path"))?;
        let key = format!("sha256:{hex}");
        if !packet.sources.iter().any(|s| s.source_digest == key) {
            return Err(invalid("Unreferenced blob file"));
        }
        blobs.insert(key, bytes);
    }
    check_packet_integrity(&packet, &blobs, &entry, limit.packet, limit.sources)?;
    Ok(CheckedPackage {
        packet,
        manifest,
        entry,
        blobs,
    })
}

/// Encode the typed packet in F7 canonical JSON after bounded shape validation.
pub fn encode_packet(packet: &Packet, limit: PacketLimits) -> Result<Vec<u8>, Diagnostic> {
    limits(limit)?;
    packet_depth(packet, limit.json_bytes)?;
    bounded_length(packet, limit.json_bytes)?;
    shape(packet, limit)?;
    serde_jcs::to_vec(packet).map_err(|e| invalid(&e.to_string()))
}

fn check_json_depth(
    value: &serde_json::Value,
    depth: usize,
    remaining: &mut usize,
) -> Result<(), Diagnostic> {
    if depth > 128 {
        return Err(error("resource-limit", "JSON nesting exceeds 128"));
    }
    *remaining = remaining
        .checked_sub(1)
        .ok_or_else(|| error("resource-limit", "JSON tree exceeds byte/node bound"))?;
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                check_json_depth(value, depth + 1, remaining)?;
            }
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                *remaining = remaining
                    .checked_sub(key.len())
                    .ok_or_else(|| error("resource-limit", "JSON keys exceed byte bound"))?;
                check_json_depth(value, depth + 1, remaining)?;
            }
        }
        serde_json::Value::String(text) => {
            *remaining = remaining
                .checked_sub(text.len())
                .ok_or_else(|| error("resource-limit", "JSON text exceeds byte bound"))?;
        }
        _ => {}
    }
    Ok(())
}
fn packet_depth(packet: &Packet, max_bytes: usize) -> Result<(), Diagnostic> {
    let mut remaining = max_bytes;
    for diagnostic in &packet.diagnostics {
        for value in diagnostic.details.values() {
            check_json_depth(value, 2, &mut remaining)?;
        }
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessFacts {
    pub allowed_source_digests: Vec<String>,
    pub basis_refs: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileOptions {
    pub include_optional: bool,
    pub entry_token_budget: usize,
    pub token_method: String,
    pub limits: BTreeMap<String, usize>,
    pub access: AccessFacts,
}
/// F5 request structure. Record meaning and policy authorization remain separate.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileBasis {
    pub schema: String,
    pub mode: String,
    pub compiler: String,
    pub sources: Vec<SourceDescriptor>,
    pub task: Option<Task>,
    pub selection_inputs: SelectionInputs,
    pub policy_facts: Vec<String>,
    pub records: Vec<BTreeMap<String, serde_json::Value>>,
    pub options: CompileOptions,
}
fn validate_basis_shape(basis: &CompileBasis) -> Result<(), Diagnostic> {
    if basis.schema != "oh.war/compile-request/1.0.0-rc.2"
        || !matches!(basis.mode.as_str(), "task" | "master")
        || basis.compiler.is_empty()
        || (basis.mode == "task") != basis.task.is_some()
    {
        return Err(invalid("Invalid compilation basis shape"));
    }
    if basis.options.token_method != "utf8-bytes-div4-ceil"
        || basis.options.entry_token_budget as u128 > 9_007_199_254_740_991
        || basis
            .options
            .limits
            .values()
            .any(|n| *n == 0 || *n as u128 > 9_007_199_254_740_991)
    {
        return Err(invalid("Invalid compilation options"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for d in &basis.options.access.allowed_source_digests {
        if !digest(d) || !seen.insert(d) {
            return Err(invalid("Invalid or duplicate access digest"));
        }
    }
    for id in basis
        .policy_facts
        .iter()
        .chain(&basis.options.access.basis_refs)
    {
        if !super::validate::identity(id) {
            return Err(invalid("Invalid policy record reference"));
        }
    }
    Ok(())
}
