// SPDX-License-Identifier: AGPL-3.0-or-later

//! Private Serde wire forms for exact legacy-disposition JSON admission.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::{
    ContentBinding, GitObjectFormat, HumanReview, ImportBinding, ImportedBodyStatus,
    LegacyAdrDisposition, LegacyAdrDispositionJudgment, LegacyAdrDispositionManifest,
    LegacyAdrDispositionManifestPayload, LegacyAdrDispositionProposal,
    LegacyAdrDispositionProposalPayload, LegacyAdrDispositionReceipt,
    LegacyAdrDispositionReceiptPayload, LegacyAdrDispositionReviewResponse,
    LegacyAdrFalsificationRecord, LegacyAdrSourceCapsule, LegacyAdrSuccessor, LegacyAdrSupport,
    LegacyExtensionValue, LegacyExtensionValueKind, LegacyJsonError, NamespacedExtensions,
    SourceBinding, WarrantBinding,
};
use crate::{JudgmentAuthority, WarUuid};

/// Seals semantic decoding behind the parent module's lexical admission pass.
pub(super) trait Sealed: Sized {
    fn decode(value: jstrict::Value) -> Result<Self, LegacyJsonError>;
}

fn decode_wire<W, T>(value: jstrict::Value) -> Result<T, LegacyJsonError>
where
    W: DeserializeOwned,
    T: From<W>,
{
    jstrict::from_value::<W>(value)
        .map(T::from)
        .map_err(|error| LegacyJsonError::TypedDecode(error.to_string()))
}

macro_rules! impl_sealed {
    ($semantic:ty => $wire:ty) => {
        impl Sealed for $semantic {
            fn decode(value: jstrict::Value) -> Result<Self, LegacyJsonError> {
                decode_wire::<$wire, Self>(value)
            }
        }
    };
}

#[derive(Debug)]
struct WireLegacyExtensionValue(LegacyExtensionValue);

#[derive(Debug, Default)]
struct WireNamespacedExtensions(NamespacedExtensions);

struct WireLegacyExtensionValueVisitor;

impl WireLegacyExtensionValueVisitor {
    fn number<E>(raw: &str) -> Result<LegacyExtensionValue, E>
    where
        E: de::Error,
    {
        super::canonical_extension_number(raw)
            .map(|value| LegacyExtensionValue(LegacyExtensionValueKind::Number(value)))
            .map_err(E::custom)
    }
}

impl<'de> Visitor<'de> for WireLegacyExtensionValueVisitor {
    type Value = LegacyExtensionValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a validated RFC 8785 JSON extension value")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(LegacyExtensionValue(LegacyExtensionValueKind::Null))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_unit()
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(LegacyExtensionValue(LegacyExtensionValueKind::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::number(&value.to_string())
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::number(&value.to_string())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::number(&value.to_string())
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::number(&value.to_string())
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        LegacyExtensionValue::try_from_f64(value).map_err(E::custom)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(LegacyExtensionValue(LegacyExtensionValueKind::String(
            value.to_owned(),
        )))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(LegacyExtensionValue(LegacyExtensionValueKind::String(
            value,
        )))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(WireLegacyExtensionValue(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(LegacyExtensionValue(LegacyExtensionValueKind::Array(
            values,
        )))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some((name, WireLegacyExtensionValue(value))) =
            map.next_entry::<String, WireLegacyExtensionValue>()?
        {
            if values.insert(name.clone(), value).is_some() {
                return Err(de::Error::custom(format_args!(
                    "duplicate key {name:?} in extension object"
                )));
            }
        }
        Ok(LegacyExtensionValue(LegacyExtensionValueKind::Object(
            values,
        )))
    }
}

impl<'de> Deserialize<'de> for WireLegacyExtensionValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(WireLegacyExtensionValueVisitor)
            .map(Self)
    }
}

struct WireNamespacedExtensionsVisitor;

impl<'de> Visitor<'de> for WireNamespacedExtensionsVisitor {
    type Value = NamespacedExtensions;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("optional extension fields with namespaced keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut extensions = BTreeMap::new();
        while let Some((name, WireLegacyExtensionValue(value))) =
            map.next_entry::<String, WireLegacyExtensionValue>()?
        {
            if !super::is_namespaced_extension_name(&name) {
                return Err(de::Error::custom(format_args!(
                    "unknown field {name:?}; optional extensions must be namespaced"
                )));
            }
            if extensions.insert(name.clone(), value).is_some() {
                return Err(de::Error::custom(format_args!(
                    "duplicate extension field {name:?}"
                )));
            }
        }
        Ok(NamespacedExtensions(extensions))
    }
}

impl<'de> Deserialize<'de> for WireNamespacedExtensions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_map(WireNamespacedExtensionsVisitor)
            .map(Self)
    }
}

