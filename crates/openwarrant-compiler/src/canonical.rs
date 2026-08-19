// SPDX-License-Identifier: AGPL-3.0-or-later
//! RFC 8785 canonicalization and domain-separated digests (SAS §65).
//!
//! Implementation selected by OW-ADR-0001 on measured conformance.

use serde::Serialize;

use crate::digest::{DigestDomain, sha256_hex};

#[derive(Debug, thiserror::Error)]
pub enum CanonicalError {
    #[error("value could not be canonicalized as RFC 8785 JSON: {0}")]
    Canonicalize(#[from] serde_json::Error),
}

/// Canonicalize a serializable value to RFC 8785 JSON bytes.
pub fn to_canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalError> {
    Ok(serde_jcs::to_vec(value)?)
}

/// Canonicalize a serializable value to an RFC 8785 JSON string.
pub fn to_canonical_string<T: Serialize>(value: &T) -> Result<String, CanonicalError> {
    Ok(serde_jcs::to_string(value)?)
}

/// The domain-separated preimage a digest is computed over (SAS §65.2).
///
/// Field names are `digest_domain` and `payload` exactly as the SAS writes them.
/// Their canonical order is fixed by RFC 8785's key sort, not by this struct's
/// declaration order, so the preimage cannot drift if the fields are reordered
/// here.
#[derive(Debug, Serialize)]
struct Preimage<'a, T: Serialize> {
    digest_domain: &'a str,
    payload: &'a T,
}

/// The exact bytes hashed for `domain` over `payload`.
///
/// Exposed rather than kept private because a digest nobody can reproduce is a
/// digest nobody can audit: given a disagreement between two implementations,
/// the first question is always "what did you hash?"
pub fn preimage_bytes<T: Serialize>(
    domain: DigestDomain,
    payload: &T,
) -> Result<Vec<u8>, CanonicalError> {
    to_canonical_bytes(&Preimage {
        digest_domain: domain.as_uri(),
        payload,
    })
}

