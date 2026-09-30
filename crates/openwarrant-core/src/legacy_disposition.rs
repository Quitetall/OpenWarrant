// SPDX-License-Identifier: AGPL-3.0-or-later
//! Disposition records for a frozen legacy ADR corpus.
//!
//! Four records remain deliberately distinct:
//!
//! - a [`LegacyAdrDispositionProposal`] is nonauthoritative triage;
//! - a [`LegacyAdrDispositionReviewResponse`] carries authorized human judgments;
//! - a [`LegacyAdrDispositionReceipt`] records one declared human judgment;
//! - a [`LegacyAdrDispositionManifest`] proves structural set coverage.
//!
//! Core validation proves shape only. It does not authenticate an actor, read a
//! Git object, recompute a digest, authorize a Warrant, or resolve one. Those
//! environmental checks belong in an application verifier.
//!
//! JSON wire bytes must enter through [`parse_legacy_json`]. Semantic protocol
//! types deliberately do not implement `Deserialize`: Serde's data model does
//! not retain number lexemes, so a public derive would bypass exact admission.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize, Serializer};

use crate::{JudgmentAuthority, WarUuid};

mod wire;

pub const LEGACY_ADR_DISPOSITION_PROPOSAL_SCHEMA: &str =
    "oh.war/legacy-adr-disposition-proposal/v1";
pub const LEGACY_ADR_DISPOSITION_PROPOSAL_KIND: &str =
    "legacy_adr_disposition_proposal_not_receipt";
pub const LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_SCHEMA: &str =
    "oh.war/legacy-adr-disposition-review-response/v1";
pub const LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_KIND: &str =
    "legacy_adr_disposition_review_response";
pub const LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_SCHEMA: &str =
    "oh.war/legacy-adr-disposition-review-subject/v1";
pub const LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_KIND: &str =
    "legacy_adr_disposition_review_subject";
pub const LEGACY_ADR_SOURCE_CAPSULE_SCHEMA: &str = "oh.war/legacy-adr-source-capsule/v1";
pub const LEGACY_ADR_FALSIFICATION_SCHEMA: &str = "oh.war/legacy-adr-falsification-observation/v1";
pub const LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA: &str = "oh.war/legacy-adr-disposition-receipt/v1";
pub const LEGACY_ADR_DISPOSITION_RECEIPT_KIND: &str = "legacy_adr_disposition_receipt";
pub const LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA: &str =
    "oh.war/legacy-adr-disposition-manifest/v1";
pub const LEGACY_ADR_DISPOSITION_MANIFEST_KIND: &str = "legacy_adr_disposition_manifest";
pub const MAX_LEGACY_JSON_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LEGACY_JSON_NESTING: usize = 64;
pub const MAX_LEGACY_JSON_VALUES: usize = 1_000_000;
pub const MAX_LEGACY_JSON_TOKENS: usize = 1_000_000;

/// One validated value from an optional namespaced JSON extension.
///
/// The representation is deliberately opaque. In particular, callers cannot
/// construct NaN, infinity, or two unequal numeric variants that serialize as
/// the same JSON number. [`parse_legacy_json`] observes each exact number token,
/// admits it only when RFC 8785 normalization preserves its mathematical
/// value, then typed deserialization stores the finite IEEE-754 value.
#[derive(Debug, Clone)]
pub struct LegacyExtensionValue(LegacyExtensionValueKind);

#[derive(Debug, Clone)]
enum LegacyExtensionValueKind {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<LegacyExtensionValue>),
    Object(BTreeMap<String, LegacyExtensionValue>),
}

impl LegacyExtensionValue {
    /// Construct a numeric extension value from an already-decoded IEEE-754
    /// value. Text input must use [`parse_legacy_json`], which also checks that
    /// RFC 8785 normalization does not change the source number's meaning.
    pub fn try_from_f64(value: f64) -> Result<Self, LegacyExtensionNumberError> {
        if !value.is_finite() {
            return Err(LegacyExtensionNumberError::NonFinite {
                found: value.to_string(),
            });
        }
        Ok(Self(LegacyExtensionValueKind::Number(normalize_zero(
            value,
        ))))
    }

    const fn rank(&self) -> u8 {
        match self.0 {
            LegacyExtensionValueKind::Null => 0,
            LegacyExtensionValueKind::Bool(_) => 1,
            LegacyExtensionValueKind::Number(_) => 2,
            LegacyExtensionValueKind::String(_) => 3,
            LegacyExtensionValueKind::Array(_) => 4,
            LegacyExtensionValueKind::Object(_) => 5,
        }
    }
}

/// Why a numeric extension value cannot enter the RFC 8785 protocol domain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LegacyExtensionNumberError {
    #[error("extension number {found:?} must be finite for RFC 8785 canonical JSON")]
    NonFinite { found: String },
    #[error("extension number {found:?} is outside the finite IEEE-754 range required by RFC 8785")]
    OutsideFiniteRange { found: String },
    #[error(
        "extension number {found:?} would change mathematical value under RFC 8785 canonicalization to {canonical:?}"
    )]
    ChangesUnderCanonicalization { found: String, canonical: String },
}

fn normalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

#[derive(Debug, PartialEq, Eq)]
struct DecimalIdentity {
    negative: bool,
    significant: String,
    exponent: i128,
}

fn parse_decimal_exponent(text: &str) -> Option<i128> {
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'+') => (false, &text[1..]),
        Some(b'-') => (true, &text[1..]),
        _ => (false, text),
    };
    if digits.is_empty() {
        return None;
    }
    let magnitude = digits.bytes().try_fold(0_i128, |value, byte| {
        byte.is_ascii_digit()
            .then_some(byte - b'0')
            .and_then(|digit| value.checked_mul(10)?.checked_add(i128::from(digit)))
    })?;
    if negative {
        magnitude.checked_neg()
    } else {
        Some(magnitude)
    }
}

fn decimal_identity(text: &str) -> Option<DecimalIdentity> {
    let (negative, unsigned) = text
        .strip_prefix('-')
        .map_or((false, text), |unsigned| (true, unsigned));
    let (mantissa, explicit_exponent) = match unsigned.find(['e', 'E']) {
        Some(index) => (
            &unsigned[..index],
            parse_decimal_exponent(&unsigned[index + 1..])?,
        ),
        None => (unsigned, 0),
    };
    let (integer, fraction) = mantissa
        .split_once('.')
        .map_or((mantissa, ""), |(integer, fraction)| (integer, fraction));
    if integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    let mut significant = String::with_capacity(integer.len() + fraction.len());
    significant.push_str(integer);
    significant.push_str(fraction);
    let first_nonzero = significant.bytes().position(|byte| byte != b'0');
    let Some(first_nonzero) = first_nonzero else {
        return Some(DecimalIdentity {
            negative: false,
            significant: "0".to_owned(),
            exponent: 0,
        });
    };
    significant.drain(..first_nonzero);

    let fraction_len = i128::try_from(fraction.len()).ok()?;
    let mut exponent = explicit_exponent.checked_sub(fraction_len)?;
    while significant.ends_with('0') {
        significant.pop();
        exponent = exponent.checked_add(1)?;
    }
    Some(DecimalIdentity {
        negative,
        significant,
        exponent,
    })
}

