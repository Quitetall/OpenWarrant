// SPDX-License-Identifier: Apache-2.0
//! Gate receipts as committed evidence (§44.6, §56.1 requirement 5).
//!
//! # The gap this closes
//!
//! `war evidence gate --run --record` writes a run and its §44.6 receipt under the
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
//!
//! # And the source it ran over (OW-WAR-0133)
//!
//! The contract digest does not see the delivered code, the deliverable set
//! or the fixtures, so a fifth check is made for a Warrant not yet resolved:
//!
//! 5. every source subject the reuse rule reads still holds — the files the
//!    Gate Definition declares as `inputs` when it declares any, the tree
//!    otherwise (outside the evidence records and the compiled projections),
//!    and every declared fixture.
//!
//! A moved subject is `evidence.stale-binding`, naming it: the file is a true
//! record of a run and not evidence about the source as it stands. A subject
//! the rule needs and the receipt does not name — every receipt minted before
//! OW-WAR-0133, a run over a dirty tree, a source that cannot be read — is
//! `evidence.reuse-unknown` (Law 15): not admissible, and not a failure. Both
//! are warnings, because the file is not wrong; requirement 5 is what they
//! leave unmet. A RESOLVED Warrant is not re-evaluated: its receipts are
//! history, and its resolution keeps binding them (RQ-059).
//!
//! # And the gate it ran (OW-WAR-0136)
//!
//! 6. the Gate Definition version the run names has not been invalidated by
//!    a signed invalidation (§45, RQ-057). A receipt of an invalidated gate
//!    is a true record of a run that happened; it is not admissible for
//!    requirement 5 on a Warrant not yet resolved, and `war check` says so
//!    as `evidence.gate-invalidated`, naming the invalidation. A record
//!    nobody signed does not count, and is reported
//!    (`evidence.invalidation-not-counted`) rather than obeyed. A RESOLVED
//!    Warrant's receipts are history here too: the invalidation disputes its
//!    resolution instead (`resolution.disputed`).

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
    /// Whether the source the receipt names still holds, judged when the
    /// record was loaded (OW-WAR-0133).
    pub reuse: Reuse,
    /// Whether the run's gate is invalidated, judged when the record was
    /// loaded (OW-WAR-0136). Always `None` for a resolved Warrant.
    pub invalidation: Option<crate::invalidation::GateInvalidation>,
}

impl GateEvidence {
    /// Why this run's gate no longer counts, when a signed invalidation says so.
    #[must_use]
    pub fn invalidated(&self) -> Option<&str> {
        match &self.invalidation {
            Some(crate::invalidation::GateInvalidation::Counted(why)) => Some(why),
            _ => None,
        }
    }
}

/// Whether a receipt's source subjects still hold (OW-WAR-0133).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reuse {
    /// The Warrant is resolved. Its receipts are history, bound by the
    /// resolution, and are not re-evaluated against today's source.
    Historical,
    /// Every subject the rule reads still holds. The string names the rule.
    Holds(String),
    /// A subject the rule reads has moved. `subject` is the recorded one.
    Moved { subject: String, detail: String },
    /// The rule needs a subject the receipt does not name, or the current
    /// source cannot be named. Law 15: neither pass nor failure.
    Unknown(String),
}

/// How a recorded run stands, for the `war check` rule it earns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// `evidence.admissible`.
    Admissible,
    /// `evidence.stale-binding`: a true record bound to an earlier contract
    /// or an earlier source.
    Stale(String),
    /// `evidence.reuse-unknown`: sealed and bound to this contract, and its
    /// source cannot be shown to hold.
    ReuseUnknown(String),
    /// `evidence.receipt-invalid`: a required pass whose receipt is not one.
    ReceiptInvalid(String),
    /// `evidence.not-a-pass`.
    NotAPass(String),
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

