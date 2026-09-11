// SPDX-License-Identifier: AGPL-3.0-or-later
//! Attestations: an in-toto Statement in a DSSE envelope, signed with the
//! same ssh key that signed the act (OW-ADR-0015).
//!
//! Pure: types, the DSSE pre-authentication encoding, and the SSHSIG armor.
//! No I/O and no key material here — `ssh-keygen` is invoked by the CLI.
//!
//! The point of the format is that a verifier that has never heard of
//! OpenWarrant can check one: DSSE and in-toto are documented outside this
//! repository, and the signature is an OpenSSH signature over the DSSE PAE.
//! One thing a foreign verifier must know, stated here because it is not in
//! either spec: the `sig` field is the raw SSHSIG blob (what `ssh-keygen -Y
//! sign` writes between the armor lines, base64-decoded), NOT a bare ed25519
//! signature. Unwrap SSHSIG or hand the PAE and the re-armored blob to
//! `ssh-keygen -Y verify -n oh.war/dsse`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// DSSE `payloadType` for an in-toto Statement.
pub const PAYLOAD_TYPE: &str = "application/vnd.in-toto+json";
/// The in-toto Statement `_type`.
pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";
/// The `ssh-keygen -Y sign -n` namespace attestations are signed under. A
/// response signature (`oh.war/response`) does not verify here, and this
/// one does not verify as a response.
pub const SSH_NAMESPACE: &str = "oh.war/dsse";
/// The predicate type prefix; the act name follows (`authorize`, `resolve`,
/// `correct`, `sas-accept`) and then `/v1`.
pub const PREDICATE_PREFIX: &str = "https://openwarrant.dev/attestation/";

/// One thing the Statement is about, with its digests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub name: String,
    /// Algorithm → lowercase hex. This tool writes `sha256` only.
    pub digest: BTreeMap<String, String>,
}

impl Subject {
    #[must_use]
    pub fn sha256(name: impl Into<String>, hex: impl Into<String>) -> Self {
        let mut digest = BTreeMap::new();
        digest.insert("sha256".to_owned(), hex.into());
        Self {
            name: name.into(),
            digest,
        }
    }
}

/// in-toto Statement v1. Generic over the predicate so this crate stays
/// free of a JSON value type; the CLI instantiates it with `serde_json::Value`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement<P> {
    #[serde(rename = "_type")]
    pub type_: String,
    pub subject: Vec<Subject>,
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    pub predicate: P,
}

impl<P> Statement<P> {
    #[must_use]
    pub fn new(act: &str, subject: Vec<Subject>, predicate: P) -> Self {
        Self {
            type_: STATEMENT_TYPE.to_owned(),
            subject,
            predicate_type: format!("{PREDICATE_PREFIX}{act}/v1"),
            predicate,
        }
    }

    /// The act name back out of the predicate type, if it is one of ours.
    #[must_use]
    pub fn act(&self) -> Option<&str> {
        self.predicate_type
            .strip_prefix(PREDICATE_PREFIX)?
            .strip_suffix("/v1")
    }
}

/// One DSSE signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    /// `SHA256:<base64>` — the OpenSSH fingerprint of the signing key.
    pub keyid: String,
    /// base64 of the raw SSHSIG blob over the PAE.
    pub sig: String,
}

/// DSSE envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    #[serde(rename = "payloadType")]
    pub payload_type: String,
    /// base64 of the JCS-canonical Statement.
    pub payload: String,
    pub signatures: Vec<Signature>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AttestationError {
    #[error("envelope payloadType is {found:?}, not {}", PAYLOAD_TYPE)]
    WrongPayloadType { found: String },
    #[error("envelope payload is not base64: {0}")]
    PayloadNotBase64(String),
    #[error("statement _type is {found:?}, not {}", STATEMENT_TYPE)]
    WrongStatementType { found: String },
    #[error("envelope carries {0} signature(s); this tool writes and checks exactly one")]
    NotExactlyOneSignature(usize),
    #[error("SSHSIG armor is malformed: {0}")]
    BadArmor(String),
}

/// DSSE pre-authentication encoding (v1):
/// `"DSSEv1" SP LEN(type) SP type SP LEN(payload) SP payload`, lengths in
/// decimal ASCII, over the RAW payload bytes (not their base64).
#[must_use]
pub fn pae(payload_type: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + payload_type.len() + 32);
    out.extend_from_slice(b"DSSEv1 ");
    out.extend_from_slice(payload_type.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload_type.as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload);
    out
}

/// The SSHSIG armor `ssh-keygen -Y sign` writes: header, base64 in 70-column
/// lines, footer.
#[must_use]
pub fn armor(blob_base64: &str) -> String {
    let mut out = String::from("-----BEGIN SSH SIGNATURE-----\n");
    let bytes = blob_base64.as_bytes();
    for chunk in bytes.chunks(70) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        out.push('\n');
    }
    out.push_str("-----END SSH SIGNATURE-----\n");
    out
}

/// The base64 blob between the armor lines, whitespace removed.
pub fn dearmor(text: &str) -> Result<String, AttestationError> {
    let mut lines = text.lines().map(str::trim);
    if lines.next() != Some("-----BEGIN SSH SIGNATURE-----") {
        return Err(AttestationError::BadArmor("missing BEGIN line".to_owned()));
    }
    let mut blob = String::new();
    let mut closed = false;
    for line in lines {
        if line == "-----END SSH SIGNATURE-----" {
            closed = true;
            break;
        }
        if line.is_empty() {
            continue;
        }
        blob.push_str(line);
    }
    if !closed {
        return Err(AttestationError::BadArmor("missing END line".to_owned()));
    }
    if blob.is_empty() {
        return Err(AttestationError::BadArmor("empty body".to_owned()));
    }
    Ok(blob)
}

