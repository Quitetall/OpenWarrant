// SPDX-License-Identifier: Apache-2.0
//! TLS for `war view ui --lan` (OW-WAR-0139, U-002).
//!
//! TLS terminates in `war`, on the same blocking accept loop as the rest of
//! the server (OW-ADR-0014: no async), through `rustls` with the `ring`
//! provider — both already in the build through `ureq`.
//!
//! Two sources of a certificate, and the server says which it is using:
//!
//! - **operator** (U-002 option A): `--cert <pem> --key <pem>`, for example
//!   from `tailscale cert` or a local CA. A device that trusts that CA opens
//!   the page with no warning.
//! - **self-signed** (U-002 option B, the labelled fallback):
//!   `--self-signed` makes a P-256 key and a certificate for `--name`, kept
//!   in the per-user state directory (mode 0600) so its fingerprint is stable
//!   across restarts. Every device shows the browser's warning on first
//!   visit; the SHA-256 fingerprint is printed at the host and carried in the
//!   pairing link, for the human to compare (basis A-002).
//!
//! A handshake that fails — a plain-HTTP request to the LAN port included —
//! gets no HTTP response at all. The certificate's self-signature is a TLS
//! artefact; it is not an authority signature and no key here can make one.

use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

/// How long a TLS handshake may take before the connection is dropped.
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(4);

/// Where the certificate came from; printed at start so the operator knows
/// which of U-002's options is in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `--cert` / `--key` (option A).
    Operator,
    /// `--self-signed` (option B, the fallback), kept at this path.
    SelfSigned(Utf8PathBuf),
}

impl Source {
    /// One line naming the source, for the host terminal and the JSON report.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Operator => "operator-supplied (U-002 option A)".to_owned(),
            Self::SelfSigned(p) => format!(
                "self-signed at {p} (U-002 option B, the fallback: every device shows a browser warning; compare the fingerprint)"
            ),
        }
    }
}

/// A loaded server configuration and the leaf certificate's fingerprint.
pub struct Tls {
    config: Arc<ServerConfig>,
    /// SHA-256 of the leaf certificate's DER, `AA:BB:…`.
    pub fingerprint: String,
    pub source: Source,
}

fn config(
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<ServerConfig, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut c = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("ui.tls-config: {e}"))?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|e| {
            format!(
                "ui.tls-certificate: the certificate and key do not make a server identity: {e}"
            )
        })?;
    c.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(c)
}

/// The colon-separated SHA-256 of a certificate's DER.
#[must_use]
pub fn fingerprint(der: &[u8]) -> String {
    let hex = openwarrant_compiler::sha256_hex(der).to_ascii_uppercase();
    hex.as_bytes()
        .chunks(2)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(":")
}

impl Tls {
    /// U-002 option A: an operator's PEM certificate chain and private key.
    pub fn operator(cert: &Utf8Path, key: &Utf8Path) -> Result<Self, String> {
        let chain: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(cert.as_std_path())
            .map_err(|e| format!("ui.tls-certificate: cannot read {cert}: {e}"))?
            .collect::<Result<_, _>>()
            .map_err(|e| {
                format!("ui.tls-certificate: {cert} is not a PEM certificate chain: {e}")
            })?;
        let Some(leaf) = chain.first() else {
            return Err(format!("ui.tls-certificate: {cert} holds no certificate"));
        };
        let fingerprint = fingerprint(leaf.as_ref());
        let key = PrivateKeyDer::from_pem_file(key.as_std_path())
            .map_err(|e| format!("ui.tls-key: cannot read a PEM private key from {key}: {e}"))?;
        Ok(Self {
            config: Arc::new(config(chain, key)?),
            fingerprint,
            source: Source::Operator,
        })
    }

