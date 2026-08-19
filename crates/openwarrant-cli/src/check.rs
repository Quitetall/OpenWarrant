// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war check` — deterministic, agent-free validation (SAS §71.7, RQ-074).
//!
//! No model, no network, no clock. Reproducibility is not a nicety here: a
//! checker whose verdict can vary is not a control.

use std::collections::BTreeMap;

use openwarrant_compiler::lower;
use openwarrant_core::{ValidatedManifest, detect_parent_cycles, milestones, obligation};

use crate::compile::{adr_overview, projections, warrant_overview};
use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{Loaded, RepoError, Repository};

/// Run every Phase 1 check over the whole corpus, or one Warrant.
pub fn run(
    repo: &Repository,
    only: Option<&str>,
    check_generated: bool,
) -> Result<Report, RepoError> {
    let dirs = match only {
        Some(alias) => vec![repo.warrant_dir(alias)?],
        None => repo.warrant_dirs()?,
    };

    let mut report = Report::default();
    let mut undisposed_warrants = 0usize;
    let mut total_warrants = 0usize;

    if dirs.is_empty() {
        report.push(Diagnostic::warn(
            "corpus.empty",
            repo.config.paths.warrants.clone(),
            "no Warrants found; nothing to check",
        ));
        return Ok(report);
    }

    let mut loaded = Vec::new();
    for dir in &dirs {
        let one = repo.load_warrant(dir)?;
        loaded.push(one);
    }

    // A parent's contract digest is computed from the parent, so verifying a
    // child's citation needs the whole corpus in hand. When only one Warrant was
    // requested, load the rest read-only so the citation can still be checked
    // rather than reported as unknowable.
    let corpus = if only.is_some() {
        let mut all = Vec::new();
        for dir in repo.warrant_dirs()? {
            all.push(repo.load_warrant(&dir)?);
        }
        all
    } else {
        loaded.clone()
    };
    let parent_digests = contract_digests(&corpus);

    for one in &loaded {
        check_one(repo, one, check_generated, &parent_digests, &mut report);
        if let Some(basis) = &one.basis {
            total_warrants += 1;
            let fully_undisposed = basis
                .atoms
                .iter()
                .filter(|a| a.role == "assurance")
                .filter_map(|a| obligation::parse(&String::from_utf8_lossy(&a.bytes)).ok())
                .any(|set| {
                    !set.obligations.is_empty() && set.undisposed().len() == set.obligations.len()
                });
            if fully_undisposed {
                undisposed_warrants += 1;
            }
        }
    }

    // Cross-Warrant checks need the whole corpus, so they run after the loop.
    // §91.2 test 12.
    let validated: Vec<ValidatedManifest> =
        loaded.iter().filter_map(|l| l.validated.clone()).collect();
    let cycles = detect_parent_cycles(&validated);
    if cycles.is_empty() {
        report.push(Diagnostic::pass(
            "composition.acyclic",
            format!(
                "parent graph is acyclic across {} Warrant(s)",
                validated.len()
            ),
        ));
    } else {
        for cycle in cycles {
            report.push(Diagnostic::error(
                "composition.cycle",
                repo.config.paths.warrants.clone(),
                format!("parent cycle: {}", cycle.path.join(" → ")),
            ));
        }
    }

    // ADR corpus (§19). A malformed ADR is an error; the Overview is a
    // projection and drift-checks exactly like a Warrant parent (§19.7).
    let adrs = repo.load_adrs()?;
    for (path, err) in &adrs.failures {
        report.push(Diagnostic::error(
            "adr.malformed",
            path.clone(),
            err.to_string(),
        ));
    }
    if adrs.failures.is_empty() && !adrs.records.is_empty() {
        report.push(Diagnostic::pass(
            "adr.parsed",
            format!("{} ADR(s) parsed", adrs.records.len()),
        ));
    }
    // Both corpus-wide projections drift-check through the same function, so
    // they cannot come to report drift differently.
    if check_generated && repo.config.generated.verify_drift {
        drift_check(
            repo,
            warrant_overview(repo),
            "warrant-overview",
            &mut report,
        );
        drift_check(repo, adr_overview(repo), "adr-overview", &mut report);
    }

    // §38.6 disposition status, aggregated once for the corpus rather than
    // blocking each Warrant. Undisposed is the normal state of planned work.
    if undisposed_warrants > 0 {
        report.note(format!(
            "{undisposed_warrants} of {total_warrants} Warrant(s) have no disposed              obligations — §38.6 yields no resolution verdict for them, which is the              expected state of work that has not been assessed"
        ));
    }

    // §28.5 coverage, reported once for the corpus. A contract digest covering
    // 8 of 17 elements is not a §28.5 contract digest, and saying so on every
    // run is what stops it being mistaken for one (OW-ADR-0004).
    let coverage = openwarrant_compiler::WarIr::current_coverage();
    if !coverage.is_complete() {
        report.note(format!(
            "contract digest covers {} of {} §28.5 elements — missing: {}",
            coverage.len(),
            openwarrant_core::ContractElement::ALL.len(),
            coverage
                .missing()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Printed on every run including a clean one, and deliberately NOT a
    // diagnostic: a report that answers "ok" while whole classes of check go
    // unasked reads as full coverage, but a scope note that blocked readiness
    // would make the verdict permanently negative and therefore meaningless.
    report.note("gate execution — a Warrant's acceptance gates are never run (Phase 6)");
    report.note("Preflight readiness (§32.7) — 'well-formed' is a claim about the record only");
    report.note("bound-atom resolution — `ref =` atoms cannot be fetched offline");
    report.note("Source Holder ambiguity and classification propagation (§91.2 tests 14, 15)");
    if !check_generated {
        report.note("generated-view drift — pass --generated to compare committed projections");
    }

    Ok(report)
}

/// Compare one generated corpus-wide projection against a fresh compilation.
///
/// Shared by the Warrant and ADR overviews so the two cannot drift apart in how
/// they report drift — a duplicated check is a check that gets fixed in one place.
fn drift_check(
    repo: &Repository,
    compiled: Result<(camino::Utf8PathBuf, String), RepoError>,
    rule: &str,
    report: &mut Report,
) {
    match compiled {
        Ok((path, expected)) => {
            let relative = repo.relative(&path);
            let name = path.file_name().unwrap_or("overview").to_owned();
            match std::fs::read_to_string(&path) {
                Ok(actual) if actual == expected => report.push(Diagnostic::pass(
                    format!("{rule}.drift"),
                    format!("{name} matches a fresh compilation"),
                )),
                Ok(_) => report.push(Diagnostic::error(
                    format!("{rule}.drift"),
                    relative,
                    format!(
                        "the committed {name} differs from a fresh compilation; it was \
                         edited by hand or its sources changed without recompiling"
                    ),
                )),
                Err(_) if repo.config.generated.commit => report.push(Diagnostic::error(
                    format!("{rule}.missing"),
                    relative,
                    format!(
                        "{name} is missing and this repository commits generated views; \
                         run `war compile`"
                    ),
                )),
                Err(_) => {}
            }
        }
        Err(err) => report.push(Diagnostic::error(
            format!("{rule}.compile"),
            repo.config.paths.warrants.clone(),
            err.to_string(),
        )),
    }
}

/// Contract digest per Warrant UUID, for verifying children's citations.
///
/// A Warrant that could not be validated has no contract digest and is simply
/// absent from the map; a child citing it gets an honest "cannot verify" rather
/// than a comparison against a value invented from broken sources.
fn contract_digests(corpus: &[Loaded]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for one in corpus {
        let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
            continue;
        };
        if let Ok(ir) = lower(basis, validated)
            && let Ok(digest) = ir.contract_digest()
        {
            out.insert(validated.uuid.to_string(), digest);
        }
    }
    out
}

fn check_one(
    repo: &Repository,
    one: &Loaded,
    check_generated: bool,
    parent_digests: &BTreeMap<String, String>,
    report: &mut Report,
) {
    let alias = one.alias();

    // Carry forward whatever loading already found.
    for diagnostic in &one.report.diagnostics {
        report.push(diagnostic.clone());
    }

    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        // The manifest was invalid; loading already reported why, and every
        // downstream check would be reporting consequences of that one fact.
        return;
    };

    report.push(Diagnostic::pass(
        "manifest.valid",
        format!("{alias}: manifest and composition are well-formed"),
    ));

    // Ordinals ascending is not required by the SAS, but a manifest whose
    // ordinals descend renders in an order its author probably did not intend.
    let ordinals: Vec<u32> = basis.manifest.atoms.iter().map(|a| a.ordinal).collect();
    let mut sorted = ordinals.clone();
    sorted.sort_unstable();
    if ordinals != sorted {
        report.push(Diagnostic::warn(
            "composition.ordinal-order",
            repo.relative(&one.dir.join("manifest.toml")),
            format!("{alias}: atom ordinals are not in ascending order; the parent renders in declared order"),
        ));
    }

    // §39.4 / RQ-055: controlled and high assurance require an adequacy review.
    // Checked structurally — the assurance atom must actually contain one.
    if validated.assurance_level.requires_adequacy_review() {
        let has_review = basis
            .atoms
            .iter()
            .filter(|a| a.role == "assurance")
            .any(|a| {
                let text = String::from_utf8_lossy(&a.bytes).to_lowercase();
                text.contains("adequacy")
            });
        if has_review {
            report.push(Diagnostic::pass(
                "assurance.adequacy-review",
                format!(
                    "{alias}: {} assurance carries an adequacy review",
                    validated.assurance_level
                ),
            ));
        } else {
            report.push(Diagnostic::error(
                "assurance.adequacy-review",
                repo.relative(&one.dir.join("manifest.toml")),
                format!(
                    "{alias}: assurance_level is {} and §39.4 requires a contract-adequacy \
                     review, but no assurance atom mentions one",
                    validated.assurance_level
                ),
            ));
        }
    }

    // §23: the milestone graph is parsed and validated, not merely carried.
    // Until OW-WAR-0007 this atom's bytes were hashed and rendered while nothing
    // read them, so a dangling stage_ref or a dependency cycle passed unnoticed.
    for atom in basis.atoms.iter().filter(|a| a.role == "milestones") {
        let text = String::from_utf8_lossy(&atom.bytes);
        match milestones::parse(&text) {
            Ok(graph) => {
                report.push(Diagnostic::pass(
                    "milestones.valid",
                    format!(
                        "{alias}: {} milestone(s), {} stage(s), acyclic with no dangling refs",
                        graph.milestones.len(),
                        graph.stages.len()
                    ),
                ));
                // Not errors — a checkpoint may rest on obligations alone, and a
                // stage may be defined ahead of the milestone that will need it.
                // Both are worth seeing.
                let orphans = graph.unreferenced_stages();
                if !orphans.is_empty() {
                    report.push(Diagnostic::warn(
                        "milestones.unreferenced-stage",
                        repo.relative(&one.dir.join(&atom.source)),
                        format!(
                            "{alias}: stage(s) no milestone references: {}",
                            orphans.join(", ")
                        ),
                    ));
                }
            }
            Err(err) => report.push(Diagnostic::error(
                "milestones.invalid",
                repo.relative(&one.dir.join(&atom.source)),
                format!("{alias}: {err}"),
            )),
        }
    }

    // §38: obligations are parsed, and milestone `obligation_refs` are resolved
    // against them. Those references dangled unchecked until OW-WAR-0016 — a
    // milestone could cite proof nobody ever wrote.
    let obligations = basis
        .atoms
        .iter()
        .filter(|a| a.role == "assurance")
        .map(|a| (a, obligation::parse(&String::from_utf8_lossy(&a.bytes))))
        .collect::<Vec<_>>();
    for (atom, parsed) in &obligations {
        match parsed {
            Ok(set) => {
                report.push(Diagnostic::pass(
                    "obligations.valid",
                    format!(
                        "{alias}: {} obligation(s), each with a bounded scope and evidence",
                        set.obligations.len()
                    ),
                ));
                // Cross-check the milestone graph's obligation_refs.
                let refs: std::collections::BTreeMap<String, Vec<String>> = basis
                    .atoms
                    .iter()
                    .filter(|a| a.role == "milestones")
                    .find_map(|a| milestones::parse(&String::from_utf8_lossy(&a.bytes)).ok())
                    .map(|g| {
                        g.milestones
                            .iter()
                            .map(|m| (m.id.clone(), m.obligation_refs.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                if let Err(err) = set.check_references(&refs) {
                    report.push(Diagnostic::error(
                        "obligations.dangling-ref",
                        repo.relative(&one.dir.join(&atom.source)),
                        format!("{alias}: {err}"),
                    ));
                }
                // §38.6 yields no verdict without full disposition — but that
                // bears on RESOLUTION readiness, not on whether the record is
                // well-formed, which is what this verdict is about.
                //
                // Reporting it as a blocking UNKNOWN would make every unstarted
                // Warrant permanently NOT READY. That is the same defect the
                // Phase 1 scope note had: a verdict that never changes carries
                // no information. It is counted and noted at corpus level below.
            }
            Err(err) => report.push(Diagnostic::error(
                "obligations.invalid",
                repo.relative(&one.dir.join(&atom.source)),
                format!("{alias}: {err}"),
            )),
        }
    }

    // §20.2 / §91.5 test 29: a child cites an EXACT parent contract revision.
    // The digest is what makes "exact" verifiable, so it is compared against the
    // parent's actual contract digest rather than merely noted as present.
    let manifest_file = repo.relative(&one.dir.join("manifest.toml"));
    for parent in &basis.manifest.parents {
        let uuid = parent.r#ref.strip_prefix("war://").unwrap_or(&parent.r#ref);
        let actual = parent_digests.get(uuid);

        match (&parent.contract_digest, actual) {
            (Some(cited), Some(actual)) => {
                let cited = cited.strip_prefix("sha256:").unwrap_or(cited);
                if cited == actual {
                    report.push(Diagnostic::pass(
                        "relations.parent-digest",
                        format!("{alias}: parent {} contract digest matches", parent.r#ref),
                    ));
                } else {
                    report.push(Diagnostic::error(
                        "relations.parent-digest",
                        manifest_file.clone(),
                        format!(
                            "{alias}: parent {} is cited at contract digest sha256:{cited} \
                             but the parent's actual contract digest is sha256:{actual}. \
                             The parent changed after this child was written — the child's \
                             basis is no longer the one it was authorized against.",
                            parent.r#ref
                        ),
                    ));
                }
            }
            (None, Some(actual)) => {
                // Not an error: the citation is incomplete, not wrong. The
                // computed value is printed so the fix is a copy-paste rather
                // than a research task.
                report.push(Diagnostic::unknown(
                    "relations.parent-digest",
                    manifest_file.clone(),
                    format!(
                        "{alias}: parent {} cites a revision but no contract_digest (§20.2). \
                         Its current digest is sha256:{actual} — add \
                         `contract_digest = \"sha256:{actual}\"` to pin it.",
                        parent.r#ref
                    ),
                ));
            }
            (_, None) => {
                report.push(Diagnostic::unknown(
                    "relations.parent-digest",
                    manifest_file.clone(),
                    format!(
                        "{alias}: parent {} is not in this repository, so its contract \
                         digest cannot be computed; cross-repository resolution needs \
                         federation",
                        parent.r#ref
                    ),
                ));
            }
        }
    }

    if check_generated {
        check_drift(repo, one, basis, validated, &alias, report);
    }
}

/// §17.3 / RQ-075: committed generated views must match a fresh compilation.
fn check_drift(
    repo: &Repository,
    one: &Loaded,
    basis: &openwarrant_compiler::CompilationBasis,
    validated: &ValidatedManifest,
    alias: &str,
    report: &mut Report,
) {
    if !repo.config.generated.verify_drift {
        report.push(Diagnostic::unknown(
            "generated.drift",
            "openwarrant.toml",
            "generated.verify_drift is false; drift was not checked",
        ));
        return;
    }

    let fresh = match projections(basis, validated) {
        Ok(fresh) => fresh,
        Err(err) => {
            report.push(Diagnostic::error(
                "generated.compile",
                repo.relative(&one.dir),
                format!("{alias}: could not compile: {err}"),
            ));
            return;
        }
    };

    let mut compared = 0usize;
    for (view, expected) in &fresh {
        let path = one.dir.join(view.filename());
        let relative = repo.relative(&path);
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == *expected => compared += 1,
            Ok(_) => report.push(Diagnostic::error(
                "generated.drift",
                relative,
                format!(
                    "{alias}: committed {} differs from a fresh compilation; \
                     it was edited by hand or its sources changed without recompiling",
                    view.filename()
                ),
            )),
            Err(_) if repo.config.generated.commit => report.push(Diagnostic::error(
                "generated.missing",
                relative,
                format!(
                    "{alias}: {} is missing and this repository commits generated views; \
                     run `war compile {alias}`",
                    view.filename()
                ),
            )),
            Err(_) => compared += 1,
        }
    }

    if compared == fresh.len() {
        report.push(Diagnostic::pass(
            "generated.drift",
            format!("{alias}: {compared} generated view(s) match a fresh compilation"),
        ));
    }
}

/// Print a report in the §71.7 shape.
pub fn print(report: &Report) {
    for diagnostic in &report.diagnostics {
        println!("{diagnostic}");
    }
    println!();
    println!(
        "{} pass · {} warn · {} unknown · {} error   (worst: {})",
        report.count(Severity::Pass),
        report.count(Severity::Warn),
        report.count(Severity::Unknown),
        report.count(Severity::Error),
        report.worst(),
    );
    if !report.notes.is_empty() {
        println!("\nNOT CHECKED:");
        for note in &report.notes {
            println!("  · {note}");
        }
    }
    println!("\n{}", report.verdict_line());
}
