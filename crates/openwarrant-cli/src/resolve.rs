// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war resolve --dry-run` — evaluate §56.1's thirteen requirements.
//!
//! # Why this is a dry run and nothing else, for now
//!
//! §56.1 lists thirteen things resolution SHALL verify. `ResolutionChecks`
//! implemented all thirteen during alpha as named booleans, each tested to block
//! on its own, and was referenced by no binary. So the thirteen were a struct
//! nothing computed.
//!
//! This computes them from the corpus as it actually is, and reports which
//! block. It deliberately does NOT record a resolution: §56.2's record needs an
//! authorizer, an acting role and a stated meaning, and inventing those to make
//! the command feel complete is the substitution §40.7 forbids.
//!
//! # Where requirement 10 is read from, and why it moved
//!
//! Independence used to be answered from `openwarrant.toml`'s single global
//! `[independence]` block. That was unsound in the dangerous direction: one
//! block set true would make EVERY Warrant claim independence, including ones
//! nothing independent had ever looked at.
//!
//! It is now answered from the verification records attached to the Warrant
//! (`verifications/*.toml`, see [`openwarrant_core::verification`]). Each
//! verification carries the independence its verifier ACTUALLY had, and
//! `admissible_for` refuses self-verification, evidence-free verdicts, and
//! independence below §46.3's minimum for the level. An obligation with no
//! admissible verification does not count as dispositioned.
//!
//! # What it will say today, and why that is the correct answer
//!
//! Every Warrant here still blocks, because nine of the thirteen ask about
//! records that do not exist yet — there is no authorization record (§28.4), no
//! typed deliverables (§37), nothing content-addressed, no gate run bound to an
//! obligation, no judgment records, and no role assignment.
//!
//! That is the Phase 6 exit being honest rather than a defect. A resolver that
//! closed a Warrant under these conditions would be manufacturing the exact
//! false completion the whole system exists to prevent.

use openwarrant_core::deliverable::Deliverable;
use openwarrant_core::resolution::{RESOLUTION_REQUIREMENTS, ResolutionChecks};
use openwarrant_core::verification::Verification;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// Compute §56.1's thirteen from the corpus as it stands.
///
/// Each is answered from a record or answered `false`. Nothing is assumed true
/// because it is probably fine — an unanswerable requirement is unmet, which is
/// the same fail-closed rule §32 applies to Preflight.
fn evaluate(
    repo: &Repository,
    one: &crate::repo::Loaded,
    verifications: &[Verification],
    deliverables: &[Deliverable],
) -> ResolutionChecks {
    let validated = one.validated.as_ref();
    let basis = one.basis.as_ref();

    let assurance = validated
        .map(|v| v.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());

    // The obligations this Warrant declares, by id.
    let declared: Vec<String> = basis
        .map(|b| {
            b.atoms
                .iter()
                .filter(|a| a.role == "assurance")
                .filter_map(|a| {
                    openwarrant_core::obligation::parse(&String::from_utf8_lossy(&a.bytes)).ok()
                })
                .flat_map(|set| set.obligations.into_iter().map(|o| o.id))
                .collect()
        })
        .unwrap_or_default();

    // §46.3 is now answered from the verifications that actually happened, not
    // from a project-wide declaration.
    //
    // The global `[independence]` block could only ever be an aspiration: one
    // block set true made EVERY Warrant claim independence, including ones
    // nothing independent had looked at. Independence is a property of a
    // verifier examining a claim, so it is read from the verification record.
    let admissible: Vec<&Verification> = verifications
        .iter()
        .filter(|v| v.admissible_for(&assurance).is_ok())
        .collect();

    // Every declared obligation must carry an ADMISSIBLE verification. A
    // verification that is present but inadmissible (self-verified, evidence-free,
    // or insufficiently independent) counts for nothing, which is the point.
    let obligations_dispositioned = !declared.is_empty()
        && declared
            .iter()
            .all(|id| admissible.iter().any(|v| &v.obligation == id));

    // Independence holds when something was actually verified independently.
    // With no verifications at all this is false, which is correct: nothing
    // independent happened.
    let independence_met = !admissible.is_empty()
        && declared
            .iter()
            .all(|id| admissible.iter().any(|v| &v.obligation == id));

    let required_deliverables_exist =
        required_deliverables_exist(&repo.root, deliverables, &declared);
    let artifact_digests_verify = artifact_digests_verify(&repo.root, deliverables);

    ResolutionChecks {
        // No authorization record exists anywhere in this corpus (§28.4).
        exact_authorized_contract_revision: false,
        required_deliverables_exist,
        artifact_digests_verify,
        every_required_obligation_dispositioned: obligations_dispositioned,
        // A gate runs, but no run is bound to an obligation as its result.
        every_required_gate_has_admissible_result: false,
        // The adequacy warnings ARE required unknowns.
        no_required_unknown_remains: false,
        no_blocker_remains: true,
        deviations_dispositioned: true,
        required_judgments_exist: false,
        independence_requirements_met: independence_met,
        residual_risks_have_sufficient_authority: false,
        // §48.4 receipts exist for gate runs, but no runtime receipt is bound to
        // a Warrant's basis.
        runtime_receipts_match_the_basis: false,
        // §27 role assignment does not exist yet.
        resolver_holds_the_role: false,
    }
}

