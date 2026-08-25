// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war check` — deterministic, agent-free validation (SAS §71.7, RQ-074).
//!
//! No model, no network, no clock. Reproducibility is not a nicety here: a
//! checker whose verdict can vary is not a control.
//!
//! That was once guaranteed by the binary having no HTTP client at all. Since
//! OW-WAR-0044 it is guaranteed by this module not using the one it has: `ureq`
//! is linked for the §67 Knowledge Fabric seam, and `war kf` is the only
//! command that dials. `war check` reaching the network would make its verdict
//! depend on someone else's uptime, which is the opposite of a control.

use std::collections::BTreeMap;

use openwarrant_compiler::{ChildRef, lower};
use openwarrant_core::{ValidatedManifest, detect_parent_cycles, milestones, obligation, seam};

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

    // §43.1 — local gate candidates. Loaded once for the corpus so an obligation
    // citing a gate can be resolved rather than taken on trust.
    let gates = load_gate_registry(repo, &mut report);
    report_independence(repo, &loaded, &mut report);

    // §20 and §21 relation conformance (OW-WAR-0043 OBL-004, §91.5 tests 30-35).
    // Built over the WHOLE corpus for the same reason parent digests are: a
    // parent/child claim is not checkable from one side of the relation.
    {
        let related: Vec<crate::relations::Related<'_>> = corpus
            .iter()
            .filter_map(|one| {
                let validated = one.validated.as_ref()?;
                let basis = one.basis.as_ref()?;
                let atom_source = basis
                    .atoms
                    .iter()
                    .map(|a| String::from_utf8_lossy(&a.bytes).into_owned())
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(crate::relations::Related {
                    alias: one.alias().to_string(),
                    manifest: validated,
                    manifest_file: one.dir.join("manifest.toml").to_string(),
                    generated_view: std::fs::read_to_string(
                        one.dir.join("generated").join("WAR.md"),
                    )
                    .ok(),
                    atom_source,
                })
            })
            .collect();
        crate::relations::check(&related, &mut report);
    }

    for one in &loaded {
        check_one(
            repo,
            one,
            &corpus,
            check_generated,
            &parent_digests,
            &gates,
            &mut report,
        );
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
    report.note("gate execution — `war gate --run` executes a registered gate (§44), but `war check` does not invoke it, so nothing here is evidence that a Warrant's acceptance gates were run");
    report.note("Preflight readiness (§32.7) — 'well-formed' is a claim about the record only");
    report.note("bound-atom resolution — `ref =` atoms cannot be fetched offline");
    // Kept, and now says WHY rather than only that. OW-WAR-0049's OBL-003 asks
    // this note to match reality: the tests are not merely unimplemented, they
    // are blocked on a decision this repository has not made, and OW-ADR-0007
    // records which decision. A note that says "unchecked" without saying what
    // would change it reads as a backlog item; this one names the blocker.
    report.note(
        "Source Holder ambiguity and classification propagation (§91.2 tests 14, 15) — \
         narrowed by OW-ADR-0008: test 14 needs a `source_holder` on atoms, which §13 \
         requires and none declares; test 15 needs a classification ordering, which \
         the SAS does not give. Neither is undecided — both are undeclared",
    );
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

/// §46.3 — report, ONCE, whether verification here meets each level's minimum.
///
/// Independence is declared per repository, so reporting it per Warrant says the
/// same sentence forty-nine times. A finding repeated once per record is a
/// finding nobody reads, which would defeat the point of surfacing it at all.
///
/// Never fatal at draft. §46.3's minimum binds when a resolution is recorded
/// (§56.1 requirement 10). A repository that could not pass its own gate on day
/// one would have the rule deleted rather than the gap closed — but silence is
/// worse, and silence is what alpha shipped: "this repository authored and
/// verified itself" was a paragraph in the roadmap that no tool could act on.
fn report_independence(repo: &Repository, loaded: &[Loaded], report: &mut Report) {
    let config_path = repo.relative(&repo.root.join("openwarrant.toml"));

    let Some(independence) = &repo.config.independence else {
        report.push(Diagnostic::warn(
            "independence.undeclared",
            config_path,
            "no independence is declared for this repository, so §46.3's minimums \
             cannot be evaluated for any Warrant. Absent is not the same as none — \
             `none` reads as examined and absent, absent reads as unexamined"
                .to_owned(),
        ));
        return;
    };

    // Group by the level actually declared, since the minimum differs per level.
    let mut by_level: BTreeMap<String, usize> = BTreeMap::new();
    for one in loaded {
        if let Some(v) = &one.validated {
            *by_level.entry(v.assurance_level.to_string()).or_insert(0) += 1;
        }
    }

    for (level, count) in by_level {
        match independence.meets(&level) {
            Ok(()) => report.push(Diagnostic::pass(
                "independence.sufficient",
                format!("{count} {level} Warrant(s): declared independence meets §46.3's minimum"),
            )),
            Err(err) => report.push(Diagnostic::warn(
                "independence.insufficient",
                config_path.clone(),
                format!("{count} {level} Warrant(s): {err}"),
            )),
        }
    }
}

/// Load `docs/gates/*.yaml` as §43.1 local candidates.
///
/// A malformed definition is an ERROR rather than a skip. A registry that
/// quietly drops the file it could not read would let every citation of that
/// gate report unresolved, or worse, let a later valid-looking file take its
/// place unnoticed.
pub(crate) fn load_gate_registry(
    repo: &Repository,
    report: &mut Report,
) -> openwarrant_core::GateRegistry {
    let mut registry = openwarrant_core::GateRegistry::default();
    let dir = repo.root.join(&repo.config.paths.gates);
    let Ok(entries) = dir.read_dir_utf8() else {
        // No gates directory is a legitimate state: a repository may declare no
        // gates. It is reported as a note so it cannot be mistaken for a
        // registry that was read and found empty.
        report.note(format!(
            "gate registry — {} does not exist, so no gate citation can be \
             resolved. §43.1 local candidates live there",
            repo.config.paths.gates
        ));
        return registry;
    };

    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .collect();
    paths.sort();

    for path in paths {
        let rel = repo.relative(&path);
        let Ok(text) = std::fs::read_to_string(&path) else {
            report.push(Diagnostic::error(
                "gate.unreadable",
                rel,
                "gate definition could not be read".to_owned(),
            ));
            continue;
        };
        let doc = match openwarrant_core::structured::parse(&text) {
            Ok(d) => d,
            Err(err) => {
                report.push(Diagnostic::error("gate.malformed", rel, format!("{err}")));
                continue;
            }
        };
        match openwarrant_core::gate::definition_from_structured(&doc) {
            Ok(def) => {
                let key = def.key();
                let provenance = def.provenance;
                match registry.insert(def) {
                    Ok(()) => report.push(Diagnostic::pass(
                        "gate.registered",
                        format!("{key}: {provenance}, qualified against its declared fault model"),
                    )),
                    Err(err) => {
                        report.push(Diagnostic::error("gate.duplicate", rel, format!("{err}")));
                    }
                }
            }
            Err(err) => report.push(Diagnostic::error("gate.invalid", rel, format!("{err}"))),
        }
    }
    registry
}

#[allow(clippy::too_many_arguments)]
fn check_one(
    repo: &Repository,
    one: &Loaded,
    corpus: &[Loaded],
    check_generated: bool,
    parent_digests: &BTreeMap<String, String>,
    gates: &openwarrant_core::GateRegistry,
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

    // §43 / RQ-056: a gate cited by an obligation must resolve to a registered,
    // bindable definition.
    //
    // OW-WAR-0019's Intent records why: in the parent project's corpus, 23 of 94
    // declared gates named a tool, script, or crate that was not in the tree.
    // Nothing read those strings, so nothing noticed.
    for atom in basis.atoms.iter().filter(|a| a.role == "assurance") {
        let text = String::from_utf8_lossy(&atom.bytes);
        let file = repo.relative(&one.dir.join(&atom.source));
        let cited = openwarrant_core::gate::cited_gate_uris(&text);
        let mut resolved = 0usize;
        for uri in &cited {
            match gates.resolve_citation(&alias, uri) {
                Ok(def) if def.lifecycle.is_bindable() => resolved += 1,
                Ok(def) => report.push(Diagnostic::error(
                    "gate.not-bindable",
                    file.clone(),
                    format!(
                        "{alias}: cites {uri}, whose lifecycle is {}. §43.4 permits \
                         binding only a qualified gate",
                        def.lifecycle
                    ),
                )),
                Err(err) => report.push(Diagnostic::error(
                    "gate.unresolved",
                    file.clone(),
                    format!("{alias}: {err}"),
                )),
            }
        }
        if resolved > 0 && resolved == cited.len() {
            report.push(Diagnostic::pass(
                "gate.resolved",
                format!("{alias}: {resolved} cited gate(s) resolve to a bindable definition"),
            ));
        }
    }

    // §40 — evidence, observations, inferences and judgments, if the assurance
    // atom records any. §40.7's six prohibited substitutions live here, and until
    // now nothing in any binary read a record they could apply to.
    for atom in basis.atoms.iter().filter(|a| a.role == "assurance") {
        let file = repo.relative(&one.dir.join(&atom.source));
        match openwarrant_core::epistemic::records::parse(&String::from_utf8_lossy(&atom.bytes)) {
            Ok(section) if section.is_empty() => {}
            Ok(section) => report.push(Diagnostic::pass(
                "evidence.valid",
                format!(
                    "{alias}: {} §40 record(s) — {} evidence, {} observation(s), \
                     {} inference(s), {} judgment(s)",
                    section.len(),
                    section.evidence.len(),
                    section.observations.len(),
                    section.inferences.len(),
                    section.judgments.len()
                ),
            )),
            Err(err) => report.push(Diagnostic::error(
                "evidence.invalid",
                file,
                format!("{alias}: {err}"),
            )),
        }
    }

    // §31 — amendment records, if this Warrant has any.
    //
    // §31 binds revisions AFTER authorization and nothing here is authorized, so
    // an amendment is not compelled. One that exists anyway is still validated:
    // a record of why a claim was narrowed is worthless if it is malformed, and
    // worse than worthless if it is malformed and nobody checks.
    let amendments = one.dir.join("amendments");
    if let Ok(entries) = amendments.read_dir_utf8() {
        let mut paths: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.into_path())
            .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
            .collect();
        paths.sort();
        for path in paths {
            let rel = repo.relative(&path);
            let Ok(text) = std::fs::read_to_string(&path) else {
                report.push(Diagnostic::error(
                    "amendment.unreadable",
                    rel,
                    format!("{alias}: amendment could not be read"),
                ));
                continue;
            };
            match openwarrant_core::structured::parse(&text)
                .map_err(|e| e.to_string())
                .and_then(|doc| {
                    openwarrant_core::autonomy::from_structured(&doc).map_err(|e| e.to_string())
                }) {
                Ok(record) => report.push(Diagnostic::pass(
                    "amendment.valid",
                    format!(
                        "{alias}: amendment {} is a {} carrying {} semantic change(s)",
                        record.id,
                        record.band,
                        record.semantic_diff.len()
                    ),
                )),
                Err(e) => report.push(Diagnostic::error(
                    "amendment.invalid",
                    rel,
                    format!("{alias}: {e}"),
                )),
            }
        }
    }

    // §39 / RQ-055: contract-adequacy review, STRUCTURALLY checked.
    //
    // This replaced a substring search that passed any assurance atom merely
    // containing the word. That search is deleted in the same commit that adds
    // this check: two checks for one rule means the weak one decides. The word
    // is deliberately not written as a string literal anywhere in this crate, so
    // a repository-wide grep for the old call site returns nothing.
    let requirement =
        openwarrant_core::AdequacyRequirement::for_level(&validated.assurance_level.to_string());
    for atom in basis.atoms.iter().filter(|a| a.role == "assurance") {
        let review = openwarrant_core::adequacy::parse(&String::from_utf8_lossy(&atom.bytes));
        let file = repo.relative(&one.dir.join(&atom.source));

        match review.validate(requirement, &validated.assurance_level.to_string()) {
            Ok(()) if requirement.requires_review() => {
                report.push(Diagnostic::pass(
                    "assurance.adequacy-review",
                    format!(
                        "{alias}: {} assurance carries an adequacy review with an \
                         adversarial question",
                        validated.assurance_level
                    ),
                ));
            }
            Ok(()) => {}
            Err(err) => report.push(Diagnostic::error(
                "assurance.adequacy-review",
                file.clone(),
                format!("{alias}: {err}"),
            )),
        }

        if requirement.requires_review() {
            // §39.3 is a SHOULD ("where economical"), so an unexecuted attack set
            // is a warning — except at high assurance, where §39.4 requires
            // executed negative controls outright.
            if !review.has_executed_attacks() {
                let severity_is_error = requirement.requires_executed_controls();
                let message = format!(
                    "{alias}: the adequacy review has executed no attacks. §39.3 wants \
                     violating artifacts planted and run; 'recorded here when run' is a \
                     plan, not evidence"
                );
                report.push(if severity_is_error {
                    Diagnostic::error("assurance.executed-attacks", file.clone(), message)
                } else {
                    Diagnostic::warn("assurance.executed-attacks", file.clone(), message)
                });
            }
            if !review.has_outcome() {
                report.push(Diagnostic::warn(
                    "assurance.adequacy-outcome",
                    file.clone(),
                    format!(
                        "{alias}: the adequacy review records no §39.2 outcome; a review \
                         that reaches no outcome is a question, not a review"
                    ),
                ));
            }
        }
    }

    // §91.2 test 10 — a GENERATED atom cannot be edited through an
    // authored-source command.
    //
    // The types for this shipped in alpha and answered no question. `Jurisdiction`
    // exists, `is_directly_editable` exists to answer "may I write this?", and
    // `Jurisdiction::from_str` was referenced by exactly one unit test — the atom's
    // declared jurisdiction travelled from frontmatter to the IR as a plain String
    // and was never parsed. So the class that decides whether an atom may be
    // hand-edited was never consulted about any atom.
    //
    // An atom listed in a manifest IS an authored source: a file in the Warrant
    // directory that a person edits and the compiler reads. Declaring it `generated`
    // or `bound` says it is a projection or someone else's record, and carrying it
    // as an editable source anyway is precisely the edit §13.3 forbids.
    for atom in &basis.atoms {
        let declared = atom.jurisdiction.parse::<openwarrant_core::Jurisdiction>();
        let Ok(declared) = declared else {
            report.push(Diagnostic::error(
                "atom.unknown-jurisdiction",
                repo.relative(&one.dir.join(&atom.source)),
                format!(
                    "{alias}: atom {} declares jurisdiction {:?}, which is not one of \
                     §13.3's three: authored, bound, generated",
                    atom.source, atom.jurisdiction
                ),
            ));
            continue;
        };
        // "Here" in §13.3's "may be READ here, may not be written here" is THIS
        // Warrant. An atom under the Warrant's own directory is its authored
        // source and must be editable; one referenced from outside — the ADR
        // corpus, say — is exactly what `bound` is for, and demanding it be
        // editable would forbid binding anything.
        //
        // A first version of this rule applied to every atom in the manifest and
        // was wrong: it and the role check below cannot both hold for an ADR
        // atom, which §16.1 places under `bound` and which a Warrant does bind.
        // The conflict is what showed the rule was too broad.
        // Resolved, not string-matched. `!source.starts_with("..")` reads the
        // same and misclassifies an absolute path, or any relative path that
        // leaves the directory without a leading `..` — both would be called
        // "owned" and then wrongly refused for declaring `bound`.
        let resolved = one.dir.join(&atom.source);
        let owned_by_this_warrant = match (resolved.canonicalize(), one.dir.canonicalize()) {
            (Ok(atom_path), Ok(dir)) => atom_path.starts_with(&dir),
            // Unresolvable means the file is missing, which `atom.missing`
            // already reports. Treat it as NOT owned so this rule stays quiet
            // rather than adding a second complaint about the same absence.
            _ => false,
        };
        if owned_by_this_warrant && !declared.is_directly_editable() {
            report.push(Diagnostic::error(
                "atom.generated-as-source",
                repo.relative(&one.dir.join(&atom.source)),
                format!(
                    "{alias}: atom {} lives in this Warrant's own directory but is \
                     declared `{declared}`, which §13.3 says is not directly editable. \
                     A projection that is hand-edited is no longer a projection.",
                    atom.source
                ),
            ));
        }
        // §16.1 assigns some roles one jurisdiction and others several. Where it
        // named one, disagreeing with it is an authoring error; where it named
        // more than one, `typical_jurisdiction` returns None and no rule applies
        // — inventing one there would be a rule the specification declined to make.
        if let Ok(role) = atom.role.parse::<openwarrant_core::AtomRole>()
            && let Some(expected) = role.typical_jurisdiction()
            && expected != declared
        {
            report.push(Diagnostic::error(
                "atom.jurisdiction-mismatch",
                repo.relative(&one.dir.join(&atom.source)),
                format!(
                    "{alias}: atom {} has role `{role}`, which §16.1 places under \
                     `{expected}`, but declares `{declared}`",
                    atom.source
                ),
            ));
        }
    }

    // §49.3 — BLUT's execution lineage stays authoritative in BLUT. Run over
    // EVERY atom, not just the ones a BLUT-shaped Warrant would use: lineage is
    // copied by hand, into whatever file the author was editing, and a rule that
    // only inspects the atoms where the copy *ought* to appear is one an
    // accidental paste walks straight past.
    for atom in &basis.atoms {
        let text = String::from_utf8_lossy(&atom.bytes);
        for (line, key) in seam::reproduced_lineage(&text) {
            report.push(Diagnostic::error(
                "lineage.reproduced",
                repo.relative(&one.dir.join(&atom.source)),
                format!(
                    // Says ATOM and LINE, where `SeamError::LineageReproduced`
                    // says RECEIPT. Two paths reach the same rule, and sharing
                    // one sentence would leave a reader unable to tell which
                    // one fired or where to look.
                    "{alias}: atom {}, line {line} carries BLUT's `{key}` as a key with a \
                     value. §49.3 — lineage stays authoritative in BLUT and the Warrant \
                     stores a reference. Write the shape inline in backticks if you need \
                     to show it; prose naming the field is not a copy.",
                    atom.source
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
                // §49.2 — `executor_args` is a JSON scalar the milestones
                // grammar cannot validate: core holds it as a raw string
                // because parsing needs a JSON crate its dependency surface
                // deliberately excludes. Checked HERE so a malformed value is
                // an authoring error found by `war check`, not a surprise the
                // first time someone lowers the Warrant.
                for stage in &graph.stages {
                    if let Err(why) = crate::blut::parse_executor_args(stage) {
                        report.push(Diagnostic::error(
                            "milestones.bad-executor-args",
                            repo.relative(&one.dir.join(&atom.source)),
                            format!("{alias}: {why}"),
                        ));
                    }
                }
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

    // The optional Bonsai sidecar names assurance obligations. Resolve those
    // names here as well as in the adapter, so an authored scope cannot look
    // valid until its first CI invocation.
    if basis.scope.is_some() {
        match crate::bonsai::validate_scope(&alias, basis) {
            Ok(()) => report.push(Diagnostic::pass(
                "bonsai-scope.valid",
                format!("{alias}: machine scope resolves to declared obligations"),
            )),
            Err(err) => report.push(Diagnostic::error(
                "bonsai-scope.invalid",
                repo.relative(&one.dir.join("scope.toml")),
                err.to_string(),
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
        let children = crate::compile::children_of(&validated.raw.uuid, corpus);
        check_drift(repo, &children, one, basis, validated, &alias, report);
    }
}

/// §17.3 / RQ-075: committed generated views must match a fresh compilation.
#[allow(clippy::too_many_arguments)]
fn check_drift(
    repo: &Repository,
    children: &[ChildRef],
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

    let fresh = match projections(basis, validated, children) {
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
        let path = one.dir.join(view.committed_filename());
        let relative = repo.relative(&path);
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == *expected => compared += 1,
            Ok(_) => report.push(Diagnostic::error(
                "generated.drift",
                relative,
                format!(
                    "{alias}: committed {} differs from a fresh compilation; \
                     it was edited by hand or its sources changed without recompiling",
                    view.committed_filename()
                ),
            )),
            Err(_) if repo.config.generated.commit => report.push(Diagnostic::error(
                "generated.missing",
                relative,
                format!(
                    "{alias}: {} is missing and this repository commits generated views; \
                     run `war compile {alias}`",
                    view.committed_filename()
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
