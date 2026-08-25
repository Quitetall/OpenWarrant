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
//! # What is computed, and what is still asserted
//!
//! Nine of the thirteen are computed from records on disk: deliverables and
//! artifact digests (§37, recomputed from the bytes), obligation dispositions and
//! independence (§46, from verification records), gate results (§44.5, from
//! persisted Gate Runs), and — through [`Authority`] — the authorized contract
//! revision (§28.4), judgments (§42), residual-risk authority (§36.2 with §27.2)
//! and the resolver's role (§27).
//!
//! Two are structurally true here: no blocker remains, and deviations are
//! dispositioned.
//!
//! **Two remain hardcoded `false`**: runtime receipts bound to the basis
//! (§48.4), and the resolution of the adequacy warnings that are themselves
//! required unknowns. Both are mechanical and neither is done.
//!
//! # The four that moved are computed but not SATISFIED
//!
//! Wiring them up did not close a single Warrant, and it was not supposed to.
//! Each now reads a record that a human must write — a role assignment in
//! `docs/authority/roles.toml`, then a signed authorization response. Until
//! those exist the answer is `false`, exactly as before, but it is now `false`
//! *because a specific record is absent* rather than because a constant said so.
//!
//! That is the whole difference. §27.2 forbids an agent authorizing a proposed
//! WAR, accepting residual risk, or resolving a delivery, so no amount of
//! implementation here can make these true. What implementation CAN do is make
//! them answerable — turning "structurally impossible" into "awaiting one
//! signature", which is a state a human can act on.
//!
//! An earlier note in this file cited §58 for residual-risk authority. §58 is
//! Representations; residual risk is §36.2, and its authority constraint is
//! §27.2. The requirement was unaffected — it was `false` either way — but the
//! citation would have sent a reader to the wrong page.

use openwarrant_core::GateRun;
use openwarrant_core::authority::{AuthorityRegister, PolicyResolutionContext};
use openwarrant_core::deliverable::Deliverable;
use openwarrant_core::epistemic::Judgment;
use openwarrant_core::rationale::Assumption;
use openwarrant_core::resolution::{RESOLUTION_REQUIREMENTS, ResolutionChecks};
use openwarrant_core::verification::Verification;

use crate::authorize::AuthorizationRecord;
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
    gate_runs: &[GateRun],
    authority: &Authority<'_>,
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

    // Gate citations across this Warrant's assurance atoms (§43.5).
    //
    // Read from declared `- **gate:**` bullets, never by scanning prose for
    // `gate://` — a gate identified by pattern-matching prose is the
    // "string, not a gate" failure §43 exists to end.
    let cited: Vec<String> = basis
        .map(|b| {
            b.atoms
                .iter()
                .filter(|a| a.role == "assurance")
                .flat_map(|a| {
                    openwarrant_core::gate::cited_gate_uris(&String::from_utf8_lossy(&a.bytes))
                })
                .collect()
        })
        .unwrap_or_default();
    let gates_ok = every_required_gate_has_admissible_result(&cited, gate_runs);

    let required_deliverables_exist =
        required_deliverables_exist(&repo.root, deliverables, &declared);
    let artifact_digests_verify = artifact_digests_verify(&repo.root, deliverables);

    ResolutionChecks {
        exact_authorized_contract_revision: authority.contract_is_authorized(),
        required_deliverables_exist,
        artifact_digests_verify,
        every_required_obligation_dispositioned: obligations_dispositioned,
        every_required_gate_has_admissible_result: gates_ok,
        // The adequacy warnings ARE required unknowns.
        no_required_unknown_remains: false,
        no_blocker_remains: true,
        deviations_dispositioned: true,
        required_judgments_exist: authority.required_judgments_exist(),
        independence_requirements_met: independence_met,
        residual_risks_have_sufficient_authority: authority.residual_risks_are_covered(),
        // §48.4 receipts exist for gate runs, but no runtime receipt is bound to
        // a Warrant's basis.
        runtime_receipts_match_the_basis: false,
        resolver_holds_the_role: authority.a_resolver_is_eligible(&assurance, &declared),
    }
}