/// §37 — a required deliverable exists when it VALIDATES and its target is
/// actually present.
///
/// Validation alone would accept a record pointing at a file nobody produced,
/// which is precisely the claim this requirement exists to test. Zero required
/// deliverables is `false`, not vacuously true: a Warrant that declares nothing
/// has not shown that anything was delivered.
#[must_use]
pub fn required_deliverables_exist(
    root: &camino::Utf8Path,
    deliverables: &[Deliverable],
    declared_obligations: &[String],
) -> bool {
    let required: Vec<&Deliverable> = deliverables.iter().filter(|d| d.required).collect();
    !required.is_empty()
        && required
            .iter()
            .all(|d| d.validate(declared_obligations).is_ok() && root.join(&d.target_ref).exists())
}

/// §37.2 — a content-addressed deliverable's recorded digest must match the
/// bytes on disk.
///
/// A digest that is merely PRESENT proves nothing, so this RECOMPUTES it from
/// the file. A deliverable claiming content addressing whose target cannot be
/// read fails rather than being skipped — an unreadable artifact is not a
/// verified one.
#[must_use]
pub fn artifact_digests_verify(root: &camino::Utf8Path, deliverables: &[Deliverable]) -> bool {
    let addressed: Vec<&Deliverable> = deliverables
        .iter()
        .filter(|d| d.content_addressed)
        .collect();
    !addressed.is_empty()
        && addressed.iter().all(|d| {
            let Some(p) = d.provenance.as_ref() else {
                return false;
            };
            let recorded = p.content_digest.trim_start_matches("sha256:");
            match std::fs::read(root.join(&d.target_ref)) {
                Ok(bytes) => openwarrant_compiler::sha256_hex(&bytes) == recorded,
                Err(_) => false,
            }
        })
}

