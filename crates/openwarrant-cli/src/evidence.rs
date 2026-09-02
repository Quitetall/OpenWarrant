// SPDX-License-Identifier: AGPL-3.0-or-later
//! Gate receipts as committed evidence (§44.6, §56.1 requirement 5).
//!
//! # The gap this closes
//!
//! `war gate --run --record` writes a run and its §44.6 receipt under the
//! receipts path, which is gitignored on purpose: a receipt carries wall-clock
//! times, and committing one as a side effect of running would dirty the tree
//! on every `cargo xtask gate`. That left requirement 5 — "every required gate
//! has admissible result" — with nowhere to read from that a fresh clone could
//! reproduce, so the corpus projection reported it unmet for every Warrant and
//! said so in a caveat.
//!
//! A receipt becomes evidence when it is recorded FOR a Warrant, bound to the
//! contract it was run against, and committed beside that Warrant's other
//! records. That is what `war evidence record <alias>` does: it runs the gates
//! the Warrant's assurance atom cites, mints the receipt directly into
//! `docs/warrants/<alias>/gate-runs/` with `subject_digests` naming the
//! Warrant's current contract digest, and stops. Nothing here decides whether
//! the Warrant is done; that remains §56's question.
//!
//! # What makes a recorded run admissible
//!
//! Four things, all checked at read time, none trusted from the file:
//!
//! 1. the run satisfies §44.5 (askable, completed, pass);
//! 2. a receipt exists beside it, and its `receipt_digest` RECOMPUTES over the
//!    other fields — an edited receipt is not a receipt;
//! 3. the receipt's verdict is the run's verdict;
//! 4. the receipt's `subject_digests` name THIS Warrant's current contract
//!    digest. A receipt bound to an earlier revision is a true record of a run
//!    that happened, and it is not evidence about the contract as it stands.
//!
//! The fourth is the one that matters most: without it, a receipt recorded once
//! would keep satisfying requirement 5 through every later edit to the
//! contract, which is exactly the "green forever" failure the receipts
//! `.gitignore` comment warns about.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::canonical::sha256_digest;
use openwarrant_compiler::digest::DigestDomain;
use openwarrant_core::{GateReceipt, GateRun};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// The directory under a Warrant where its recorded runs live.
pub const GATE_RUNS_DIR: &str = "gate-runs";

/// One recorded run and, if present, its receipt.
#[derive(Debug, Clone)]
pub struct GateEvidence {
    pub run: GateRun,
    pub run_path: Utf8PathBuf,
    pub receipt: Option<GateReceipt>,
    pub receipt_path: Utf8PathBuf,
}

/// The subject-digest form a receipt uses to name a contract.
#[must_use]
pub fn contract_subject(contract_digest: &str) -> String {
    let bare = contract_digest.trim_start_matches("sha256:");
    format!("contract:sha256:{bare}")
}

/// Whether a receipt's seal recomputes over its other fields.
///
/// The digest was computed with `receipt_digest` empty (see
/// `gate_cmd::receipt::mint`), so it is recomputed the same way.
#[must_use]
pub fn receipt_digest_recomputes(receipt: &GateReceipt) -> bool {
    let mut unsealed = receipt.clone();
    unsealed.receipt_digest = String::new();
    match sha256_digest(DigestDomain::GateRun, &unsealed) {
        Ok(d) => receipt.receipt_digest == format!("sha256:{d}"),
        Err(_) => false,
    }
}