/// How a recorded run stands against the contract and source as they are.
///
/// The checks run in the order the module comment lists; the first that
/// fails decides. The reasons are sentences because they are printed by
/// `war check` and read by the person who has to act.
#[must_use]
pub fn standing(evidence: &GateEvidence, contract_digest: Option<&str>) -> Standing {
    if !evidence.run.satisfies_required_pass() {
        return Standing::NotAPass(format!(
            "the run is not a §44.5 required pass (askability {}, execution {}, verdict {})",
            evidence.run.askability, evidence.run.execution_status, evidence.run.verdict
        ));
    }
    let Some(receipt) = &evidence.receipt else {
        return Standing::ReceiptInvalid(format!(
            "no §44.6 receipt beside the run (expected {})",
            evidence.receipt_path
        ));
    };
    if let Err(e) = receipt.validate() {
        return Standing::ReceiptInvalid(format!("the receipt is incomplete: {e}"));
    }
    if !receipt_digest_recomputes(receipt) {
        return Standing::ReceiptInvalid(
            "the receipt's `receipt_digest` does not recompute over its fields — it was edited \
             after it was sealed, and an edited receipt is not a receipt"
                .to_owned(),
        );
    }
    if receipt.verdict != evidence.run.verdict {
        return Standing::ReceiptInvalid(format!(
            "the run says verdict {} and its receipt says {}",
            evidence.run.verdict, receipt.verdict
        ));
    }
    let Some(digest) = contract_digest else {
        return Standing::ReceiptInvalid(
            "the Warrant does not compile, so there is no contract to bind to".to_owned(),
        );
    };
    let wanted = contract_subject(digest);
    if !receipt.subject_digests.iter().any(|s| s == &wanted) {
        return Standing::Stale(format!(
            "the receipt is bound to {} and the contract now compiles to {wanted} — a run \
             against an earlier revision is a record, not evidence about this one",
            if receipt.subject_digests.is_empty() {
                "no subject".to_owned()
            } else {
                receipt.subject_digests.join(", ")
            }
        ));
    }
    // A true record bound to this contract, of a gate a human has since
    // invalidated: a record, not evidence (OW-WAR-0136).
    if let Some(why) = evidence.invalidated() {
        return Standing::Stale(why.to_owned());
    }
    match &evidence.reuse {
        Reuse::Historical | Reuse::Holds(_) => Standing::Admissible,
        Reuse::Moved { subject, detail } => Standing::Stale(format!(
            "the receipt names {subject} and {detail} — a run over an earlier source is a \
             record, not evidence about this one"
        )),
        Reuse::Unknown(why) => Standing::ReuseUnknown(format!(
            "reuse UNKNOWN: {why}. Not admissible and not a failure; `war evidence record` \
             mints a receipt that names its source"
        )),
    }
}

/// Why a recorded run is not admissible for the contract and source as they
/// stand. `Ok(())` means [`standing`] is admissible.
pub fn admissibility(evidence: &GateEvidence, contract_digest: Option<&str>) -> Result<(), String> {
    match standing(evidence, contract_digest) {
        Standing::Admissible => Ok(()),
        Standing::Stale(why)
        | Standing::ReuseUnknown(why)
        | Standing::ReceiptInvalid(why)
        | Standing::NotAPass(why) => Err(why),
    }
}

