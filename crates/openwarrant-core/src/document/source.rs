// SPDX-License-Identifier: Apache-2.0
//! Supplied source facts and integrity checks. No acquisition or authority inference.
use super::{
    Diagnostic, Dialect, ParseLimits, ValidationOptions, Validity, parse_document,
    validate_document,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HolderKind {
    Local,
    Git,
    External,
    Example,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holder {
    pub kind: HolderKind,
    pub locator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

/// Caller-supplied labels are claims, including an `established` authority label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMetadata {
    pub trust: String,
    pub classification: String,
    pub taints: Vec<String>,
    pub authority: String,
    pub basis_refs: Vec<String>,
}
impl SourceMetadata {
    pub fn unestablished(trust: &str, classification: &str) -> Self {
        Self {
            trust: trust.into(),
            classification: classification.into(),
            taints: vec![],
            authority: "unestablished".into(),
            basis_refs: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentIdentity {
    pub id: String,
    pub revision: u64,
    pub schema: String,
    pub kind: String,
}

/// Unchecked wire data. Use the source checks before trusting its byte claims.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceDescriptor {
    pub path: String,
    pub source_digest: String,
    pub byte_length: usize,
    pub holder: Holder,
    pub metadata: SourceMetadata,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentIdentity>,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceLimits {
    pub source_bytes: usize,
    pub total_bytes: usize,
    pub sources: usize,
    pub units: usize,
    pub metadata_bytes: usize,
    pub output_bytes: usize,
}

#[derive(Debug)]
pub struct CheckedSources<'a> {
    descriptors: Vec<&'a SourceDescriptor>,
    blobs: BTreeMap<&'a str, CheckedBlob<'a>>,
    output_bytes: usize,
}
#[derive(Debug)]
struct CheckedBlob<'a> {
    bytes: &'a [u8],
    identity: Option<DocumentIdentity>,
    units: BTreeMap<String, std::ops::Range<usize>>,
}
/// Exact F3 wire reference. `*` denotes the original whole blob, not closure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundReference {
    pub source_digest: String,
    pub unit: String,
    pub start: usize,
    pub end: usize,
}
impl<'a> CheckedSources<'a> {
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
    /// F3 lock table ordered by source path, using the repository's RFC 8785 codec.
    /// This is not a signed subject or a new digest domain.
    pub fn canonical_lock(&self) -> Result<Vec<u8>, Diagnostic> {
        encoded_length(&self.descriptors, self.output_bytes)?;
        serde_jcs::to_vec(&self.descriptors)
            .map_err(|e| Diagnostic::error("source-invalid", e.to_string(), 0..0))
    }
    /// Check a provider's declared range against supplied bytes. No target search,
    /// dependency traversal or access/authority decision is performed.
    pub fn check_reference(&self, reference: &BoundReference) -> Result<&'a [u8], Diagnostic> {
        check_reference_shape(reference)?;
        let source = self
            .blobs
            .get(reference.source_digest.as_str())
            .ok_or_else(|| {
                Diagnostic::error(
                    "source-missing",
                    "Reference digest is absent from supplied sources",
                    0..0,
                )
            })?;
        let expected = if reference.unit == "*" {
            0..source.bytes.len()
        } else {
            source.units.get(&reference.unit).cloned().ok_or_else(|| {
                Diagnostic::error(
                    "target-missing",
                    "Named unit is absent or source is opaque",
                    0..0,
                )
            })?
        };
        if expected != (reference.start..reference.end) {
            return Err(Diagnostic::error(
                "range-mismatch",
                "Declared range does not equal the exact unit or whole source",
                0..0,
            ));
        }
        Ok(&source.bytes[expected])
    }
}

fn raw_digest(bytes: &[u8]) -> String {
    let hex: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("sha256:{hex}")
}

/// Validate supplied descriptors against their blobs. This does not capture files,
/// resolve dependencies, establish access rights or evaluate context completeness.
pub fn check_sources<'a>(
    descriptors: &'a [SourceDescriptor],
    blobs: &'a BTreeMap<String, Vec<u8>>,
    limits: SourceLimits,
) -> Result<CheckedSources<'a>, Diagnostic> {
    limits.check()?;
    if descriptors.len() > limits.sources || blobs.len() > limits.sources {
        return Err(limit_error());
    }
    let mut total = 0usize;
    for bytes in blobs.values() {
        if bytes.len() > limits.source_bytes {
            return Err(limit_error());
        }
        total = total
            .checked_add(bytes.len())
            .filter(|n| *n <= limits.total_bytes)
            .ok_or_else(limit_error)?;
    }
    for descriptor in descriptors {
        total = total
            .checked_add(check_descriptor(descriptor, limits)?)
            .filter(|n| *n <= limits.total_bytes)
            .ok_or_else(limit_error)?;
    }
    let mut checked: BTreeMap<&str, CheckedBlob<'a>> = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let mut identities = BTreeMap::new();
    let mut units = 0usize;
    for descriptor in descriptors {
        if !paths.insert(&descriptor.path) {
            return Err(Diagnostic::error(
                "source-ambiguous",
                "More than one snapshot for a source path",
                0..0,
            ));
        }
        if let Some(previous) = checked.get(descriptor.source_digest.as_str()) {
            if previous.bytes.len() != descriptor.byte_length {
                return Err(Diagnostic::error(
                    "source-mismatch",
                    "Source length differs from supplied bytes",
                    0..0,
                ));
            }
            if previous.identity != descriptor.document {
                return Err(Diagnostic::error(
                    "source-ambiguous",
                    "One digest has incompatible document interpretations",
                    0..0,
                ));
            }
            continue;
        }
        let bytes = blobs.get(&descriptor.source_digest).ok_or_else(|| {
            Diagnostic::error("source-missing", "No supplied blob for source digest", 0..0)
        })?;
        if raw_digest(bytes) != descriptor.source_digest {
            return Err(Diagnostic::error(
                "digest-mismatch",
                "Supplied bytes do not match source digest",
                0..0,
            ));
        }
        if bytes.len() != descriptor.byte_length {
            return Err(Diagnostic::error(
                "source-mismatch",
                "Source length differs from supplied bytes",
                0..0,
            ));
        }
        let dialect = descriptor
            .document
            .as_ref()
            .map(|d| match d.schema.as_str() {
                "oh.war/document/1.0.0-rc.2" => Dialect::Rc2,
                "oh.war/document/1.0.0-rc.3" => Dialect::Rc3,
                _ => unreachable!("schema checked before blob processing"),
            });
        if dialect.is_some() && units == limits.units {
            return Err(limit_error());
        }
        let parsed = parse_checked(
            bytes,
            dialect,
            SourceLimits {
                units: limits.units - units,
                ..limits
            },
        )?;
        let identity = parsed.as_ref().map(document_identity);
        if identity != descriptor.document {
            return Err(Diagnostic::error(
                "source-mismatch",
                "Document identity differs from original bytes",
                0..0,
            ));
        }
        if let Some(document) = &descriptor.document
            && let Some(previous) =
                identities.insert((&document.id, document.revision), &descriptor.source_digest)
            && previous != &descriptor.source_digest
        {
            return Err(Diagnostic::error(
                "source-ambiguous",
                "Different bytes claim one identity and revision",
                0..0,
            ));
        }
        let unit_map: BTreeMap<_, _> = parsed
            .into_iter()
            .flat_map(|d| d.units)
            .map(|u| (u.id, u.span))
            .collect();
        units += unit_map.len();
        checked.insert(
            &descriptor.source_digest,
            CheckedBlob {
                bytes,
                identity,
                units: unit_map,
            },
        );
    }
    let mut descriptors: Vec<_> = descriptors.iter().collect();
    descriptors.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(CheckedSources {
        descriptors,
        blobs: checked,
        output_bytes: limits.output_bytes,
    })
}
impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            source_bytes: 8 * 1024 * 1024,
            total_bytes: 256 * 1024 * 1024,
            sources: 4096,
            units: 65_536,
            metadata_bytes: 1024 * 1024,
            output_bytes: 64 * 1024 * 1024,
        }
    }
}
impl SourceLimits {
    fn check(self) -> Result<(), Diagnostic> {
        if [
            self.source_bytes,
            self.total_bytes,
            self.sources,
            self.units,
            self.metadata_bytes,
            self.output_bytes,
        ]
        .contains(&0)
        {
            return Err(limit_error());
        }
        Ok(())
    }
    fn parse_limits(self) -> ParseLimits {
        ParseLimits {
            source_bytes: self.source_bytes,
            metadata_bytes: self.metadata_bytes,
            units: self.units,
            ..ParseLimits::default()
        }
    }
}
fn limit_error() -> Diagnostic {
    Diagnostic::error(
        "resource-limit",
        "Source resource limit exceeded or zero",
        0..0,
    )
}