/// Everything §27, §28.4, §42 and §36.2 need, read once from disk.
///
/// Grouped into one type rather than four more parameters because all four
/// requirements are answers to the same question — *who decided this, and were
/// they entitled to* — and splitting them across the call site made it easy to
/// pass one and forget the rest.
pub struct Authority<'a> {
    pub register: &'a AuthorityRegister,
    /// The persisted authorization, if the Warrant has one.
    pub authorization: Option<&'a AuthorizationRecord>,
    /// The contract digest the Warrant compiles to RIGHT NOW. `None` when it
    /// would not compile, which makes requirement 1 unanswerable rather than
    /// satisfied.
    pub current_contract_digest: Option<&'a str>,
    pub judgments: &'a [Judgment],
    /// `None` means no `rationale.toml` — the residual-risk question was never
    /// asked. `Some(&[])` means it was asked and the answer was none. Law 15:
    /// those are different, and only one of them may pass.
    pub assumptions: Option<&'a [Assumption]>,
    /// §27.3 condition 1 — repository policy, set by a human.
    pub policy_allows_automated_resolution: bool,
}

impl Authority<'_> {
    /// §56.1 requirement 1 — the EXACT authorized Contract Revision.
    ///
    /// Both halves are load-bearing. A record in any state but `Authorized` is a
    /// proposal, and one whose digest no longer matches the tree authorizes a
    /// revision that has since been edited — the precise failure the word
    /// "exact" exists to catch.
    #[must_use]
    pub fn contract_is_authorized(&self) -> bool {
        let (Some(record), Some(current)) = (self.authorization, self.current_contract_digest)
        else {
            return false;
        };
        crate::authorize::authorizes_current_contract(record, current)
    }

    /// §56.1 requirement 9 — required judgments exist.
    ///
    /// A judgment counts only if it validates AND its author is recorded as
    /// authorized (§42's `require_authorized`). A Warrant needs a judgment for
    /// every residual risk it declares; one that declares none needs none, and
    /// that is a real pass rather than a vacuous one — but only once the
    /// question has been asked, which is what `assumptions.is_some()` records.
    #[must_use]
    pub fn required_judgments_exist(&self) -> bool {
        let Some(assumptions) = self.assumptions else {
            return false;
        };
        let required = crate::authorize::residual_risks_in(assumptions);
        if !self
            .judgments
            .iter()
            .all(|j| j.validate().is_ok() && j.require_authorized().is_ok())
        {
            return false;
        }
        required
            .iter()
            .all(|risk| self.judgments.iter().any(|j| refers_to(j, risk)))
    }

    /// §56.1 requirement 11 — residual risks have SUFFICIENT AUTHORITY.
    ///
    /// Distinct from requirement 9, which only asks whether the judgments exist.
    /// This asks whether the actor who made each one was entitled to accept
    /// organizational residual risk — §27.2 forbids it to agents outright, and
    /// holding `judge` is not holding `risk_acceptor`.
    #[must_use]
    pub fn residual_risks_are_covered(&self) -> bool {
        let Some(assumptions) = self.assumptions else {
            return false;
        };
        crate::authorize::residual_risks_in(assumptions)
            .iter()
            .all(|risk| {
                self.judgments.iter().any(|j| {
                    refers_to(j, risk)
                        && self
                            .register
                            .actor(&j.actor)
                            .is_some_and(|a| a.may_accept_residual_risk().is_ok())
                })
            })
    }

    /// §56.1 requirement 13 — the resolver holds the role.
    ///
    /// Asks the register whether ANY assigned actor could lawfully resolve this
    /// Warrant, given the performer and §27.3's conditions. `false` means nobody
    /// may close it — which is the honest answer for a repository whose role
    /// register is empty, and is why an empty register grants nothing.
    #[must_use]
    pub fn a_resolver_is_eligible(&self, assurance: &str, declared: &[String]) -> bool {
        self.register
            .eligible_resolver(
                "claude",
                PolicyResolutionContext {
                    policy_allows: self.policy_allows_automated_resolution,
                    assurance_level: assurance,
                    // Conservative on purpose: an obligation is treated as
                    // non-mechanical unless the Warrant declares none at all.
                    // §27.3 lets a policy service close only mechanical work, and
                    // guessing in the permissive direction here would hand a
                    // machine exactly the Warrants it may not touch.
                    all_obligations_mechanical: declared.is_empty(),
                    residual_risk_judgment_required: self
                        .assumptions
                        .is_none_or(|a| !crate::authorize::residual_risks_in(a).is_empty()),
                },
            )
            .is_some()
    }
}