/// Judge whether the source `receipt` names still holds, for a Warrant not
/// yet resolved (Q-001: declared inputs decide, the tree when there are none;
/// declared fixtures always).
#[must_use]
pub fn reuse_of(repo: &Repository, gate: &str, receipt: &GateReceipt) -> Reuse {
    use crate::gate_cmd::source;
    let ex = source::Exclusions::of(repo);
    let named = |prefix: &str| {
        receipt
            .subject_digests
            .iter()
            .find(|s| s.starts_with(prefix))
            .cloned()
    };
    let Some(declared) = source::declared(repo, gate) else {
        return Reuse::Unknown(format!(
            "no Gate Definition for {gate} is registered, so what it reads cannot be named"
        ));
    };

    // Fixtures first: whichever rule decides, a changed fixture is a changed
    // question.
    if !declared.fixtures.is_empty() && receipt.fixture_digests.is_empty() {
        return Reuse::Unknown(format!(
            "{gate} declares fixtures and the receipt names none"
        ));
    }
    for recorded in &receipt.fixture_digests {
        let Some((path, _)) = recorded.split_once("#sha256:") else {
            return Reuse::Unknown(format!("fixture digest {recorded:?} names no path"));
        };
        match source::fixture_digests(&repo.root, &[path.to_owned()]) {
            Ok(now) if now.first() == Some(recorded) => {}
            Ok(now) => {
                return Reuse::Moved {
                    subject: recorded.clone(),
                    detail: format!("the fixture now digests to {}", now.join(", ")),
                };
            }
            Err(e) => {
                return Reuse::Moved {
                    subject: recorded.clone(),
                    detail: e,
                };
            }
        }
    }

    if !declared.inputs.is_empty() {
        let Some(recorded) = named(source::INPUTS) else {
            return Reuse::Unknown(format!(
                "{gate} declares the files it reads ({}) and the receipt does not name their \
                 digest",
                declared.inputs.join(", ")
            ));
        };
        return match source::inputs_digest(&repo.root, &declared.inputs, &ex) {
            Ok(now) if now == recorded => {
                Reuse::Holds(format!("inputs {}", declared.inputs.join(", ")))
            }
            Ok(now) => Reuse::Moved {
                subject: recorded,
                detail: format!(
                    "the files {gate} declares it reads ({}) now digest to {now}",
                    declared.inputs.join(", ")
                ),
            },
            Err(e) => Reuse::Unknown(format!("the declared inputs cannot be digested: {e}")),
        };
    }

    // The (a) fallback: a gate that declares no inputs is bound to the tree.
    let Some(tree) = named(source::TREE) else {
        return Reuse::Unknown(
            "the receipt names no tree and its gate declares no inputs, so nothing says what \
             source it ran over"
                .to_owned(),
        );
    };
    if receipt.subject_digests.iter().any(|s| s == source::DIRTY) {
        return Reuse::Unknown(format!(
            "the run started from {tree} with uncommitted changes, and a dirty tree has no name"
        ));
    }
    let sha = tree.trim_start_matches(source::TREE);
    match source::moved_since(&repo.root, sha, &ex) {
        Ok(moved) if moved.is_empty() => Reuse::Holds(tree.clone()),
        Ok(moved) => Reuse::Moved {
            subject: tree,
            detail: format!(
                "{} path(s) outside the evidence records, projections, authority records, \
                 verification records and ticket bookkeeping have changed since ({}{})",
                moved.len(),
                moved.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
                if moved.len() > 3 { ", …" } else { "" }
            ),
        },
        Err(e) => Reuse::Unknown(format!("the tree {sha} cannot be compared: {e}")),
    }
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
    let Ok(entries) = crate::vfs::read_dir(&dir) else {
        return Ok(vec![]);
    };
    let mut paths: Vec<Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.as_str().ends_with(".run.toml"))
        .collect();
    paths.sort();
    let resolved = crate::vfs::is_file(warrant_dir.join("resolution.toml"));
    // One answer per gate per load: a Warrant's runs of one gate share it.
    let mut invalidations: std::collections::BTreeMap<
        String,
        Option<crate::invalidation::GateInvalidation>,
    > = std::collections::BTreeMap::new();

    let mut out = Vec::with_capacity(paths.len());
    for run_path in paths {
        let text = crate::vfs::read_to_string(&run_path).map_err(|source| RepoError::Io {
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
        let receipt = if crate::vfs::is_file(&receipt_path) {
            let body =
                crate::vfs::read_to_string(&receipt_path).map_err(|source| RepoError::Io {
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
        let reuse = match &receipt {
            _ if resolved => Reuse::Historical,
            Some(r) => reuse_of(repo, &run.gate, r),
            None => Reuse::Unknown("no receipt".to_owned()),
        };
        let invalidation = if resolved {
            None
        } else {
            invalidations
                .entry(run.gate.clone())
                .or_insert_with(|| crate::invalidation::gate_invalidation(repo, &run.gate))
                .clone()
        };
        out.push(GateEvidence {
            run,
            run_path,
            receipt,
            receipt_path,
            reuse,
            invalidation,
        });
    }
    Ok(out)
}

/// The `sync.receipt_attached` payload: what its journal key is made of.
fn attached_payload(key: &str, verdict: &str, receipt_digest: &str) -> String {
    format!(
        "{{\"gate\":\"{key}\",\"verdict\":\"{verdict}\",\"receipt_digest\":\"{receipt_digest}\"}}"
    )
}

/// `war evidence record <alias> [--gate <key>]`.
///
/// Runs each gate the Warrant cites (or the one named), for real, and mints
/// the receipt into the Warrant's `gate-runs/` bound to its current contract
/// digest. Refuses a Warrant that does not compile: there is nothing to bind
/// the receipt to. Refuses a named gate the Warrant does not cite: a receipt
/// for a gate no obligation asks about is not evidence of anything.
///
/// `evidence_ref` is the Bonsai binding (t-dec1): the Bonsai evidence gate
/// verifies a supplied `war admin bonsai check` document, and its receipt binds
/// that document by bytes, `file:<path>#sha256:<digest>`, beside the
/// contract and deliverable subjects. The Bonsai gate is not run without
/// one (`evidence.bonsai-evidence-ref-required`, naming the remedy); a
/// reference is refused when no Bonsai gate is being recorded.
pub fn record(
    repo: &Repository,
    alias: &str,
    only: Option<&str>,
    evidence_ref: Option<&str>,
) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!(
            "{alias}: the manifest did not validate, so there is no contract to bind a receipt to"
        )));
    };
    // OW-ADR-0031: a kind without `evidence` binds no receipt.
    one.require(&repo.profiles, openwarrant_core::Capability::Evidence)?;
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

    if let Some(r) = evidence_ref
        && !targets.iter().any(|k| crate::gate_cmd::is_bonsai_gate(k))
    {
        return Err(RepoError::Message(format!(
            "--evidence-ref {r:?} is a Bonsai binding, and no Bonsai evidence gate is being \
             recorded; it is taken only with --gate software.repo.bonsai-evidence@<version>"
        )));
    }

    let out_dir = dir.join(GATE_RUNS_DIR);
    // OW-WAR-0133: the deliverable set's bytes beside the contract. The tree,
    // the declared inputs and the fixtures are added by the runner, which
    // observes them before each gate is spawned.
    let set: Vec<(String, String)> = repo
        .load_deliverables(&dir)?
        .records
        .iter()
        .map(|d| (d.id.clone(), d.target_ref.clone()))
        .collect();
    let subject = vec![
        contract_subject(&contract_digest),
        crate::gate_cmd::source::deliverables_digest(&repo.root, &set),
    ];
    let mut report = Report::default();
    report.note(format!(
        "{alias}: recording {} gate run(s) bound to {}",
        targets.len(),
        subject[0]
    ));
    // OW-WAR-0130 (§67.4), before the first write. A gate whose run is
    // already on file, admissible for THIS contract, and journalled by this
    // performer is an equivalent retry: the run is not repeated, nothing is
    // written, and the act says it replayed. A run that failed, went stale,
    // or never reached the journal is not replayed; it is asked again. The
    // same receipt journalled by another actor is a conflict, and no gate
    // runs.
    let actor = format!("agent://{}", repo.performer());
    let on_file = load(repo, &dir)?;
    let mut fresh: Vec<&String> = Vec::new();
    for key in &targets {
        let prior = on_file.iter().find_map(|e| {
            let receipt = e.receipt.as_ref()?;
            (&e.run.gate == key && admissibility(e, Some(&contract_digest)).is_ok())
                .then_some((e, receipt))
        });
        let Some((e, receipt)) = prior else {
            fresh.push(key);
            continue;
        };
        let payload = attached_payload(key, &receipt.verdict.to_string(), &receipt.receipt_digest);
        match crate::journal_cmd::already_recorded(
            &dir,
            crate::journal_cmd::RECEIPT_ATTACHED,
            &payload,
            &actor,
        )? {
            crate::journal_cmd::Prior::Fresh => fresh.push(key),
            crate::journal_cmd::Prior::Conflict { recorded_by } => {
                report.push(crate::journal_cmd::conflict(
                    repo.relative(&dir.join(crate::journal_cmd::FILE)),
                    crate::journal_cmd::RECEIPT_ATTACHED,
                    &recorded_by,
                    &actor,
                ));
                return Ok(report);
            }
            crate::journal_cmd::Prior::Equivalent { occurred_at } => {
                report.push(Diagnostic::pass(
                    "evidence.replayed",
                    format!(
                        "{alias}: {key} already has an admissible {} run bound to {}, journalled \
                         at {occurred_at} → {}; an equivalent retry replays, runs nothing and \
                         writes nothing",
                        receipt.verdict,
                        subject[0],
                        repo.relative(&e.run_path)
                    ),
                ));
            }
        }
    }
    for key in fresh {
        let refs: Vec<String> = if crate::gate_cmd::is_bonsai_gate(key) {
            let Some(r) = evidence_ref else {
                report.push(Diagnostic::error(
                    "evidence.bonsai-evidence-ref-required",
                    key.clone(),
                    format!(
                        "{alias}: {key} verifies a supplied Bonsai evidence document, and its \
                         receipt binds that document by bytes; nothing was run. Remedy: write a \
                         passing document with `war admin bonsai check --warrant {alias} --base <sha> \
                         --head <sha> --bonsai <binary>` to the file the gate definition passes \
                         to --evidence, commit it (an untracked file marks the run \
                         worktree:dirty), then `war evidence record {alias} --gate {key} \
                         --evidence-ref file:<that path>#sha256:<its sha256>`"
                    ),
                ));
                continue;
            };
            vec![r.to_owned()]
        } else {
            Vec::new()
        };
        let sub = crate::gate_cmd::run_for(
            repo,
            true,
            Some(key),
            true,
            &subject,
            &refs,
            Some(&out_dir),
            Some(alias),
        )?;
        let minted = sub.diagnostics.iter().any(|d| d.rule == "gate-run.receipt");
        for d in sub.diagnostics {
            report.push(d);
        }
        for n in sub.notes {
            report.note(n);
        }
        if minted
            && let Some(receipt) = load(repo, &dir)?
                .into_iter()
                .find(|e| &e.run.gate == key)
                .and_then(|e| e.receipt)
        {
            crate::journal_cmd::record(
                &dir,
                &validated.uuid.to_string(),
                crate::journal_cmd::RECEIPT_ATTACHED,
                &actor,
                &attached_payload(key, &receipt.verdict.to_string(), &receipt.receipt_digest),
            )?;
        }
    }
    Ok(report)
}

