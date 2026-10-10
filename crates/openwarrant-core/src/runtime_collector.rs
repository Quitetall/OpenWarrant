// SPDX-License-Identifier: Apache-2.0
//! Candidate collector enrollment contract (OW-WAR-0149), not assurance.
//!
//! The host supplies an authenticated, rollback-protected current authority and
//! a trusted signature adapter. This pure SDK checks their exact relationships.
//! It does not authenticate that authority, protect a file/process, observe an
//! executable, activate configuration, or establish human review. A caller must
//! not satisfy `SignatureCheck` by decoding assertions from the signed input.
use crate::{authority_transition::Revision, contract::ActorKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "oh.war/runtime-collector-enrollment/v1-draft.1";
pub const NAMESPACE: &str = "openwarrant-runtime-collector-v1";
const DOMAIN: &[u8] = b"openwarrant-runtime-collector-enrollment-v1\0";
const MAX_BYTES: usize = 65_536;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub kind: String,
    pub identity: String,
    pub version: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enrollment {
    pub schema: String,
    pub repository: String,
    pub authority_digest: String,
    pub provider: Provider,
    /// Exact binary bytes, observed and protected by the host; never a PATH label.
    pub verifier_digest: String,
    /// A principal holding `runtime-collector` in the same current authority.
    pub collector: String,
    /// Explicit canonical Warrant UUIDs. There is no implicit wildcard grant.
    pub warrants: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signed {
    pub enrollment: Enrollment,
    /// Principal -> signature over exact canonical enrollment bytes/namespace.
    pub signatures: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    #[error("collector enrollment rejected: {0}")]
    Rejected(&'static str),
    #[error("collector enrollment unavailable: {0}")]
    Unavailable(&'static str),
}
#[derive(Clone, Debug)]
pub struct SignatureEvidence {
    pub valid: bool,
    /// Authenticator evidence, not an inference from a key name or TTY.
    pub user_present: Option<bool>,
}
pub trait SignatureCheck {
    fn verify(
        &self,
        public_key: &str,
        namespace: &str,
        payload: &[u8],
        signature: &str,
    ) -> Result<SignatureEvidence, Fault>;
}
/// Cannot be decoded or constructed from a caller's claimed verdict.
/// Authentication here means the supplied trusted adapter checked the signature.
/// It is neither a human-review record nor a statement of runtime enforcement.
#[derive(Debug)]
pub struct AuthenticatedEnrollment {
    enrollment: Enrollment,
}
pub struct Use<'a> {
    pub repository: &'a str,
    pub warrant: &'a str,
    pub provider: &'a Provider,
    pub verifier_digest: &'a str,
    pub collector: &'a str,
}
fn text(s: &str, limit: usize) -> bool {
    !s.is_empty() && s.len() <= limit && s.bytes().all(|b| b.is_ascii_graphic())
}
fn sha256(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn canonical(value: &impl Serialize) -> Result<Vec<u8>, Fault> {
    let bytes = serde_jcs::to_vec(value).map_err(|_| Fault::Rejected("canonical JSON"))?;
    if bytes.len() > MAX_BYTES {
        return Err(Fault::Rejected("wire budget"));
    }
    Ok(bytes)
}
impl Enrollment {
    fn validate(&self) -> Result<(), Fault> {
        if self.schema != SCHEMA
            || !text(&self.repository, 128)
            || !sha256(&self.authority_digest)
            || !sha256(&self.verifier_digest)
            || !text(&self.collector, 64)
            || !matches!(self.provider.kind.as_str(), "katana" | "blut")
            || !text(&self.provider.identity, 256)
            || !text(&self.provider.version, 128)
            || self.warrants.is_empty()
            || self.warrants.len() > 64
            || self
                .warrants
                .iter()
                .any(|w| uuid::Uuid::parse_str(w).map_or(true, |u| u.to_string() != *w))
        {
            return Err(Fault::Rejected("schema or bounded configuration"));
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        self.validate()?;
        canonical(self)
    }
    pub fn digest(&self) -> Result<String, Fault> {
        let mut hash = Sha256::new();
        hash.update(DOMAIN);
        hash.update(self.encode()?);
        let hex = hash
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        Ok(format!("sha256:{hex}"))
    }
    fn current(&self, authority: &Revision, repository: &str) -> Result<(), Fault> {
        self.validate()?;
        authority
            .validate()
            .map_err(|_| Fault::Rejected("invalid current authority"))?;
        if authority.version() != 2 {
            return Err(Fault::Unavailable(
                "current authority does not establish actor kind",
            ));
        }
        if self.repository != repository
            || authority.repository != repository
            || authority
                .digest()
                .map_err(|_| Fault::Rejected("authority digest"))?
                != self.authority_digest
        {
            return Err(Fault::Rejected("wrong repository or stale authority"));
        }
        if !authority
            .principals
            .get(&self.collector)
            .is_some_and(|p| p.roles.contains("runtime-collector"))
        {
            return Err(Fault::Rejected("collector has no current role"));
        }
        Ok(())
    }
}
impl Signed {
    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        self.enrollment.validate()?;
        if self.signatures.is_empty()
            || self.signatures.len() > 32
            || self
                .signatures
                .iter()
                .any(|(p, s)| !text(p, 64) || s.is_empty() || s.len() > 16_384)
        {
            return Err(Fault::Rejected("signature bounds"));
        }
        canonical(self)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Fault> {
        if bytes.len() > MAX_BYTES {
            return Err(Fault::Rejected("wire budget"));
        }
        let signed: Self =
            serde_json::from_slice(bytes).map_err(|_| Fault::Rejected("JSON schema"))?;
        if signed.encode()? != bytes {
            return Err(Fault::Rejected("noncanonical or duplicate JSON"));
        }
        Ok(signed)
    }
    pub fn authenticate(
        &self,
        authority: &Revision,
        repository: &str,
        verifier: &dyn SignatureCheck,
    ) -> Result<AuthenticatedEnrollment, Fault> {
        self.encode()?;
        self.enrollment.current(authority, repository)?;
        let payload = self.enrollment.encode()?;
        for (id, signature) in &self.signatures {
            let principal = authority
                .principals
                .get(id)
                .ok_or(Fault::Rejected("unknown signer"))?;
            if !principal.roles.contains("authority-admin") {
                return Err(Fault::Rejected("signer has no current administrative role"));
            }
            if principal.kind.is_none() {
                return Err(Fault::Unavailable("signer actor kind not established"));
            }
            if principal.kind != Some(ActorKind::Human) {
                return Err(Fault::Rejected(
                    "signer is not a current human administrator",
                ));
            }
            let evidence =
                verifier.verify(&principal.public_key, NAMESPACE, &payload, signature)?;
            if !evidence.valid {
                return Err(Fault::Rejected("invalid signature"));
            }
            if authority
                .policy
                .as_ref()
                .is_some_and(|p| p.require_user_presence)
            {
                match evidence.user_present {
                    Some(true) => {}
                    Some(false) => return Err(Fault::Rejected("required presence refused")),
                    None => return Err(Fault::Unavailable("required presence not observed")),
                }
            }
        }
        Ok(AuthenticatedEnrollment {
            enrollment: self.enrollment.clone(),
        })
    }
}
impl AuthenticatedEnrollment {
    pub fn enrollment(&self) -> &Enrollment {
        &self.enrollment
    }
    /// Checks supplied observations. The host must actually establish their bytes
    /// and provenance, and keep authority current through launch/capture.
    pub fn allows(&self, authority: &Revision, observed: &Use<'_>) -> Result<(), Fault> {
        self.enrollment.current(authority, observed.repository)?;
        if &self.enrollment.provider != observed.provider
            || self.enrollment.verifier_digest != observed.verifier_digest
            || self.enrollment.collector != observed.collector
            || !self.enrollment.warrants.contains(observed.warrant)
        {
            return Err(Fault::Rejected("runtime inputs outside enrollment"));
        }
        Ok(())
    }
}
