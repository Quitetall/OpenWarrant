// SPDX-License-Identifier: Apache-2.0
//! Portable transport for an existing legacy Dispatch. This module verifies
//! captured bytes and explicit capability membership, not semantic selection,
//! signer authority, readiness, or a harness sandbox.
use crate::{DigestDomain, WarIr, sha256_digest, sha256_hex};
use openwarrant_core::{context::ContextManifest, execution::StageDispatch};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA: &str = "oh.war/dispatch-bundle/1";
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SOURCES: usize = 4096;
const SUBMISSION: &[u8] = include_bytes!("../../../schemas/oh.war/stage-submission/v1.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub policy_ref: String,
    pub allow: Vec<String>,
}
impl Policy {
    pub fn deny_all() -> Self {
        Self {
            schema: "oh.war/dispatch-capabilities/1".into(),
            policy_ref: "policy://none-declared".into(),
            allow: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub schema: String,
    pub dispatch: StageDispatch,
    pub context: ContextManifest,
    /// Captured canonical compilation basis. Optional unselected source bytes
    /// are not copied merely to reproduce their already bound hashes.
    pub contract: WarIr,
    /// Context IDs, manifest/basis support IDs, and explicit artifact references
    /// resolve only in this map. The reader never fetches or opens these as paths.
    pub sources: BTreeMap<String, Vec<u8>>,
    pub policy: Vec<u8>,
}
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub String);
type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error {
    Error(message.into())
}
fn convert(e: impl std::fmt::Display) -> Error {
    invalid(e.to_string())
}
fn require(ok: bool, message: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(invalid(message)) }
}
pub fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}
pub fn submission_schema() -> &'static [u8] {
    SUBMISSION
}
impl Bundle {
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_jcs::to_vec(self).map_err(convert)?;
        require(bytes.len() <= MAX_BYTES, "bundle-resource-limit")?;
        Ok(bytes)
    }
    fn source(&self, id: &str) -> Result<&[u8]> {
        self.sources
            .get(id)
            .map(Vec::as_slice)
            .ok_or_else(|| invalid(format!("bundle-missing-source: {id}")))
    }
    fn bound_source(&self, id: &str, digest: &str) -> Result<()> {
        require(
            content_digest(self.source(id)?) == digest,
            &format!("bundle-source-digest: {id}"),
        )
    }
    pub fn validate(&self) -> Result<()> {
        // This transport accepts the captured compiler basis, not populated
        // runtime/assurance/extensions trees. Refuse borrowed unsupported JSON
        // before any recursive serialization or cloning (including encode).
        require(
            [
                &self.contract.execution,
                &self.contract.assurance_case,
                &self.contract.resolution,
                &self.contract.extensions,
            ]
            .iter()
            .all(|v| v.is_none()),
            "bundle-unsupported-ir-section",
        )?;
        let mut budget = ByteBudget(MAX_BYTES);
        serde_json::to_writer(&mut budget, self).map_err(|_| invalid("bundle-resource-limit"))?;
        require(self.schema == SCHEMA, "bundle-schema")?;
        require(self.sources.len() <= MAX_SOURCES, "bundle-resource-limit")?;
        let size = self
            .sources
            .values()
            .try_fold(0usize, |n, b| n.checked_add(b.len()))
            .ok_or_else(|| invalid("bundle-resource-limit"))?;
        require(size <= MAX_BYTES, "bundle-resource-limit")?;
        let d = &self.dispatch;
        let ir = &self.contract;
        require(
            ir.api_version == crate::API_VERSION && ir.kind == crate::KIND,
            "bundle-ir-identity",
        )?;
        require(
            ir.integrity.algorithm == "sha256"
                && ir.integrity.workspace_basis_digest == d.workspace_basis_digest,
            "bundle-ir-integrity",
        )?;
        let mut composition = serde_json::json!({"atoms":ir.source_and_composition.atoms});
        if let Some(scope) = &ir.source_and_composition.scope {
            composition["scope"] = serde_json::to_value(scope).map_err(convert)?;
        }
        require(
            sha256_digest(DigestDomain::CompositionRevision, &composition).map_err(convert)?
                == ir.integrity.composition_revision_digest,
            "bundle-composition-digest",
        )?;
        for reserved in [
            &d.workspace_basis_ref,
            &d.context_manifest_ref,
            &d.warrant_ref,
            &d.capability_authorization.policy_ref,
        ] {
            require(
                !self.sources.contains_key(reserved),
                "bundle-reserved-reference-collision",
            )?;
        }
        let required: Vec<String> = ir
            .source_and_composition
            .atoms
            .iter()
            .filter(|a| a.required)
            .map(|a| a.source.clone())
            .collect();
        d.validate(&required).map_err(convert)?;
        self.context.validate().map_err(convert)?;
        let mut unsigned = d.clone();
        unsigned.dispatch_digest.clear();
        require(
            sha256_digest(DigestDomain::Dispatch, &unsigned).map_err(convert)? == d.dispatch_digest,
            "bundle-dispatch-digest",
        )?;
        require(
            ir.contract_digest().map_err(convert)? == d.contract_digest,
            "bundle-contract-digest",
        )?;
        require(
            d.warrant_ref == format!("war://{}", ir.identity.uuid)
                && d.contract_revision == ir.contract_revision,
            "bundle-contract-identity",
        )?;
        let basis = serde_json::json!({"format_basis":ir.format_basis,"source_and_composition":ir.source_and_composition});
        require(
            sha256_digest(DigestDomain::WorkspaceBasis, &basis).map_err(convert)?
                == d.workspace_basis_digest,
            "bundle-basis-digest",
        )?;
        require(
            self.context.workspace_basis_digest == d.workspace_basis_digest
                && self.context.workspace_basis_ref == d.workspace_basis_ref,
            "bundle-context-basis",
        )?;
        let context_digest =
            sha256_digest(DigestDomain::ContextManifest, &self.context).map_err(convert)?;
        require(
            context_digest == d.context_manifest_digest,
            "bundle-context-digest",
        )?;
        require(
            d.context_manifest_ref
                == format!("artifact://context-manifest/sha256:{context_digest}"),
            "bundle-context-reference",
        )?;
        require(
            d.workspace_basis_ref
                == format!("basis://{}", ir.source_and_composition.manifest_source),
            "bundle-basis-reference",
        )?;
        self.bound_source(
            &ir.source_and_composition.manifest_source,
            &format!("sha256:{}", ir.source_and_composition.manifest_digest),
        )?;
        if let Some(scope) = &ir.source_and_composition.scope {
            self.bound_source(
                &scope.source,
                &format!("sha256:{}", scope.scope_source_digest),
            )?;
        }
        if let Some(digest) = &ir.format_basis.sas_digest {
            self.bound_source("sas://captured", digest)?;
        }
        require(
            d.submission_schema_ref == "schema://oh.war/stage-submission/v1",
            "bundle-submission-reference",
        )?;
        require(
            self.source(&d.submission_schema_ref)? == SUBMISSION,
            "bundle-submission-schema",
        )?;
        let mut seen = std::collections::BTreeSet::new();
        for item in &self.context.included {
            require(seen.insert(&item.id), "bundle-duplicate-context")?;
            require(
                !self.context.omitted.iter().any(|o| o.id == item.id),
                "bundle-contradictory-context",
            )?;
            self.bound_source(&item.id, &item.content_digest)?;
            if let Some(atom) = ir
                .source_and_composition
                .atoms
                .iter()
                .find(|a| item.id == a.source || item.id.starts_with(&format!("{}#", a.source)))
            {
                let parent = ir
                    .source_and_composition
                    .manifest_source
                    .rsplit_once('/')
                    .map(|(p, _)| p)
                    .unwrap_or("");
                let path = if parent.is_empty() {
                    atom.source.clone()
                } else {
                    format!("{parent}/{}", atom.source)
                };
                require(item.holder.path == path, "bundle-source-holder-mismatch")?;
                if item.id == atom.source {
                    self.bound_source(&item.id, &format!("sha256:{}", atom.atom_source_digest))?;
                } else {
                    let source = format!("provenance://{}", item.holder.path);
                    self.bound_source(&source, &format!("sha256:{}", atom.atom_source_digest))?;
                    let full = std::str::from_utf8(self.source(&source)?).map_err(convert)?;
                    let heading = &item.id[atom.source.len() + 1..];
                    let body = openwarrant_core::sections::find(full, heading)
                        .ok_or_else(|| invalid("bundle-section-missing"))?;
                    require(
                        body.as_bytes() == self.source(&item.id)?,
                        "bundle-section-mismatch",
                    )?;
                }
            }
        }
        for atom in ir
            .source_and_composition
            .atoms
            .iter()
            .filter(|a| a.required)
        {
            require(
                self.context
                    .included
                    .iter()
                    .any(|i| i.id == atom.source && i.required),
                "bundle-required-context-omitted",
            )?;
            self.bound_source(&atom.source, &format!("sha256:{}", atom.atom_source_digest))?;
        }
        for id in d
            .input_artifacts
            .iter()
            .chain(&d.prior_failure_evidence_refs)
        {
            self.source(id)?;
        }
        let policy: Policy = serde_json::from_slice(&self.policy).map_err(convert)?;
        require(
            policy.schema == "oh.war/dispatch-capabilities/1",
            "bundle-policy-schema",
        )?;
        require(
            policy.policy_ref == d.capability_authorization.policy_ref,
            "bundle-policy-reference",
        )?;
        let mut capabilities = std::collections::BTreeSet::new();
        require(
            policy.allow.iter().all(|s| {
                !s.is_empty() && s.trim() == s && !s.contains('*') && capabilities.insert(s)
            }),
            "bundle-policy-capabilities",
        )?;
        if d.capability_authorization.policy_ref == "policy://none-declared"
            && d.capability_authorization.digest.is_empty()
        {
            require(
                policy.allow.is_empty(),
                "bundle-undeclared-policy-must-deny",
            )?;
        } else {
            require(
                content_digest(&self.policy) == d.capability_authorization.digest,
                "bundle-policy-digest",
            )?;
        }
        Ok(())
    }
}
/// A checked package is anchored to an independently supplied package digest.
/// It does not authenticate the source of that digest or issue authority.
pub struct CheckedBundle {
    bundle: Bundle,
}
pub fn check(bytes: &[u8], expected_digest: &str) -> Result<CheckedBundle> {
    require(bytes.len() <= MAX_BYTES, "bundle-resource-limit")?;
    require(
        content_digest(bytes) == expected_digest,
        "bundle-root-digest",
    )?;
    let bundle: Bundle = serde_json::from_slice(bytes).map_err(convert)?;
    bundle.validate()?;
    require(
        serde_jcs::to_vec(&bundle).map_err(convert)? == bytes,
        "bundle-noncanonical-or-duplicate-fields",
    )?;
    Ok(CheckedBundle { bundle })
}
impl CheckedBundle {
    pub fn dispatch(&self) -> &StageDispatch {
        &self.bundle.dispatch
    }
    pub fn read(&self, id: &str) -> Result<std::borrow::Cow<'_, [u8]>> {
        use std::borrow::Cow;
        let d = &self.bundle.dispatch;
        let value = if id == d.workspace_basis_ref {
            Some(
                serde_json::json!({"format_basis":self.bundle.contract.format_basis,
                "source_and_composition":self.bundle.contract.source_and_composition}),
            )
        } else if id == d.context_manifest_ref {
            Some(serde_json::to_value(&self.bundle.context).map_err(convert)?)
        } else if id == d.warrant_ref {
            Some(serde_json::to_value(&self.bundle.contract).map_err(convert)?)
        } else if id == d.capability_authorization.policy_ref {
            return Ok(Cow::Borrowed(&self.bundle.policy));
        } else {
            None
        };
        match value {
            Some(value) => Ok(Cow::Owned(serde_jcs::to_vec(&value).map_err(convert)?)),
            None => self.bundle.source(id).map(Cow::Borrowed),
        }
    }
    /// Exact membership is a pre-action decision for a caller's trusted policy.
    /// This does not execute or sandbox anything. A caller must establish policy
    /// authority independently, then enforce this decision before using a tool.
    pub fn require_capability(
        &self,
        requested: &str,
        trusted_policy_digest: Option<&str>,
    ) -> Result<()> {
        let policy: Policy = serde_json::from_slice(&self.bundle.policy).map_err(convert)?;
        require(
            policy.allow.iter().any(|s| s == requested),
            &format!("capability-denied: {requested}"),
        )?;
        require(
            trusted_policy_digest == Some(content_digest(&self.bundle.policy).as_str()),
            "capability-policy-not-trusted",
        )?;
        Ok(())
    }
}

// Count escaped JSON output without allocating it. Typed metadata has fixed
// depth after the unsupported IR sections are rejected above.
struct ByteBudget(usize);
impl std::io::Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.0 {
            return Err(std::io::Error::other("bundle-resource-limit"));
        }
        self.0 -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
