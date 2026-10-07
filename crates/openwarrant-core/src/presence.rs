// SPDX-License-Identifier: Apache-2.0

//! What an sshsig signature says about the key that made it (OW-WAR-0138).
//!
//! `war sign --ssh-sign` proves that a key listed in `allowed_signers`
//! signed an act. Whether a person was at the key when it signed is a
//! different question. For an ordinary key the signature cannot answer it:
//! the confirmation dialog of `ssh-add -c` leaves no mark in the bytes, so a
//! key loaded without `-c` signs silently and the record looks the same.
//!
//! A FIDO key (`sk-ssh-ed25519@openssh.com`, `sk-ecdsa-sha2-nistp256@openssh.com`)
//! is different. Its signature carries the authenticator's flags and a
//! counter, and they are INSIDE what is signed (PROTOCOL.u2f): the
//! authenticator signs `sha256(application) ‖ flags ‖ counter ‖
//! sha256(message)`. A flag cannot be set after the fact without breaking the
//! signature.
//!
//! OpenSSH 10.5p1's `ssh-keygen -Y verify` does not act on those flags: it
//! accepts an `sk` signature whose user-presence flag is clear, prints the
//! same "Good signature" line for it, and rejects `no-touch-required` and
//! `verify-required` as unknown options in `allowed_signers`. The flags appear
//! only in its `-vv` debug output (observed 2026-09-25; OW-WAR-0138 U-002). So
//! `war` reads them itself, from the signature blob, AFTER `ssh-keygen` has
//! accepted the signature — this module parses, it never verifies. Parsing a
//! blob nobody verified tells you what the blob claims, not what happened.
//!
//! The rule this module exists to keep: **never infer presence.** Only an
//! `sk` signature whose user-presence bit is set is [`Presence::Verified`].
//! An ordinary key, an `sk` signature without the bit, and anything this
//! module cannot read are [`Presence::Unverified`] or an error — never a pass.
//!
//! Pure: no I/O, no process, no dependency. Tested against signatures made by
//! a generated `sk`-shaped key (software, not an authenticator) that
//! `ssh-keygen -Y verify` accepts.

use thiserror::Error;

/// The authenticator's "user present" flag (a touch).
pub const FLAG_USER_PRESENCE: u8 = 0x01;
/// The authenticator's "user verified" flag (a PIN or biometric).
pub const FLAG_USER_VERIFICATION: u8 = 0x04;

const MAGIC: &[u8] = b"SSHSIG";
const ARMOR_BEGIN: &str = "-----BEGIN SSH SIGNATURE-----";
const ARMOR_END: &str = "-----END SSH SIGNATURE-----";

/// Whether the signature itself shows a person was at the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// An `sk` signature with the user-presence flag set.
    Verified,
    /// Anything else. Not "absent": the signature cannot say.
    Unverified,
}

impl Presence {
    /// The word a record carries: `verified` or `unverified`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
        }
    }
}

impl std::fmt::Display for Presence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a security key's signature adds after the signature bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Authenticator {
    pub flags: u8,
    pub counter: u32,
}

/// One parsed sshsig (PROTOCOL.sshsig), read and NOT verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshSig {
    /// The public key blob, exactly as it is in the signature.
    pub public_key: Vec<u8>,
    /// The key's type, from the key blob (`ssh-ed25519`, `sk-ssh-ed25519@openssh.com`, …).
    pub key_type: String,
    pub namespace: String,
    pub hash_algorithm: String,
    /// The signature's own type string (`webauthn-sk-…` for a WebAuthn signature).
    pub signature_type: String,
    /// Present exactly when the key is a security key.
    pub authenticator: Option<Authenticator>,
}

impl SshSig {
    /// A security key (`sk-…`) made this signature.
    #[must_use]
    pub const fn is_security_key(&self) -> bool {
        self.authenticator.is_some()
    }