/// SHA-256 over the domain-separated canonical preimage, lowercase hex.
///
/// §65.1: the algorithm is always explicit. This function is SHA-256 and says so
/// in its name; a future algorithm gets a new function rather than a flag, so no
/// caller can compute a digest without knowing which one it got.
pub fn sha256_digest<T: Serialize>(
    domain: DigestDomain,
    payload: &T,
) -> Result<String, CanonicalError> {
    Ok(sha256_hex(&preimage_bytes(domain, payload)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The official cyberphone/json-canonicalization vectors, vendored under
    /// `conformance/rfc8785/`. External expectations: an implementation-derived
    /// snapshot would assert only that we are self-consistent.
    #[test]
    fn official_rfc8785_vectors() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/rfc8785");
        let names = [
            "arrays",
            "french",
            "structures",
            "unicode",
            "values",
            "weird",
        ];
        let mut checked = 0;
        for name in names {
            let input = std::fs::read_to_string(format!("{dir}/in-{name}.json"))
                .unwrap_or_else(|e| panic!("missing vector in-{name}.json: {e}"));
            let expected = std::fs::read_to_string(format!("{dir}/out-{name}.json"))
                .unwrap_or_else(|e| panic!("missing vector out-{name}.json: {e}"));
            let value: serde_json::Value =
                serde_json::from_str(&input).expect("vector input parses");
            assert_eq!(
                to_canonical_string(&value).expect("canonicalizes"),
                expected.trim_end_matches('\n'),
                "RFC 8785 vector {name}"
            );
            checked += 1;
        }
        // Count asserted so a vector silently disappearing from disk shows up as
        // a failure rather than as a shorter, still-green run.
        assert_eq!(checked, 6, "all six official vectors must be present");
    }

    /// ES6 number serialization (RFC 8785 §3.2.2.3) — where implementations
    /// that use Rust's `ryu` instead of `ryu-js` diverge.
    #[test]
    fn es6_number_boundaries() {
        let cases: &[(&str, &str)] = &[
            ("0", "0"),
            ("-0", "0"),
            ("1e20", "100000000000000000000"),
            ("1e21", "1e+21"),
            ("1e-6", "0.000001"),
            ("1e-7", "1e-7"),
            ("1e30", "1e+30"),
            ("5e-324", "5e-324"),
            ("9007199254740992", "9007199254740992"),
            ("1.7976931348623157e308", "1.7976931348623157e+308"),
            ("2.2250738585072014e-308", "2.2250738585072014e-308"),
            ("1.0", "1"),
            ("100.0", "100"),
        ];
        for (input, expected) in cases {
            let value: serde_json::Value = serde_json::from_str(input).expect("parses");
            assert_eq!(
                &to_canonical_string(&value).expect("canonicalizes"),
                expected,
                "number {input}"
            );
        }
    }

    /// RFC 8785 sorts object keys by UTF-16 code unit, NOT by UTF-8 byte order.
    /// The two agree throughout the BMP and diverge above it, so this is the
    /// case that actually discriminates a correct implementation.
    #[test]
    fn keys_sort_by_utf16_code_unit_not_utf8_bytes() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"é":1,"😀":2,"a":3,"￿":4}"#).expect("parses");
        let canonical = to_canonical_string(&value).expect("canonicalizes");

        // UTF-16: a=0061, é=00E9, 😀=D83D DE00 (high surrogate), ￿=FFFF.
        // So the emoji sorts BEFORE U+FFFF. Naive UTF-8 byte order would put
        // U+FFFF (EF BF BF) before the emoji (F0 9F 98 80).
        assert_eq!(canonical, "{\"a\":3,\"é\":1,\"😀\":2,\"\u{ffff}\":4}");
        assert!(
            canonical.find('😀').unwrap() < canonical.find('\u{ffff}').unwrap(),
            "emoji must precede U+FFFF under UTF-16 ordering"
        );
    }

    /// §91.1 test 6: different digest domains produce different preimages.
    /// Checked over the full pairwise set — 105 pairs — because a sample leaves
    /// the untested pair as the one that collides.
    #[test]
    fn every_domain_pair_yields_a_distinct_digest() {
        let payload = json!({"same": "payload", "for": "every domain"});
        let mut digests = std::collections::BTreeMap::new();
        for domain in DigestDomain::ALL {
            let d = sha256_digest(domain, &payload).expect("digests");
            if let Some(other) = digests.insert(d.clone(), domain) {
                panic!("{domain:?} and {other:?} collided on {d}");
            }
        }
        assert_eq!(digests.len(), DigestDomain::ALL.len());
    }

    #[test]
    fn the_preimage_has_the_shape_the_sas_specifies() {
        let bytes = preimage_bytes(DigestDomain::Contract, &json!({"a": 1})).expect("preimage");
        assert_eq!(
            String::from_utf8(bytes).expect("utf-8"),
            r#"{"digest_domain":"oh.war/contract/v1","payload":{"a":1}}"#
        );
    }

    /// Key order in the source must not change the canonical output, which is
    /// the property that makes a digest a function of meaning rather than of
    /// how the JSON happened to be written.
    #[test]
    fn source_key_order_does_not_affect_the_digest() {
        let a: serde_json::Value = serde_json::from_str(r#"{"b":2,"a":1}"#).expect("parses");
        let b: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).expect("parses");
        assert_eq!(
            sha256_digest(DigestDomain::Manifest, &a).expect("digest"),
            sha256_digest(DigestDomain::Manifest, &b).expect("digest")
        );
    }

    /// Determinism asserted across two independent computations, not by
    /// comparing a value to itself.
    #[test]
    fn digests_are_stable_across_runs() {
        let payload = json!({"x": [1, 2, 3], "y": {"z": true}});
        let first = sha256_digest(DigestDomain::WarExport, &payload).expect("digest");
        let second = sha256_digest(DigestDomain::WarExport, &payload).expect("digest");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64, "sha256 hex is 64 characters");
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