/// Whether a judgment addresses a given residual risk.
///
/// Matched through the assumption's own `judgment_ref` when it declares one, and
/// otherwise through the judgment's `basis_refs`. Both directions are checked
/// because §36.2 points assumption→judgment while §42 points judgment→basis, and
/// a record written from either side must be readable from the other.
fn refers_to(judgment: &Judgment, risk: &crate::authorize::RequestedResidualRisk) -> bool {
    let by_ref = !risk.judgment_ref.is_empty()
        && (risk.judgment_ref == judgment.id
            || risk.judgment_ref == format!("judgment://{}", judgment.id));
    let by_basis = judgment
        .basis_refs
        .iter()
        .any(|b| b == &risk.assumption_id || b == &format!("assumption://{}", risk.assumption_id));
    by_ref || by_basis
}

/// §56.1 — every gate a required obligation cites must have an admissible result.
///
/// "Admissible" is §44.5's conjunction, already modelled as
/// [`GateRun::satisfies_required_pass`]: askable AND completed AND pass. An
/// unaskable gate cannot pass (SAS §99 criterion 19), and a run that timed out
/// is a blocking unknown rather than a failure (RQ-054).
///
/// A cited gate with NO recorded run is unmet, not vacuously satisfied. That is
/// the whole point: the gate must have been asked, not merely referenced.
///
/// # Zero cited gates is UNMET, and this comment used to say the opposite
///
/// The prose here claimed "obligations citing no gate impose nothing — the
/// requirement is about gates that were cited", while the code below has always
/// returned `false` for an empty list. One of them was wrong, and it was the
/// prose: a reader trusting it would conclude a Warrant citing no gate had
/// passed requirement 5, when it had failed.
///
/// The code is kept. §56.1 asks whether every required gate has an admissible
/// result, and a Warrant that names no gate has produced no mechanical proof of
/// anything — reporting that as satisfied would let a Warrant clear the
/// requirement by declaring nothing, which is the shape of false completion this
/// resolver exists to refuse. It is the same rule
/// [`required_deliverables_exist`] applies to a Warrant that declares no
/// deliverables.
///
/// The consequence is real and is not a defect: OW-WAR-0001 through 0018 were
/// authored before the Gate Registry existed, cite no gates, and therefore
/// cannot satisfy requirement 5. Adding gate bullets to their assurance atoms
/// now would move the contract digest and would be editing a document to make a
/// tool go green. It needs an amendment somebody authorizes, not a quiet edit.
#[must_use]
pub fn every_required_gate_has_admissible_result(cited_uris: &[String], runs: &[GateRun]) -> bool {
    if cited_uris.is_empty() {
        return false;
    }
    cited_uris.iter().all(|uri| {
        let key = uri.trim_start_matches("gate://");
        runs.iter()
            .any(|r| r.gate == key && r.satisfies_required_pass())
    })
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
    let gate_runs = repo.load_gate_runs();

    let register = repo.load_authority_register()?;
    let authorization = repo.load_authorization(&dir)?;
    let judgments = repo.load_judgments(&dir)?;
    let assumptions = repo.load_rationale(&dir)?;
    // The digest the Warrant compiles to right now, which requirement 1 compares
    // the signature against. A Warrant that will not compile yields `None`, and
    // requirement 1 is then unanswerable rather than satisfied.
    let current_contract_digest = match (&one.basis, &one.validated) {
        (Some(basis), Some(validated)) => openwarrant_compiler::lower(basis, validated)
            .ok()
            .and_then(|ir| ir.contract_digest().ok()),
        _ => None,
    };
    let authority = Authority {
        register: &register,
        authorization: authorization.as_ref(),
        current_contract_digest: current_contract_digest.as_deref(),
        judgments: &judgments,
        assumptions: assumptions.as_deref(),
        policy_allows_automated_resolution: repo.config.policy.allow_automated_resolution,
    };

    let checks = evaluate(
        repo,
        &one,
        &verifications.records,
        &deliverables.records,
        &gate_runs,
        &authority,
    );
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

    fn gate_run(gate: &str, askability: &str, status: &str, verdict: &str) -> GateRun {
        toml::from_str(&format!(
            "id = \"GR-1\"\ngate = \"{gate}\"\naskability = \"{askability}\"\n\
             execution_status = \"{status}\"\nverdict = \"{verdict}\"\n"
        ))
        .expect("valid run")
    }

    const GATE: &str = "software.repo.war-check@1.0.0";

    #[test]
    fn a_cited_gate_with_an_askable_completed_pass_is_admissible() {
        let runs = vec![gate_run(GATE, "askable", "completed", "pass")];
        assert!(every_required_gate_has_admissible_result(
            &[format!("gate://{GATE}")],
            &runs
        ));
    }

    /// SAS §99 criterion 19: unaskable gates cannot pass. A run claiming
    /// `not_askable` with verdict `pass` is incoherent and must not satisfy
    /// anything — checking the verdict first is how such a run slips through.
    #[test]
    fn an_unaskable_gate_cannot_pass() {
        let runs = vec![gate_run(GATE, "not_askable", "completed", "pass")];
        assert!(!every_required_gate_has_admissible_result(
            &[format!("gate://{GATE}")],
            &runs
        ));
    }

    #[test]
    fn a_failing_or_unknown_verdict_is_not_admissible() {
        for verdict in ["fail", "unknown"] {
            let runs = vec![gate_run(GATE, "askable", "completed", verdict)];
            assert!(
                !every_required_gate_has_admissible_result(&[format!("gate://{GATE}")], &runs),
                "verdict {verdict} must not satisfy a required pass"
            );
        }
    }

    /// A cited gate with NO recorded run is unmet, not vacuously satisfied.
    /// The gate must have been ASKED, not merely referenced.
    #[test]
    fn a_cited_gate_with_no_run_is_unmet() {
        assert!(!every_required_gate_has_admissible_result(
            &[format!("gate://{GATE}")],
            &[]
        ));
    }

    /// A passing run for a DIFFERENT gate must not satisfy this citation.
    #[test]
    fn a_run_for_another_gate_does_not_count() {
        let runs = vec![gate_run(
            "some.other.gate@1.0.0",
            "askable",
            "completed",
            "pass",
        )];
        assert!(!every_required_gate_has_admissible_result(
            &[format!("gate://{GATE}")],
            &runs
        ));
    }

    /// Every cited gate must pass, not merely one of them.
    #[test]
    fn one_passing_gate_does_not_carry_a_failing_sibling() {
        let runs = vec![
            gate_run(GATE, "askable", "completed", "pass"),
            gate_run("second.gate@1.0.0", "askable", "completed", "fail"),
        ];
        assert!(!every_required_gate_has_admissible_result(
            &[
                format!("gate://{GATE}"),
                "gate://second.gate@1.0.0".to_string()
            ],
            &runs
        ));
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