pub(super) fn deserialize_namespaced_extensions<'de, D>(
    deserializer: D,
) -> Result<NamespacedExtensions, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(WireNamespacedExtensionsVisitor)
}

#[derive(Debug, Deserialize)]
struct WireContentBinding {
    reference: String,
    sha256: String,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireWarrantBinding {
    warrant_id: String,
    authorized_revision: u32,
    contract_digest: String,
    authorization_ref: String,
    authorization_sha256: String,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireSourceBinding {
    adr_id: String,
    repository_identity: String,
    git_object_format: GitObjectFormat,
    exact_ref: String,
    ref_tip_at_review: String,
    commit_sha: String,
    path: String,
    source_sha256: String,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLegacyAdrSourceCapsule {
    schema: String,
    source: WireSourceBinding,
    exact_source: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLegacyAdrFalsificationRecord {
    schema: String,
    subject_source_sha256: String,
    statement: String,
    observations: Vec<WireContentBinding>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum WireImportBinding {
    Migrated {
        artifact: WireContentBinding,
        imported_body_sha256: String,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
    DeliberatelyExcluded {
        source_capsule: WireContentBinding,
        imported_body_status: ImportedBodyStatus,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WireLegacyAdrSupport {
    ImplementationArtifact {
        artifact: WireContentBinding,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
    RequiredGatePass {
        run: WireContentBinding,
        definition: WireContentBinding,
        binding: WireContentBinding,
        selection: Box<WireContentBinding>,
        stdout: Box<WireContentBinding>,
        stderr: Box<WireContentBinding>,
        fixtures: Vec<WireContentBinding>,
        receipt: Box<WireContentBinding>,
        subject_source_sha256: String,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
    FalsificationObservation {
        record: WireContentBinding,
        subject_source_sha256: String,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
    HistoricalContext {
        artifact: WireContentBinding,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WireLegacyAdrSuccessor {
    Adr {
        source: WireSourceBinding,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
    ResolvedWarrant {
        warrant_uuid: WarUuid,
        warrant: WireWarrantBinding,
        resolution: WireContentBinding,
        resolution_digest: String,
        #[serde(flatten, default)]
        extensions: WireNamespacedExtensions,
    },
}

#[derive(Debug, Deserialize)]
struct WireHumanReview {
    actor: String,
    role_assignment_ref: String,
    role_assignment_sha256: String,
    reviewed_at: String,
    response: WireContentBinding,
    judgment_id: String,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionJudgment {
    id: String,
    adr_id: String,
    review_subject_digest: String,
    statement: String,
    actor: String,
    acting_role: String,
    meaning: String,
    authority: JudgmentAuthority,
    #[serde(default)]
    limitations: Vec<String>,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionReviewResponse {
    schema: String,
    kind: String,
    warrant: WireWarrantBinding,
    reviewed_at: String,
    judgments: Vec<WireLegacyAdrDispositionJudgment>,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionProposalPayload {
    warrant: WireWarrantBinding,
    source: WireSourceBinding,
    migration: WireImportBinding,
    proposed_disposition: LegacyAdrDisposition,
    reason: String,
    #[serde(default)]
    evidence: Vec<WireLegacyAdrSupport>,
    #[serde(default)]
    successors: Vec<WireLegacyAdrSuccessor>,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionProposal {
    schema: String,
    kind: String,
    payload: WireLegacyAdrDispositionProposalPayload,
    proposal_digest: String,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionReceiptPayload {
    warrant: WireWarrantBinding,
    source: WireSourceBinding,
    migration: WireImportBinding,
    review: WireHumanReview,
    disposition: LegacyAdrDisposition,
    reason: String,
    #[serde(default)]
    evidence: Vec<WireLegacyAdrSupport>,
    #[serde(default)]
    successors: Vec<WireLegacyAdrSuccessor>,
    #[serde(default)]
    proposal_digest: Option<String>,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionReceipt {
    schema: String,
    kind: String,
    payload: WireLegacyAdrDispositionReceiptPayload,
    receipt_digest: String,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionManifestPayload {
    final_adr_id: String,
    warrant: WireWarrantBinding,
    sha_f: String,
    verification_as_of: String,
    source_import: WireContentBinding,
    sha_f_predecessor_ids: Vec<String>,
    off_branch_ids: Vec<String>,
    receipts: Vec<WireLegacyAdrDispositionReceipt>,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

#[derive(Debug, Deserialize)]
struct WireLegacyAdrDispositionManifest {
    schema: String,
    kind: String,
    payload: WireLegacyAdrDispositionManifestPayload,
    manifest_digest: String,
    #[serde(flatten, default)]
    extensions: WireNamespacedExtensions,
}

impl From<WireLegacyExtensionValue> for LegacyExtensionValue {
    fn from(value: WireLegacyExtensionValue) -> Self {
        value.0
    }
}

impl From<WireNamespacedExtensions> for NamespacedExtensions {
    fn from(value: WireNamespacedExtensions) -> Self {
        value.0
    }
}

impl From<WireContentBinding> for ContentBinding {
    fn from(value: WireContentBinding) -> Self {
        Self {
            reference: value.reference,
            sha256: value.sha256,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireWarrantBinding> for WarrantBinding {
    fn from(value: WireWarrantBinding) -> Self {
        Self {
            warrant_id: value.warrant_id,
            authorized_revision: value.authorized_revision,
            contract_digest: value.contract_digest,
            authorization_ref: value.authorization_ref,
            authorization_sha256: value.authorization_sha256,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireSourceBinding> for SourceBinding {
    fn from(value: WireSourceBinding) -> Self {
        Self {
            adr_id: value.adr_id,
            repository_identity: value.repository_identity,
            git_object_format: value.git_object_format,
            exact_ref: value.exact_ref,
            ref_tip_at_review: value.ref_tip_at_review,
            commit_sha: value.commit_sha,
            path: value.path,
            source_sha256: value.source_sha256,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrSourceCapsule> for LegacyAdrSourceCapsule {
    fn from(value: WireLegacyAdrSourceCapsule) -> Self {
        Self {
            schema: value.schema,
            source: value.source.into(),
            exact_source: value.exact_source,
        }
    }
}

impl From<WireLegacyAdrFalsificationRecord> for LegacyAdrFalsificationRecord {
    fn from(value: WireLegacyAdrFalsificationRecord) -> Self {
        Self {
            schema: value.schema,
            subject_source_sha256: value.subject_source_sha256,
            statement: value.statement,
            observations: value.observations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<WireImportBinding> for ImportBinding {
    fn from(value: WireImportBinding) -> Self {
        match value {
            WireImportBinding::Migrated {
                artifact,
                imported_body_sha256,
                extensions,
            } => Self::Migrated {
                artifact: artifact.into(),
                imported_body_sha256,
                extensions: extensions.0,
            },
            WireImportBinding::DeliberatelyExcluded {
                source_capsule,
                imported_body_status,
                extensions,
            } => Self::DeliberatelyExcluded {
                source_capsule: source_capsule.into(),
                imported_body_status,
                extensions: extensions.0,
            },
        }
    }
}

impl From<WireLegacyAdrSupport> for LegacyAdrSupport {
    fn from(value: WireLegacyAdrSupport) -> Self {
        match value {
            WireLegacyAdrSupport::ImplementationArtifact {
                artifact,
                extensions,
            } => Self::ImplementationArtifact {
                artifact: artifact.into(),
                extensions: extensions.0,
            },
            WireLegacyAdrSupport::RequiredGatePass {
                run,
                definition,
                binding,
                selection,
                stdout,
                stderr,
                fixtures,
                receipt,
                subject_source_sha256,
                extensions,
            } => Self::RequiredGatePass {
                run: run.into(),
                definition: definition.into(),
                binding: binding.into(),
                selection: Box::new((*selection).into()),
                stdout: Box::new((*stdout).into()),
                stderr: Box::new((*stderr).into()),
                fixtures: fixtures.into_iter().map(Into::into).collect(),
                receipt: Box::new((*receipt).into()),
                subject_source_sha256,
                extensions: extensions.0,
            },
            WireLegacyAdrSupport::FalsificationObservation {
                record,
                subject_source_sha256,
                extensions,
            } => Self::FalsificationObservation {
                record: record.into(),
                subject_source_sha256,
                extensions: extensions.0,
            },
            WireLegacyAdrSupport::HistoricalContext {
                artifact,
                extensions,
            } => Self::HistoricalContext {
                artifact: artifact.into(),
                extensions: extensions.0,
            },
        }
    }
}

impl From<WireLegacyAdrSuccessor> for LegacyAdrSuccessor {
    fn from(value: WireLegacyAdrSuccessor) -> Self {
        match value {
            WireLegacyAdrSuccessor::Adr { source, extensions } => Self::Adr {
                source: source.into(),
                extensions: extensions.0,
            },
            WireLegacyAdrSuccessor::ResolvedWarrant {
                warrant_uuid,
                warrant,
                resolution,
                resolution_digest,
                extensions,
            } => Self::ResolvedWarrant {
                warrant_uuid,
                warrant: warrant.into(),
                resolution: resolution.into(),
                resolution_digest,
                extensions: extensions.0,
            },
        }
    }
}

impl From<WireHumanReview> for HumanReview {
    fn from(value: WireHumanReview) -> Self {
        Self {
            actor: value.actor,
            role_assignment_ref: value.role_assignment_ref,
            role_assignment_sha256: value.role_assignment_sha256,
            reviewed_at: value.reviewed_at,
            response: value.response.into(),
            judgment_id: value.judgment_id,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionJudgment> for LegacyAdrDispositionJudgment {
    fn from(value: WireLegacyAdrDispositionJudgment) -> Self {
        Self {
            id: value.id,
            adr_id: value.adr_id,
            review_subject_digest: value.review_subject_digest,
            statement: value.statement,
            actor: value.actor,
            acting_role: value.acting_role,
            meaning: value.meaning,
            authority: value.authority,
            limitations: value.limitations,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionReviewResponse> for LegacyAdrDispositionReviewResponse {
    fn from(value: WireLegacyAdrDispositionReviewResponse) -> Self {
        Self {
            schema: value.schema,
            kind: value.kind,
            warrant: value.warrant.into(),
            reviewed_at: value.reviewed_at,
            judgments: value.judgments.into_iter().map(Into::into).collect(),
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionProposalPayload> for LegacyAdrDispositionProposalPayload {
    fn from(value: WireLegacyAdrDispositionProposalPayload) -> Self {
        Self {
            warrant: value.warrant.into(),
            source: value.source.into(),
            migration: value.migration.into(),
            proposed_disposition: value.proposed_disposition,
            reason: value.reason,
            evidence: value.evidence.into_iter().map(Into::into).collect(),
            successors: value.successors.into_iter().map(Into::into).collect(),
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionProposal> for LegacyAdrDispositionProposal {
    fn from(value: WireLegacyAdrDispositionProposal) -> Self {
        Self {
            schema: value.schema,
            kind: value.kind,
            payload: value.payload.into(),
            proposal_digest: value.proposal_digest,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionReceiptPayload> for LegacyAdrDispositionReceiptPayload {
    fn from(value: WireLegacyAdrDispositionReceiptPayload) -> Self {
        Self {
            warrant: value.warrant.into(),
            source: value.source.into(),
            migration: value.migration.into(),
            review: value.review.into(),
            disposition: value.disposition,
            reason: value.reason,
            evidence: value.evidence.into_iter().map(Into::into).collect(),
            successors: value.successors.into_iter().map(Into::into).collect(),
            proposal_digest: value.proposal_digest,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionReceipt> for LegacyAdrDispositionReceipt {
    fn from(value: WireLegacyAdrDispositionReceipt) -> Self {
        Self {
            schema: value.schema,
            kind: value.kind,
            payload: value.payload.into(),
            receipt_digest: value.receipt_digest,
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionManifestPayload> for LegacyAdrDispositionManifestPayload {
    fn from(value: WireLegacyAdrDispositionManifestPayload) -> Self {
        Self {
            final_adr_id: value.final_adr_id,
            warrant: value.warrant.into(),
            sha_f: value.sha_f,
            verification_as_of: value.verification_as_of,
            source_import: value.source_import.into(),
            sha_f_predecessor_ids: value.sha_f_predecessor_ids,
            off_branch_ids: value.off_branch_ids,
            receipts: value.receipts.into_iter().map(Into::into).collect(),
            extensions: value.extensions.0,
        }
    }
}

impl From<WireLegacyAdrDispositionManifest> for LegacyAdrDispositionManifest {
    fn from(value: WireLegacyAdrDispositionManifest) -> Self {
        Self {
            schema: value.schema,
            kind: value.kind,
            payload: value.payload.into(),
            manifest_digest: value.manifest_digest,
            extensions: value.extensions.0,
        }
    }
}

impl_sealed!(LegacyExtensionValue => WireLegacyExtensionValue);
impl_sealed!(NamespacedExtensions => WireNamespacedExtensions);
impl_sealed!(ContentBinding => WireContentBinding);
impl_sealed!(WarrantBinding => WireWarrantBinding);
impl_sealed!(SourceBinding => WireSourceBinding);
impl_sealed!(LegacyAdrSourceCapsule => WireLegacyAdrSourceCapsule);
impl_sealed!(LegacyAdrFalsificationRecord => WireLegacyAdrFalsificationRecord);
impl_sealed!(ImportBinding => WireImportBinding);
impl_sealed!(LegacyAdrSupport => WireLegacyAdrSupport);
impl_sealed!(LegacyAdrSuccessor => WireLegacyAdrSuccessor);
impl_sealed!(HumanReview => WireHumanReview);
impl_sealed!(LegacyAdrDispositionJudgment => WireLegacyAdrDispositionJudgment);
impl_sealed!(LegacyAdrDispositionReviewResponse => WireLegacyAdrDispositionReviewResponse);
impl_sealed!(LegacyAdrDispositionProposalPayload => WireLegacyAdrDispositionProposalPayload);
impl_sealed!(LegacyAdrDispositionProposal => WireLegacyAdrDispositionProposal);
impl_sealed!(LegacyAdrDispositionReceiptPayload => WireLegacyAdrDispositionReceiptPayload);
impl_sealed!(LegacyAdrDispositionReceipt => WireLegacyAdrDispositionReceipt);
impl_sealed!(LegacyAdrDispositionManifestPayload => WireLegacyAdrDispositionManifestPayload);
impl_sealed!(LegacyAdrDispositionManifest => WireLegacyAdrDispositionManifest);
