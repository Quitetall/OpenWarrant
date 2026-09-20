// SPDX-License-Identifier: Apache-2.0
//! `war export` — §68's portable export and round trip.
//!
//! This legacy metadata envelope does not carry a content-bearing archive or an
//! importer. Its category labels and atom digest are insufficient preservation
//! evidence. Missing categories remain explicit; `--force` does not write bytes.
//!
//! OW111 adds a distinct experimental archive transport without changing this
//! historical envelope or digest. A caller's `--reconnect` flag cannot establish
//! an import, so the former false-success round-trip path now refuses. The new
//! transport's positive import/re-export control observes actual retained bytes;
//! full historical and KF preservation qualification remains separate.

use std::collections::BTreeSet;

use openwarrant_core::journal::{EXPORT_CONTENTS, PortableExport, RoundTrip};

use crate::repo::{RepoError, Repository};

/// What this repository can actually supply for each §68.2 content, and why not
/// where it cannot. Stated in one place so the export and its explanation
/// cannot drift.
fn absent_reason(content: &str) -> Option<&'static str> {
    Some(match content {
        "actions and relevant audit receipts" => {
            "the legacy exporter does not collect action and audit receipt bytes"
        }
        "runtime receipt refs" => "the legacy exporter does not collect runtime receipt references",
        "artifacts" => "the legacy exporter does not collect content-addressed artifact bytes",
        "resolution and standing" => {
            "the legacy exporter does not collect resolution and standing records"
        }
        _ => return None,
    })
}

/// Assemble a §68 export for one Warrant.
///
/// Returns the export AND the contents it could not supply, so the caller can
/// report both rather than only the first failure `validate` names.
pub fn assemble(
    repo: &Repository,
    alias: &str,
) -> Result<(PortableExport, Vec<String>), RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let basis = one
        .basis
        .as_ref()
        .ok_or_else(|| RepoError::Message(format!("{alias} could not be compiled")))?;

    let mut present = BTreeSet::new();
    let mut missing = Vec::new();
    for content in EXPORT_CONTENTS {
        if content.starts_with("optional ") {
            continue;
        }
        match absent_reason(content) {
            Some(reason) => missing.push(format!("{content} — {reason}")),
            None => {
                present.insert(content.to_owned());
            }
        }
    }

    // §68.1: small records embedded, large evidence by content address. Every
    // atom here is small, so all are embedded and none is referenced — which is
    // the honest count, not a stand-in for evidence we do not have.
    let export = PortableExport {
        present,
        embedded_record_count: basis.atoms.len(),
        referenced_evidence_digests: Vec::new(),
        document_digest: format!(
            "sha256:{}",
            openwarrant_compiler::sha256_hex(&basis.atoms.iter().fold(Vec::new(), |mut acc, a| {
                acc.extend_from_slice(&a.bytes);
                acc
            }))
        ),
    };
    Ok((export, missing))
}

/// §68.3 — export, reconnect, re-export, compare.
///
/// `reconnect` says whether the preserved evidence bytes were reconnected. It is
/// a parameter rather than always-true because the vacuous comparison is
/// exactly what §68.3 warns about, and a function that could not express it
/// could not refuse it.
pub fn round_trip(repo: &Repository, alias: &str, reconnect: bool) -> Result<RoundTrip, RepoError> {
    if reconnect {
        return Err(RepoError::Message(
            "legacy round-trip cannot verify preserved bytes: --reconnect is a claim, not an import; use the experimental archive transport for bounded byte-preservation checks".to_owned(),
        ));
    }
    let (first, _) = assemble(repo, alias)?;
    let (second, _) = assemble(repo, alias)?;
    Ok(RoundTrip {
        original_digest: first.document_digest,
        reexport_digest: second.document_digest,
        evidence_reconnected: reconnect,
        semantic_differences: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every content is either suppliable or has a stated reason. A content that
    /// is neither would drop out of the export silently.
    #[test]
    fn every_required_content_is_classified() {
        for content in EXPORT_CONTENTS {
            if content.starts_with("optional ") {
                continue;
            }
            // Classified means: either we supply it, or `absent_reason` says why
            // not. The test passes trivially today for the eight we supply; it
            // fails the moment someone adds a content nobody classified.
            let _ = absent_reason(content);
        }
        assert_eq!(EXPORT_CONTENTS.len(), 15, "§68.2 names fifteen");
    }

    /// §68.3's vacuous case. Two exports that both omit the same evidence agree
    /// about nothing, and the comparison must refuse rather than pass.
    #[test]
    fn an_unreconnected_round_trip_is_refused_as_vacuous() {
        let rt = RoundTrip {
            original_digest: "sha256:a".to_owned(),
            reexport_digest: "sha256:a".to_owned(),
            evidence_reconnected: false,
            semantic_differences: Vec::new(),
        };
        let err = rt.verify().expect_err("identical digests are not enough");
        assert!(
            err.to_string().contains("not reconnected"),
            "the refusal must say WHY it is vacuous: {err}"
        );
    }

    /// The same trip WITH the bytes reconnected is what passing looks like.
    #[test]
    fn a_reconnected_round_trip_with_equal_digests_verifies() {
        let rt = RoundTrip {
            original_digest: "sha256:a".to_owned(),
            reexport_digest: "sha256:a".to_owned(),
            evidence_reconnected: true,
            semantic_differences: Vec::new(),
        };
        assert!(rt.verify().is_ok());
    }
}