fn canonical_extension_number(raw: &str) -> Result<f64, LegacyExtensionNumberError> {
    // Caller supplies a lexical number produced by jstrict's RFC 8259 parser.
    // Keeping that token intact is what lets this comparison detect rounding.
    let exact = raw;
    let value =
        exact
            .parse::<f64>()
            .map_err(|_| LegacyExtensionNumberError::OutsideFiniteRange {
                found: exact.to_owned(),
            })?;
    if !value.is_finite() {
        return Err(LegacyExtensionNumberError::OutsideFiniteRange {
            found: exact.to_owned(),
        });
    }
    let value = normalize_zero(value);
    let canonical = serde_jcs::to_string(&value).map_err(|_| {
        LegacyExtensionNumberError::OutsideFiniteRange {
            found: exact.to_owned(),
        }
    })?;
    if decimal_identity(exact) != decimal_identity(&canonical) {
        return Err(LegacyExtensionNumberError::ChangesUnderCanonicalization {
            found: exact.to_owned(),
            canonical,
        });
    }
    Ok(value)
}

impl Serialize for LegacyExtensionValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            LegacyExtensionValueKind::Null => serializer.serialize_unit(),
            LegacyExtensionValueKind::Bool(value) => serializer.serialize_bool(*value),
            LegacyExtensionValueKind::Number(value) => serializer.serialize_f64(*value),
            LegacyExtensionValueKind::String(value) => serializer.serialize_str(value),
            LegacyExtensionValueKind::Array(values) => values.serialize(serializer),
            LegacyExtensionValueKind::Object(values) => values.serialize(serializer),
        }
    }
}

impl PartialEq for LegacyExtensionValue {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LegacyExtensionValue {}

impl PartialOrd for LegacyExtensionValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LegacyExtensionValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (LegacyExtensionValueKind::Null, LegacyExtensionValueKind::Null) => Ordering::Equal,
            (LegacyExtensionValueKind::Bool(left), LegacyExtensionValueKind::Bool(right)) => {
                left.cmp(right)
            }
            (LegacyExtensionValueKind::Number(left), LegacyExtensionValueKind::Number(right)) => {
                left.total_cmp(right)
            }
            (LegacyExtensionValueKind::String(left), LegacyExtensionValueKind::String(right)) => {
                left.cmp(right)
            }
            (LegacyExtensionValueKind::Array(left), LegacyExtensionValueKind::Array(right)) => {
                left.cmp(right)
            }
            (LegacyExtensionValueKind::Object(left), LegacyExtensionValueKind::Object(right)) => {
                left.cmp(right)
            }
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

/// Why JSON bytes cannot enter the legacy disposition protocol domain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LegacyJsonError {
    #[error("legacy disposition JSON is not valid UTF-8: {0}")]
    Utf8(String),
    #[error("legacy disposition JSON syntax is invalid: {0}")]
    Syntax(String),
    #[error("legacy disposition JSON value is outside the RFC 8785 domain: {0}")]
    CanonicalDomain(String),
    #[error("legacy disposition JSON exceeds structural limits: {0}")]
    Limit(String),
    #[error("legacy disposition JSON does not match the requested record type: {0}")]
    TypedDecode(String),
}

fn validate_json_domain(value: &jstrict::Value) -> Result<(), LegacyJsonError> {
    let mut stack = vec![(value, 0_usize)];
    let mut values = 0_usize;
    while let Some((value, depth)) = stack.pop() {
        values = values.saturating_add(1);
        if values > MAX_LEGACY_JSON_VALUES {
            return Err(LegacyJsonError::Limit(format!(
                "value count exceeds {MAX_LEGACY_JSON_VALUES}"
            )));
        }
        match value {
            jstrict::Value::Number(number) => {
                canonical_extension_number(number.as_str())
                    .map_err(|error| LegacyJsonError::CanonicalDomain(error.to_string()))?;
            }
            jstrict::Value::Array(items) => {
                let child_depth = depth.saturating_add(1);
                if !items.is_empty() && child_depth > MAX_LEGACY_JSON_NESTING {
                    return Err(LegacyJsonError::Limit(format!(
                        "nesting exceeds {MAX_LEGACY_JSON_NESTING}"
                    )));
                }
                stack.extend(items.iter().map(|item| (item, child_depth)));
            }
            jstrict::Value::Object(object) => {
                let mut keys = BTreeSet::new();
                for entry in object.iter() {
                    if !keys.insert(entry.key.as_str()) {
                        return Err(LegacyJsonError::CanonicalDomain(format!(
                            "duplicate object key {:?} is not admitted",
                            entry.key.as_str()
                        )));
                    }
                }
                let child_depth = depth.saturating_add(1);
                if !object.is_empty() && child_depth > MAX_LEGACY_JSON_NESTING {
                    return Err(LegacyJsonError::Limit(format!(
                        "nesting exceeds {MAX_LEGACY_JSON_NESTING}"
                    )));
                }
                stack.extend(object.iter().map(|entry| (&entry.value, child_depth)));
            }
            jstrict::Value::Null | jstrict::Value::Boolean(_) | jstrict::Value::String(_) => {}
        }
    }
    Ok(())
}

fn preflight_json_limits(bytes: &[u8]) -> Result<(), LegacyJsonError> {
    let mut depth = 0_usize;
    let mut tokens = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_primitive = false;

    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => {
                increment_preflight_token(&mut tokens)?;
                in_string = true;
                in_primitive = false;
            }
            b'{' | b'[' => {
                increment_preflight_token(&mut tokens)?;
                depth = depth.saturating_add(1);
                if depth > MAX_LEGACY_JSON_NESTING {
                    return Err(LegacyJsonError::Limit(format!(
                        "nesting exceeds {MAX_LEGACY_JSON_NESTING} before tree allocation"
                    )));
                }
                in_primitive = false;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                in_primitive = false;
            }
            b',' | b':' | b' ' | b'\t' | b'\r' | b'\n' => in_primitive = false,
            b'-' | b'0'..=b'9' | b't' | b'f' | b'n' if !in_primitive => {
                increment_preflight_token(&mut tokens)?;
                in_primitive = true;
            }
            _ => {}
        }
    }
    Ok(())
}