impl Envelope {
    /// Structural checks a verifier does before any cryptography: payload
    /// type, base64, exactly one signature. Returns the raw payload bytes the
    /// PAE is rebuilt from; parsing them as a Statement is the caller's (it
    /// needs a JSON parser this crate does not carry).
    pub fn open(&self) -> Result<Vec<u8>, AttestationError> {
        if self.payload_type != PAYLOAD_TYPE {
            return Err(AttestationError::WrongPayloadType {
                found: self.payload_type.clone(),
            });
        }
        if self.signatures.len() != 1 {
            return Err(AttestationError::NotExactlyOneSignature(
                self.signatures.len(),
            ));
        }
        base64_decode(&self.payload).map_err(AttestationError::PayloadNotBase64)
    }
}

impl<P> Statement<P> {
    /// The one check on a parsed Statement that is ours to make.
    pub fn check_type(&self) -> Result<(), AttestationError> {
        if self.type_ == STATEMENT_TYPE {
            Ok(())
        } else {
            Err(AttestationError::WrongStatementType {
                found: self.type_.clone(),
            })
        }
    }
}

/// Standard base64 with padding, as in DSSE and SSHSIG. Hand-written so this
/// crate stays free of I/O and of a dependency for forty lines.
#[must_use]
pub fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u32, String> {
        Ok(match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            other => return Err(format!("byte {other:#04x} is not base64")),
        })
    }
    let clean: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !clean.len().is_multiple_of(4) {
        return Err(format!("length {} is not a multiple of 4", clean.len()));
    }
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let pad = chunk.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || chunk[..4 - pad].contains(&b'=') {
            return Err("misplaced padding".to_owned());
        }
        let mut n = 0u32;
        for &c in &chunk[..4 - pad] {
            n = (n << 6) | val(c)?;
        }
        n <<= 6 * pad as u32;
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The DSSE specification's own vector.
    #[test]
    fn pae_matches_the_dsse_spec_vector() {
        let got = pae("http://example.com/HelloWorld", b"hello world");
        assert_eq!(
            got,
            b"DSSEv1 29 http://example.com/HelloWorld 11 hello world"
        );
        // Lengths are of the raw bytes, not characters: a multibyte payload.
        let got = pae("t", "é".as_bytes());
        assert_eq!(got, b"DSSEv1 1 t 2 \xc3\xa9");
    }

    #[test]
    fn base64_round_trips_and_refuses_junk() {
        for s in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0, 255, 7, 128],
        ] {
            assert_eq!(base64_decode(&base64_encode(s)).unwrap(), s);
        }
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert!(base64_decode("Zm8").is_err());
        assert!(base64_decode("Zm=8").is_err());
        assert!(base64_decode("Zm!8").is_err());
    }

    #[test]
    fn armor_round_trips_and_dearmor_refuses_what_is_not_armor() {
        let blob = "A".repeat(150);
        let armored = armor(&blob);
        assert!(armored.starts_with("-----BEGIN SSH SIGNATURE-----\n"));
        assert_eq!(armored.lines().count(), 5);
        assert_eq!(dearmor(&armored).unwrap(), blob);
        assert!(dearmor("nope").is_err());
        assert!(dearmor("-----BEGIN SSH SIGNATURE-----\nAAAA\n").is_err());
        assert!(dearmor("-----BEGIN SSH SIGNATURE-----\n-----END SSH SIGNATURE-----\n").is_err());
    }

    #[test]
    fn open_checks_structure_before_any_cryptography() {
        let statement = Statement::new(
            "authorize",
            vec![Subject::sha256("docs/x.toml", "ab".repeat(32))],
            serde_json::json!({"actor": "a"}),
        );
        assert_eq!(statement.act(), Some("authorize"));
        let payload = serde_jcs::to_string(&statement).unwrap();
        let env = Envelope {
            payload_type: PAYLOAD_TYPE.to_owned(),
            payload: base64_encode(payload.as_bytes()),
            signatures: vec![Signature {
                keyid: "SHA256:x".to_owned(),
                sig: "AAAA".to_owned(),
            }],
        };
        let raw = env.open().unwrap();
        assert_eq!(raw, payload.as_bytes());
        let back: Statement<serde_json::Value> = serde_json::from_slice(&raw).unwrap();
        assert_eq!(back, statement);
        back.check_type().unwrap();
        let mut bad = env.clone();
        bad.payload_type = "text/plain".to_owned();
        assert!(matches!(
            bad.open(),
            Err(AttestationError::WrongPayloadType { .. })
        ));
        let mut bad = env.clone();
        bad.signatures.clear();
        assert!(matches!(
            bad.open(),
            Err(AttestationError::NotExactlyOneSignature(0))
        ));
        let other = Statement::new("x", vec![], serde_json::json!({}));
        let other = Statement {
            type_: "other".to_owned(),
            ..other
        };
        assert!(matches!(
            other.check_type(),
            Err(AttestationError::WrongStatementType { .. })
        ));
        // Unknown fields are refused at parse: the envelope is a closed shape.
        assert!(
            serde_json::from_str::<Envelope>(
                r#"{"payloadType":"a","payload":"","signatures":[],"extra":1}"#
            )
            .is_err()
        );
    }
}