    /// [`Presence::Verified`] only for a security key whose signature carries
    /// the user-presence flag. Everything else is unverified.
    #[must_use]
    pub fn presence(&self) -> Presence {
        match self.authenticator {
            Some(a) if a.flags & FLAG_USER_PRESENCE != 0 => Presence::Verified,
            _ => Presence::Unverified,
        }
    }

    /// The security key also verified the user (PIN or biometric).
    #[must_use]
    pub fn user_verified(&self) -> bool {
        self.authenticator
            .is_some_and(|a| a.flags & FLAG_USER_VERIFICATION != 0)
    }

    /// One line for a report: key type, presence, and the flags that decided it.
    #[must_use]
    pub fn describe(&self) -> String {
        match self.authenticator {
            Some(a) => format!(
                "{} key, flags 0x{:02x}, counter {}: presence {}{}",
                self.key_type,
                a.flags,
                a.counter,
                self.presence(),
                if self.user_verified() {
                    ", user verified"
                } else {
                    ""
                }
            ),
            None => format!(
                "{} key: presence {} (an ordinary key's signature carries no presence flag)",
                self.key_type,
                self.presence()
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PresenceError {
    #[error("not an armored SSH signature: {0}")]
    Armor(String),
    #[error("the signature's base64 is malformed at byte {0}")]
    Base64(usize),
    #[error("the signature blob is malformed: {0}")]
    Blob(String),
}

/// Parse an armored signature (`-----BEGIN SSH SIGNATURE-----`).
pub fn parse_armored(text: &str) -> Result<SshSig, PresenceError> {
    let start = text
        .find(ARMOR_BEGIN)
        .ok_or_else(|| PresenceError::Armor(format!("no `{ARMOR_BEGIN}` line")))?;
    let body = &text[start + ARMOR_BEGIN.len()..];
    let end = body
        .find(ARMOR_END)
        .ok_or_else(|| PresenceError::Armor(format!("no `{ARMOR_END}` line")))?;
    let b64: String = body[..end].split_whitespace().collect();
    parse_blob(&base64_decode(&b64)?)
}

/// Parse the binary sshsig blob.
pub fn parse_blob(raw: &[u8]) -> Result<SshSig, PresenceError> {
    let mut r = Reader::new(raw);
    if r.take(MAGIC.len())? != MAGIC {
        return Err(PresenceError::Blob("no SSHSIG preamble".into()));
    }
    let version = r.u32()?;
    if version != 1 {
        return Err(PresenceError::Blob(format!(
            "sshsig version {version}; only 1 is known"
        )));
    }
    let public_key = r.string()?.to_vec();
    let namespace = utf8(r.string()?, "namespace")?;
    let _reserved = r.string()?;
    let hash_algorithm = utf8(r.string()?, "hash algorithm")?;
    let signature = r.string()?;
    r.end("after the signature")?;

    let key_type = utf8(Reader::new(&public_key).string()?, "key type")?;
    let mut s = Reader::new(signature);
    let signature_type = utf8(s.string()?, "signature type")?;
    let _sig = s.string()?;
    let authenticator = if is_sk_key_type(&key_type) {
        if !signature_matches_sk_key(&key_type, &signature_type) {
            return Err(PresenceError::Blob(format!(
                "a {key_type} key with a {signature_type} signature"
            )));
        }
        let flags = s.u8()?;
        let counter = s.u32()?;
        // A WebAuthn signature carries origin, client data and extensions
        // after the counter; a plain one carries nothing more.
        if signature_type.starts_with("webauthn-") {
            s.string()?;
            s.string()?;
            s.string()?;
        }
        s.end("after the authenticator fields")?;
        Some(Authenticator { flags, counter })
    } else {
        s.end("after an ordinary signature")?;
        None
    };
    Ok(SshSig {
        public_key,
        key_type,
        namespace,
        hash_algorithm,
        signature_type,
        authenticator,
    })
}

/// The key blob an `allowed_signers` line names, from its `<keytype> <base64>`
/// fields. The blob's own type must be the type the line says, or the line
/// is lying about what it lists.
pub fn public_key_blob(keytype: &str, b64: &str) -> Result<Vec<u8>, PresenceError> {
    let blob = base64_decode(b64)?;
    let inner = utf8(Reader::new(&blob).string()?, "key type")?;
    if inner != keytype {
        return Err(PresenceError::Blob(format!(
            "the line says {keytype} and the key blob is {inner}"
        )));
    }
    Ok(blob)
}

fn is_sk_key_type(t: &str) -> bool {
    t.starts_with("sk-")
}

fn signature_matches_sk_key(key: &str, sig: &str) -> bool {
    sig == key || (key == "sk-ecdsa-sha2-nistp256@openssh.com" && sig == format!("webauthn-{key}"))
}

fn utf8(b: &[u8], what: &str) -> Result<String, PresenceError> {
    String::from_utf8(b.to_vec()).map_err(|_| PresenceError::Blob(format!("{what} is not UTF-8")))
}

struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], PresenceError> {
        let end = self
            .at
            .checked_add(n)
            .filter(|e| *e <= self.buf.len())
            .ok_or_else(|| {
                PresenceError::Blob(format!(
                    "truncated: {n} byte(s) wanted at offset {}, {} left",
                    self.at,
                    self.buf.len().saturating_sub(self.at)
                ))
            })?;
        let out = &self.buf[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, PresenceError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, PresenceError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn string(&mut self) -> Result<&'a [u8], PresenceError> {
        let n = self.u32()? as usize;
        self.take(n)
    }

    fn end(&self, what: &str) -> Result<(), PresenceError> {
        if self.at == self.buf.len() {
            Ok(())
        } else {
            Err(PresenceError::Blob(format!(
                "{} unexpected byte(s) {what}",
                self.buf.len() - self.at
            )))
        }
    }
}

/// Standard base64 with padding, as OpenSSH writes it. Whitespace is the
/// caller's to strip.
pub fn base64_decode(s: &str) -> Result<Vec<u8>, PresenceError> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(PresenceError::Base64(bytes.len()));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (q, chunk) in bytes.chunks(4).enumerate() {
        let last = q + 1 == bytes.len() / 4;
        let pad = chunk.iter().rev().take_while(|c| **c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return Err(PresenceError::Base64(q * 4));
        }
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            let v = if i >= 4 - pad {
                0
            } else {
                val(*c).ok_or(PresenceError::Base64(q * 4 + i))?
            };
            n = (n << 6) | v;
        }
        let three = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&three[..3 - pad]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures: signatures under `oh.war/response` over
    // `schema = "oh.war/authorization-response/v1"\n`, made by a generated
    // software key shaped as a security key (the authenticator's signing rule
    // applied in software: no hardware was touched). Each was accepted by
    // `ssh-keygen -Y verify` (OpenSSH 10.5p1) against an allowed_signers line
    // for its key before being pasted here; that includes SK_ED_NONE, whose
    // user-presence flag is clear — the observation that made this module
    // necessary.

    /// sk-ssh-ed25519, flags 0x01 (user present), counter 42.
    const SK_ED_UP: &str = "-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAAEoAAAAac2stc3NoLWVkMjU1MTlAb3BlbnNzaC5jb20AAAAgA6EHv/
POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbgAAAAEc3NoOgAAAA9vaC53YXIvcmVzcG9u
c2UAAAAAAAAABnNoYTUxMgAAAGcAAAAac2stc3NoLWVkMjU1MTlAb3BlbnNzaC5jb20AAA
BA1kmeLUy4xFGxd8zMBmKNqenjIzCBUZC2oC3MIWTxWsYKAEZ6cz4NT3Fx3YL6SdPYv7GH
5V/G8aNDvOm7mlxMCAEAAAAq
-----END SSH SIGNATURE-----
";
    /// The same key and message, flags 0x00: no touch.
    const SK_ED_NONE: &str = "-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAAEoAAAAac2stc3NoLWVkMjU1MTlAb3BlbnNzaC5jb20AAAAgA6EHv/
POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbgAAAAEc3NoOgAAAA9vaC53YXIvcmVzcG9u
c2UAAAAAAAAABnNoYTUxMgAAAGcAAAAac2stc3NoLWVkMjU1MTlAb3BlbnNzaC5jb20AAA
BAJa0j8niTYU7d+xXnDnGAmeprhyulQD9eI1P7WN58T9p6CFhZIKFR1n+EZg95M6Ej5o8r
K5dkILRQrl7LjrDcBAAAAAAq
-----END SSH SIGNATURE-----
";
    /// The same key and message, flags 0x05: present and verified.
    const SK_ED_UV: &str = "-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAAEoAAAAac2stc3NoLWVkMjU1MTlAb3BlbnNzaC5jb20AAAAgA6EHv/
POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbgAAAAEc3NoOgAAAA9vaC53YXIvcmVzcG9u
c2UAAAAAAAAABnNoYTUxMgAAAGcAAAAac2stc3NoLWVkMjU1MTlAb3BlbnNzaC5jb20AAA
BAECuWt5aciFG5lyTWr2KV5aluSzq5Aludon+JWD6q6pSDK2/Uu5alRB2ub/EdzTNLIPli
Y5El1vMZ5ZZtgnTKBQUAAAAq
-----END SSH SIGNATURE-----
";
    /// sk-ecdsa-sha2-nistp256, flags 0x01, counter 9.
    const SK_EC_UP: &str = "-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAAH8AAAAic2stZWNkc2Etc2hhMi1uaXN0cDI1NkBvcGVuc3NoLmNvbQ
AAAAhuaXN0cDI1NgAAAEEE+1A4jylJjQqTrSXsTDQDe508w8ykeH62/tq+KzAD6sifd2XK
nWKI5v9zT1zQjzpZIc9UshuzmLUKwNJXf6B0cgAAAARzc2g6AAAAD29oLndhci9yZXNwb2
5zZQAAAAAAAAAGc2hhNTEyAAAAeAAAACJzay1lY2RzYS1zaGEyLW5pc3RwMjU2QG9wZW5z
c2guY29tAAAASQAAACBkMTe3cKjp613XGMpQkIzEAQQo5a5BOYWmltTJaO6HcwAAACEA57
BBjUkQIXrjUSGlE7lDX96DgfKdh6riRq2kSsTwASwBAAAACQ==
-----END SSH SIGNATURE-----
";
    /// An ordinary ed25519 key, signed by `ssh-keygen -Y sign`.
    const ED: &str = "-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgA1wgZUTpZehhKLp+vbXv18dnV5
LmZ6v4JY974W9V06EAAAAPb2gud2FyL3Jlc3BvbnNlAAAAAAAAAAZzaGE1MTIAAABTAAAA
C3NzaC1lZDI1NTE5AAAAQORddZJohm65fvLILFiqZDA/I0WcKb0DzH7DzQl8hbHQYmH9pc
7wSKhngh8vBz2UgtI8MzQN06FxAa8pkhEICAE=
-----END SSH SIGNATURE-----
";
    const SK_ED_PUB: &str = "AAAAGnNrLXNzaC1lZDI1NTE5QG9wZW5zc2guY29tAAAAIAOhB7/zzhC+HXDdGOdLwJln5NYwm6UNXx3chmQSVTG4AAAABHNzaDo=";
    const ED_PUB: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIANcIGVE6WXoYSi6fr2179fHZ1eS5mer+CWPe+FvVdOh";

    #[test]
    fn a_touched_security_key_is_verified() {
        let s = parse_armored(SK_ED_UP).unwrap();
        assert_eq!(s.key_type, "sk-ssh-ed25519@openssh.com");
        assert_eq!(s.namespace, "oh.war/response");
        assert_eq!(s.hash_algorithm, "sha512");
        assert_eq!(
            s.authenticator,
            Some(Authenticator {
                flags: 0x01,
                counter: 42
            })
        );
        assert_eq!(s.presence(), Presence::Verified);
        assert!(!s.user_verified());
        assert!(
            s.describe().contains("presence verified"),
            "{}",
            s.describe()
        );
    }

    #[test]
    fn a_security_key_without_the_flag_is_unverified() {
        let s = parse_armored(SK_ED_NONE).unwrap();
        assert!(s.is_security_key());
        assert_eq!(s.authenticator.unwrap().flags, 0);
        assert_eq!(s.presence(), Presence::Unverified);
    }

    #[test]
    fn user_verification_is_read_beside_presence() {
        let s = parse_armored(SK_ED_UV).unwrap();
        assert_eq!(s.presence(), Presence::Verified);
        assert!(s.user_verified());
    }

    #[test]
    fn an_ecdsa_security_key_is_read_the_same_way() {
        let s = parse_armored(SK_EC_UP).unwrap();
        assert_eq!(s.key_type, "sk-ecdsa-sha2-nistp256@openssh.com");
        assert_eq!(
            s.authenticator,
            Some(Authenticator {
                flags: 0x01,
                counter: 9
            })
        );
        assert_eq!(s.presence(), Presence::Verified);
    }

    #[test]
    fn an_ordinary_key_is_never_verified() {
        let s = parse_armored(ED).unwrap();
        assert_eq!(s.key_type, "ssh-ed25519");
        assert!(!s.is_security_key());
        assert_eq!(s.presence(), Presence::Unverified);
        assert!(s.describe().contains("carries no presence flag"));
    }

    #[test]
    fn the_signature_names_the_key_allowed_signers_lists() {
        let s = parse_armored(SK_ED_UP).unwrap();
        assert_eq!(
            s.public_key,
            public_key_blob("sk-ssh-ed25519@openssh.com", SK_ED_PUB).unwrap()
        );
        let e = parse_armored(ED).unwrap();
        assert_eq!(
            e.public_key,
            public_key_blob("ssh-ed25519", ED_PUB).unwrap()
        );
        assert_ne!(s.public_key, e.public_key);
        // A line whose type disagrees with its own blob is refused.
        assert!(public_key_blob("ssh-ed25519", SK_ED_PUB).is_err());
    }

    #[test]
    fn a_flipped_flag_byte_parses_and_is_not_thereby_verified() {
        // Parsing is not verification: the flipped byte reads as present, and
        // `ssh-keygen -Y verify` refuses exactly this blob ("incorrect
        // signature"). The caller's order — verify first, then read — is the
        // control; this test pins that the parser alone claims nothing more.
        let raw = armored_bytes(SK_ED_NONE);
        let mut flipped = raw.clone();
        let at = flipped.len() - 5;
        flipped[at] ^= FLAG_USER_PRESENCE;
        let s = parse_blob(&flipped).unwrap();
        assert_eq!(s.presence(), Presence::Verified);
        assert_ne!(raw, flipped);
    }

    #[test]
    fn a_malformed_signature_is_an_error_not_a_presence() {
        assert!(matches!(
            parse_armored("hello"),
            Err(PresenceError::Armor(_))
        ));
        let raw = armored_bytes(SK_ED_UP);
        // Truncated inside the authenticator fields.
        assert!(parse_blob(&raw[..raw.len() - 2]).is_err());
        // Trailing bytes.
        let mut long = raw.clone();
        long.push(0);
        assert!(parse_blob(&long).is_err());
        // Wrong preamble.
        let mut bad = raw;
        bad[0] = b'X';
        assert!(parse_blob(&bad).is_err());
        assert!(matches!(
            base64_decode("A$=="),
            Err(PresenceError::Base64(_))
        ));
    }

    #[test]
    fn base64_round_trips_what_openssh_writes() {
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("Zg==").unwrap(), b"f");
        assert_eq!(base64_decode("Zm8=").unwrap(), b"fo");
        assert_eq!(base64_decode("Zm9v").unwrap(), b"foo");
        assert!(base64_decode("Zg=").is_err());
        assert!(base64_decode("Zg==Zm9v").is_err());
    }

    fn armored_bytes(text: &str) -> Vec<u8> {
        let b64: String = text.lines().filter(|l| !l.starts_with("-----")).collect();
        base64_decode(&b64).unwrap()
    }
}