fn increment_preflight_token(tokens: &mut usize) -> Result<(), LegacyJsonError> {
    *tokens = tokens.saturating_add(1);
    if *tokens > MAX_LEGACY_JSON_TOKENS {
        return Err(LegacyJsonError::Limit(format!(
            "token count exceeds {MAX_LEGACY_JSON_TOKENS} before tree allocation"
        )));
    }
    Ok(())
}

fn parse_admitted_json(bytes: &[u8]) -> Result<jstrict::Value, LegacyJsonError> {
    if bytes.len() > MAX_LEGACY_JSON_BYTES {
        return Err(LegacyJsonError::Limit(format!(
            "byte length {} exceeds {MAX_LEGACY_JSON_BYTES}",
            bytes.len()
        )));
    }
    preflight_json_limits(bytes)?;
    let text =
        std::str::from_utf8(bytes).map_err(|error| LegacyJsonError::Utf8(error.to_string()))?;
    let value = jstrict::parse::parse_str_value(text)
        .map_err(|error| LegacyJsonError::Syntax(error.to_string()))?;
    validate_json_domain(&value)?;
    Ok(value)
}

/// Marker for semantic legacy records admitted from exact JSON bytes.
///
/// This trait is sealed. External crates can use it as a generic bound but
/// cannot provide an alternate decoder that skips lexical admission.
///
/// Direct Serde admission does not compile:
///
/// ```compile_fail
/// use openwarrant_core::legacy_disposition::LegacyAdrDispositionManifest;
///
/// let _: LegacyAdrDispositionManifest = serde_json::from_slice(b"{}").unwrap();
/// ```
#[allow(private_bounds)]
pub trait LegacyJsonRecord: wire::Sealed {}

impl<T> LegacyJsonRecord for T where T: wire::Sealed {}

/// Parse one legacy disposition JSON artifact without losing lexical meaning.
///
/// Parsing is deliberately two-phase. `jstrict` first preserves duplicate
/// entries and exact number tokens and rejects invalid RFC 8259 syntax. The
/// numeric domain check then refuses values whose RFC 8785 form would change
/// mathematical meaning. Only that admitted tree enters typed Serde decoding.
pub fn parse_legacy_json<T>(bytes: &[u8]) -> Result<T, LegacyJsonError>
where
    T: LegacyJsonRecord,
{
    let value = parse_admitted_json(bytes)?;
    <T as wire::Sealed>::decode(value)
}

/// Parse strict RFC 8259 JSON for application-owned support records.
///
/// This shares lexical-number, duplicate-key, and allocation limits with the
/// sealed semantic protocol path. It does not make `Deserialize` an admission
/// API for the semantic legacy record types, which do not implement that trait.
pub fn parse_strict_json<T>(bytes: &[u8]) -> Result<T, LegacyJsonError>
where
    T: DeserializeOwned,
{
    let value = parse_admitted_json(bytes)?;
    jstrict::from_value(value).map_err(|error| LegacyJsonError::TypedDecode(error.to_string()))
}

/// Unknown optional fields preserved at one protocol-record boundary.
///
/// A field is namespaced when it has nonempty text on both sides of its first
/// `.`. Non-namespaced unknown fields are treated as required vocabulary this
/// build does not understand and fail closed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize)]
#[serde(transparent)]
pub struct NamespacedExtensions(BTreeMap<String, LegacyExtensionValue>);

impl<'de> Deserialize<'de> for NamespacedExtensions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        wire::deserialize_namespaced_extensions(deserializer)
    }
}

impl NamespacedExtensions {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&LegacyExtensionValue> {
        self.0.get(name)
    }

    /// Iterate in lexical key order, matching canonical map semantics.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &LegacyExtensionValue)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }
}