/// `war check` rules for a Warrant's recorded runs.
///
/// Every recorded run is reported: admissible ones as `evidence.admissible`,
/// the rest by why they are not. A stale binding and an unknown reuse are
/// WARNINGS — the file is a true record — and every other defect is an ERROR,
/// because a run file whose receipt does not reseal, or disagrees with it, is
/// a claim rather than a record and must not sit in the tree looking like one.
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
        let path = repo.relative(&e.run_path);
        let gate = &e.run.gate;
        if let Some(crate::invalidation::GateInvalidation::NotCounted(why)) = &e.invalidation {
            report.push(Diagnostic::warn(
                "evidence.invalidation-not-counted",
                path.clone(),
                format!("{alias}: {gate} · {why}"),
            ));
        }
        match standing(e, contract_digest) {
            Standing::Stale(why) if e.invalidated() == Some(why.as_str()) => {
                report.push(Diagnostic::warn(
                    "evidence.gate-invalidated",
                    path,
                    format!("{alias}: {gate} · {why}"),
                ));
            }
            Standing::Admissible => report.push(Diagnostic::pass(
                "evidence.admissible",
                format!(
                    "{alias}: {gate} · required pass, receipt reseals and is bound to the current \
                     contract{}",
                    match &e.reuse {
                        Reuse::Holds(rule) => format!(" and to its source ({rule})"),
                        Reuse::Historical =>
                            " (resolved: the source is not re-evaluated)".to_owned(),
                        _ => String::new(),
                    }
                ),
            )),
            Standing::Stale(why) => report.push(Diagnostic::warn(
                "evidence.stale-binding",
                path,
                format!("{alias}: {gate} · {why}"),
            )),
            Standing::ReuseUnknown(why) => report.push(Diagnostic::warn(
                "evidence.reuse-unknown",
                path,
                format!("{alias}: {gate} · {why}"),
            )),
            Standing::ReceiptInvalid(why) => report.push(Diagnostic::error(
                "evidence.receipt-invalid",
                path,
                format!("{alias}: {gate} · {why}"),
            )),
            Standing::NotAPass(why) => report.push(Diagnostic::warn(
                "evidence.not-a-pass",
                path,
                format!("{alias}: {gate} · {why}"),
            )),
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
            reuse: Reuse::Holds("tree:t".into()),
            invalidation: None,
        }
    }

    #[test]
    fn a_receipt_of_an_invalidated_gate_is_a_record_not_evidence() {
        let mut e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, &contract_subject(C))),
        );
        e.invalidation = Some(crate::invalidation::GateInvalidation::NotCounted(
            "unsigned".into(),
        ));
        assert!(
            admissibility(&e, Some(C)).is_ok(),
            "an invalidation nobody signed never counts"
        );
        e.invalidation = Some(crate::invalidation::GateInvalidation::Counted(
            "g@1.0.0 was invalidated by S".into(),
        ));
        let why = admissibility(&e, Some(C)).unwrap_err();
        assert!(why.contains("invalidated by S"), "{why}");
        assert!(admissible_runs(&[e], Some(C)).is_empty());
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
    fn a_moved_source_is_a_record_not_evidence() {
        let mut e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, &contract_subject(C))),
        );
        e.reuse = Reuse::Moved {
            subject: "tree:aa".into(),
            detail: "1 path(s) outside the evidence records have changed since (src/a)".into(),
        };
        match standing(&e, Some(C)) {
            Standing::Stale(why) => assert!(why.contains("tree:aa"), "{why}"),
            other => panic!("{other:?}"),
        }
        assert!(admissible_runs(&[e], Some(C)).is_empty());
    }

    #[test]
    fn an_unknown_reuse_is_neither_admissible_nor_a_failure() {
        let mut e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, &contract_subject(C))),
        );
        e.reuse = Reuse::Unknown("the receipt names no tree".into());
        assert!(matches!(standing(&e, Some(C)), Standing::ReuseUnknown(_)));
        assert!(admissible_runs(&[e], Some(C)).is_empty());
    }

    /// OW-WAR-0133 AM-002: status labels a reuse-unknown run `reuse_unknown`
    /// and a moved source `stale_binding` — two standings, two labels, the
    /// same split `war check` makes.
    #[test]
    fn status_labels_reuse_unknown_and_stale_binding_apart() {
        let sealed = || {
            evidence(
                run("pass"),
                Some(receipt(Verdict::Pass, &contract_subject(C))),
            )
        };
        let mut unknown = sealed();
        unknown.reuse = Reuse::Unknown("the receipt names no tree".into());
        let (class, why) = crate::status::run_class(&unknown, Some(C));
        assert_eq!(class, "reuse_unknown");
        assert!(why.is_some_and(|w| w.contains("reuse UNKNOWN")));

        let mut moved = sealed();
        moved.reuse = Reuse::Moved {
            subject: "tree:aa".into(),
            detail: "1 path(s) changed".into(),
        };
        assert_eq!(crate::status::run_class(&moved, Some(C)).0, "stale_binding");
        let earlier = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, "contract:sha256:ffff")),
        );
        assert_eq!(
            crate::status::run_class(&earlier, Some(C)).0,
            "stale_binding"
        );
        assert_eq!(
            crate::status::run_class(&sealed(), Some(C)),
            ("admissible".to_owned(), None)
        );
    }

    #[test]
    fn a_resolved_warrant_is_not_re_evaluated() {
        let mut e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, &contract_subject(C))),
        );
        e.reuse = Reuse::Historical;
        assert_eq!(standing(&e, Some(C)), Standing::Admissible);
    }

    #[test]
    fn the_contract_is_judged_before_the_source() {
        let mut e = evidence(
            run("pass"),
            Some(receipt(Verdict::Pass, "contract:sha256:ffff")),
        );
        e.reuse = Reuse::Unknown("no tree".into());
        match standing(&e, Some(C)) {
            Standing::Stale(why) => assert!(why.contains("earlier revision"), "{why}"),
            other => panic!("{other:?}"),
        }
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