    /// U-002 option B, the fallback: a self-signed certificate for `name`,
    /// made once and kept under `dir` (mode 0600), remade after a year.
    pub fn self_signed(name: &str, dir: &Utf8Path) -> Result<Self, String> {
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let cert_path = dir.join(format!("{safe}.cert.der"));
        let key_path = dir.join(format!("{safe}.key.der"));
        let fresh = std::fs::metadata(&cert_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age < Duration::from_secs(360 * 86_400));
        let (cert, key) = match (fresh, std::fs::read(&cert_path), std::fs::read(&key_path)) {
            (true, Ok(c), Ok(k)) => (c, k),
            _ => {
                let (c, k) = generate(name)?;
                super::pairing::write_private(&key_path, &k)?;
                super::pairing::write_private(&cert_path, &c)?;
                (c, k)
            }
        };
        let fingerprint = fingerprint(&cert);
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key));
        Ok(Self {
            config: Arc::new(config(vec![CertificateDer::from(cert)], key)?),
            fingerprint,
            source: Source::SelfSigned(cert_path),
        })
    }

    /// Complete a TLS handshake on an accepted connection within the limit.
    /// Anything else — plain HTTP, a stalled client — is an error, and the
    /// caller drops the connection without writing a byte of HTTP.
    pub fn accept(
        &self,
        tcp: TcpStream,
    ) -> Result<StreamOwned<ServerConnection, TcpStream>, String> {
        let mut tcp = tcp;
        tcp.set_nonblocking(false).map_err(|e| e.to_string())?;
        tcp.set_read_timeout(Some(Duration::from_secs(1)))
            .map_err(|e| e.to_string())?;
        tcp.set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let mut conn =
            ServerConnection::new(Arc::clone(&self.config)).map_err(|e| e.to_string())?;
        let deadline = Instant::now() + HANDSHAKE_LIMIT;
        while conn.is_handshaking() {
            if Instant::now() >= deadline {
                return Err("handshake not completed in time".to_owned());
            }
            match conn.complete_io(&mut tcp) {
                Ok((0, 0)) => return Err("the peer closed during the handshake".to_owned()),
                Ok(_) => {}
                // A pause longer than one read's timeout is not a failed
                // handshake; HANDSHAKE_LIMIT is the bound (t-26ca).
                Err(e) if super::stalled(&e) => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(StreamOwned::new(conn, tcp))
    }
}

// ---- the fallback's certificate: a minimal DER writer ----------------------
//
// Only what a self-signed P-256 server certificate needs: X.509 v3, a random
// positive serial, ecdsa-with-SHA256, CN and subjectAltName for `name`, and
// extendedKeyUsage serverAuth. The unit test below hands the result to
// rustls on both sides of a handshake, so a malformed encoding fails there.

fn tlv(tag: u8, parts: &[&[u8]]) -> Vec<u8> {
    let n: usize = parts.iter().map(|p| p.len()).sum();
    let mut out = vec![tag];
    if n < 0x80 {
        out.push(n as u8);
    } else if n < 0x100 {
        out.extend([0x81, n as u8]);
    } else {
        out.extend([0x82, (n >> 8) as u8, n as u8]);
    }
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

const SEQ: u8 = 0x30;
const SET: u8 = 0x31;
/// ecdsa-with-SHA256, 1.2.840.10045.4.3.2
const OID_ECDSA_SHA256: &[u8] = &[0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
/// id-ecPublicKey 1.2.840.10045.2.1, prime256v1 1.2.840.10045.3.1.7
const OID_EC_PUBLIC_KEY: &[u8] = &[0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
const OID_P256: &[u8] = &[0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
const OID_CN: &[u8] = &[0x06, 0x03, 0x55, 0x04, 0x03];
const OID_SAN: &[u8] = &[0x06, 0x03, 0x55, 0x1d, 0x11];
const OID_EKU: &[u8] = &[0x06, 0x03, 0x55, 0x1d, 0x25];
const OID_SERVER_AUTH: &[u8] = &[0x06, 0x08, 0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01];

fn utc_time(unix: i64) -> Result<Vec<u8>, String> {
    let t = chrono::DateTime::<chrono::Utc>::from_timestamp(unix, 0)
        .ok_or("ui.tls-certificate: the clock is out of range")?;
    Ok(tlv(
        0x17,
        &[t.format("%y%m%d%H%M%SZ").to_string().as_bytes()],
    ))
}

/// A self-signed certificate and its PKCS#8 key, both DER.
fn generate(name: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
    use ring::rand::SecureRandom;
    use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
        .map_err(|_| "ui.tls-certificate: could not generate a key")?;
    let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng)
        .map_err(|_| "ui.tls-certificate: could not load the generated key")?;
    let mut serial = [0u8; 16];
    rng.fill(&mut serial)
        .map_err(|_| "ui.tls-certificate: no random source")?;
    serial[0] = (serial[0] & 0x7f) | 0x01;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let alg = tlv(SEQ, &[OID_ECDSA_SHA256]);
    let cn = format!("war ui {name}");
    let dn = tlv(
        SEQ,
        &[&tlv(
            SET,
            &[&tlv(SEQ, &[OID_CN, &tlv(0x0c, &[cn.as_bytes()])])],
        )],
    );
    let validity = tlv(
        SEQ,
        &[&utc_time(now - 86_400)?, &utc_time(now + 397 * 86_400)?],
    );
    let mut point = vec![0u8];
    point.extend_from_slice(pair.public_key().as_ref());
    let spki = tlv(
        SEQ,
        &[
            &tlv(SEQ, &[OID_EC_PUBLIC_KEY, OID_P256]),
            &tlv(0x03, &[&point]),
        ],
    );
    let general_name = match name.trim_matches(['[', ']']).parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => tlv(0x87, &[&ip.octets()]),
        Ok(std::net::IpAddr::V6(ip)) => tlv(0x87, &[&ip.octets()]),
        Err(_) => tlv(0x82, &[name.as_bytes()]),
    };
    let san = tlv(SEQ, &[OID_SAN, &tlv(0x04, &[&tlv(SEQ, &[&general_name])])]);
    let eku = tlv(
        SEQ,
        &[OID_EKU, &tlv(0x04, &[&tlv(SEQ, &[OID_SERVER_AUTH])])],
    );
    let extensions = tlv(0xa3, &[&tlv(SEQ, &[&san, &eku])]);
    let tbs = tlv(
        SEQ,
        &[
            &[0xa0, 0x03, 0x02, 0x01, 0x02],
            &tlv(0x02, &[&serial]),
            &alg,
            &dn,
            &validity,
            &dn,
            &spki,
            &extensions,
        ],
    );
    let sig = pair
        .sign(&rng, &tbs)
        .map_err(|_| "ui.tls-certificate: could not sign the certificate")?;
    let mut bits = vec![0u8];
    bits.extend_from_slice(sig.as_ref());
    let cert = tlv(SEQ, &[&tbs, &alg, &tlv(0x03, &[&bits])]);
    Ok((cert, pkcs8.as_ref().to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn der_lengths_use_the_long_form_past_127() {
        assert_eq!(tlv(0x04, &[&[0u8; 3]])[..2], [0x04, 3]);
        assert_eq!(tlv(0x04, &[&[0u8; 200]])[..3], [0x04, 0x81, 200]);
        assert_eq!(tlv(0x04, &[&[0u8; 300]])[..4], [0x04, 0x82, 0x01, 0x2c]);
    }

    /// The fallback's certificate completes a real handshake: a rustls
    /// client that trusts exactly it connects to a rustls server using it,
    /// for the name it was made for — and a client asking for another name
    /// is refused.
    #[test]
    fn a_self_signed_certificate_serves_its_name_and_no_other() {
        let (cert, key) = generate("war-host.test").unwrap();
        let server = Arc::new(
            config(
                vec![CertificateDer::from(cert.clone())],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)),
            )
            .unwrap(),
        );
        let handshake = |name: &'static str| -> Result<Vec<u8>, String> {
            let mut roots = rustls::RootCertStore::empty();
            roots.add(CertificateDer::from(cert.clone())).unwrap();
            let client = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let config = Arc::clone(&server);
            let t = std::thread::spawn(move || {
                let (tcp, _) = listener.accept().unwrap();
                let tls = Tls {
                    config,
                    fingerprint: String::new(),
                    source: Source::Operator,
                };
                if let Ok(mut s) = tls.accept(tcp) {
                    let _ = s.write_all(b"ok");
                    let _ = s.flush();
                    s.conn.send_close_notify();
                    let _ = s.flush();
                }
            });
            let tcp = TcpStream::connect(addr).unwrap();
            let conn = rustls::ClientConnection::new(
                Arc::new(client),
                rustls_pki_types::ServerName::try_from(name).unwrap(),
            )
            .unwrap();
            let mut s = StreamOwned::new(conn, tcp);
            let mut got = Vec::new();
            let r = s
                .read_to_end(&mut got)
                .map(|_| got)
                .map_err(|e| e.to_string());
            t.join().unwrap();
            r
        };
        assert_eq!(handshake("war-host.test").unwrap(), b"ok");
        assert!(handshake("another.test").is_err());
    }

    #[test]
    fn an_ip_name_gets_an_ip_subject_alt_name() {
        let (cert, _) = generate("127.0.0.2").unwrap();
        // iPAddress [7] with the four octets.
        assert!(cert.windows(6).any(|w| w == [0x87, 4, 127, 0, 0, 2]));
    }

    #[test]
    fn fingerprints_are_colon_separated_sha256() {
        let f = fingerprint(b"x");
        assert_eq!(f.len(), 32 * 3 - 1);
        assert!(f.chars().all(|c| c == ':' || c.is_ascii_hexdigit()));
    }
}