fn is_namespaced_extension_name(name: &str) -> bool {
    name.split_once('.')
        .is_some_and(|(namespace, local_name)| !namespace.is_empty() && !local_name.is_empty())
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LegacyDispositionError {
    #[error("{record} has schema {found:?}; expected {expected:?}")]
    SchemaMismatch {
        record: &'static str,
        found: String,
        expected: &'static str,
    },
    #[error("{record} has kind {found:?}; expected {expected:?}")]
    KindMismatch {
        record: &'static str,
        found: String,
        expected: &'static str,
    },
    #[error("ADR identifier {found:?} is not exactly four ASCII digits")]
    MalformedAdrId { found: String },
    #[error("ADR {adr_id} source path {path:?} is not its NNNN-*.md record")]
    SourcePathMismatch { adr_id: String, path: String },
    #[error("ADR {adr_id} field {field} is blank")]
    BlankField { adr_id: String, field: &'static str },
    #[error("ADR {adr_id} field {field} is malformed {kind}: {found:?}")]
    MalformedIdentity {
        adr_id: String,
        field: &'static str,
        kind: &'static str,
        found: String,
    },
    #[error(
        "ADR {adr_id} review time {found:?} is not canonical Gregorian UTC YYYY-MM-DDTHH:MM:SSZ"
    )]
    MalformedReviewTime { adr_id: String, found: String },
    #[error(
        "ADR {adr_id} disposition {disposition:?} names no content-bound evidence or successor"
    )]
    SupportRequired {
        adr_id: String,
        disposition: LegacyAdrDisposition,
    },
    #[error("ADR {adr_id} is superseded but names no content-bound successor")]
    SuccessorRequired { adr_id: String },
    #[error("ADR {adr_id} {field} bindings are not sorted and unique")]
    ContentBindingsNotCanonical { adr_id: String, field: &'static str },
    #[error("ADR {adr_id} proposal digest is malformed: {found:?}")]
    MalformedProposalDigest { adr_id: String, found: String },
    #[error("ADR {adr_id} receipt digest is malformed: {found:?}")]
    MalformedReceiptDigest { adr_id: String, found: String },
    #[error("manifest digest is malformed: {found:?}")]
    MalformedManifestDigest { found: String },
    #[error(
        "manifest verification_as_of {found:?} is not canonical Gregorian UTC YYYY-MM-DDTHH:MM:SSZ"
    )]
    MalformedManifestVerificationTime { found: String },
    #[error("identifier inventory {inventory} is not sorted and unique")]
    IdentifierInventoryNotCanonical { inventory: &'static str },
    #[error("receipt inventory is not sorted by ADR id")]
    ReceiptInventoryNotCanonical,
    #[error("ADR {adr_id} appears in both SHA-F and off-branch inventories")]
    OverlappingInventory { adr_id: String },
    #[error("final ADR {adr_id} cannot disposition itself as a predecessor")]
    FinalAdrInInventory { adr_id: String },
    #[error("ADR {adr_id} has more than one disposition receipt")]
    DuplicateReceipt { adr_id: String },
    #[error("receipt digest {digest} is reused")]
    DuplicateReceiptDigest { digest: String },
    #[error("source binding for ADR {adr_id} is reused")]
    DuplicateSourceBinding { adr_id: String },
    #[error("ADR successor {adr_id} is declared more than once")]
    DuplicateSuccessorAdr { adr_id: String },
    #[error("resolved Warrant successor {warrant_uuid} is declared more than once")]
    DuplicateSuccessorWarrant { warrant_uuid: String },
    #[error("resolved Warrant successor UUID {found} is not RFC 4122 UUIDv7")]
    SuccessorWarrantUuidNotV7 { found: String },
    #[error(
        "receipt coverage differs from inventory: expected {expected}, found {found}; missing [{missing}], unexpected [{unexpected}]"
    )]
    ReceiptCoverageMismatch {
        expected: usize,
        found: usize,
        missing: String,
        unexpected: String,
    },
    #[error("SHA-F ADR {adr_id} binds commit {found}, expected {expected}")]
    FrozenCommitMismatch {
        adr_id: String,
        expected: String,
        found: String,
    },
    #[error("off-branch ADR {adr_id} incorrectly binds SHA-F {sha_f}")]
    OffBranchCommitMatchesFreeze { adr_id: String, sha_f: String },
    #[error("SHA-F ADR {adr_id} is not bound to the manifest source import")]
    SourceImportMismatch { adr_id: String },
    #[error("ADR {adr_id} receipt binds a different governing Warrant")]
    WarrantBindingMismatch { adr_id: String },
    #[error("Warrant {warrant_id} has invalid authorized revision 0")]
    InvalidAuthorizedRevision { warrant_id: String },
    #[error("legacy ADR disposition review response contains no judgments")]
    EmptyReviewResponse,
    #[error("review response time {found:?} is not canonical Gregorian UTC YYYY-MM-DDTHH:MM:SSZ")]
    MalformedResponseReviewTime { found: String },
    #[error("review response judgments are not sorted by both ADR id and judgment id")]
    ReviewJudgmentsNotCanonical,
    #[error("review judgment id {judgment_id:?} is reused")]
    DuplicateReviewJudgmentId { judgment_id: String },
    #[error("ADR {adr_id} has more than one review judgment")]
    DuplicateReviewedAdr { adr_id: String },
    #[error("review judgment {judgment_id:?} is not an authorized human judgment")]
    UnauthorizedReviewJudgment { judgment_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyAdrDisposition {
    Complete,
    Superseded,
    Deprecated,
    Rejected,
    Falsified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitObjectFormat {
    Sha1,
    Sha256,
}

/// An excluded legacy ADR has no imported body by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedBodyStatus {
    NotApplicable,
}

impl GitObjectFormat {
    const fn hex_len(self) -> usize {
        match self {
            Self::Sha1 => 40,
            Self::Sha256 => 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ContentBinding {
    pub reference: String,
    pub sha256: String,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct WarrantBinding {
    pub warrant_id: String,
    pub authorized_revision: u32,
    /// OpenWarrant's domain-separated Contract digest (lowercase SHA-256 hex).
    pub contract_digest: String,
    pub authorization_ref: String,
    pub authorization_sha256: String,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceBinding {
    pub adr_id: String,
    pub repository_identity: String,
    pub git_object_format: GitObjectFormat,
    pub exact_ref: String,
    pub ref_tip_at_review: String,
    pub commit_sha: String,
    pub path: String,
    pub source_sha256: String,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

/// Exact excluded-source capsule admitted through the sealed JSON boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrSourceCapsule {
    pub schema: String,
    pub source: SourceBinding,
    pub exact_source: String,
}

/// Content-bound falsification observation set admitted as one sealed record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrFalsificationRecord {
    pub schema: String,
    pub subject_source_sha256: String,
    pub statement: String,
    pub observations: Vec<ContentBinding>,
}

/// Migration state is algebraic: excluded records cannot accidentally carry an
/// import binding. Their exact bytes remain content-bound through a nonsemantic
/// source capsule instead of a fabricated import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ImportBinding {
    Migrated {
        artifact: ContentBinding,
        imported_body_sha256: String,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
    DeliberatelyExcluded {
        source_capsule: ContentBinding,
        imported_body_status: ImportedBodyStatus,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
}

/// Closed support vocabulary for one terminal legacy disposition.
///
/// Generic bytes are deliberately not evidence. Each variant declares what
/// the artifact means, while the application verifier checks the corresponding
/// source, Gate Run, receipt, or observation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LegacyAdrSupport {
    ImplementationArtifact {
        artifact: ContentBinding,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
    RequiredGatePass {
        run: ContentBinding,
        definition: ContentBinding,
        binding: ContentBinding,
        selection: Box<ContentBinding>,
        stdout: Box<ContentBinding>,
        stderr: Box<ContentBinding>,
        fixtures: Vec<ContentBinding>,
        receipt: Box<ContentBinding>,
        subject_source_sha256: String,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
    FalsificationObservation {
        record: ContentBinding,
        subject_source_sha256: String,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
    HistoricalContext {
        artifact: ContentBinding,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
}

/// A successor is an ADR source identity or a resolved Warrant, never an
/// arbitrary file that happens to exist.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LegacyAdrSuccessor {
    Adr {
        source: SourceBinding,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
    ResolvedWarrant {
        warrant_uuid: WarUuid,
        warrant: WarrantBinding,
        resolution: ContentBinding,
        resolution_digest: String,
        #[serde(flatten, default)]
        extensions: NamespacedExtensions,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HumanReview {
    pub actor: String,
    pub role_assignment_ref: String,
    pub role_assignment_sha256: String,
    pub reviewed_at: String,
    /// Exact response artifact containing the authorized human judgment.
    pub response: ContentBinding,
    /// Judgment selected from `response` for this ADR receipt.
    pub judgment_id: String,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

/// One independently supplied human disposition judgment.
///
/// `authority` must be [`JudgmentAuthority::Authorized`]. Agent output belongs
/// in [`LegacyAdrDispositionProposal`], never this response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionJudgment {
    pub id: String,
    pub adr_id: String,
    pub review_subject_digest: String,
    pub statement: String,
    pub actor: String,
    pub acting_role: String,
    pub meaning: String,
    pub authority: JudgmentAuthority,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

impl LegacyAdrDispositionJudgment {
    pub fn validate_structure(&self) -> Result<(), LegacyDispositionError> {
        validate_adr_id(&self.adr_id)?;
        require_nonblank(&self.adr_id, "judgment_id", &self.id)?;
        validate_digest(
            &self.adr_id,
            "review_subject_digest",
            &self.review_subject_digest,
        )?;
        require_nonblank(&self.adr_id, "judgment_statement", &self.statement)?;
        require_nonblank(&self.adr_id, "judgment_actor", &self.actor)?;
        require_nonblank(&self.adr_id, "judgment_acting_role", &self.acting_role)?;
        require_nonblank(&self.adr_id, "judgment_meaning", &self.meaning)?;
        if self.authority != JudgmentAuthority::Authorized {
            return Err(LegacyDispositionError::UnauthorizedReviewJudgment {
                judgment_id: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// Dedicated, human-authored response to legacy ADR disposition subjects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionReviewResponse {
    pub schema: String,
    pub kind: String,
    pub warrant: WarrantBinding,
    pub reviewed_at: String,
    pub judgments: Vec<LegacyAdrDispositionJudgment>,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

impl LegacyAdrDispositionReviewResponse {
    pub fn validate_structure(&self) -> Result<(), LegacyDispositionError> {
        validate_tag(
            "review response",
            &self.schema,
            LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_SCHEMA,
            &self.kind,
            LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_KIND,
        )?;
        let first = self
            .judgments
            .first()
            .ok_or(LegacyDispositionError::EmptyReviewResponse)?;
        validate_warrant(&first.adr_id, &self.warrant)?;
        if !is_canonical_utc(&self.reviewed_at) {
            return Err(LegacyDispositionError::MalformedResponseReviewTime {
                found: self.reviewed_at.clone(),
            });
        }

        let mut judgment_ids = BTreeSet::new();
        let mut adr_ids = BTreeSet::new();
        for judgment in &self.judgments {
            judgment.validate_structure()?;
            if !judgment_ids.insert(judgment.id.as_str()) {
                return Err(LegacyDispositionError::DuplicateReviewJudgmentId {
                    judgment_id: judgment.id.clone(),
                });
            }
            if !adr_ids.insert(judgment.adr_id.as_str()) {
                return Err(LegacyDispositionError::DuplicateReviewedAdr {
                    adr_id: judgment.adr_id.clone(),
                });
            }
        }
        if self
            .judgments
            .windows(2)
            .any(|pair| pair[0].adr_id >= pair[1].adr_id || pair[0].id >= pair[1].id)
        {
            return Err(LegacyDispositionError::ReviewJudgmentsNotCanonical);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionProposalPayload {
    pub warrant: WarrantBinding,
    pub source: SourceBinding,
    pub migration: ImportBinding,
    pub proposed_disposition: LegacyAdrDisposition,
    pub reason: String,
    #[serde(default)]
    pub evidence: Vec<LegacyAdrSupport>,
    #[serde(default)]
    pub successors: Vec<LegacyAdrSuccessor>,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionProposal {
    pub schema: String,
    pub kind: String,
    pub payload: LegacyAdrDispositionProposalPayload,
    pub proposal_digest: String,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyDispositionDigestPreimage<'a, T: Serialize> {
    pub schema: &'a str,
    pub kind: &'a str,
    pub payload: &'a T,
    #[serde(flatten)]
    pub extensions: &'a NamespacedExtensions,
}

impl LegacyAdrDispositionProposal {
    #[must_use]
    pub fn digest_preimage(
        &self,
    ) -> LegacyDispositionDigestPreimage<'_, LegacyAdrDispositionProposalPayload> {
        LegacyDispositionDigestPreimage {
            schema: &self.schema,
            kind: &self.kind,
            payload: &self.payload,
            extensions: &self.extensions,
        }
    }

    pub fn validate_structure(&self) -> Result<(), LegacyDispositionError> {
        validate_tag(
            "proposal",
            &self.schema,
            LEGACY_ADR_DISPOSITION_PROPOSAL_SCHEMA,
            &self.kind,
            LEGACY_ADR_DISPOSITION_PROPOSAL_KIND,
        )?;
        let id = &self.payload.source.adr_id;
        validate_warrant(id, &self.payload.warrant)?;
        validate_source(&self.payload.source)?;
        validate_import(id, &self.payload.migration)?;
        validate_outcome_support(
            id,
            self.payload.proposed_disposition,
            &self.payload.reason,
            &self.payload.evidence,
            &self.payload.successors,
        )?;
        if !is_sha256(&self.proposal_digest) {
            return Err(LegacyDispositionError::MalformedProposalDigest {
                adr_id: id.clone(),
                found: self.proposal_digest.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionReceiptPayload {
    pub warrant: WarrantBinding,
    pub source: SourceBinding,
    pub migration: ImportBinding,
    pub review: HumanReview,
    pub disposition: LegacyAdrDisposition,
    pub reason: String,
    #[serde(default)]
    pub evidence: Vec<LegacyAdrSupport>,
    #[serde(default)]
    pub successors: Vec<LegacyAdrSuccessor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_digest: Option<String>,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

/// Receipt fields fixed before a human reviews a proposed disposition.
///
/// Human review metadata is intentionally absent: response production and
/// receipt assembly cannot change the subject the Judge evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionReviewSubjectPayload<'a> {
    pub warrant: &'a WarrantBinding,
    pub source: &'a SourceBinding,
    pub migration: &'a ImportBinding,
    pub disposition: LegacyAdrDisposition,
    pub reason: &'a str,
    pub evidence: &'a [LegacyAdrSupport],
    pub successors: &'a [LegacyAdrSuccessor],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_digest: Option<&'a str>,
    #[serde(flatten)]
    pub extensions: &'a NamespacedExtensions,
}

/// Domain-tagged serialization preimage bound by a human review judgment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionReviewSubjectPreimage<'a> {
    pub schema: &'static str,
    pub kind: &'static str,
    pub payload: LegacyAdrDispositionReviewSubjectPayload<'a>,
}

impl LegacyAdrDispositionReceiptPayload {
    #[must_use]
    pub fn review_subject_preimage(&self) -> LegacyAdrDispositionReviewSubjectPreimage<'_> {
        LegacyAdrDispositionReviewSubjectPreimage {
            schema: LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_SCHEMA,
            kind: LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_KIND,
            payload: LegacyAdrDispositionReviewSubjectPayload {
                warrant: &self.warrant,
                source: &self.source,
                migration: &self.migration,
                disposition: self.disposition,
                reason: &self.reason,
                evidence: &self.evidence,
                successors: &self.successors,
                proposal_digest: self.proposal_digest.as_deref(),
                extensions: &self.extensions,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionReceipt {
    pub schema: String,
    pub kind: String,
    pub payload: LegacyAdrDispositionReceiptPayload,
    pub receipt_digest: String,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

impl LegacyAdrDispositionReceipt {
    #[must_use]
    pub fn digest_preimage(
        &self,
    ) -> LegacyDispositionDigestPreimage<'_, LegacyAdrDispositionReceiptPayload> {
        LegacyDispositionDigestPreimage {
            schema: &self.schema,
            kind: &self.kind,
            payload: &self.payload,
            extensions: &self.extensions,
        }
    }

    pub fn validate_structure(&self) -> Result<(), LegacyDispositionError> {
        validate_tag(
            "receipt",
            &self.schema,
            LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA,
            &self.kind,
            LEGACY_ADR_DISPOSITION_RECEIPT_KIND,
        )?;
        let id = &self.payload.source.adr_id;
        validate_warrant(id, &self.payload.warrant)?;
        validate_source(&self.payload.source)?;
        validate_import(id, &self.payload.migration)?;
        validate_review(id, &self.payload.review)?;
        validate_outcome_support(
            id,
            self.payload.disposition,
            &self.payload.reason,
            &self.payload.evidence,
            &self.payload.successors,
        )?;
        if let Some(digest) = &self.payload.proposal_digest {
            validate_digest(id, "proposal_digest", digest)?;
        }
        if !is_sha256(&self.receipt_digest) {
            return Err(LegacyDispositionError::MalformedReceiptDigest {
                adr_id: id.clone(),
                found: self.receipt_digest.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionManifestPayload {
    pub final_adr_id: String,
    pub warrant: WarrantBinding,
    pub sha_f: String,
    /// Declared observation boundary for application-level chronology checks.
    pub verification_as_of: String,
    pub source_import: ContentBinding,
    pub sha_f_predecessor_ids: Vec<String>,
    pub off_branch_ids: Vec<String>,
    pub receipts: Vec<LegacyAdrDispositionReceipt>,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyAdrDispositionManifest {
    pub schema: String,
    pub kind: String,
    pub payload: LegacyAdrDispositionManifestPayload,
    pub manifest_digest: String,
    #[serde(flatten, default)]
    pub extensions: NamespacedExtensions,
}

impl LegacyAdrDispositionManifest {
    #[must_use]
    pub fn digest_preimage(
        &self,
    ) -> LegacyDispositionDigestPreimage<'_, LegacyAdrDispositionManifestPayload> {
        LegacyDispositionDigestPreimage {
            schema: &self.schema,
            kind: &self.kind,
            payload: &self.payload,
            extensions: &self.extensions,
        }
    }

    /// Structural coverage only. The application verifier must derive the
    /// authoritative inventories from Git and compare them with this payload.
    pub fn validate_structure(&self) -> Result<(), LegacyDispositionError> {
        validate_tag(
            "manifest",
            &self.schema,
            LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA,
            &self.kind,
            LEGACY_ADR_DISPOSITION_MANIFEST_KIND,
        )?;
        let payload = &self.payload;
        validate_adr_id(&payload.final_adr_id)?;
        validate_warrant(&payload.final_adr_id, &payload.warrant)?;
        validate_hex_identity(
            &payload.final_adr_id,
            "sha_f",
            &payload.sha_f,
            40,
            "40-character SHA-1 commit id",
        )?;
        if !is_canonical_utc(&payload.verification_as_of) {
            return Err(LegacyDispositionError::MalformedManifestVerificationTime {
                found: payload.verification_as_of.clone(),
            });
        }
        validate_content_binding(&payload.final_adr_id, &payload.source_import)?;
        if !is_sha256(&self.manifest_digest) {
            return Err(LegacyDispositionError::MalformedManifestDigest {
                found: self.manifest_digest.clone(),
            });
        }
        validate_inventory("sha_f_predecessor_ids", &payload.sha_f_predecessor_ids)?;
        validate_inventory("off_branch_ids", &payload.off_branch_ids)?;
        if payload
            .receipts
            .windows(2)
            .any(|pair| pair[0].payload.source.adr_id > pair[1].payload.source.adr_id)
        {
            return Err(LegacyDispositionError::ReceiptInventoryNotCanonical);
        }

        let base: BTreeSet<&str> = payload
            .sha_f_predecessor_ids
            .iter()
            .map(String::as_str)
            .collect();
        let off: BTreeSet<&str> = payload.off_branch_ids.iter().map(String::as_str).collect();
        if base.contains(payload.final_adr_id.as_str())
            || off.contains(payload.final_adr_id.as_str())
        {
            return Err(LegacyDispositionError::FinalAdrInInventory {
                adr_id: payload.final_adr_id.clone(),
            });
        }
        if let Some(overlap) = base.intersection(&off).next() {
            return Err(LegacyDispositionError::OverlappingInventory {
                adr_id: (*overlap).to_owned(),
            });
        }

        let mut actual = BTreeMap::<&str, &LegacyAdrDispositionReceipt>::new();
        let mut digests = BTreeSet::<&str>::new();
        let mut sources = BTreeSet::<&SourceBinding>::new();
        for receipt in &payload.receipts {
            receipt.validate_structure()?;
            let id = receipt.payload.source.adr_id.as_str();
            if !sources.insert(&receipt.payload.source) {
                return Err(LegacyDispositionError::DuplicateSourceBinding {
                    adr_id: id.to_owned(),
                });
            }
            if actual.insert(id, receipt).is_some() {
                return Err(LegacyDispositionError::DuplicateReceipt {
                    adr_id: id.to_owned(),
                });
            }
            if !digests.insert(&receipt.receipt_digest) {
                return Err(LegacyDispositionError::DuplicateReceiptDigest {
                    digest: receipt.receipt_digest.clone(),
                });
            }
            if receipt.payload.warrant != payload.warrant {
                return Err(LegacyDispositionError::WarrantBindingMismatch {
                    adr_id: id.to_owned(),
                });
            }
        }

        let expected: BTreeSet<&str> = base.union(&off).copied().collect();
        let found: BTreeSet<&str> = actual.keys().copied().collect();
        if expected != found {
            return Err(LegacyDispositionError::ReceiptCoverageMismatch {
                expected: expected.len(),
                found: found.len(),
                missing: expected
                    .difference(&found)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", "),
                unexpected: found
                    .difference(&expected)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }

        for id in base {
            let receipt = actual[id];
            if receipt.payload.source.commit_sha != payload.sha_f {
                return Err(LegacyDispositionError::FrozenCommitMismatch {
                    adr_id: id.to_owned(),
                    expected: payload.sha_f.clone(),
                    found: receipt.payload.source.commit_sha.clone(),
                });
            }
            match &receipt.payload.migration {
                ImportBinding::Migrated { artifact, .. } if artifact == &payload.source_import => {}
                _ => {
                    return Err(LegacyDispositionError::SourceImportMismatch {
                        adr_id: id.to_owned(),
                    });
                }
            }
        }
        for id in off {
            if actual[id].payload.source.commit_sha == payload.sha_f {
                return Err(LegacyDispositionError::OffBranchCommitMatchesFreeze {
                    adr_id: id.to_owned(),
                    sha_f: payload.sha_f.clone(),
                });
            }
        }
        Ok(())
    }
}

fn validate_tag(
    record: &'static str,
    schema: &str,
    expected_schema: &'static str,
    kind: &str,
    expected_kind: &'static str,
) -> Result<(), LegacyDispositionError> {
    if schema != expected_schema {
        return Err(LegacyDispositionError::SchemaMismatch {
            record,
            found: schema.to_owned(),
            expected: expected_schema,
        });
    }
    if kind != expected_kind {
        return Err(LegacyDispositionError::KindMismatch {
            record,
            found: kind.to_owned(),
            expected: expected_kind,
        });
    }
    Ok(())
}

fn validate_warrant(id: &str, warrant: &WarrantBinding) -> Result<(), LegacyDispositionError> {
    require_nonblank(id, "warrant_id", &warrant.warrant_id)?;
    if warrant.authorized_revision == 0 {
        return Err(LegacyDispositionError::InvalidAuthorizedRevision {
            warrant_id: warrant.warrant_id.clone(),
        });
    }
    validate_plain_sha256(id, "contract_digest", &warrant.contract_digest)?;
    require_nonblank(id, "authorization_ref", &warrant.authorization_ref)?;
    validate_digest(id, "authorization_sha256", &warrant.authorization_sha256)
}

fn validate_source(source: &SourceBinding) -> Result<(), LegacyDispositionError> {
    let id = &source.adr_id;
    validate_adr_id(id)?;
    require_nonblank(id, "repository_identity", &source.repository_identity)?;
    if !source.exact_ref.starts_with("refs/") || source.exact_ref.contains(char::is_whitespace) {
        return Err(LegacyDispositionError::MalformedIdentity {
            adr_id: id.clone(),
            field: "exact_ref",
            kind: "full Git ref",
            found: source.exact_ref.clone(),
        });
    }
    let hex_len = source.git_object_format.hex_len();
    validate_hex_identity(
        id,
        "ref_tip_at_review",
        &source.ref_tip_at_review,
        hex_len,
        "Git commit id",
    )?;
    validate_hex_identity(
        id,
        "commit_sha",
        &source.commit_sha,
        hex_len,
        "Git commit id",
    )?;
    validate_source_path(id, &source.path)?;
    validate_digest(id, "source_sha256", &source.source_sha256)
}

fn validate_import(id: &str, import: &ImportBinding) -> Result<(), LegacyDispositionError> {
    match import {
        ImportBinding::Migrated {
            artifact,
            imported_body_sha256,
            ..
        } => {
            validate_content_binding(id, artifact)?;
            validate_digest(id, "imported_body_sha256", imported_body_sha256)
        }
        ImportBinding::DeliberatelyExcluded {
            source_capsule,
            imported_body_status: ImportedBodyStatus::NotApplicable,
            ..
        } => validate_content_binding(id, source_capsule),
    }
}

fn validate_review(id: &str, review: &HumanReview) -> Result<(), LegacyDispositionError> {
    require_nonblank(id, "review_actor", &review.actor)?;
    require_nonblank(id, "role_assignment_ref", &review.role_assignment_ref)?;
    validate_digest(id, "role_assignment_sha256", &review.role_assignment_sha256)?;
    if !is_canonical_utc(&review.reviewed_at) {
        return Err(LegacyDispositionError::MalformedReviewTime {
            adr_id: id.to_owned(),
            found: review.reviewed_at.clone(),
        });
    }
    validate_content_binding(id, &review.response)?;
    require_nonblank(id, "review_judgment_id", &review.judgment_id)?;
    Ok(())
}

fn validate_outcome_support(
    id: &str,
    disposition: LegacyAdrDisposition,
    reason: &str,
    evidence: &[LegacyAdrSupport],
    successors: &[LegacyAdrSuccessor],
) -> Result<(), LegacyDispositionError> {
    require_nonblank(id, "reason", reason)?;
    validate_support(id, evidence)?;
    validate_successors(id, successors)?;
    if disposition == LegacyAdrDisposition::Superseded && successors.is_empty() {
        return Err(LegacyDispositionError::SuccessorRequired {
            adr_id: id.to_owned(),
        });
    }
    if evidence.is_empty() && successors.is_empty() {
        return Err(LegacyDispositionError::SupportRequired {
            adr_id: id.to_owned(),
            disposition,
        });
    }
    let qualifies = match disposition {
        LegacyAdrDisposition::Complete => evidence.iter().any(|support| {
            matches!(
                support,
                LegacyAdrSupport::ImplementationArtifact { .. }
                    | LegacyAdrSupport::RequiredGatePass { .. }
            )
        }),
        LegacyAdrDisposition::Falsified => evidence
            .iter()
            .any(|support| matches!(support, LegacyAdrSupport::FalsificationObservation { .. })),
        LegacyAdrDisposition::Superseded => !successors.is_empty(),
        LegacyAdrDisposition::Deprecated | LegacyAdrDisposition::Rejected => true,
    };
    if !qualifies {
        return Err(LegacyDispositionError::SupportRequired {
            adr_id: id.to_owned(),
            disposition,
        });
    }
    Ok(())
}

fn validate_support(id: &str, support: &[LegacyAdrSupport]) -> Result<(), LegacyDispositionError> {
    for item in support {
        match item {
            LegacyAdrSupport::ImplementationArtifact { artifact, .. }
            | LegacyAdrSupport::HistoricalContext { artifact, .. } => {
                validate_content_binding(id, artifact)?;
            }
            LegacyAdrSupport::RequiredGatePass {
                run,
                definition,
                binding,
                selection,
                stdout,
                stderr,
                fixtures,
                receipt,
                subject_source_sha256,
                ..
            } => {
                validate_content_binding(id, run)?;
                validate_content_binding(id, definition)?;
                validate_content_binding(id, binding)?;
                validate_content_binding(id, selection)?;
                validate_content_binding(id, stdout)?;
                validate_content_binding(id, stderr)?;
                for fixture in fixtures {
                    validate_content_binding(id, fixture)?;
                }
                if fixtures.windows(2).any(|pair| pair[0] >= pair[1]) {
                    return Err(LegacyDispositionError::ContentBindingsNotCanonical {
                        adr_id: id.to_owned(),
                        field: "gate_fixtures",
                    });
                }
                validate_content_binding(id, receipt)?;
                let references: BTreeSet<&str> =
                    [run, definition, binding, selection, stdout, stderr, receipt]
                        .into_iter()
                        .map(|artifact| artifact.reference.as_str())
                        .chain(fixtures.iter().map(|fixture| fixture.reference.as_str()))
                        .collect();
                if references.len() != 7 + fixtures.len() {
                    return Err(LegacyDispositionError::ContentBindingsNotCanonical {
                        adr_id: id.to_owned(),
                        field: "gate_bundle_references",
                    });
                }
                validate_digest(id, "gate_subject_source_sha256", subject_source_sha256)?;
            }
            LegacyAdrSupport::FalsificationObservation {
                record,
                subject_source_sha256,
                ..
            } => {
                validate_content_binding(id, record)?;
                validate_digest(
                    id,
                    "falsification_subject_source_sha256",
                    subject_source_sha256,
                )?;
            }
        }
    }
    if support.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(LegacyDispositionError::ContentBindingsNotCanonical {
            adr_id: id.to_owned(),
            field: "evidence",
        });
    }
    Ok(())
}

fn validate_successors(
    id: &str,
    successors: &[LegacyAdrSuccessor],
) -> Result<(), LegacyDispositionError> {
    let mut adr_ids = BTreeSet::new();
    let mut warrant_uuids = BTreeSet::new();
    for successor in successors {
        match successor {
            LegacyAdrSuccessor::Adr { source, .. } => {
                validate_source(source)?;
                if source.adr_id == id {
                    return Err(LegacyDispositionError::DuplicateSourceBinding {
                        adr_id: id.to_owned(),
                    });
                }
                if !adr_ids.insert(source.adr_id.as_str()) {
                    return Err(LegacyDispositionError::DuplicateSuccessorAdr {
                        adr_id: source.adr_id.clone(),
                    });
                }
            }
            LegacyAdrSuccessor::ResolvedWarrant {
                warrant_uuid,
                warrant,
                resolution,
                resolution_digest,
                ..
            } => {
                let uuid = warrant_uuid.as_uuid();
                if uuid.get_version_num() != 7 || uuid.get_variant() != uuid::Variant::RFC4122 {
                    return Err(LegacyDispositionError::SuccessorWarrantUuidNotV7 {
                        found: warrant_uuid.to_string(),
                    });
                }
                validate_warrant(id, warrant)?;
                validate_content_binding(id, resolution)?;
                validate_digest(id, "successor_resolution_digest", resolution_digest)?;
                if !warrant_uuids.insert(*warrant_uuid) {
                    return Err(LegacyDispositionError::DuplicateSuccessorWarrant {
                        warrant_uuid: warrant_uuid.to_string(),
                    });
                }
            }
        }
    }
    if successors.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(LegacyDispositionError::ContentBindingsNotCanonical {
            adr_id: id.to_owned(),
            field: "successors",
        });
    }
    Ok(())
}

fn validate_content_binding(
    id: &str,
    binding: &ContentBinding,
) -> Result<(), LegacyDispositionError> {
    require_nonblank(id, "content_reference", &binding.reference)?;
    validate_digest(id, "content_sha256", &binding.sha256)
}

fn validate_adr_id(id: &str) -> Result<(), LegacyDispositionError> {
    if id.len() == 4 && id.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(LegacyDispositionError::MalformedAdrId {
            found: id.to_owned(),
        })
    }
}

fn validate_source_path(id: &str, path: &str) -> Result<(), LegacyDispositionError> {
    let filename = path.rsplit('/').next().unwrap_or_default();
    let valid = !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
        && filename.starts_with(&format!("{id}-"))
        && filename.ends_with(".md");
    if valid {
        Ok(())
    } else {
        Err(LegacyDispositionError::SourcePathMismatch {
            adr_id: id.to_owned(),
            path: path.to_owned(),
        })
    }
}

fn require_nonblank(
    id: &str,
    field: &'static str,
    value: &str,
) -> Result<(), LegacyDispositionError> {
    if value.trim().is_empty() {
        Err(LegacyDispositionError::BlankField {
            adr_id: id.to_owned(),
            field,
        })
    } else {
        Ok(())
    }
}

fn validate_digest(
    id: &str,
    field: &'static str,
    value: &str,
) -> Result<(), LegacyDispositionError> {
    if is_sha256(value) {
        Ok(())
    } else {
        Err(LegacyDispositionError::MalformedIdentity {
            adr_id: id.to_owned(),
            field,
            kind: "sha256 digest",
            found: value.to_owned(),
        })
    }
}

fn validate_plain_sha256(
    id: &str,
    field: &'static str,
    value: &str,
) -> Result<(), LegacyDispositionError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(LegacyDispositionError::MalformedIdentity {
            adr_id: id.to_owned(),
            field,
            kind: "lowercase SHA-256 hex",
            found: value.to_owned(),
        })
    }
}

fn validate_hex_identity(
    id: &str,
    field: &'static str,
    value: &str,
    hex_len: usize,
    kind: &'static str,
) -> Result<(), LegacyDispositionError> {
    if value.len() == hex_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(LegacyDispositionError::MalformedIdentity {
            adr_id: id.to_owned(),
            field,
            kind,
            found: value.to_owned(),
        })
    }
}

fn is_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

/// Whether `value` is an exact Gregorian UTC second (`YYYY-MM-DDTHH:MM:SSZ`).
#[must_use]
pub fn is_canonical_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
    {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| {
        std::str::from_utf8(&bytes[range])
            .ok()
            .and_then(|part| part.parse::<u32>().ok())
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        parse(0..4),
        parse(5..7),
        parse(8..10),
        parse(11..13),
        parse(14..16),
        parse(17..19),
    ) else {
        return false;
    };
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day) && hour < 24 && minute < 60 && second < 60
}

fn validate_inventory(name: &'static str, ids: &[String]) -> Result<(), LegacyDispositionError> {
    for id in ids {
        validate_adr_id(id)?;
    }
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(LegacyDispositionError::IdentifierInventoryNotCanonical { inventory: name });
    }
    Ok(())
}
