// SPDX-License-Identifier: Apache-2.0
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

use std::collections::{BTreeMap, BTreeSet};

use openwarrant_compiler::{ChildRef, lower};
use openwarrant_core::{ValidatedManifest, detect_parent_cycles, milestones, obligation, seam};

use crate::compile::{adr_overview, projections, warrant_overview};

// OW-WAR-0119: the corpus identity rules. A child of this module, not a
// sibling in `lib.rs`, because `war check` is their only caller.
#[path = "identity_check.rs"]
mod identity_check;
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

    // OW-WAR-0121: a temp file of the atomic write path is a write that
    // stopped before its rename. The record it was meant to replace is whole
    // (old or absent); the temp file is named so a person can remove it.
    check_stray_temps(repo, only.map(|_| dirs.as_slice()), &mut report);

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

    // OW-ADR-0022: currency is derived from relations, once, over the whole
    // corpus, and read by everything below that asks it.
    let currencies = crate::relations::currencies(&corpus);

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
        crate::relations::check(&related, &currencies, &mut report);
    }
    // OW-ADR-0023: the roadmap record, and the Warrants it holds to a phase.
    crate::roadmap_cmd::check(repo, &corpus, &mut report);
    let roadmap = crate::roadmap_cmd::load(repo).ok().flatten();

    // OW-ADR-0021: which Warrant governs each path NOW. Built once — it
    // verifies one attestation per owning Warrant, and the drift decision
    // below asks it for every content-addressed deliverable in the corpus.
    let ownership = crate::ownership::Ownership::index_with(repo, &currencies)?;
    // OW-WAR-0125: each recorded SAS revision split into sections, read from
    // the document or from history at most once per run.
    let sas_sections = crate::sas::RevisionSections::new(repo);

    let shared = Shared {
        corpus: &corpus,
        parent_digests: &parent_digests,
        gates: &gates,
        ownership: &ownership,
        roadmap: roadmap.as_ref(),
        sas_sections: &sas_sections,
    };
    for one in &loaded {
        check_one(repo, one, shared, check_generated, &mut report);
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
    // §12 identity (OW-WAR-0119), once per run: duplicates and alias
    // resolution need every Warrant and every ADR in hand, so this sits after
    // both are loaded rather than inside the per-Warrant loop.
    identity_check::check(repo, &corpus, &loaded, &adrs.records, &mut report);
    // §101 — the SAS is a controlled document. The bytes on disk are held to
    // the latest recorded revision's digest. Until OW-WAR-0058 nothing in code
    // compared them: the digest appeared in six places, all prose.
    match repo.load_sas_revisions() {
        Err(err) => report.push(Diagnostic::error(
            "sas.revision-malformed",
            repo.config.paths.sas.clone(),
            err.to_string(),
        )),
        Ok(revisions) => match crate::sas::pin_of(&revisions) {
            None => report.push(Diagnostic::warn(
                "sas.unrecorded",
                repo.config.paths.sas.clone(),
                "no SAS revision is recorded; the document is pinned by prose only. \
                 `war sas propose <version>` records one"
                    .to_owned(),
            )),
            Some(pin) => match repo.sas_document() {
                Err(err) => report.push(Diagnostic::error(
                    "sas.document",
                    repo.config.paths.sas.clone(),
                    err.to_string(),
                )),
                Ok((path, bytes)) => {
                    let actual = openwarrant_compiler::sha256_hex(&bytes);
                    if actual == pin.sha256 {
                        report.push(Diagnostic::pass(
                            "sas.pinned",
                            format!(
                                "{} matches revision {} ({}), sha256:{}",
                                repo.relative(&path),
                                pin.version,
                                pin.state,
                                &pin.sha256[..12]
                            ),
                        ));
                    } else if let Some(proposed) = revisions
                        .iter()
                        .find(|r| r.sha256 == actual && r.version != pin.version)
                    {
                        // The remedy the error names, taken: the document IS a
                        // recorded proposal awaiting a human. The accepted
                        // revision stays normative until then (§101.2).
                        report.push(Diagnostic::warn(
                            "sas.proposed-unaccepted",
                            repo.relative(&path),
                            format!(
                                "the document is revision {} ({}), sha256:{}; the accepted \
                                 revision {} remains normative until `war sign {}` accepts it",
                                proposed.version,
                                proposed.state,
                                &actual[..12],
                                pin.version,
                                proposed.version
                            ),
                        ));
                    } else {
                        report.push(Diagnostic::error(
                            "sas.digest-drift",
                            repo.relative(&path),
                            format!(
                                "the document is sha256:{actual} but revision {} ({}) records \
                                 sha256:{}. §101.6: the accepted revision is normative and mirrors \
                                 state its exact digest. Either restore the bytes or propose a \
                                 new revision — an edited document under an unchanged record is \
                                 the failure this check exists for",
                                pin.version, pin.state, pin.sha256
                            ),
                        ));
                    }
                }
            },
        },
    }

    // Accepting a SAS revision is the act that makes a specification normative
    // for every Warrant that pins it, so it is held to the same rule as an
    // authorization: a human signature over the acceptance response's exact
    // bytes, naming the digest of the document accepted. Leaving this act out
    // would have left one path where a record is believed for its contents.
    let sas_revisions = repo.load_sas_revisions().unwrap_or_default();
    let sas_pin = crate::sas::pin_of(&sas_revisions).map(|p| p.version.clone());
    for rev in &sas_revisions {
        let Some(acceptance) = &rev.acceptance else {
            continue;
        };
        let subject = format!("SAS-{}", rev.version);
        let verdict = crate::authority_check::verify(
            repo,
            crate::authority_check::Act::Accept,
            &subject,
            &acceptance.accepted_by,
            Some(&rev.sha256),
        );
        if verdict.is_signed() {
            report.push(Diagnostic::pass(
                "authority.signed",
                format!("{subject}: {}", verdict.why()),
            ));
        } else if sas_pin.as_deref() == Some(rev.version.as_str()) {
            report.push(Diagnostic::error(
                verdict.rule(),
                format!("docs/sas/revisions/{}.toml", rev.version),
                format!("{subject}: {}", verdict.why()),
            ));
        } else {
            // A superseded revision is normative for nothing. Its acceptance
            // predates enforcement and cannot be signed now without dating the
            // act wrongly, so it is reported and not treated as a live failure.
            report.push(Diagnostic::warn(
                verdict.rule(),
                format!("docs/sas/revisions/{}.toml", rev.version),
                format!(
                    "{subject}: superseded by {} and normative for nothing; {}",
                    sas_pin.as_deref().unwrap_or("no accepted revision"),
                    verdict.why()
                ),
            ));
        }
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
        drift_check(
            repo,
            crate::status::corpus_status_md(repo),
            "corpus-status",
            &mut report,
        );
        drift_check(
            repo,
            crate::status::corpus_status_json(repo),
            "corpus-status",
            &mut report,
        );
        drift_check(
            repo,
            crate::status::corpus_status_html(repo),
            "corpus-status",
            &mut report,
        );
        drift_check(
            repo,
            crate::timeline::corpus_timeline_json(repo),
            "corpus-timeline",
            &mut report,
        );
        drift_check(
            repo,
            crate::timeline::corpus_pending_json(repo),
            "corpus-pending",
            &mut report,
        );
        // The SAS normative projection (E1), when there is a document.
        if repo.sas_document().is_ok() {
            match crate::compile::sas_normative(repo) {
                Ok(files) => {
                    for file in files {
                        drift_check(repo, Ok(file), "sas-normative", &mut report);
                    }
                }
                Err(e) => drift_check(repo, Err(e), "sas-normative", &mut report),
            }
            // The section index (OW-WAR-0125), by the same function.
            match crate::compile::sas_sections(repo) {
                Ok(files) => {
                    for file in files {
                        drift_check(repo, Ok(file), "sas-sections", &mut report);
                    }
                }
                Err(e) => drift_check(repo, Err(e), "sas-sections", &mut report),
            }
        }
        // OW-ADR-0022: the master document, and the history when this
        // repository keeps one. `generated.drift`, the rule every projection
        // of a Warrant reports under: CURRENT.md is written by `compile` and by
        // nothing else. With `history = false` nothing is compiled for
        // HISTORY.md, so nothing is compared.
        match crate::compile::master_documents(repo) {
            Ok(files) => {
                for file in files {
                    drift_check(repo, Ok(file), "generated", &mut report);
                }
            }
            Err(e) => drift_check(repo, Err(e), "generated", &mut report),
        }
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
    if only.is_none() {
        check_roadmap_status_claims(repo, &mut report);
    }

    Ok(report)
}

/// `storage.stray-temp` (warning) for each file `atomic::stray` finds under
/// `docs/`; with one Warrant asked for, only those inside its directory.
fn check_stray_temps(
    repo: &Repository,
    within: Option<&[camino::Utf8PathBuf]>,
    report: &mut Report,
) {
    for s in crate::compile::atomic::stray(&repo.root) {
        if within.is_some_and(|dirs| !dirs.iter().any(|d| s.temp.starts_with(d))) {
            continue;
        }
        report.push(Diagnostic::warn(
            "storage.stray-temp",
            repo.relative(&s.temp),
            format!(
                "{} is a temp file left by a write that stopped before its rename; it was meant \
                 to replace {}, which is whole as it stands (the previous bytes, or absent). \
                 Nothing reads the temp file. Remove it once you know which act stopped",
                repo.relative(&s.temp),
                repo.relative(&s.record)
            ),
        ));
    }
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
/// §37.2 — a declared deliverable's recorded digest must still match its bytes.
///
/// # Why this belongs in `war check` and not only in `war resolve`
///
/// `war resolve` already recomputes these digests, and it caught the drift both
/// times it happened. It caught it *late*: resolve is run deliberately, by
/// someone asking about one Warrant, so a stale record sat in `main` until
/// somebody thought to look. Twice in two days an unrelated pull request edited
/// a file some Warrant had declared — once `ci.yml` under a Bonsai change, once
/// `ci.yml` again under a dependency bump — and each time OW-WAR-0001 silently
/// lost a requirement.
///
/// A record that no longer describes the bytes it names is false as written, so
/// this is an ERROR rather than a warning. The remedy is small and specific:
/// either the artifact changed and the record must be regenerated, or the
/// artifact changed and should not have.
///
/// # Deliberately not silent about the missing case
///
/// A deliverable declaring `content_addressed` with no provenance, or with a
/// target that cannot be read, is reported rather than skipped. "The digest does
/// not match" and "there is nothing to match against" are different problems and
/// a check that collapsed them would send the reader to the wrong fix.
/// Does a signed record bind this Warrant's deliverable digests?
///
/// Exactly one thing does: a resolution, whose §56.2 record carries
/// `artifact_manifest_digest` = sha256 of `deliverables.toml`. Moving a pin
/// under it would change what the resolution resolved, so that is an ERROR and
/// `war correct` is the act for it (OW-WAR-0064).
///
/// An authorization does NOT. The contract digest covers intent, scope,
/// obligations, milestones and stages — not which bytes a file happens to have
/// while the work is being done. Treating it as binding sent a signed but
/// unresolved Warrant to `war correct`, which refuses with
/// `correction.not-resolved`: an error whose only remedy was itself refused.
///
/// So a pin that no one has resolved against is out of date, not violated —
/// `war pins --refresh` re-records it. That distinction is why `war check` can
/// run during ordinary work: a repository with nineteen unsigned Warrants
/// pinning living source files reported ten ERRORs and NOT READY on every
/// commit, forever, protecting nothing.
pub fn resolution_binds(repo: &Repository, one: &Loaded) -> bool {
    repo.load_resolution(&one.dir).ok().flatten().is_some()
}

fn check_deliverable_digests(
    repo: &Repository,
    one: &Loaded,
    alias: &str,
    ownership: &crate::ownership::Ownership,
    report: &mut Report,
) {
    let bound = resolution_binds(repo, one);
    let authorization = repo.load_authorization(&one.dir).ok().flatten();
    let authorized_at = authorization
        .as_ref()
        .and_then(|a| a.revision.authorization.as_ref())
        .map(|a| a.effective_time.clone());
    let deliverables = match repo.load_deliverables(&one.dir) {
        Ok(set) => set,
        Err(err) => {
            report.push(Diagnostic::error(
                "deliverable.unreadable",
                repo.relative(&one.dir.join("deliverables.toml")),
                format!("{alias}: {err}"),
            ));
            return;
        }
    };
    let file = repo.relative(&one.dir.join("deliverables.toml"));
    for (path, why) in &deliverables.failures {
        report.push(Diagnostic::error(
            "deliverable.malformed",
            path.clone(),
            format!("{why} — an unreadable deliverables file is not an absent one"),
        ));
    }

    // OW-ADR-0021: the set the authorizer signed for is the set that owns.
    // A record with no `owned` set predates ownership and owns nothing, so
    // there is nothing to compare; one with a set is held to it both ways.
    // `authorize` refuses a set that moved between drafting and signing
    // (`authorize.stale-deliverables`); these two catch the edit AFTER.
    if let Some(record) = &authorization
        && !record.owned.is_empty()
        && deliverables.failures.is_empty()
    {
        let declared: BTreeSet<(&str, &str)> = deliverables
            .records
            .iter()
            .map(|d| (d.id.as_str(), d.target_ref.as_str()))
            .collect();
        let owned: BTreeSet<(&str, &str)> = record
            .owned
            .iter()
            .map(|d| (d.id.as_str(), d.target_ref.as_str()))
            .collect();
        let mut agree = true;
        for (id, target) in declared.difference(&owned) {
            agree = false;
            report.push(Diagnostic::warn(
                "deliverable.undeclared-at-authorization",
                file.clone(),
                format!(
                    "{alias}: {id} → {target} is declared now but was not in the set the \
                     authorization signed for, so {alias} does not own it and no other \
                     Warrant's pin on it becomes historical. Ownership widens only through \
                     an amendment and a re-authorization (§31, OW-ADR-0021)"
                ),
            ));
        }
        for (id, target) in owned.difference(&declared) {
            agree = false;
            report.push(Diagnostic::error(
                "deliverable.declared-then-removed",
                file.clone(),
                format!(
                    "{alias}: the authorization signed for {id} → {target} and \
                     deliverables.toml no longer declares it. A signed claim on a path \
                     cannot be withdrawn by deleting the line — restore the declaration, \
                     or amend and re-authorize (§31, OW-ADR-0021)"
                ),
            ));
        }
        if agree {
            report.push(Diagnostic::pass(
                "deliverable.owned",
                format!(
                    "{alias}: the {} deliverable(s) declared are the {} the authorization \
                     signed for",
                    declared.len(),
                    owned.len()
                ),
            ));
        }
    }

    let addressed: Vec<_> = deliverables
        .records
        .iter()
        .filter(|d| d.content_addressed)
        .collect();
    if addressed.is_empty() {
        return;
    }

    // OW-WAR-0064 — a resolved Warrant's pin may have been superseded by an
    // authorized correction. The structural checks on the correction records
    // live in `correct::check`; the chain is resolved HERE, beside the digest
    // comparison, so "corrected" and "drifted" are decided by one computation.
    crate::correct::check(repo, one, alias, report);
    let corrections = repo.load_corrections(&one.dir).unwrap_or_default();

    let mut drifted = 0usize;
    let mut corrected = 0usize;
    let mut historical = 0usize;
    for deliverable in &addressed {
        let Some(provenance) = deliverable.provenance.as_ref() else {
            report.push(Diagnostic::error(
                "deliverable.no-provenance",
                file.clone(),
                format!(
                    "{alias}: {} declares content addressing and carries no provenance, so there \
                     is no digest to check",
                    deliverable.id
                ),
            ));
            drifted += 1;
            continue;
        };
        // Only sha256 is supported, and an unsupported algorithm is said so
        // rather than compared. `trim_start_matches` would leave "md5:abc"
        // intact, which never equals a sha256 hex, so the check still refuses —
        // but it refuses as "digest drift", sending the reader to regenerate a
        // record whose algorithm is the actual problem. The verdict was already
        // right; the diagnosis was not.
        let Some(recorded) = provenance.content_digest.strip_prefix("sha256:") else {
            report.push(Diagnostic::error(
                "deliverable.unsupported-digest",
                file.clone(),
                format!(
                    "{alias}: {} records {:?}, and only sha256: is supported. This is not                      drift — the record cannot be checked at all",
                    deliverable.id, provenance.content_digest
                ),
            ));
            drifted += 1;
            continue;
        };
        let (chain, head) =
            crate::correct::head_for(&corrections, &deliverable.id, &provenance.content_digest);
        let head = match head {
            Ok(h) => h,
            Err(openwarrant_core::correction::ChainError::Gap { expected, found }) => {
                report.push(Diagnostic::error(
                    "correction.sequence-gap",
                    file.clone(),
                    format!(
                        "{alias}: {} corrections are numbered 1..n without gaps; expected {expected}, \
                         found {found}",
                        deliverable.id
                    ),
                ));
                drifted += 1;
                continue;
            }
            Err(e @ openwarrant_core::correction::ChainError::SupersededNeverDelivered { .. }) => {
                report.push(Diagnostic::error(
                    "correction.superseded-never-delivered",
                    file.clone(),
                    format!("{alias}: {} — {e}", deliverable.id),
                ));
                drifted += 1;
                continue;
            }
        };
        let head_hex = head.trim_start_matches("sha256:");
        match std::fs::read(repo.root.join(&deliverable.target_ref)) {
            Ok(bytes) => {
                let actual = openwarrant_compiler::sha256_hex(&bytes);
                if actual == head_hex {
                    if !chain.is_empty() {
                        corrected += 1;
                        report.push(Diagnostic::pass(
                            "deliverable.corrected",
                            format!(
                                "{alias}: {} corrected {} time(s); sha256:{recorded} superseded, \
                                 now {head}",
                                deliverable.id,
                                chain.len()
                            ),
                        ));
                    }
                } else if let Some(newer) = bound
                    .then(|| {
                        ownership.newer_than(
                            &deliverable.target_ref,
                            alias,
                            authorized_at.as_deref(),
                        )
                    })
                    .flatten()
                {
                    // OW-ADR-0021: a later authorized Warrant declares this
                    // path, so the bytes are its to answer for. This pin is a
                    // claim about a moment — `war pins --history` finds the
                    // commit — and neither drift nor a correction applies.
                    // Decided before the correction branch on purpose: a
                    // chain that stopped matching because a NEWER OWNER moved
                    // the file is not a correction that corrects nothing.
                    historical += 1;
                    report.push(Diagnostic::pass(
                        "deliverable.superseded-by",
                        format!(
                            "{alias}: {} pinned {} at {head}; {}/{} (authorized {}) now \
                             governs that path, so this pin is historical and the bytes \
                             are that Warrant's to answer for (OW-ADR-0021) — `war pins \
                             --history {}`",
                            deliverable.id,
                            deliverable.target_ref,
                            newer.alias,
                            newer.deliverable_id,
                            newer.authorized_at,
                            deliverable.target_ref
                        ),
                    ));
                } else if !chain.is_empty() {
                    report.push(Diagnostic::error(
                        "correction.new-digest-mismatch",
                        file.clone(),
                        format!(
                            "{alias}: {} — the latest correction records {head} but the file is \
                             sha256:{actual}; it corrects nothing. A further change is a further \
                             correction, `war correct {alias} {}`",
                            deliverable.id, deliverable.id
                        ),
                    ));
                    drifted += 1;
                } else if bound {
                    report.push(Diagnostic::error(
                        "deliverable.digest-drift",
                        file.clone(),
                        format!(
                            "{alias}: {} records sha256:{recorded} for {} but the file is now \
                             sha256:{actual}. A resolution binds this manifest and no later \
                             authorized Warrant declares {}, so the artifact moved outside any \
                             authorization — restore it; declare it as a deliverable of a \
                             Warrant and have that Warrant authorized (OW-ADR-0021); or record \
                             why it moved: `war correct {alias} {}` (OW-WAR-0064)",
                            deliverable.id,
                            deliverable.target_ref,
                            deliverable.target_ref,
                            deliverable.id
                        ),
                    ));
                    drifted += 1;
                } else {
                    // Nobody has signed for this Warrant, so the pin is a note
                    // about a file, not a promise about it.
                    report.push(Diagnostic::warn(
                        "deliverable.pin-stale",
                        file.clone(),
                        format!(
                            "{alias}: {} records sha256:{recorded} for {} and the file is now \
                             sha256:{actual}. No resolution binds this manifest, so the pin is \
                             out of date rather than violated — `war pins --refresh {alias}`",
                            deliverable.id, deliverable.target_ref
                        ),
                    ));
                }
            }
            Err(err) => {
                let why = format!(
                    "{alias}: {} names {} and it cannot be read: {err}. An unreadable artifact \
                     is not a verified one",
                    deliverable.id, deliverable.target_ref
                );
                if bound {
                    report.push(Diagnostic::error(
                        "deliverable.target-unreadable",
                        file.clone(),
                        why,
                    ));
                    drifted += 1;
                } else {
                    report.push(Diagnostic::warn(
                        "deliverable.target-unreadable",
                        file.clone(),
                        why,
                    ));
                }
            }
        }
    }

    if drifted == 0 {
        report.push(Diagnostic::pass(
            "deliverable.digests",
            format!(
                "{alias}: {} content-addressed deliverable(s) match their bytes{}{}",
                addressed.len(),
                if corrected > 0 {
                    format!(" ({corrected} through an authorized correction)")
                } else {
                    String::new()
                },
                if historical > 0 {
                    format!(" ({historical} historical, governed by a later Warrant)")
                } else {
                    String::new()
                }
            ),
        ));
    }
}

/// §34 and §105 — the references a Warrant makes to the SAS and the Roadmap.
///
/// # Why these were never checked before
///
/// `Implements.contribution` is an `Option<String>` on disk and `RoadmapRef` is
/// a bare `String`, and both were copied verbatim into the IR. So a manifest
/// could say `contribution = "mostly"` or `roadmap://OW-PHASE-11/x` and nothing
/// would object — `war show` says outright that only `war://` is resolved. The
/// corpus projection groups Warrants by phase and derives requirement status
/// from contributions, so both now have to parse, and a value that does not
/// parse is reported here rather than silently dropped from a count.
///
/// An absent contribution is a warning, not an error: §34.2 says a WAR SHOULD
/// declare it, and turning a SHOULD into a refusal would be reading a rule into
/// the text.
fn check_traceability(
    repo: &Repository,
    one: &Loaded,
    alias: &str,
    roadmap: Option<&crate::roadmap_cmd::Loaded>,
    report: &mut Report,
) {
    use openwarrant_core::traceability::{Contribution, RequirementRef, RoadmapRef};

    let Some(basis) = one.basis.as_ref() else {
        return;
    };
    let file = repo.relative(&one.dir.join("manifest.toml"));
    let mut bad = 0usize;

    for r in &basis.manifest.roadmap {
        match RoadmapRef::parse(&r.r#ref) {
            Err(err) => {
                report.push(Diagnostic::error(
                    "roadmap.malformed",
                    file.clone(),
                    format!("{alias}: {err}"),
                ));
                bad += 1;
            }
            // A `roadmap://` ref names THIS program's §98: another prefix is a
            // phase of a SAS this repository does not carry (slice C5).
            Ok(parsed) if parsed.prefix != repo.config.project.namespace.as_str() => {
                report.push(Diagnostic::error(
                    "roadmap.wrong-namespace",
                    file.clone(),
                    format!(
                        "{alias}: {} names phase prefix {:?}; this repository's namespace is \
                         {:?}, and its SAS is the only §98 a roadmap ref can point into",
                        r.r#ref,
                        parsed.prefix,
                        repo.config.project.namespace.as_str()
                    ),
                ));
                bad += 1;
            }
            // OW-ADR-0023: with a roadmap record, the phase must be one of its
            // phases; without one, §98's 0..=10.
            Ok(parsed) => match roadmap {
                Some(rm) => {
                    if !crate::roadmap_cmd::check_ref(rm, alias, &parsed, &file, report) {
                        bad += 1;
                    }
                }
                None if parsed.phase > RoadmapRef::MAX_PHASE => {
                    report.push(Diagnostic::error(
                        "roadmap.malformed",
                        file.clone(),
                        format!(
                            "{alias}: {} names phase {}; §98 defines 0..=10 and this program has no roadmap record to say otherwise",
                            r.r#ref, parsed.phase
                        ),
                    ));
                    bad += 1;
                }
                None => {}
            },
        }
    }

    // §106 of the SAS as it stands: an `implements` ref must name a row that
    // exists. `None` when the document cannot be read — then nothing is
    // refused and nothing is vouched for (OW-WAR-0063).
    let known_requirements: Option<std::collections::BTreeMap<String, String>> = repo
        .sas_document()
        .ok()
        .map(|(_, bytes)| openwarrant_core::sas::section_106(&String::from_utf8_lossy(&bytes)))
        .filter(|m| !m.is_empty());
    for i in &basis.manifest.implements {
        if let Err(err) = RequirementRef::parse(&i.r#ref) {
            report.push(Diagnostic::error(
                "traceability.requirement-ref",
                file.clone(),
                format!("{alias}: {err}"),
            ));
            bad += 1;
        }
        if let Ok(rq) = RequirementRef::parse(&i.r#ref)
            && let Some(known) = &known_requirements
            && !known.contains_key(&rq.canonical())
        {
            bad += 1;
            report.push(Diagnostic::error(
                "traceability.unknown-requirement",
                repo.relative(&one.dir.join("manifest.toml")),
                format!(
                    "{alias}: implements {} and §106 of the pinned SAS has no such row (§34.1: a Warrant implements a requirement that exists)",
                    i.r#ref
                ),
            ));
        }
        match i.contribution.as_deref() {
            None => report.push(Diagnostic::warn(
                "traceability.contribution-unstated",
                file.clone(),
                format!(
                    "{alias}: {} declares no contribution; §34.2 says a WAR SHOULD state \
                     one of {}",
                    i.r#ref,
                    Contribution::known()
                ),
            )),
            Some(text) => {
                if let Err(err) = text.parse::<Contribution>() {
                    report.push(Diagnostic::error(
                        "traceability.contribution",
                        file.clone(),
                        format!("{alias}: {}: {err}", i.r#ref),
                    ));
                    bad += 1;
                }
            }
        }
    }

    if bad == 0 && !(basis.manifest.roadmap.is_empty() && basis.manifest.implements.is_empty()) {
        report.push(Diagnostic::pass(
            "traceability.refs",
            format!(
                "{alias}: {} requirement ref(s) and {} roadmap ref(s) parse",
                basis.manifest.implements.len(),
                basis.manifest.roadmap.len()
            ),
        ));
    }
}

/// What every Warrant's check reads from the corpus as a whole: built once
/// in `run`, never per Warrant.
#[derive(Clone, Copy)]
struct Shared<'a> {
    corpus: &'a [Loaded],
    /// Contract digests by alias, for a child's citation of its parent.
    parent_digests: &'a BTreeMap<String, String>,
    gates: &'a openwarrant_core::GateRegistry,
    /// OW-ADR-0021: which Warrant governs each path now.
    ownership: &'a crate::ownership::Ownership,
    /// OW-ADR-0023: the roadmap record, when the program has one.
    roadmap: Option<&'a crate::roadmap_cmd::Loaded>,
    /// OW-WAR-0125: recorded SAS revisions by section, for citations.
    sas_sections: &'a crate::sas::RevisionSections<'a>,
}

fn check_one(
    repo: &Repository,
    one: &Loaded,
    shared: Shared<'_>,
    check_generated: bool,
    report: &mut Report,
) {
    let Shared {
        corpus,
        parent_digests,
        gates,
        ownership,
        roadmap,
        sas_sections,
    } = shared;
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

    check_deliverable_digests(repo, one, &alias, ownership, report);
    check_traceability(repo, one, &alias, roadmap, report);
    {
        // §44.6 recorded runs and the §56.2 record, both held to the contract
        // as it compiles NOW (OW-WAR-0059).
        let current = match (&one.basis, &one.validated) {
            (Some(basis), Some(validated)) => openwarrant_compiler::lower(basis, validated)
                .ok()
                .and_then(|ir| ir.contract_digest().ok()),
            _ => None,
        };
        crate::evidence::check(repo, &one.dir, &alias, current.as_deref(), report);
        crate::resolution_cmd::check(repo, &one.dir, &alias, current.as_deref(), report);
        let uuid = one.validated.as_ref().map(|v| v.uuid.to_string());
        crate::journal_cmd::check(repo, &one.dir, &alias, uuid.as_deref(), report);
        // §14 — the SAS pin must name a recorded revision, and a Warrant pinned
        // behind the latest revision is said so. The pin is the latest
        // amendment's `sas_revision` when one names it (OW-ADR-0016), else the
        // authorization's.
        let all = repo.load_sas_revisions().unwrap_or_default();
        let amended = crate::repo::amendment_sas_revision(&one.dir);
        if let Some((v, amendment_path)) = &amended {
            let file = repo.relative(amendment_path);
            match all.iter().find(|r| &r.version == v) {
                None => report.push(Diagnostic::error(
                    "sas.pin-unknown",
                    file,
                    format!("{alias}: an amendment re-pins to SAS revision {v}, and no record of it exists under docs/sas/revisions/ — `war sas propose {v}` first"),
                )),
                Some(rev) => {
                    // The re-pinned revision must still carry every row this
                    // Warrant implements; a requirement that vanished under it
                    // is a broken trace, not a silent one.
                    for i in &basis.manifest.implements {
                        if let Ok(rq) = openwarrant_core::traceability::RequirementRef::parse(&i.r#ref)
                            && !rev.requirements.contains_key(&rq.canonical())
                        {
                            report.push(Diagnostic::error(
                                "sas.repin-unknown-requirement",
                                file.clone(),
                                format!("{alias}: re-pinned to SAS {v}, whose §106 has no {} — the Warrant implements a requirement that revision does not have", i.r#ref),
                            ));
                        }
                    }
                    if repo.load_resolution(&one.dir).ok().flatten().is_some() {
                        report.push(Diagnostic::warn(
                            "sas.repin-resolved",
                            file,
                            format!("{alias}: re-pinned to SAS {v} after resolution; the resolution binds the contract as it was and is reported stale, never moved (§56.3)"),
                        ));
                    }
                }
            }
        }
        // Is this authorization one a human signed? Until this call existed the
        // answer was "it says so in the file". A forged authorization.toml
        // naming the owner passed `war check` with zero errors and satisfied
        // §56.1 requirement 1 (demonstrated 2026-09-19 against 1.0.0-alpha.1).
        if let Ok(Some(a)) = repo.load_authorization(&one.dir)
            && let Some(auth) = &a.revision.authorization
        {
            {
                let verdict = crate::authority_check::verify(
                    repo,
                    crate::authority_check::Act::Authorize,
                    &alias,
                    &auth.authorizer,
                    Some(&a.revision.contract_digest),
                );
                let at = repo.relative(&one.dir.join("authorization.toml"));
                if verdict.is_signed() {
                    report.push(Diagnostic::pass(
                        verdict.rule(),
                        format!("{alias}: {}", verdict.why()),
                    ));
                } else {
                    report.push(Diagnostic::error(
                        verdict.rule(),
                        at,
                        format!("{alias}: {}", verdict.why()),
                    ));
                }
            }
        }
        // A resolution is the act that says the work is done. It was trusted on
        // content alone for exactly as long as the authorization was.
        if let Ok(Some(r)) = repo.load_resolution(&one.dir) {
            let verdict = crate::authority_check::verify(
                repo,
                crate::authority_check::Act::Resolve,
                &alias,
                r.resolution.resolved_by_ref.trim_start_matches("person://"),
                Some(&r.resolution.contract_digest),
            );
            let at = repo.relative(&one.dir.join("resolution.toml"));
            if verdict.is_signed() {
                report.push(Diagnostic::pass(
                    "authority.signed",
                    format!("{alias} resolution: {}", verdict.why()),
                ));
            } else {
                report.push(Diagnostic::error(
                    verdict.rule(),
                    at,
                    format!("{alias} resolution: {}", verdict.why()),
                ));
            }
        }
        // Each correction moves a delivered file past the wall, so each needs
        // its own signature over its own bytes.
        if let Ok(set) = repo.load_corrections(&one.dir) {
            for (_, c) in &set.records {
                let subject = format!("{alias}.{}", c.correction.deliverable_id);
                let verdict = crate::authority_check::verify(
                    repo,
                    crate::authority_check::Act::Correct,
                    &subject,
                    c.correction
                        .authorized_by_ref
                        .trim_start_matches("person://"),
                    Some(&c.correction.new_digest),
                );
                if verdict.is_signed() {
                    report.push(Diagnostic::pass(
                        "authority.signed",
                        format!("{subject} correction: {}", verdict.why()),
                    ));
                } else {
                    report.push(Diagnostic::error(
                        verdict.rule(),
                        repo.relative(&one.dir.join("corrections")),
                        format!("{subject} correction: {}", verdict.why()),
                    ));
                }
            }
        }
        if let Ok(Some(a)) = repo.load_authorization(&one.dir)
            && let Some(v) = &a.sas_revision
            && amended.is_none()
        {
            if !all.iter().any(|r| &r.version == v) {
                report.push(Diagnostic::error(
                    "sas.pin-unknown",
                    repo.relative(&one.dir.join("authorization.toml")),
                    format!("{alias}: authorized against SAS revision {v}, and no record of it exists under docs/sas/revisions/"),
                ));
            } else if let Some(latest) = repo.latest_sas_revision().ok().flatten()
                && &latest.version != v
            {
                // A resolved Warrant executes nothing further under any
                // Basis; the revision it was accepted under is its history,
                // not a debt. Only a Warrant still open to execution is asked
                // to re-pin (OW-WAR-0112, OW-ADR-0021).
                if repo.load_resolution(&one.dir).ok().flatten().is_some() {
                    report.push(Diagnostic::pass(
                        "sas.pin-historical",
                        format!(
                            "{alias}: resolved under SAS {v}; {} is in force now and asks \
                             nothing of a Warrant that executes no further",
                            latest.version
                        ),
                    ));
                } else {
                    report.push(Diagnostic::warn(
                        "sas.pin-superseded",
                        repo.relative(&one.dir.join("authorization.toml")),
                        format!("{alias}: authorized against SAS {v}; the latest recorded revision is {} — the contract keeps its Basis until an amendment carrying `sas_revision: \"{}\"` re-pins it and a human re-authorizes (OW-ADR-0016)", latest.version, latest.version),
                    ));
                }
            }
        }
    }

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

    check_section_refs(repo, one, &alias, sas_sections, report);

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

    // OW-ADR-0022 — an atom's relation to the projections is its role. A role
    // no row of the composition table renders is text nobody reads, and is
    // refused rather than silently dropped from both projections.
    for atom in &basis.atoms {
        if openwarrant_compiler::role_row(&atom.role).is_none() {
            report.push(Diagnostic::error(
                "atom.role-unprojected",
                repo.relative(&one.dir.join(&atom.source)),
                format!(
                    "{alias}: atom {} has role `{}`, which no projection renders — the \
                     master document and the history compose only the roles in \
                     `current.rs`'s ROLE_SECTIONS ({}). Give it one of those roles, or \
                     fold its text into the atom whose question it answers",
                    atom.source,
                    atom.role,
                    openwarrant_compiler::ROLE_SECTIONS
                        .iter()
                        .map(|r| r.role)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }

    // OW-ADR-0022 — presets ask; they do not answer. A heading a preset marked
    // `<!-- required -->` whose body is still only the preset's comments is a
    // question nobody answered: allowed in a draft (a warning), refused once
    // an authorization is on record for the Warrant (an error), because a
    // signed atom that still asks is a contract with a hole in it.
    let authorized = one.dir.join("authorization.toml").is_file();
    for atom in basis.atoms.iter().filter(|a| a.source.ends_with(".md")) {
        let text = String::from_utf8_lossy(&atom.bytes);
        for heading in unanswered_required_headings(&text) {
            let file = repo.relative(&one.dir.join(&atom.source));
            let message = format!(
                "{alias}: atom {} — the preset's required heading `{heading}` is \
                 unanswered: its body is only the preset's comment{}",
                atom.source,
                if authorized {
                    ". An authorization is on record for this Warrant, and a signed \
                     atom may not leave a required question open"
                } else {
                    ". Answer it before asking for authorization; a draft may be \
                     unfinished"
                }
            );
            report.push(if authorized {
                Diagnostic::error("atom.preset-unanswered", file, message)
            } else {
                Diagnostic::warn("atom.preset-unanswered", file, message)
            });
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

    check_parent_citations(repo, one, &alias, basis, corpus, parent_digests, report);

    if check_generated {
        let children = crate::compile::children_of(&validated.raw.uuid, corpus);
        check_drift(repo, &children, one, basis, validated, &alias, report);
    }
}

/// `sas.section-ref` and `sas.section-current` (OW-WAR-0125): an amendment
/// whose `governing_adr_or_policy` is `sas://<NS>-SAS-<n>[.<m>]` cites a
/// section of the SAS, and the citation is held to the revision the Warrant
/// is pinned to — the latest amendment's `sas_revision`, else the
/// authorization's, else the latest recorded revision.
///
/// - `sas.section-ref`: the section or subsection exists at the pinned
///   revision, under the namespace its §106 carries. An error otherwise, and
///   for a `sas://…-SAS-…` value that is neither a requirement nor a section.
/// - `sas.section-current`: its digest at the pinned revision against the
///   latest accepted revision. Equal passes; different (or gone) warns,
///   naming both revisions; bytes that cannot be read are UNKNOWN with the
///   reason (Law 15), never a pass.
///
/// A pin naming no recorded revision is `sas.pin-unknown`'s to report; the
/// section rules say nothing about a revision that does not exist.
fn check_section_refs(
    repo: &Repository,
    one: &Loaded,
    alias: &str,
    sections: &crate::sas::RevisionSections<'_>,
    report: &mut Report,
) {
    let Ok(entries) = one.dir.join("amendments").read_dir_utf8() else {
        return;
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .collect();
    paths.sort();
    let mut cited = Vec::new();
    for path in paths {
        let Some(record) = std::fs::read_to_string(&path).ok().and_then(|text| {
            openwarrant_core::structured::parse(&text)
                .ok()
                .and_then(|doc| openwarrant_core::autonomy::from_structured(&doc).ok())
        }) else {
            continue; // amendment.invalid / amendment.unreadable report it
        };
        let file = repo.relative(&path);
        let value = record.governing_adr_or_policy.trim().to_owned();
        match openwarrant_core::sas_sections::parse_ref(&value) {
            Ok(None) => {}
            Ok(Some(r)) => cited.push((file, record.id, value, r)),
            Err(why) => report.push(Diagnostic::error(
                "sas.section-ref",
                file,
                format!("{alias}: amendment {} — {why}", record.id),
            )),
        }
    }
    if cited.is_empty() {
        return;
    }
    let revisions = sections.revisions();
    let pinned = crate::repo::amendment_sas_revision(&one.dir)
        .map(|(v, _)| v)
        .or_else(|| {
            repo.load_authorization(&one.dir)
                .ok()
                .flatten()
                .and_then(|a| a.sas_revision)
        })
        .or_else(|| crate::sas::pin_of(revisions).map(|r| r.version.clone()));
    let Some(pinned) = pinned.filter(|v| revisions.iter().any(|r| &r.version == v)) else {
        return;
    };
    let latest = revisions
        .iter()
        .filter(|r| r.is_accepted())
        .max_by(|a, b| a.version.cmp(&b.version))
        .map(|r| r.version.clone());
    for (file, id, value, r) in cited {
        let at_pin = match sections.get(&pinned) {
            Ok(split) => split,
            Err(why) => {
                for rule in ["sas.section-ref", "sas.section-current"] {
                    report.push(Diagnostic::unknown(
                        rule,
                        file.clone(),
                        format!(
                            "{alias}: amendment {id} cites {value}; SAS {pinned}, which the \
                             Warrant is pinned to, cannot be read: {why}"
                        ),
                    ));
                }
                continue;
            }
        };
        if let Some(ns) = &at_pin.namespace
            && ns != &r.namespace
        {
            report.push(Diagnostic::error(
                "sas.section-ref",
                file,
                format!(
                    "{alias}: amendment {id} cites {value}, but SAS {pinned} is namespace {ns}; \
                     a section is cited as sas://{ns}-SAS-{}",
                    r.section
                ),
            ));
            continue;
        }
        let Some(digest) = openwarrant_core::sas_sections::resolve(&at_pin.sections, &r) else {
            report.push(Diagnostic::error(
                "sas.section-ref",
                file,
                format!(
                    "{alias}: amendment {id} cites {value}, and SAS {pinned} has no {} {}. A \
                     citation names a section of the revision the Warrant is pinned to \
                     (docs/sas/generated/SECTIONS.md lists the revision in force)",
                    if r.section.contains('.') {
                        "subsection"
                    } else {
                        "section"
                    },
                    r.section
                ),
            ));
            continue;
        };
        report.push(Diagnostic::pass(
            "sas.section-ref",
            format!("{alias}: {value} names a section of SAS {pinned}"),
        ));
        let Some(latest) = &latest else {
            report.push(Diagnostic::unknown(
                "sas.section-current",
                file,
                format!(
                    "{alias}: {value} — no SAS revision is accepted, so there is nothing to \
                     compare SAS {pinned} with"
                ),
            ));
            continue;
        };
        if latest == &pinned {
            report.push(Diagnostic::pass(
                "sas.section-current",
                format!("{alias}: {value} is cited at SAS {pinned}, the latest accepted revision"),
            ));
            continue;
        }
        match sections.get(latest) {
            Err(why) => report.push(Diagnostic::unknown(
                "sas.section-current",
                file,
                format!(
                    "{alias}: {value} at SAS {pinned} cannot be compared with SAS {latest}: {why}"
                ),
            )),
            Ok(now) => match openwarrant_core::sas_sections::resolve(&now.sections, &r) {
                Some(d) if d == digest => report.push(Diagnostic::pass(
                    "sas.section-current",
                    format!("{alias}: {value} is unchanged between SAS {pinned} and SAS {latest}"),
                )),
                Some(_) => report.push(Diagnostic::warn(
                    "sas.section-current",
                    file,
                    format!(
                        "{alias}: {value} changed between SAS {pinned}, which the Warrant is \
                         pinned to, and SAS {latest}, the latest accepted revision; what the \
                         amendment relied on is not what the SAS now says"
                    ),
                )),
                None => report.push(Diagnostic::warn(
                    "sas.section-current",
                    file,
                    format!(
                        "{alias}: {value} exists at SAS {pinned}, which the Warrant is pinned \
                         to, and not at SAS {latest}, the latest accepted revision"
                    ),
                )),
            },
        }
    }
}

/// One parent's authorized revisions, read as far as they can be: the latest
/// from its `authorization.toml`, earlier ones from retained history through
/// `contract_history::resolve` (OW-WAR-0123, A-001). A revision history cannot
/// supply is carried as the reason, never guessed.
struct ParentRevisions<'a> {
    repo: &'a Repository,
    alias: String,
    latest: u32,
    latest_digest: String,
    read: BTreeMap<u32, Result<String, String>>,
}

impl ParentRevisions<'_> {
    fn digest(&mut self, revision: u32) -> Result<String, String> {
        if revision == self.latest {
            return Ok(self.latest_digest.clone());
        }
        let (repo, alias) = (self.repo, self.alias.as_str());
        self.read
            .entry(revision)
            .or_insert_with(|| {
                let (value, _) = crate::contract_history::resolve(repo, alias, revision)
                    .map_err(|e| e.to_string())?;
                crate::contract_history::parse_ir(&value)
                    .map_err(|e| e.to_string())?
                    .contract_digest()
                    .map_err(|e| e.to_string())
            })
            .clone()
    }
}

/// §20.2 / §91.5 test 29 / RQ-023: a child cites an EXACT parent contract
/// revision, and the number and the digest must name the same one.
///
/// Until OW-WAR-0123 the digest was compared with the parent as it compiles
/// now and the number was never read, so a child could not rest on an older
/// exact revision, and "revision 1" at revision 2's digest passed. Now the
/// cited revision is looked up in the parent's authorization records:
///
/// - a revision the parent never had is an error (`relations.parent-revision`);
/// - the cited revision's own digest passes; an older one also warns that the
///   parent has moved (`relations.parent-moved`);
/// - another revision's digest is named as that revision — an error for an
///   unauthorized child, where the fix is an edit; a warning for an authorized
///   one, whose correction is an amendment (U-001, answered (b));
/// - a digest of no revision is an error (`relations.parent-digest`), as is a
///   parent whose working contract no longer compiles to its latest
///   authorized digest;
/// - what retained history cannot answer is UNKNOWN (Law 15).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn check_parent_citations(
    repo: &Repository,
    one: &Loaded,
    alias: &str,
    basis: &openwarrant_compiler::CompilationBasis,
    corpus: &[Loaded],
    parent_digests: &BTreeMap<String, String>,
    report: &mut Report,
) {
    let manifest_file = repo.relative(&one.dir.join("manifest.toml"));
    let child_authorized = matches!(repo.load_authorization(&one.dir), Ok(Some(_)));
    for parent in &basis.manifest.parents {
        let r#ref = parent.r#ref.as_str();
        let uuid = r#ref.strip_prefix("war://").unwrap_or(r#ref);
        let in_corpus = corpus.iter().find(|l| {
            l.validated
                .as_ref()
                .is_some_and(|v| v.uuid.to_string() == uuid)
        });
        let Some(parent_one) = in_corpus else {
            report.push(Diagnostic::unknown(
                "relations.parent-digest",
                manifest_file.clone(),
                format!(
                    "{alias}: parent {ref} is not in this repository, so its contract \
                     digest cannot be computed; cross-repository resolution needs \
                     federation"
                ),
            ));
            continue;
        };
        let parent_alias = parent_one.alias();
        let current = parent_digests.get(uuid);
        let Some(cited) = parent.contract_digest.as_deref() else {
            // Not an error: the citation is incomplete, not wrong. The value is
            // printed so the fix is a copy-paste rather than a research task.
            let hint = current.map_or_else(
                || "its contract does not compile, so no digest can be offered".to_owned(),
                |d| format!("its current digest is sha256:{d} — add `contract_digest = \"sha256:{d}\"` to pin it"),
            );
            report.push(Diagnostic::unknown(
                "relations.parent-digest",
                manifest_file.clone(),
                format!(
                    "{alias}: parent {ref} cites a revision but no contract_digest (§20.2). {hint}."
                ),
            ));
            continue;
        };
        let cited = cited.strip_prefix("sha256:").unwrap_or(cited);
        // The manifest refuses a parent without a revision; 1 is never reached
        // for a valid one.
        let r = parent.contract_revision.unwrap_or(1);

        let authorization = match repo.load_authorization(&parent_one.dir) {
            Ok(a) => a,
            Err(err) => {
                report.push(Diagnostic::unknown(
                    "relations.parent-revision",
                    manifest_file.clone(),
                    format!(
                        "{alias}: parent {parent_alias}'s authorization record cannot be read, \
                         so revision {r} cannot be looked up: {err}"
                    ),
                ));
                continue;
            }
        };
        let Some(authorization) = authorization else {
            // A draft parent (U-002): it has one revision, the one it compiles
            // to now. The comparison is today's.
            if r != 1 {
                report.push(Diagnostic::error(
                    "relations.parent-revision",
                    manifest_file.clone(),
                    format!(
                        "{alias}: parent {parent_alias} is cited at revision {r}, which does not \
                         exist: {parent_alias} has no authorized revision, so a citation of it \
                         can only be revision 1"
                    ),
                ));
                continue;
            }
            push_current_comparison(report, &manifest_file, alias, r#ref, cited, current);
            continue;
        };
        if authorization.revision.state != openwarrant_core::contract::RevisionState::Authorized {
            report.push(Diagnostic::unknown(
                "relations.parent-revision",
                manifest_file.clone(),
                format!(
                    "{alias}: parent {parent_alias}'s authorization record is not in the \
                     authorized state, so its revisions cannot be read from it"
                ),
            ));
            continue;
        }
        let latest = authorization.revision.revision;
        if r == 0 || r > latest {
            report.push(Diagnostic::error(
                "relations.parent-revision",
                manifest_file.clone(),
                format!(
                    "{alias}: parent {parent_alias} is cited at revision {r}, which does not \
                     exist: its latest authorized revision is {latest}"
                ),
            ));
            continue;
        }
        let mut revisions = ParentRevisions {
            repo,
            alias: parent_alias.clone(),
            latest,
            latest_digest: authorization.revision.contract_digest.clone(),
            read: BTreeMap::new(),
        };
        let wrong_revision = |report: &mut Report, belongs: u32| {
            let (severity, tail) = if child_authorized {
                (
                    Severity::Warn,
                    format!(
                        "{alias}'s contract is authorized, so the signed record stands as it \
                         is; making the number and the digest agree is an amendment of {alias}"
                    ),
                )
            } else {
                (
                    Severity::Error,
                    format!(
                        "cite revision {belongs}, or revision {r} at its own digest — `war new \
                         --parent {parent_alias}` writes the latest exactly"
                    ),
                )
            };
            report.push(Diagnostic {
                severity,
                rule: "relations.parent-revision".to_owned(),
                file: Some(manifest_file.clone()),
                message: format!(
                    "{alias}: parent {parent_alias} is cited as revision {r} at \
                     sha256:{cited}, which is the digest of revision {belongs}; {tail}"
                ),
            });
        };

        // The latest digest is on disk; no history is needed to see that an
        // older number was paired with it. The citation still rests on the
        // latest revision's content, so an unauthorized edit of the parent is
        // caught for it exactly as for a correct citation of the latest.
        if r < latest && cited == revisions.latest_digest {
            wrong_revision(report, latest);
            push_current_comparison(report, &manifest_file, alias, r#ref, cited, current);
            continue;
        }
        let own = match revisions.digest(r) {
            Ok(own) => own,
            Err(why) => {
                report.push(Diagnostic::unknown(
                    "relations.parent-revision",
                    manifest_file.clone(),
                    format!(
                        "{alias}: parent {parent_alias} is cited at revision {r}, whose digest \
                         only retained history holds, and it cannot be read: {why}. Neither \
                         pass nor error — run `war check` in a full clone"
                    ),
                ));
                continue;
            }
        };
        if own == cited {
            report.push(Diagnostic::pass(
                "relations.parent-revision",
                format!("{alias}: parent {parent_alias} is cited at revision {r}, at that revision's digest"),
            ));
            if r < latest {
                report.push(Diagnostic::warn(
                    "relations.parent-moved",
                    manifest_file.clone(),
                    format!(
                        "{alias}: parent {parent_alias} has moved from revision {r} to revision \
                         {latest}; the citation of revision {r} is exact and stays sound. \
                         Re-citing revision {latest} is an amendment of {alias}"
                    ),
                ));
            } else {
                // The latest authorized revision: the parent's working contract
                // must still compile to it, or the parent was edited without an
                // authorization — today's `relations.parent-digest` finding.
                push_current_comparison(report, &manifest_file, alias, r#ref, cited, current);
            }
            continue;
        }
        let mut unreadable = None;
        let mut belongs = None;
        for other in (1..=latest).rev().filter(|&o| o != r) {
            match revisions.digest(other) {
                Ok(d) if d == cited => {
                    belongs = Some(other);
                    break;
                }
                Ok(_) => {}
                Err(why) => {
                    unreadable.get_or_insert(why);
                }
            }
        }
        match (belongs, unreadable) {
            (Some(other), _) => wrong_revision(report, other),
            (None, None) => report.push(Diagnostic::error(
                "relations.parent-digest",
                manifest_file.clone(),
                format!(
                    "{alias}: parent {parent_alias} is cited as revision {r} at \
                     sha256:{cited}, which is the digest of no authorized revision of it; \
                     revision {r} is sha256:{own}"
                ),
            )),
            // Unauthorized: an error whichever revision the digest might name.
            (None, Some(why)) if !child_authorized => report.push(Diagnostic::error(
                "relations.parent-digest",
                manifest_file.clone(),
                format!(
                    "{alias}: parent {parent_alias} is cited as revision {r} at \
                     sha256:{cited}, which is not revision {r}'s digest sha256:{own}; whether \
                     it is an earlier revision's could not be read ({why})"
                ),
            )),
            (None, Some(why)) => report.push(Diagnostic::unknown(
                "relations.parent-revision",
                manifest_file.clone(),
                format!(
                    "{alias}: parent {parent_alias} is cited as revision {r} at \
                     sha256:{cited}, which is not revision {r}'s digest; which revision it \
                     names needs retained history that cannot be read: {why}"
                ),
            )),
        }
    }
}

/// The comparison with the parent as it compiles now: for a draft parent, and
/// for a citation of the latest authorized revision.
fn push_current_comparison(
    report: &mut Report,
    manifest_file: &str,
    alias: &str,
    r#ref: &str,
    cited: &str,
    current: Option<&String>,
) {
    match current {
        Some(actual) if actual == cited => report.push(Diagnostic::pass(
            "relations.parent-digest",
            format!("{alias}: parent {ref} contract digest matches"),
        )),
        Some(actual) => report.push(Diagnostic::error(
            "relations.parent-digest",
            manifest_file.to_owned(),
            format!(
                "{alias}: parent {ref} is cited at contract digest sha256:{cited} \
                 but the parent's actual contract digest is sha256:{actual}. \
                 The parent changed after this child was written — the child's \
                 basis is no longer the one it was authorized against."
            ),
        )),
        None => report.push(Diagnostic::unknown(
            "relations.parent-digest",
            manifest_file.to_owned(),
            format!(
                "{alias}: parent {ref}'s contract does not compile, so the citation \
                 cannot be compared with it"
            ),
        )),
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
    // §76.2, OW-WAR-0112: the same remedies, once each, with a count — the
    // list a reader works down. A red corpus of forty findings is usually
    // three commands.
    let mut remedies: BTreeMap<(String, &'static str), usize> = BTreeMap::new();
    for diagnostic in &report.diagnostics {
        if let Some(r) = crate::remedy::remedy_for(diagnostic) {
            *remedies.entry((r.command(), r.kind.label())).or_default() += 1;
        }
    }
    if !remedies.is_empty() {
        println!("\nREMEDIES:");
        for ((command, kind), n) in &remedies {
            println!("  {kind:<5} {command}   (×{n})");
        }
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

/// A hand-maintained roadmap may not claim a Warrant is **resolved**: the
/// Release axis is §56.2 records, and OW-WAR-0032 was marked resolved in
/// prose while its record said otherwise (slice C6). Bold `resolved` is the
/// exact token the projection's caveat used to count.
fn check_roadmap_status_claims(repo: &Repository, report: &mut Report) {
    let dir = repo.root.join(&repo.config.paths.roadmap);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let claims = text.matches("**resolved**").count();
        if claims > 0 {
            let rel = camino::Utf8PathBuf::from_path_buf(path)
                .map(|p| repo.relative(&p))
                .unwrap_or_else(|p| p.display().to_string());
            report.push(Diagnostic::error(
                "roadmap.status-claim",
                rel.clone(),
                format!(
                    "{rel} marks something **resolved** {claims} time(s); resolution is a §56.2 \
                     record, and CORPUS_STATUS.md is compiled from those — delete the claim"
                ),
            ));
        }
    }
}

/// The headings of a preset atom marked `<!-- required -->` whose body —
/// everything up to the next heading, HTML comments removed — is blank.
/// Headings inside a code fence are not headings.
pub(crate) fn unanswered_required_headings(text: &str) -> Vec<String> {
    let body = crate::compile::atom_body(text);
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence && line.starts_with('#') {
            sections.push((
                line.trim_start_matches('#').trim().to_owned(),
                String::new(),
            ));
        } else if let Some((_, b)) = sections.last_mut() {
            b.push_str(line);
            b.push('\n');
        }
    }
    sections
        .into_iter()
        .filter(|(_, b)| b.lines().any(|l| l.trim() == "<!-- required -->"))
        .filter(|(_, b)| strip_comments(b).trim().is_empty())
        .map(|(h, _)| h)
        .collect()
}

fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod preset_tests {
    use super::unanswered_required_headings;

    #[test]
    fn a_required_heading_left_as_its_comment_is_unanswered() {
        let t = "# Intent\n\n## Problem\n<!-- required -->\n<!-- What is wrong? -->\n\n## Non-goals\n<!-- optional: delete if not applicable -->\n<!-- What is out? -->\n";
        assert_eq!(unanswered_required_headings(t), vec!["Problem".to_owned()]);
    }

    #[test]
    fn an_answer_or_a_deleted_optional_heading_passes() {
        let t = "# Intent\n\n## Problem\n<!-- required -->\n<!-- What is wrong? -->\nThe parser drops a key.\n";
        assert!(unanswered_required_headings(t).is_empty());
    }
}
