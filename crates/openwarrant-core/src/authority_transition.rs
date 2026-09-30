// SPDX-License-Identifier: Apache-2.0
//! Pure candidate authority-transition codec and eligibility checks.
//!
//! No signatures, human review, storage isolation or bootstrap trust are established
//! here. The caller must authenticate supplied signer identities against the exact
//! previous public keys, and protect the previous state and activation transaction.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REVISION_SCHEMA: &str = "oh.war/authority-revision/1";
pub const PROPOSAL_SCHEMA: &str = "oh.war/authority-proposal/1";
pub const MAX_BYTES: usize = 65_536;
const REVISION_DOMAIN: &[u8] = b"openwarrant-authority-revision-v1\0";
const PROPOSAL_DOMAIN: &[u8] = b"openwarrant-authority-proposal-v1\0";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Principal {
    pub public_key: String,
    pub roles: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub schema: String,
    pub repository: String,
    pub sequence: u64,
    pub principals: BTreeMap<String, Principal>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Update,
    Recover,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub schema: String,
    pub operation: Operation,
    pub previous_digest: String,
    pub next: Revision,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
fn err(code: &'static str, message: &str) -> Error {
    Error {
        code,
        message: message.into(),
    }
}
impl Revision {
    pub fn validate(&self) -> Result<(), Error> {
        if self.sequence > 9_007_199_254_740_991 {
            return Err(err(
                "authority-sequence",
                "Sequence exceeds exact JSON integer range",
            ));
        }
        if self.schema != REVISION_SCHEMA {
            return Err(err(
                "authority-schema",
                "Unsupported authority revision schema",
            ));
        }
        if !safe_name(&self.repository, 128)
            || self.principals.is_empty()
            || self.principals.len() > 128
        {
            return Err(err(
                "authority-bounds",
                "Invalid repository identity or principal count",
            ));
        }
        let mut admin = false;
        for (id, principal) in &self.principals {
            if !safe_name(id, 64)
                || principal.roles.len() > 32
                || principal.roles.iter().any(|r| !safe_name(r, 64))
            {
                return Err(err(
                    "authority-principal",
                    "Invalid principal identifier or roles",
                ));
            }
            if !valid_key(&principal.public_key) {
                return Err(err(
                    "authority-key",
                    "Expected canonical two-token ssh-ed25519 public key",
                ));
            }
            admin |= principal.roles.contains("authority-admin");
        }
        if !admin {
            return Err(err(
                "authority-admin",
                "At least one authority-admin must remain",
            ));
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        encode(self)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let value: Self = decode(bytes)?;
        value.validate()?;
        Ok(value)
    }
    pub fn digest(&self) -> Result<String, Error> {
        Ok(hash(REVISION_DOMAIN, &self.encode()?))
    }
}
impl Proposal {
    fn validate(&self) -> Result<(), Error> {
        if self.schema != PROPOSAL_SCHEMA {
            return Err(err(
                "authority-schema",
                "Unsupported authority proposal schema",
            ));
        }
        if !self
            .previous_digest
            .strip_prefix("sha256:")
            .is_some_and(|v| {
                v.len() == 64
                    && v.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        {
            return Err(err(
                "authority-digest",
                "Expected lowercase sha256 previous digest",
            ));
        }
        self.next.validate()
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        encode(self)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let value: Self = decode(bytes)?;
        value.validate()?;
        Ok(value)
    }
    /// Domain-separated exact bytes for the external signature adapter.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, Error> {
        let bytes = self.encode()?;
        let mut output = Vec::with_capacity(PROPOSAL_DOMAIN.len() + bytes.len());
        output.extend_from_slice(PROPOSAL_DOMAIN);
        output.extend_from_slice(&bytes);
        Ok(output)
    }
    pub fn digest(&self) -> Result<String, Error> {
        Ok(hash(PROPOSAL_DOMAIN, &self.encode()?))
    }
    pub fn validate_against(&self, previous: &Revision) -> Result<(), Error> {
        self.encode()?;
        previous.validate()?;
        if previous.repository != self.next.repository {
            return Err(err(
                "authority-repository",
                "Repository identity cannot change",
            ));
        }
        if self.previous_digest != previous.digest()? {
            return Err(err(
                "authority-parent",
                "Previous trusted revision does not match proposal",
            ));
        }
        if previous.sequence.checked_add(1) != Some(self.next.sequence) {
            return Err(err(
                "authority-sequence",
                "Next sequence must increment previous sequence exactly once",
            ));
        }
        Ok(())
    }
}
/// Evaluate authenticated signer facts supplied by a trusted adapter. This does
/// not authenticate the facts, verify signatures or activate the revision.
pub fn authorize_transition(
    previous: &Revision,
    proposal: &Proposal,
    authenticated_signers: &BTreeSet<String>,
) -> Result<(), Error> {
    proposal.validate_against(previous)?;
    let role = match proposal.operation {
        Operation::Update => "authority-admin",
        Operation::Recover => "authority-recovery",
    };
    if authenticated_signers.is_empty()
        || authenticated_signers.iter().any(|id| {
            !previous
                .principals
                .get(id)
                .is_some_and(|p| p.roles.contains(role))
        })
    {
        return Err(err(
            "authority-signer",
            "Every supplied signer must have the required role in previous trusted state; at least one is required",
        ));
    }
    Ok(())
}
fn safe_name(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && s != "."
        && s != ".."
}
fn hash(domain: &[u8], bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(bytes);
    format!(
        "sha256:{}",
        hash.finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
fn encode(value: &impl Serialize) -> Result<Vec<u8>, Error> {
    let bytes = serde_jcs::to_vec(value)
        .map_err(|_| err("authority-json", "Cannot canonicalize authority record"))?;
    if bytes.len() > MAX_BYTES {
        return Err(err(
            "authority-bounds",
            "Authority record exceeds wire limit",
        ));
    }
    Ok(bytes)
}
fn decode<T: serde::de::DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(err(
            "authority-bounds",
            "Authority record exceeds wire limit",
        ));
    }
    let value: T = serde_json::from_slice(bytes)
        .map_err(|_| err("authority-json", "Invalid authority record JSON"))?;
    if encode(&value)? != bytes {
        return Err(err(
            "authority-canonical",
            "Authority records must use exact canonical JSON without duplicate fields or values",
        ));
    }
    Ok(value)
}
// Ed25519 SSH wire keys have a fixed 51-byte encoding, so their canonical
// base64 encoding has exactly 68 characters and no padding. Decode in place.
fn valid_key(key: &str) -> bool {
    let Some(encoded) = key.strip_prefix("ssh-ed25519 ") else {
        return false;
    };
    if encoded.len() != 68 {
        return false;
    }
    let mut bytes = [0u8; 51];
    for (input, output) in encoded
        .as_bytes()
        .chunks_exact(4)
        .zip(bytes.chunks_exact_mut(3))
    {
        let mut n = 0u32;
        for b in input {
            let value = match b {
                b'A'..=b'Z' => b - b'A',
                b'a'..=b'z' => b - b'a' + 26,
                b'0'..=b'9' => b - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                _ => return false,
            };
            n = (n << 6) | u32::from(value);
        }
        output[0] = (n >> 16) as u8;
        output[1] = (n >> 8) as u8;
        output[2] = n as u8;
    }
    bytes[..19] == *b"\0\0\0\x0bssh-ed25519\0\0\0\x20"
}
