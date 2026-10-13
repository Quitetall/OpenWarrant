// SPDX-License-Identifier: Apache-2.0
//! Candidate signed capture attribution, not permission or execution assurance.
//! The host supplies trusted current authority, authenticated enrollment, checked
//! recorded binding, actual payload bytes and a real signature adapter. This
//! authenticates a collector's claim over bytes; it does not establish that the
//! claim is true, that a process ran, human presence, spend or confinement.
use super::{AuthenticatedEnrollment, SignatureCheck, Use};
use crate::{authority_transition::Revision, document::runtime::RuntimeBinding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCHEMA: &str = "oh.war/runtime-capture-attribution/v1-draft.1";
pub const NAMESPACE: &str = "openwarrant-runtime-capture-v1";
const MAX_WIRE: usize = 65_536;
const MAX_PAYLOAD: usize = 6 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    #[error("capture attribution rejected: {0}")]
    Rejected(&'static str),
    #[error("capture attribution unavailable: {0}")]
    Unavailable(&'static str),
}
impl From<super::Fault> for Fault {
    fn from(value: super::Fault) -> Self {
        match value {
            super::Fault::Rejected(reason) => Self::Rejected(reason),
            super::Fault::Unavailable(reason) => Self::Unavailable(reason),
        }
    }
}
/// Explicit host observations, not values decoded from the attribution itself.
pub struct Context<'a> {
    pub authority: &'a Revision,
    pub enrollment: &'a AuthenticatedEnrollment,
    pub usage: &'a Use<'a>,
    pub binding: &'a RuntimeBinding,
    pub payload: &'a [u8],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proof {
    pub schema: String,
    pub collector: String,
    pub authority_digest: String,
    pub enrollment_digest: String,
    pub warrant_ref: String,
    pub contract_digest: String,
    pub dispatch_digest: String,
    pub stage_id: String,
    pub attempt_id: String,
    /// Local content identity only, never a replacement provider receipt seal.
    pub payload_digest: String,
}
fn canonical(value: &impl Serialize) -> Result<Vec<u8>, Fault> {
    let bytes = serde_jcs::to_vec(value).map_err(|_| Fault::Rejected("canonical encoding"))?;
    if bytes.len() > MAX_WIRE {
        return Err(Fault::Rejected("attribution wire limit"));
    }
    Ok(bytes)
}
impl Proof {
    /// Prepare exact bytes for the collector's signer; this signs nothing.
    pub fn for_context(context: &Context<'_>) -> Result<Self, Fault> {
        context
            .enrollment
            .allows(context.authority, context.usage)?;
        let binding = context.binding;
        if context.payload.is_empty() || context.payload.len() > MAX_PAYLOAD {
            return Err(Fault::Rejected("capture payload limit"));
        }
        if binding.warrant_ref != format!("war://{}", context.usage.warrant)
            || [
                &binding.contract_digest,
                &binding.dispatch_digest,
                &binding.stage_id,
                &binding.attempt_id,
            ]
            .iter()
            .any(|value| {
                value.is_empty() || value.len() > 256 || value.chars().any(char::is_control)
            })
        {
            return Err(Fault::Rejected(
                "incomplete or contradictory recorded binding",
            ));
        }
        Ok(Self {
            schema: SCHEMA.into(),
            collector: context.usage.collector.into(),
            authority_digest: context
                .authority
                .digest()
                .map_err(|_| Fault::Unavailable("current authority digest"))?,
            enrollment_digest: context.enrollment.enrollment().digest()?,
            warrant_ref: binding.warrant_ref.clone(),
            contract_digest: binding.contract_digest.clone(),
            dispatch_digest: binding.dispatch_digest.clone(),
            stage_id: binding.stage_id.clone(),
            attempt_id: binding.attempt_id.clone(),
            payload_digest: format!(
                "sha256:{}",
                Sha256::digest(context.payload)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        if self.schema != SCHEMA {
            return Err(Fault::Unavailable("unsupported attribution schema"));
        }
        canonical(self)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedAttribution {
    pub proof: Proof,
    pub signature: String,
}
/// No public constructor or wire decoder. Does not confer execution eligibility.
#[derive(Debug)]
pub struct AuthenticatedAttribution {
    proof: Proof,
}
impl AuthenticatedAttribution {
    pub fn proof(&self) -> &Proof {
        &self.proof
    }
}
impl SignedAttribution {
    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        self.proof.encode()?;
        if self.signature.is_empty() || self.signature.len() > 16_384 {
            return Err(Fault::Rejected("signature bounds"));
        }
        canonical(self)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Fault> {
        if bytes.len() > MAX_WIRE {
            return Err(Fault::Rejected("attribution wire limit"));
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| Fault::Rejected("attribution syntax"))?;
        if value["proof"]["schema"]
            .as_str()
            .is_some_and(|schema| schema != SCHEMA)
        {
            return Err(Fault::Unavailable("unsupported attribution schema"));
        }
        let decoded: Self =
            serde_json::from_slice(bytes).map_err(|_| Fault::Rejected("attribution shape"))?;
        if decoded.encode()? != bytes {
            return Err(Fault::Rejected("exact canonical attribution required"));
        }
        Ok(decoded)
    }
    pub fn authenticate(
        &self,
        context: &Context<'_>,
        verifier: &dyn SignatureCheck,
    ) -> Result<AuthenticatedAttribution, Fault> {
        self.encode()?;
        let expected = Proof::for_context(context)?;
        if self.proof != expected {
            return Err(Fault::Rejected(
                "attribution does not match current captured subject",
            ));
        }
        let principal = context
            .authority
            .principals
            .get(&self.proof.collector)
            .ok_or(Fault::Rejected("current collector principal missing"))?;
        let evidence = verifier.verify(
            &principal.public_key,
            NAMESPACE,
            &self.proof.encode()?,
            &self.signature,
        )?;
        if !evidence.valid {
            return Err(Fault::Rejected("collector signature invalid"));
        }
        // Presence is not inferred here: enrollment admission applies its own
        // human policy, and a collector may sign its own attributed report.
        Ok(AuthenticatedAttribution { proof: expected })
    }
}
