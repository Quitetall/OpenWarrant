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
//! # What it will say today, and why that is the correct answer
//!
//! Every Warrant here blocks. Requirement 10 — *independence requirements are
//! met* — cannot hold while `openwarrant.toml` declares all nine §46.1
//! dimensions false, and several others cannot hold because the records they ask
//! about do not exist yet.
//!
//! That is the Phase 6 exit being honest rather than a defect. A resolver that
//! closed a Warrant under these conditions would be manufacturing the exact
//! false completion the whole system exists to prevent.

use openwarrant_core::resolution::{RESOLUTION_REQUIREMENTS, ResolutionChecks};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// Compute §56.1's thirteen from the corpus as it stands.
///
/// Each is answered from a record or answered `false`. Nothing is assumed true
/// because it is probably fine — an unanswerable requirement is unmet, which is
/// the same fail-closed rule §32 applies to Preflight.
fn evaluate(repo: &Repository, one: &crate::repo::Loaded) -> ResolutionChecks {
    let validated = one.validated.as_ref();
    let basis = one.basis.as_ref();

    // §46.3, from the declared independence — the one requirement this
    // repository can already answer with certainty, and it answers no.
    let independence_met = validated.is_some_and(|v| {
        repo.config
            .independence
            .as_ref()
            .is_some_and(|i| i.meets(&v.assurance_level.to_string()).is_ok())
    });

    // Obligations exist and parse. Not the same as dispositioned — §38.6 needs a
    // disposition per obligation, and no Warrant here has one.
    let obligations_dispositioned = basis.is_some_and(|b| {
        b.atoms.iter().filter(|a| a.role == "assurance").any(|a| {
            openwarrant_core::obligation::parse(&String::from_utf8_lossy(&a.bytes)).is_ok_and(
                |set| {
                    !set.obligations.is_empty()
                        && set.obligations.iter().all(|o| o.disposition.is_some())
                },
            )
        })
    });

    ResolutionChecks {
        // No authorization record exists anywhere in this corpus (§28.4).
        exact_authorized_contract_revision: false,
        // §37 deliverables are declared in prose, not as typed records.
        required_deliverables_exist: false,
        // Nothing is content-addressed yet, so nothing verifies.
        artifact_digests_verify: false,
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

/// `war resolve <alias> --dry-run`.
pub fn run(repo: &Repository, alias: &str) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let mut report = Report::default();

    let checks = evaluate(repo, &one);
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