/// Count serialized bytes without allocating the output. JSON ordering does not
/// change the length of these integer/string-only records when encoded as JCS.
fn encoded_length(value: &impl Serialize, limit: usize) -> Result<usize, Diagnostic> {
    struct Counter {
        length: usize,
        limit: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.length = self
                .length
                .checked_add(bytes.len())
                .filter(|n| *n <= self.limit)
                .ok_or_else(|| std::io::Error::other("resource-limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { length: 0, limit };
    serde_json::to_writer(&mut counter, value).map_err(|_| limit_error())?;
    Ok(counter.length)
}

fn check_digest(digest: &str) -> Result<(), Diagnostic> {
    if !digest.strip_prefix("sha256:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        return Err(Diagnostic::error(
            "digest-invalid",
            "Expected raw sha256 digest with 64 lowercase hexadecimal digits",
            0..0,
        ));
    }
    Ok(())
}
fn check_provenance(
    path: &str,
    holder: &Holder,
    metadata: &SourceMetadata,
) -> Result<(), Diagnostic> {
    if !super::references::path(path) {
        return Err(Diagnostic::error(
            "target-invalid",
            "Source path must be capture-root-relative",
            0..0,
        ));
    }
    if holder.locator.is_empty()
        || holder.commit.as_ref().is_some_and(|s| {
            !matches!(s.len(), 40 | 64) || !s.bytes().all(|b| b.is_ascii_hexdigit())
        })
    {
        return Err(Diagnostic::error(
            "source-invalid",
            "Invalid holder locator or Git commit",
            0..0,
        ));
    }
    if !matches!(metadata.authority.as_str(), "unestablished" | "established")
        || metadata
            .basis_refs
            .iter()
            .any(|s| !super::validate::identity(s))
    {
        return Err(Diagnostic::error(
            "source-invalid",
            "Invalid authority claim or basis record identity",
            0..0,
        ));
    }
    Ok(())
}
fn check_descriptor(d: &SourceDescriptor, limits: SourceLimits) -> Result<usize, Diagnostic> {
    limits.check()?;
    let length = encoded_length(d, limits.metadata_bytes.min(limits.total_bytes))?;
    check_digest(&d.source_digest)?;
    if d.byte_length > limits.source_bytes || d.byte_length as u128 > 9_007_199_254_740_991 {
        return Err(limit_error());
    }
    check_provenance(&d.path, &d.holder, &d.metadata)?;
    if let Some(doc) = &d.document {
        if !matches!(
            doc.schema.as_str(),
            "oh.war/document/1.0.0-rc.2" | "oh.war/document/1.0.0-rc.3"
        ) {
            return Err(Diagnostic::error(
                "unsupported-schema",
                "Unsupported document schema",
                0..0,
            ));
        }
        if !super::validate::identity(&doc.id)
            || doc.revision == 0
            || doc.revision > 9_007_199_254_740_991
            || !matches!(doc.kind.as_str(), "warrant" | "sas" | "adr" | "context")
        {
            return Err(Diagnostic::error(
                "source-invalid",
                "Invalid document identity, revision or kind",
                0..0,
            ));
        }
    }
    Ok(length)
}
fn check_reference_shape(reference: &BoundReference) -> Result<(), Diagnostic> {
    check_digest(&reference.source_digest)?;
    if reference.unit != "*" && !super::scan::unit_id(&reference.unit) {
        return Err(Diagnostic::error("target-invalid", "Invalid unit ID", 0..0));
    }
    if reference.start > reference.end {
        return Err(Diagnostic::error(
            "range-mismatch",
            "Range start exceeds end",
            0..0,
        ));
    }
    if reference.end as u128 > 9_007_199_254_740_991 {
        return Err(limit_error());
    }
    Ok(())
}
/// Decode one F5 descriptor without acquiring its bytes or establishing its claims.
pub fn decode_source_descriptor(
    bytes: &[u8],
    limits: SourceLimits,
) -> Result<SourceDescriptor, Diagnostic> {
    limits.check()?;
    if bytes.len() > limits.metadata_bytes.min(limits.total_bytes) {
        return Err(limit_error());
    }
    let value = serde_json::from_slice(bytes)
        .map_err(|e| Diagnostic::error("source-invalid", e.to_string(), 0..0))?;
    check_descriptor(&value, limits)?;
    Ok(value)
}
pub fn encode_source_descriptor(
    value: &SourceDescriptor,
    limits: SourceLimits,
) -> Result<Vec<u8>, Diagnostic> {
    check_descriptor(value, limits)?;
    encoded_length(value, limits.output_bytes)?;
    serde_jcs::to_vec(value).map_err(|e| Diagnostic::error("source-invalid", e.to_string(), 0..0))
}
pub fn decode_bound_reference(
    bytes: &[u8],
    limits: SourceLimits,
) -> Result<BoundReference, Diagnostic> {
    limits.check()?;
    if bytes.len() > limits.metadata_bytes.min(limits.total_bytes) {
        return Err(limit_error());
    }
    let value: BoundReference = serde_json::from_slice(bytes)
        .map_err(|e| Diagnostic::error("source-invalid", e.to_string(), 0..0))?;
    check_reference_shape(&value)?;
    if value.end > limits.source_bytes {
        return Err(limit_error());
    }
    Ok(value)
}
pub fn encode_bound_reference(
    value: &BoundReference,
    limits: SourceLimits,
) -> Result<Vec<u8>, Diagnostic> {
    limits.check()?;
    check_reference_shape(value)?;
    if value.end > limits.source_bytes {
        return Err(limit_error());
    }
    encoded_length(
        value,
        limits
            .metadata_bytes
            .min(limits.total_bytes)
            .min(limits.output_bytes),
    )?;
    serde_jcs::to_vec(value).map_err(|e| Diagnostic::error("source-invalid", e.to_string(), 0..0))
}

/// Describe explicit bytes. The caller owns capture, stable-read evidence and access.
pub fn describe_source(
    path: &str,
    holder: Holder,
    metadata: SourceMetadata,
    bytes: &[u8],
    dialect: Option<Dialect>,
    limits: SourceLimits,
) -> Result<SourceDescriptor, Diagnostic> {
    limits.check()?;
    if bytes.len() > limits.source_bytes.min(limits.total_bytes)
        || path.len() > limits.metadata_bytes
    {
        return Err(limit_error());
    }
    encoded_length(
        &(path, &holder, &metadata),
        limits.metadata_bytes.min(limits.total_bytes - bytes.len()),
    )?;
    check_provenance(path, &holder, &metadata)?;
    let parsed = parse_checked(bytes, dialect, limits)?;
    let document = parsed.as_ref().map(document_identity);
    let descriptor = SourceDescriptor {
        path: path.into(),
        source_digest: raw_digest(bytes),
        byte_length: bytes.len(),
        holder,
        metadata,
        document,
    };
    let size = check_descriptor(&descriptor, limits)?;
    if bytes
        .len()
        .checked_add(size)
        .is_none_or(|n| n > limits.total_bytes)
    {
        return Err(limit_error());
    }
    Ok(descriptor)
}

fn parse_checked(
    bytes: &[u8],
    dialect: Option<Dialect>,
    limits: SourceLimits,
) -> Result<Option<super::Document<'_>>, Diagnostic> {
    dialect
        .map(|dialect| {
            let parsed = parse_document(bytes, dialect, limits.parse_limits())?;
            let report = validate_document(&parsed, &ValidationOptions::default());
            if report.validity != Validity::Valid {
                return Err(report
                    .diagnostics
                    .into_iter()
                    .find(|d| d.severity == super::Severity::Error)
                    .expect("invalid document has diagnostic"));
            }
            Ok(parsed)
        })
        .transpose()
}
fn document_identity(parsed: &super::Document<'_>) -> DocumentIdentity {
    let fields = parsed.metadata();
    DocumentIdentity {
        id: fields["id"].as_str().expect("validated id").into(),
        revision: fields["revision"].as_integer().expect("validated revision") as u64,
        schema: parsed.dialect().schema().into(),
        kind: fields["kind"].as_str().expect("validated kind").into(),
    }
}