/// Why a recorded run is not admissible for the contract as it stands.
///
/// `Ok(())` means every check above passed. The reasons are sentences because
/// they are printed by `war check` and read by the person who has to act.
pub fn admissibility(evidence: &GateEvidence, contract_digest: Option<&str>) -> Result<(), String> {
    if !evidence.run.satisfies_required_pass() {
        return Err(format!(
            "the run is not a §44.5 required pass (askability {}, execution {}, verdict {})",
            evidence.run.askability, evidence.run.execution_status, evidence.run.verdict
        ));
    }
    let Some(receipt) = &evidence.receipt else {
        return Err(format!(
            "no §44.6 receipt beside the run (expected {})",
            evidence.receipt_path
        ));
    };
    if let Err(e) = receipt.validate() {
        return Err(format!("the receipt is incomplete: {e}"));
    }
    if !receipt_digest_recomputes(receipt) {
        return Err(
            "the receipt's `receipt_digest` does not recompute over its fields — it was edited \
             after it was sealed, and an edited receipt is not a receipt"
                .to_owned(),
        );
    }
    if receipt.verdict != evidence.run.verdict {
        return Err(format!(
            "the run says verdict {} and its receipt says {}",
            evidence.run.verdict, receipt.verdict
        ));
    }
    let Some(digest) = contract_digest else {
        return Err("the Warrant does not compile, so there is no contract to bind to".to_owned());
    };
    let wanted = contract_subject(digest);
    if !receipt.subject_digests.iter().any(|s| s == &wanted) {
        return Err(format!(
            "the receipt is bound to {} and the contract now compiles to {wanted} — a run \
             against an earlier revision is a record, not evidence about this one",
            if receipt.subject_digests.is_empty() {
                "no subject".to_owned()
            } else {
                receipt.subject_digests.join(", ")
            }
        ));
    }
    Ok(())
}

/// The runs that count towards requirement 5 for this contract.
#[must_use]
pub fn admissible_runs(evidence: &[GateEvidence], contract_digest: Option<&str>) -> Vec<GateRun> {
    evidence
        .iter()
        .filter(|e| admissibility(e, contract_digest).is_ok())
        .map(|e| e.run.clone())
        .collect()
}

/// Read every `gate-runs/*.run.toml` under a Warrant, with its receipt.
///
/// A run file that will not parse is an error: an unreadable record must not
/// read as an absent one. A missing receipt is NOT an error here — it is
/// reported by `admissibility`, because a run without a receipt is a true
/// state (the gate did not complete) rather than a broken file.
pub fn load(repo: &Repository, warrant_dir: &Utf8Path) -> Result<Vec<GateEvidence>, RepoError> {
    let dir = warrant_dir.join(GATE_RUNS_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(vec![]);
    };
    let mut paths: Vec<Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.as_str().ends_with(".run.toml"))
        .collect();
    paths.sort();

    let mut out = Vec::with_capacity(paths.len());
    for run_path in paths {
        let text = std::fs::read_to_string(&run_path).map_err(|source| RepoError::Io {
            context: format!("could not read {run_path}"),
            source,
        })?;
        let run: GateRun = toml::from_str(&text).map_err(|e| {
            RepoError::Message(format!("{}: not a gate run: {e}", repo.relative(&run_path)))
        })?;
        let receipt_path = Utf8PathBuf::from(
            run_path
                .as_str()
                .strip_suffix(".run.toml")
                .map(|stem| format!("{stem}.receipt.json"))
                .unwrap_or_default(),
        );
        let receipt = if receipt_path.is_file() {
            let body = std::fs::read_to_string(&receipt_path).map_err(|source| RepoError::Io {
                context: format!("could not read {receipt_path}"),
                source,
            })?;
            Some(serde_json::from_str::<GateReceipt>(&body).map_err(|e| {
                RepoError::Message(format!(
                    "{}: not a §44.6 receipt: {e}",
                    repo.relative(&receipt_path)
                ))
            })?)
        } else {
            None
        };
        out.push(GateEvidence {
            run,
            run_path,
            receipt,
            receipt_path,
        });
    }
    Ok(out)
}

