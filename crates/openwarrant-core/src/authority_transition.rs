// SPDX-License-Identifier: Apache-2.0
//! Pure candidate authority-transition codec and eligibility checks.
//!
//! No signatures, human review, storage isolation or bootstrap trust are established
//! here. The caller must authenticate supplied signer identities against the exact
//! previous public keys, and protect the previous state and activation transaction.
//!
//! # Revision v2 (OW-WAR-0138)
//!
//! A v1 revision (OW-WAR-0096) names keys and roles and nothing else, so it
//! cannot answer §27.2's refusal by kind: whether the principal who signed is a
//! human (A-003). v2 adds, per principal, the `actor` name records carry and
//! the actor's `kind`, and one `policy` table holding the keys that decide
//! whether work may close. Both are optional fields that a v1 revision never
//! serializes, so every v1 byte and digest is unchanged, and a v1 revision that
//! carries either is refused rather than read. v2 digests under its own
//! domain. A v1-to-v2 change is a signed transition like any other, and v2
//! cannot go back to v1.
//!
//! CONTINGENT on the owner answering OW-WAR-0138 U-001 option A.
use crate::contract::ActorKind;
use crate::independence::Independence;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REVISION_SCHEMA: &str = "oh.war/authority-revision/1";
/// OW-WAR-0138: actor kind per principal and the protected policy table.
pub const REVISION_SCHEMA_V2: &str = "oh.war/authority-revision/2";
pub const PROPOSAL_SCHEMA: &str = "oh.war/authority-proposal/1";
pub const MAX_BYTES: usize = 65_536;
const REVISION_DOMAIN: &[u8] = b"openwarrant-authority-revision-v1\0";
const REVISION_DOMAIN_V2: &[u8] = b"openwarrant-authority-revision-v2\0";
const PROPOSAL_DOMAIN: &[u8] = b"openwarrant-authority-proposal-v1\0";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Principal {
    pub public_key: String,
    pub roles: BTreeSet<String>,
    /// v2: the actor name authority records carry for this principal
    /// (`authorizer`, `resolved_by_ref`). Absent means this principal is an
    /// authority key only and signs no act `war` reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// v2: what the actor is (§27). Present exactly when `actor` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ActorKind>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub schema: String,
    pub repository: String,
    pub sequence: u64,
    pub principals: BTreeMap<String, Principal>,
    /// v2: the policy keys that decide whether work may close. Required in
    /// v2, refused in v1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<Policy>,
}
/// The protected policy keys (docs/AUTHENTICATION.md, OW-WAR-0138 U-003).
/// Each mirrors the `openwarrant.toml` key of the same name; when a store
/// governs, this value is the one `war` uses and a differing file value is
/// `policy.unprotected-divergence`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// `[policy] allow_automated_resolution`, §27.3 condition 1.
    pub allow_automated_resolution: bool,
    /// `[policy] require_user_presence`.
    pub require_user_presence: bool,
    /// `[verify] verifier_argv`.
    pub verifier_argv: Vec<String>,
    /// `[independence]`; absent means undeclared, as in `openwarrant.toml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independence: Option<Independence>,
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
    /// 1 or 2, from the schema; anything else is refused by [`Self::validate`].
    #[must_use]
    pub fn version(&self) -> u8 {
        if self.schema == REVISION_SCHEMA_V2 {
            2
        } else {
            1
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        if self.sequence > 9_007_199_254_740_991 {
            return Err(err(
                "authority-sequence",
                "Sequence exceeds exact JSON integer range",
            ));
        }
        let v2 = match self.schema.as_str() {
            REVISION_SCHEMA => false,
            REVISION_SCHEMA_V2 => true,
            _ => {
                return Err(err(
                    "authority-schema",
                    "Unsupported authority revision schema",
                ));
            }
        };
        if !v2
            && (self.policy.is_some()
                || self
                    .principals
                    .values()
                    .any(|p| p.actor.is_some() || p.kind.is_some()))
        {
            return Err(err(
                "authority-schema",
                "A v1 authority revision carries no actor, kind or policy; use the v2 schema",
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
        let mut actors = BTreeSet::new();
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
            let key_ok = if v2 {
                valid_key(&principal.public_key) || valid_sk_key(&principal.public_key)
            } else {
                valid_key(&principal.public_key)
            };
            if !key_ok {
                return Err(err(
                    "authority-key",
                    if v2 {
                        "Expected a canonical two-token ssh-ed25519 or sk-ssh-ed25519@openssh.com public key"
                    } else {
                        "Expected canonical two-token ssh-ed25519 public key"
                    },
                ));
            }
            match (&principal.actor, principal.kind) {
                (None, None) => {}
                (Some(actor), Some(_)) => {
                    if !safe_actor(actor) {
                        return Err(err(
                            "authority-actor",
                            "Actor names are 1-128 bytes of printable text without leading or trailing space",
                        ));
                    }
                    if !actors.insert(actor.as_str()) {
                        return Err(err(
                            "authority-actor",
                            "One actor name is bound to exactly one principal",
                        ));
                    }
                }
                _ => {
                    return Err(err(
                        "authority-actor",
                        "A principal names both an actor and its kind, or neither",
                    ));
                }
            }
            admin |= principal.roles.contains("authority-admin");
        }
        if !admin {
            return Err(err(
                "authority-admin",
                "At least one authority-admin must remain",
            ));
        }
        if v2 {
            let Some(policy) = &self.policy else {
                return Err(err(
                    "authority-policy",
                    "A v2 authority revision carries a policy table",
                ));
            };
            if policy.verifier_argv.len() > 64
                || policy
                    .verifier_argv
                    .iter()
                    .any(|a| a.is_empty() || a.len() > 4096 || a.contains('\0'))
            {
                return Err(err(
                    "authority-policy",
                    "verifier_argv holds at most 64 non-empty arguments of at most 4096 bytes",
                ));
            }
        }
        Ok(())
    }
    /// The principal a record's actor name is bound to, with its entry. v2
    /// only: a v1 revision binds no actor.
    #[must_use]
    pub fn principal_for_actor(&self, actor: &str) -> Option<(&str, &Principal)> {
        self.principals
            .iter()
            .find(|(_, p)| p.actor.as_deref() == Some(actor))
            .map(|(id, p)| (id.as_str(), p))
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
        let domain = if self.version() == 2 {
            REVISION_DOMAIN_V2
        } else {
            REVISION_DOMAIN
        };
        Ok(hash(domain, &self.encode()?))
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
        if previous.version() > self.next.version() {
            return Err(err(
                "authority-downgrade",
                "A v2 authority revision cannot be replaced by a v1 revision; recovery is a forward transition",
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
/// An actor name as records carry it: "Brian Lam". Printable, bounded, no
/// padding that would make two names look alike.
fn safe_actor(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.trim() == s && !s.chars().any(char::is_control)
}
/// v2: a FIDO security key, `sk-ssh-ed25519@openssh.com`, whose signatures
/// carry the authenticator's presence flag (OW-WAR-0138, presence.rs). The
/// wire blob is string(type) string(32-byte key) string(application), and the
/// base64 must be exactly what re-encoding the blob gives.
fn valid_sk_key(key: &str) -> bool {
    const TYPE: &str = "sk-ssh-ed25519@openssh.com";
    let Some(encoded) = key.strip_prefix("sk-ssh-ed25519@openssh.com ") else {
        return false;
    };
    let Ok(blob) = crate::presence::base64_decode(encoded) else {
        return false;
    };
    if base64_encode(&blob) != encoded {
        return false;
    }
    let mut at = 0usize;
    let mut next = || -> Option<&[u8]> {
        let len = u32::from_be_bytes(blob.get(at..at + 4)?.try_into().ok()?) as usize;
        let s = blob.get(at + 4..at + 4 + len)?;
        at += 4 + len;
        Some(s)
    };
    let (Some(kt), Some(pk), Some(app)) = (next(), next(), next()) else {
        return false;
    };
    kt == TYPE.as_bytes()
        && pk.len() == 32
        && !app.is_empty()
        && app.len() <= 256
        && app.iter().all(u8::is_ascii_graphic)
        && at == blob.len()
}
fn base64_encode(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn v1() -> Revision {
        Revision {
            schema: REVISION_SCHEMA.into(),
            repository: "example-project".into(),
            sequence: 0,
            principals: BTreeMap::from([(
                "alice".into(),
                Principal {
                    public_key: KEY.into(),
                    roles: BTreeSet::from(["authority-admin".into()]),
                    actor: None,
                    kind: None,
                },
            )]),
            policy: None,
        }
    }

    fn v2() -> Revision {
        let mut r = v1();
        r.schema = REVISION_SCHEMA_V2.into();
        r.sequence = 1;
        let alice = r.principals.get_mut("alice").unwrap();
        alice.actor = Some("Alice Example".into());
        alice.kind = Some(ActorKind::Human);
        alice.roles.insert("authorizer".into());
        r.policy = Some(Policy::default());
        r
    }

    fn sk_key() -> String {
        let mut blob = Vec::new();
        for part in [
            b"sk-ssh-ed25519@openssh.com".as_slice(),
            &[7u8; 32],
            b"ssh:".as_slice(),
        ] {
            blob.extend_from_slice(&(part.len() as u32).to_be_bytes());
            blob.extend_from_slice(part);
        }
        format!("sk-ssh-ed25519@openssh.com {}", base64_encode(&blob))
    }

    /// 0096's v1 bytes and digest are frozen: the digest below was computed
    /// outside this crate (sha256 over the domain and sorted, compact JSON),
    /// and the v2 fields never appear in a v1 encoding.
    #[test]
    fn a_v1_revision_keeps_its_bytes_and_digest() {
        let r = v1();
        let bytes = r.encode().unwrap();
        assert_eq!(
            String::from_utf8(bytes.clone()).unwrap(),
            format!(
                "{{\"principals\":{{\"alice\":{{\"public_key\":\"{KEY}\",\"roles\":[\"authority-admin\"]}}}},\"repository\":\"example-project\",\"schema\":\"oh.war/authority-revision/1\",\"sequence\":0}}"
            )
        );
        assert_eq!(
            r.digest().unwrap(),
            "sha256:e11e0fdbbfca43d8aaaa8a2bdcdc07a17b95c1b2e6a4e381e13fcb804524b300"
        );
        assert_eq!(Revision::decode(&bytes).unwrap(), r);
        assert_eq!(r.version(), 1);
    }

    #[test]
    fn a_v1_revision_carrying_v2_fields_is_refused() {
        let mut r = v1();
        r.policy = Some(Policy::default());
        assert_eq!(r.validate().unwrap_err().code, "authority-schema");
        let mut r = v1();
        let a = r.principals.get_mut("alice").unwrap();
        a.actor = Some("Alice".into());
        a.kind = Some(ActorKind::Human);
        assert_eq!(r.validate().unwrap_err().code, "authority-schema");
    }

    #[test]
    fn a_v2_revision_round_trips_under_its_own_domain() {
        let r = v2();
        r.validate().unwrap();
        let bytes = r.encode().unwrap();
        assert_eq!(Revision::decode(&bytes).unwrap(), r);
        // Another schema, another domain: never the same digest.
        let mut plain = v1();
        plain.sequence = 1;
        assert_ne!(r.digest().unwrap(), plain.digest().unwrap());
        assert_eq!(
            r.principal_for_actor("Alice Example").map(|(id, _)| id),
            Some("alice")
        );
        assert!(r.principal_for_actor("alice").is_none());
    }

    #[test]
    fn a_v2_revision_needs_a_policy_and_whole_actor_bindings() {
        let mut r = v2();
        r.policy = None;
        assert_eq!(r.validate().unwrap_err().code, "authority-policy");
        let mut r = v2();
        r.principals.get_mut("alice").unwrap().kind = None;
        assert_eq!(r.validate().unwrap_err().code, "authority-actor");
        let mut r = v2();
        r.principals.get_mut("alice").unwrap().actor = Some(" Alice".into());
        assert_eq!(r.validate().unwrap_err().code, "authority-actor");
        let mut r = v2();
        let twin = r.principals["alice"].clone();
        r.principals.insert("alice2".into(), twin);
        assert_eq!(r.validate().unwrap_err().code, "authority-actor");
        let mut r = v2();
        r.policy.as_mut().unwrap().verifier_argv = vec![String::new()];
        assert_eq!(r.validate().unwrap_err().code, "authority-policy");
    }

    #[test]
    fn a_security_key_is_accepted_in_v2_only() {
        let mut r = v2();
        r.principals.get_mut("alice").unwrap().public_key = sk_key();
        r.validate().unwrap();
        let mut r = v1();
        r.principals.get_mut("alice").unwrap().public_key = sk_key();
        assert_eq!(r.validate().unwrap_err().code, "authority-key");
        let mut r = v2();
        r.principals.get_mut("alice").unwrap().public_key = format!("{}=", sk_key());
        assert_eq!(r.validate().unwrap_err().code, "authority-key");
    }

    #[test]
    fn v1_to_v2_is_a_transition_and_v2_to_v1_is_refused() {
        let previous = v1();
        let up = Proposal {
            schema: PROPOSAL_SCHEMA.into(),
            operation: Operation::Update,
            previous_digest: previous.digest().unwrap(),
            next: v2(),
        };
        up.validate_against(&previous).unwrap();
        // Still a signed transition: no signer, no change.
        assert_eq!(
            authorize_transition(&previous, &up, &BTreeSet::new())
                .unwrap_err()
                .code,
            "authority-signer"
        );
        authorize_transition(&previous, &up, &BTreeSet::from(["alice".into()])).unwrap();
        let mut back = v1();
        back.sequence = 2;
        let down = Proposal {
            schema: PROPOSAL_SCHEMA.into(),
            operation: Operation::Update,
            previous_digest: v2().digest().unwrap(),
            next: back,
        };
        assert_eq!(
            down.validate_against(&v2()).unwrap_err().code,
            "authority-downgrade"
        );
    }

    /// A policy change moves the proposal's signing bytes, so a signature
    /// over one policy is not a signature over another.
    #[test]
    fn a_policy_change_moves_the_signing_bytes() {
        let previous = v1();
        let a = Proposal {
            schema: PROPOSAL_SCHEMA.into(),
            operation: Operation::Update,
            previous_digest: previous.digest().unwrap(),
            next: v2(),
        };
        let mut b = a.clone();
        b.next.policy.as_mut().unwrap().allow_automated_resolution = true;
        assert_ne!(a.signing_bytes().unwrap(), b.signing_bytes().unwrap());
    }
}