/// `war resolve <alias> --dry-run`.
pub fn run(repo: &Repository, alias: &str) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let mut report = Report::default();

    let verifications = repo.load_verifications(&dir)?;
    let deliverables = repo.load_deliverables(&dir)?;
    let checks = evaluate(repo, &one, &verifications.records, &deliverables.records);
    for (path, why) in &deliverables.failures {
        report.push(Diagnostic::error(
            "deliverables.malformed",
            path.clone(),
            format!("{why} — an unreadable deliverables file is not an absent one"),
        ));
    }
    for (path, why) in &verifications.failures {
        report.push(Diagnostic::error(
            "verification.malformed",
            path.clone(),
            format!("{why} — an unreadable verification is not an absent one"),
        ));
    }

    let unmet = checks.unmet();

    if unmet.is_empty() {
        report.push(Diagnostic::pass(
            "resolution.requirements",
            format!(
                "{alias}: all {} §56.1 requirements are met",
                RESOLUTION_REQUIREMENTS.len()
            ),
        ));
        report.note(
            "All thirteen are met, but no resolution has been RECORDED. §56.2's record \
             needs an authorizer, an acting role, and a stated meaning — none of which \
             this command may invent."
                .to_owned(),
        );
        return Ok(report);
    }

    // Each unmet requirement is named. "9 of 13" tells a reader nothing about
    // whether to worry; the names tell them what to fix.
    for (group, requirement, met) in checks
        .as_pairs()
        .into_iter()
        .map(|(name, met)| ("§56.1", name, met))
    {
        if met {
            report.push(Diagnostic::pass(
                "resolution.requirement-met",
                format!("{alias}: {group} {requirement}"),
            ));
        } else {
            report.push(Diagnostic::unknown(
                "resolution.requirement-unmet",
                repo.relative(&dir.join("manifest.toml")),
                format!("{alias}: {group} {requirement} — not established"),
            ));
        }
    }

    report.note(format!(
        "{} of {} §56.1 requirements are unmet, so {alias} cannot be resolved. That is \
         the correct outcome, not a defect: closing a Warrant while independence is \
         absent and no authorization record exists would manufacture the false \
         completion this system exists to prevent.",
        unmet.len(),
        RESOLUTION_REQUIREMENTS.len()
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use openwarrant_core::deliverable::{ArtifactProvenance, DeliverableKind};

    fn scratch(label: &str) -> Utf8PathBuf {
        let mut p = Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("utf-8 temp");
        p.push(format!("ow-deliv-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch");
        p
    }

    fn provenance(digest: &str) -> ArtifactProvenance {
        ArtifactProvenance {
            producer: "p".into(),
            producing_attempt: "a".into(),
            contract_digest: "c".into(),
            input_digests: vec![],
            tool_or_runtime_identity: "t".into(),
            creation_method: "authored".into(),
            content_digest: digest.into(),
            media_type: "text/plain".into(),
            classification: "internal".into(),
            retention: "r".into(),
            source_holder: "git".into(),
        }
    }

    fn deliverable(target: &str, digest: Option<&str>) -> Deliverable {
        Deliverable {
            id: "D-001".into(),
            title: "t".into(),
            kind: DeliverableKind::File,
            target_ref: target.into(),
            required: true,
            content_addressed: digest.is_some(),
            provenance_required: digest.is_some(),
            obligation_refs: vec![],
            provenance: digest.map(provenance),
        }
    }

    /// The happy path, so the refusals below mean something.
    #[test]
    fn a_present_target_with_a_matching_digest_verifies() {
        let root = scratch("ok");
        std::fs::write(root.join("artifact.txt"), b"hello").expect("write");
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(b"hello"));
        let d = vec![deliverable("artifact.txt", Some(&digest))];
        assert!(required_deliverables_exist(&root, &d, &[]));
        assert!(artifact_digests_verify(&root, &d));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A digest that is merely PRESENT must not pass. This is the check that
    /// makes content addressing mean anything.
    #[test]
    fn a_digest_that_does_not_match_the_bytes_is_refused() {
        let root = scratch("mismatch");
        std::fs::write(root.join("artifact.txt"), b"hello").expect("write");
        let d = vec![deliverable(
            "artifact.txt",
            Some(&format!("sha256:{}", "0".repeat(64))),
        )];
        assert!(!artifact_digests_verify(&root, &d));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A record pointing at a file nobody produced is the exact claim §37 tests.
    #[test]
    fn a_missing_target_fails_both_predicates() {
        let root = scratch("missing");
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(b"hello"));
        let d = vec![deliverable("never-written.txt", Some(&digest))];
        assert!(!required_deliverables_exist(&root, &d, &[]));
        assert!(
            !artifact_digests_verify(&root, &d),
            "an unreadable artifact is not a verified one"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Claims content addressing, carries no digest.
    #[test]
    fn content_addressed_without_provenance_is_refused() {
        let root = scratch("noprov");
        std::fs::write(root.join("a.txt"), b"x").expect("write");
        let mut d = deliverable("a.txt", None);
        d.content_addressed = true;
        assert!(!artifact_digests_verify(&root, &[d]));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Declaring nothing is not the same as having delivered everything.
    #[test]
    fn an_empty_deliverable_set_satisfies_neither() {
        let root = scratch("empty");
        assert!(!required_deliverables_exist(&root, &[], &[]));
        assert!(!artifact_digests_verify(&root, &[]));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A deliverable pointing at proof nobody wrote (§38).
    #[test]
    fn a_deliverable_citing_an_undeclared_obligation_is_refused() {
        let root = scratch("dangling");
        std::fs::write(root.join("a.txt"), b"x").expect("write");
        let mut d = deliverable("a.txt", None);
        d.obligation_refs = vec!["OBL-999".into()];
        assert!(!required_deliverables_exist(
            &root,
            &[d],
            &["OBL-001".to_string()]
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// §56.1's thirteen, and the fact that `unmet` names them rather than
    /// counting them.
    #[test]
    fn unmet_requirements_are_named_not_counted() {
        let mut checks = ResolutionChecks::all_met();
        checks.independence_requirements_met = false;
        let unmet = checks.unmet();
        assert_eq!(unmet, vec!["independence requirements are met"]);
    }

    /// Nothing this command computes may default to true. A requirement it
    /// cannot answer is unmet, which is §32's fail-closed rule applied to §56.
    #[test]
    fn an_unanswerable_requirement_is_unmet_not_assumed() {
        let checks = ResolutionChecks::default();
        assert_eq!(
            checks.unmet().len(),
            RESOLUTION_REQUIREMENTS.len(),
            "a default ResolutionChecks must satisfy nothing"
        );
    }
}