/// `war evidence record <alias> [--gate <key>]`.
///
/// Runs each gate the Warrant cites (or the one named), for real, and mints
/// the receipt into the Warrant's `gate-runs/` bound to its current contract
/// digest. Refuses a Warrant that does not compile: there is nothing to bind
/// the receipt to. Refuses a named gate the Warrant does not cite: a receipt
/// for a gate no obligation asks about is not evidence of anything.
pub fn record(repo: &Repository, alias: &str, only: Option<&str>) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!(
            "{alias}: the manifest did not validate, so there is no contract to bind a receipt to"
        )));
    };
    let ir = openwarrant_compiler::lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("{alias}: could not compile contract: {e}")))?;
    let contract_digest = ir
        .contract_digest()
        .map_err(|e| RepoError::Message(format!("{alias}: could not digest contract: {e}")))?;

    let cited: Vec<String> = crate::resolve::cited_gate_keys(&one);
    if cited.is_empty() {
        return Err(RepoError::Message(format!(
            "{alias}: the assurance atom cites no gate, so there is no required result to record. \
             Requirement 5 needs an amendment that names one, not a receipt for a gate nobody asked for"
        )));
    }
    let targets: Vec<String> = match only {
        Some(key) => {
            let key = key.trim_start_matches("gate://");
            if !cited.iter().any(|c| c == key) {
                return Err(RepoError::Message(format!(
                    "{alias} does not cite gate://{key}; it cites {}",
                    cited.join(", ")
                )));
            }
            vec![key.to_owned()]
        }
        None => cited,
    };

    let out_dir = dir.join(GATE_RUNS_DIR);
    let subject = vec![contract_subject(&contract_digest)];
    let mut report = Report::default();
    report.note(format!(
        "{alias}: recording {} gate run(s) bound to {}",
        targets.len(),
        subject[0]
    ));
    for key in &targets {
        let sub = crate::gate_cmd::run(repo, true, Some(key), true, &subject, &[], Some(&out_dir))?;
        for d in sub.diagnostics {
            report.push(d);
        }
        for n in sub.notes {
            report.note(n);
        }
    }
    Ok(report)
}

/// `war check` rules for a Warrant's recorded runs.
///
/// Every recorded run is reported: admissible ones as `evidence.admissible`,
/// the rest by why they are not. A stale binding is a WARNING — the file is a
/// true record — and every other defect is an ERROR, because a run file whose
/// receipt does not reseal, or disagrees with it, is a claim rather than a
/// record and must not sit in the tree looking like one.
pub fn check(
    repo: &Repository,
    warrant_dir: &Utf8Path,
    alias: &str,
    contract_digest: Option<&str>,
    report: &mut Report,
) {
    let evidence = match load(repo, warrant_dir) {
        Ok(e) => e,
        Err(e) => {
            report.push(Diagnostic::error(
                "evidence.malformed",
                repo.relative(&warrant_dir.join(GATE_RUNS_DIR)),
                format!("{alias}: {e} — an unreadable run is not an absent one"),
            ));
            return;
        }
    };
    for e in &evidence {
        match admissibility(e, contract_digest) {
            Ok(()) => report.push(Diagnostic::pass(
                "evidence.admissible",
                format!(
                    "{alias}: {} · required pass, receipt reseals and is bound to the current contract",
                    e.run.gate
                ),
            )),
            Err(why) => {
                let stale = e.receipt.as_ref().is_some_and(|r| {
                    receipt_digest_recomputes(r)
                        && r.verdict == e.run.verdict
                        && r.validate().is_ok()
                        && e.run.satisfies_required_pass()
                });
                let path = repo.relative(&e.run_path);
                if stale {
                    report.push(Diagnostic::warn(
                        "evidence.stale-binding",
                        path,
                        format!("{alias}: {} · {why}", e.run.gate),
                    ));
                } else if e.run.satisfies_required_pass() {
                    report.push(Diagnostic::error(
                        "evidence.receipt-invalid",
                        path,
                        format!("{alias}: {} · {why}", e.run.gate),
                    ));
                } else {
                    report.push(Diagnostic::warn(
                        "evidence.not-a-pass",
                        path,
                        format!("{alias}: {} · {why}", e.run.gate),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::gate_run::Verdict;

    fn receipt(verdict: Verdict, subject: &str) -> GateReceipt {
        let mut r = GateReceipt {
            run_id: "GR-x".into(),
            gate_definition_digest: "sha256:aa".into(),
            gate_binding_digest: "unbound:x".into(),
            subject_digests: vec![subject.to_owned()],
            fixture_digests: vec![],
            runner: "test".into(),
            runtime_environment: "test".into(),
            arguments: vec![],
            working_directory: "/".into(),
            started_at: "2026-09-02T00:00:00Z".into(),
            completed_at: "2026-09-02T00:00:01Z".into(),
            exit_result: "pass".into(),
            selected_test_count: 0,
            selected_test_manifest: vec![],
            raw_evidence_refs: vec![],
            stdout_ref: "o".into(),
            stderr_ref: "e".into(),
            resource_usage: "none".into(),
            verdict,
            receipt_digest: String::new(),
        };
        r.receipt_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateRun, &r).expect("digest")
        );
        r
    }

    fn run(verdict: &str) -> GateRun {
        toml::from_str(&format!(
            "id = \"GR-x\"\ngate = \"g@1.0.0\"\naskability = \"askable\"\n\
             execution_status = \"completed\"\nverdict = \"{verdict}\"\n"
        ))
        .expect("run")
    }

    fn evidence(run: GateRun, receipt: Option<GateReceipt>) -> GateEvidence {
        GateEvidence {
            run,
            run_path: "x.run.toml".into(),
            receipt,
            receipt_path: "x.receipt.json".into(),
        }
    }

    const C: &str = "sha256:0123";

    #[test]
    fn a_sealed_bound_passing_receipt_is_admissible() {
        let e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, &contract_subject(C))),
        );
        assert!(admissibility(&e, Some(C)).is_ok());
        assert_eq!(admissible_runs(&[e], Some(C)).len(), 1);
    }

    #[test]
    fn an_edited_receipt_does_not_reseal() {
        let mut r = receipt(Verdict::Fail, &contract_subject(C));
        r.verdict = Verdict::Pass; // the tampering a plant performs
        let e = evidence(run("pass"), Some(r));
        let why = admissibility(&e, Some(C)).unwrap_err();
        assert!(why.contains("does not recompute"), "{why}");
    }

    #[test]
    fn a_receipt_for_an_earlier_contract_is_a_record_not_evidence() {
        let e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, "contract:sha256:ffff")),
        );
        let why = admissibility(&e, Some(C)).unwrap_err();
        assert!(why.contains("earlier revision"), "{why}");
        assert!(admissible_runs(&[e], Some(C)).is_empty());
    }

    #[test]
    fn a_run_without_a_receipt_is_not_admissible() {
        let e = evidence(run("pass"), None);
        assert!(
            admissibility(&e, Some(C))
                .unwrap_err()
                .contains("no §44.6 receipt")
        );
    }

    #[test]
    fn a_run_and_receipt_that_disagree_are_refused() {
        let e = evidence(
            run("pass"),
            Some(receipt(Verdict::Fail, &contract_subject(C))),
        );
        assert!(
            admissibility(&e, Some(C))
                .unwrap_err()
                .contains("says verdict")
        );
    }

    #[test]
    fn a_failing_run_is_not_a_pass_however_well_sealed() {
        let e = evidence(
            run("fail"),
            Some(receipt(Verdict::Fail, &contract_subject(C))),
        );
        assert!(
            admissibility(&e, Some(C))
                .unwrap_err()
                .contains("not a §44.5")
        );
    }

    #[test]
    fn no_contract_means_nothing_to_bind_to() {
        let e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, &contract_subject(C))),
        );
        assert!(admissibility(&e, None).is_err());
    }
}
