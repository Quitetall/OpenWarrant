// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war status` — build the corpus projection from records (SAS §17.5 `status`,
//! corpus form; §34.3; §98; §102 decision 8).
//!
//! # What this reads, and what it refuses to read
//!
//! Every input is a record: manifests, atoms, the sidecar records `war resolve`
//! reads (deliverables, verifications, rationale, authorization, judgments),
//! and SAS §106 for the requirement titles. It computes each Warrant through
//! [`crate::resolve::assess`] — the same function `war resolve` uses — so the
//! two cannot disagree.
//!
//! It does NOT read `docs/roadmap/PRODUCTION_ROADMAP.md`'s status column. That
//! file says "resolved" forty-eight times; no resolution record exists on
//! disk, and `war resolve` refuses to write one. A hand-written claim is not a
//! record, and the projection says so in its caveats rather than choosing
//! between the two.
//!
//! # Requirement status is derived, and `satisfied` needs a resolution
//!
//! `traceability::derive_all` has modelled §34.3 since Phase 1 with no caller.
//! It is called here, and `Implements.warrant_resolved` is `false` for every
//! link because no §56.2 record exists. When one does, this is the one place
//! that reads it. "Would satisfy" is a separate, forward-looking count and is
//! never fed into `warrant_resolved` — doing so would make §34.3's `satisfied`
//! mean "an agent thinks so", which is the ticked box the section forbids.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::Utf8PathBuf;
use openwarrant_core::status::{
    Achieved, CORPUS_STATUS_SCHEMA, CorpusStatus, ImplementsClaim, MilestoneState,
    NothingActionable, ObjectiveStatus, Reached, ReleaseSummary, RequirementCounts,
    RequirementLadder, ResolutionSummary, StageRef, Validity, WarrantLadder, WarrantRung,
    WarrantStatus,
};
use openwarrant_core::traceability::{
    Contribution, Implements as Link, RequirementRef, RoadmapRef, derive_all,
};
use openwarrant_core::{Provenance, WarrantState, milestones};

use crate::diagnostic::Severity;
use crate::repo::{Loaded, RepoError, Repository};
use crate::resolve::assess;

/// Build the projection.
pub fn build(repo: &Repository) -> Result<CorpusStatus, RepoError> {
    let mut loaded: Vec<Loaded> = Vec::new();
    for dir in repo.warrant_dirs()? {
        loaded.push(repo.load_warrant(&dir)?);
    }
    loaded.sort_by_key(Loaded::alias);

    // Per-Warrant.
    let mut warrants: Vec<WarrantStatus> = Vec::new();
    let mut links: Vec<Link> = Vec::new();
    let mut rung_by_alias: BTreeMap<String, WarrantRung> = BTreeMap::new();
    let mut satisfied_by_alias: std::collections::BTreeSet<String> = Default::default();
    let mut milestones_by_alias: BTreeMap<String, Vec<MilestoneState>> = BTreeMap::new();

    for one in &loaded {
        let alias = one.alias();
        let valid = one.validated.is_some() && one.basis.is_some();
        let validity = if valid {
            Validity::Valid
        } else {
            Validity::Invalid {
                reason: one
                    .report
                    .diagnostics
                    .iter()
                    .find(|d| d.severity == Severity::Error)
                    .map(|d| d.message.clone())
                    .unwrap_or_else(|| "manifest did not validate".to_owned()),
            }
        };

        let (roadmap, implements, title) = match &one.basis {
            Some(b) => (
                b.manifest
                    .roadmap
                    .iter()
                    .filter_map(|r| RoadmapRef::parse(&r.r#ref).ok())
                    .collect::<Vec<_>>(),
                b.manifest
                    .implements
                    .iter()
                    .filter_map(|i| {
                        let requirement = RequirementRef::parse(&i.r#ref).ok()?;
                        let contribution =
                            i.contribution.as_deref()?.parse::<Contribution>().ok()?;
                        Some(ImplementsClaim {
                            requirement,
                            contribution,
                        })
                    })
                    .collect::<Vec<_>>(),
                Some(b.manifest.title.clone()),
            ),
            None => (vec![], vec![], None),
        };

        let (checks, would, unestablished, blocking, evidence_refs, established, current_digest) =
            if valid {
                // Requirement 5 reads the Warrant's own committed `gate-runs/`
                // (OW-WAR-0059), so this projection and `war resolve` answer
                // from the same tracked inputs and a fresh clone reproduces it.
                let a = assess(repo, one)?;
                (
                    Some(a.checks),
                    a.would_resolve_satisfied,
                    a.unestablished,
                    {
                        let mut b = a.blocking_unknowns;
                        b.sort();
                        b
                    },
                    a.evidence_refs,
                    a.established,
                    a.current_contract_digest,
                )
            } else {
                (None, None, vec![], vec![], vec![], vec![], None)
            };

        // §56.2 — read from the record, and counted only when the record binds
        // the contract as it compiles now. A resolution of an earlier revision
        // is reported (so a reader sees it) and derives nothing.
        let record = repo.load_resolution(&one.dir).ok().flatten();
        let resolution = record.as_ref().map(|r| ResolutionSummary {
            common_outcome: r.resolution.common_outcome.to_string(),
            profile_outcome: r.resolution.profile_outcome.clone(),
            resolved_by_ref: r.resolution.resolved_by_ref.clone(),
            effective_at: r.resolution.effective_at.clone(),
            binds_current_contract: Some(r.resolution.contract_digest.as_str())
                == current_digest.as_deref(),
        });
        let resolved = resolution
            .as_ref()
            .is_some_and(|r| r.binds_current_contract);
        // §34.3 `satisfied`: a RESOLVED Warrant with evidence. A Warrant resolved
        // `not_satisfied` or `cancelled` is resolved and satisfies nothing.
        let resolved_satisfied = resolved
            && record.as_ref().is_some_and(|r| {
                r.resolution.common_outcome
                    == openwarrant_core::resolution::CommonOutcome::Satisfied
            });
        let rung = WarrantRung::derive(valid, checks.as_ref(), would, resolved);
        rung_by_alias.insert(alias.clone(), rung);
        if resolved_satisfied {
            satisfied_by_alias.insert(alias.clone());
        }

        let ms = one.basis.as_ref().and_then(|b| {
            b.atoms
                .iter()
                .filter(|a| a.role == "milestones")
                .find_map(|a| {
                    std::str::from_utf8(&a.bytes)
                        .ok()
                        .and_then(|t| milestones::parse(t).ok())
                })
                .map(|g| milestone_states(&g, &established))
        });
        if let Some(m) = &ms {
            milestones_by_alias.insert(alias.clone(), m.clone());
        }

        for c in &implements {
            links.push(Link {
                warrant: alias.clone(),
                requirement_ref: c.requirement.clone(),
                intended_contribution: c.contribution,
                warrant_resolved: resolved_satisfied,
                evidence_refs: evidence_refs.clone(),
            });
        }

        warrants.push(WarrantStatus {
            alias: alias.clone(),
            resolution,
            title,
            validity,
            rung,
            roadmap,
            implements,
            state: valid.then(|| {
                if resolved {
                    WarrantState::resolved_recorded(
                        record
                            .as_ref()
                            .and_then(|r| r.resolution.common_outcome.to_string().parse().ok())
                            .unwrap_or(openwarrant_core::CommonOutcome::None),
                    )
                } else {
                    WarrantState::draft(Provenance::Derived)
                }
            }),
            unmet: checks
                .as_ref()
                .map(|c| c.unmet().into_iter().map(str::to_owned).collect())
                .unwrap_or_default(),
            checks,
            would_resolve_satisfied: would,
            unestablished,
            blocking_unknowns: blocking,
            milestones: ms,
        });
    }

    // Objectives — every §98 phase, then `unassigned` last.
    let prefix = warrants
        .iter()
        .flat_map(|w| w.roadmap.iter().map(|r| r.prefix.clone()))
        .fold(BTreeMap::<String, usize>::new(), |mut m, p| {
            *m.entry(p).or_insert(0) += 1;
            m
        })
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(p, _)| p)
        .unwrap_or_else(|| "OW".to_owned());

    let mut objectives: Vec<ObjectiveStatus> = Vec::new();
    for (n, title, exit) in openwarrant_core::status::PHASES {
        let members: Vec<&WarrantStatus> = warrants
            .iter()
            .filter(|w| {
                w.roadmap
                    .first()
                    .is_some_and(|r| r.phase == n && r.prefix == prefix)
            })
            .collect();
        let exit_warrant = members
            .iter()
            .find(|w| w.roadmap.first().is_some_and(RoadmapRef::is_exit))
            .map(|w| w.alias.clone());
        let mut ladder = WarrantLadder::default();
        for m in &members {
            ladder.count(m.rung);
        }
        let achieved = if exit.is_none() {
            Achieved::NotDerivable {
                why: "§98 defines no Exit for this phase".to_owned(),
            }
        } else if members.is_empty() {
            Achieved::NotDerivable {
                why: "no Warrant names this phase".to_owned(),
            }
        } else if let Some(e) = &exit_warrant {
            match rung_by_alias.get(e) {
                Some(WarrantRung::Resolved) if satisfied_by_alias.contains(e) => Achieved::Recorded,
                Some(WarrantRung::Resolved) => Achieved::Blocked {
                    by: vec![format!("{e} (resolved, not satisfied)")],
                },
                Some(r) if *r >= WarrantRung::WouldSatisfy => Achieved::ExitWarrantWouldSatisfy,
                _ => Achieved::Blocked {
                    by: members
                        .iter()
                        .filter(|w| w.rung < WarrantRung::WouldSatisfy)
                        .map(|w| w.alias.clone())
                        .collect(),
                },
            }
        } else {
            Achieved::NotDerivable {
                why: "no member carries the `exit` slug".to_owned(),
            }
        };
        objectives.push(ObjectiveStatus {
            roadmap_ref: Some(RoadmapRef {
                prefix: prefix.clone(),
                phase: n,
                slug: None,
            }),
            title: title.to_owned(),
            exit_criterion: exit.map(str::to_owned),
            exit_warrant,
            warrants: members.iter().map(|w| w.alias.clone()).collect(),
            ladder,
            achieved,
        });
    }
    {
        let members: Vec<&WarrantStatus> =
            warrants.iter().filter(|w| w.roadmap.is_empty()).collect();
        let mut ladder = WarrantLadder::default();
        for m in &members {
            ladder.count(m.rung);
        }
        objectives.push(ObjectiveStatus {
            roadmap_ref: None,
            title: "unassigned — declares no [[roadmap]]".to_owned(),
            exit_criterion: None,
            exit_warrant: None,
            warrants: members.iter().map(|w| w.alias.clone()).collect(),
            ladder,
            achieved: Achieved::NotDerivable {
                why: "a Warrant naming no phase belongs to no Objective".to_owned(),
            },
        });
    }

    // Requirements — seeded from §106 so `unaddressed` is listed by id.
    let titles = section_106(repo);
    let mut by_req: BTreeMap<RequirementRef, Vec<Link>> = BTreeMap::new();
    for r in titles.keys() {
        by_req.entry(r.clone()).or_default();
    }
    for l in &links {
        by_req
            .entry(l.requirement_ref.clone())
            .or_default()
            .push(l.clone());
    }
    let statuses = derive_all(&links);
    let requirements: Vec<RequirementLadder> = by_req
        .into_iter()
        .map(|(requirement, mut ls)| {
            ls.sort_by(|a, b| a.warrant.cmp(&b.warrant));
            let status = statuses
                .get(&requirement.canonical())
                .copied()
                .unwrap_or(openwarrant_core::RequirementStatus::Unaddressed);
            let would_satisfy = ls
                .iter()
                .filter(|l| {
                    l.intended_contribution.can_satisfy_alone()
                        && rung_by_alias
                            .get(&l.warrant)
                            .is_some_and(|r| *r >= WarrantRung::WouldSatisfy)
                })
                .count();
            RequirementLadder {
                title: titles.get(&requirement).cloned(),
                requirement,
                status,
                would_satisfy,
                links: ls,
            }
        })
        .collect();
    let mut counts = RequirementCounts::default();
    for r in &requirements {
        counts.count(r.status);
    }

    // Next actionable.
    let (next_actionable, nothing_actionable) =
        next_actionable(&objectives, &warrants, &milestones_by_alias);

    // Caveats a reader needs before the numbers.
    let mut caveats = vec![
        "Gate runs are read ONLY from each Warrant's committed `gate-runs/` (§44.6 receipts \
         minted by `war evidence record`, bound to the contract digest they ran against). The \
         gitignored `docs/receipts/` scratch path is never read, so this projection reproduces \
         from a fresh clone. §56.1 requirement 5 reads unmet for every Warrant that has not \
         recorded a run, which is a true state, not a caveat."
            .to_owned(),
    ];
    let roadmap_claims = hand_written_resolved_claims(repo);
    let recorded = warrants.iter().filter(|w| w.resolution.is_some()).count();
    if roadmap_claims > 0 {
        caveats.push(format!(
            "`docs/roadmap/PRODUCTION_ROADMAP.md` marks Warrants \"resolved\" {roadmap_claims} time(s). \
             The Release axis above counts §56.2 records (`resolution.toml`), of which there \
             are {recorded}. The rest of those are hand-written claims, not records, and \
             nothing above reads them."
        ));
    }
    let prefixes: BTreeSet<&str> = warrants
        .iter()
        .flat_map(|w| w.roadmap.iter().map(|r| r.prefix.as_str()))
        .collect();
    if prefixes.len() > 1 {
        caveats.push(format!(
            "Roadmap refs carry {} different prefixes ({}). Objectives are grouped under the \
             most common one, {prefix}; Warrants naming another prefix's phases are not \
             grouped and fall to `unassigned`. A per-prefix Objective set is not built yet.",
            prefixes.len(),
            prefixes.iter().copied().collect::<Vec<_>>().join(", ")
        ));
    }
    let invalid = warrants
        .iter()
        .filter(|w| w.rung == WarrantRung::Invalid)
        .count();
    if invalid > 0 {
        caveats.push(format!(
            "{invalid} Warrant(s) did not validate. They are listed, not omitted — a denominator \
             that quietly shrinks is the cheap way to look further along."
        ));
    }
    if titles.is_empty() {
        caveats.push(
            "SAS §106 could not be read, so requirement titles are absent and only requirements \
             named by a manifest appear; unaddressed ones cannot be listed."
                .to_owned(),
        );
    }

    Ok(CorpusStatus {
        schema: CORPUS_STATUS_SCHEMA.to_owned(),
        provenance: Provenance::Derived,
        provenance_note: {
            let recorded = warrants.iter().filter(|w| w.resolution.is_some()).count();
            if recorded == 0 {
                "Every state on this page is derived from the records' shape. Nothing journals §24 \
                 transitions and no §56.2 resolution has been recorded, so no Warrant can read above \
                 `would_satisfy` and no requirement above `in_progress` until a human signs."
                    .to_owned()
            } else {
                format!(
                    "Every state on this page is derived from the records' shape. Nothing journals §24 \
                     transitions. {recorded} Warrant(s) carry a §56.2 resolution record and read \
                     `resolved` from it; every other Warrant reads at most `would_satisfy`, and a \
                     requirement reads `satisfied` only through a resolved Warrant with evidence."
                )
            }
        },
        release: match repo.latest_sas_revision()? {
            Some(r) => ReleaseSummary {
                version: Some(r.version.clone()),
                digest: Some(format!("sha256:{}", r.sha256)),
                requirements: counts,
                note: match r.state {
                    openwarrant_core::SasRevisionState::Accepted => format!(
                        "Revision {} is ACCEPTED (§101.2) and normative; every compiled Warrant is \
                         pinned to it.",
                        r.version
                    ),
                    openwarrant_core::SasRevisionState::Proposed => format!(
                        "Revision {} is PROPOSED, not accepted. Every compiled Warrant is pinned to \
                         its digest; §101.2's acceptance is a human's, and has not happened.",
                        r.version
                    ),
                },
            },
            None => ReleaseSummary {
                version: None,
                digest: None,
                requirements: counts,
                note: "No SAS revision is recorded (`war sas propose`). The requirement ladder is \
                       against §106 as read from the document on disk."
                    .to_owned(),
            },
        },
        objectives,
        warrants,
        requirements,
        next_actionable,
        nothing_actionable,
        caveats,
    })
}

/// §23.1 milestones, with whether each is evidenced by verifications.
fn milestone_states(
    graph: &milestones::MilestoneGraph,
    established: &[String],
) -> Vec<MilestoneState> {
    let established: BTreeSet<&str> = established.iter().map(String::as_str).collect();
    // Evidenced is a property of the milestone alone; blocked/unblocked of its
    // dependencies. Two passes so the second can see the first's answers.
    let evidenced: BTreeSet<&str> = graph
        .milestones
        .iter()
        .filter(|m| {
            !m.obligation_refs.is_empty()
                && m.obligation_refs
                    .iter()
                    .all(|o| established.contains(o.as_str()))
        })
        .map(|m| m.id.as_str())
        .collect();
    graph
        .milestones
        .iter()
        .map(|m| {
            let reached = if m.obligation_refs.is_empty() {
                Reached::Declared
            } else if evidenced.contains(m.id.as_str()) {
                Reached::Evidenced {
                    obligations: m.obligation_refs.clone(),
                }
            } else {
                let blocked: Vec<String> = m
                    .depends_on
                    .iter()
                    .filter(|d| !evidenced.contains(d.as_str()))
                    .cloned()
                    .collect();
                if blocked.is_empty() {
                    Reached::Unblocked
                } else {
                    Reached::BlockedOn {
                        milestones: blocked,
                    }
                }
            };
            MilestoneState {
                id: m.id.clone(),
                title: m.title.clone(),
                depends_on: m.depends_on.clone(),
                stages: m.stage_refs.clone(),
                obligations: m.obligation_refs.clone(),
                reached,
            }
        })
        .collect()
}

/// The stages an agent could pick up now, or why there are none.
///
/// Objectives ascending; the first one with an unblocked stage wins, so an
/// agent is pointed at the lowest unachieved phase. Never empty-and-silent:
/// when nothing is unblocked, the lowest phase's blockers are named instead.
fn next_actionable(
    objectives: &[ObjectiveStatus],
    warrants: &[WarrantStatus],
    milestones: &BTreeMap<String, Vec<MilestoneState>>,
) -> (Vec<StageRef>, Option<NothingActionable>) {
    let by_alias: BTreeMap<&str, &WarrantStatus> =
        warrants.iter().map(|w| (w.alias.as_str(), w)).collect();
    let mut lowest_blocked: Option<&ObjectiveStatus> = None;

    for o in objectives {
        let Some(oref) = &o.roadmap_ref else {
            continue; // `unassigned` is never actionable.
        };
        if matches!(
            o.achieved,
            Achieved::Recorded | Achieved::ExitWarrantWouldSatisfy
        ) {
            continue;
        }
        if o.warrants.is_empty() {
            continue;
        }
        lowest_blocked.get_or_insert(o);
        let mut out = Vec::new();
        for alias in &o.warrants {
            let Some(w) = by_alias.get(alias.as_str()) else {
                continue;
            };
            if w.rung >= WarrantRung::WouldSatisfy || w.rung == WarrantRung::Invalid {
                continue;
            }
            let Some(ms) = milestones.get(alias) else {
                continue;
            };
            for m in ms {
                let ready = match &m.reached {
                    Reached::Unblocked => true,
                    Reached::Declared => m.depends_on.is_empty(),
                    _ => false,
                };
                if !ready {
                    continue;
                }
                for s in &m.stages {
                    out.push(StageRef {
                        warrant: alias.clone(),
                        milestone: m.id.clone(),
                        stage: s.clone(),
                        objective: oref.clone(),
                        why: format!(
                            "{} is {}; {alias} is {}; {} is the lowest unachieved Objective",
                            m.id,
                            match &m.reached {
                                Reached::Unblocked => "unblocked",
                                _ => "declared with no dependencies",
                            },
                            match w.rung {
                                WarrantRung::Draft => "draft",
                                WarrantRung::ReadyToResolve => "ready to resolve",
                                _ => "unresolved",
                            },
                            oref.canonical()
                        ),
                    });
                }
            }
        }
        if !out.is_empty() {
            return (out, None);
        }
    }

    // Never `(empty, None)`. An agent reading the JSON must always find either
    // a stage or a reason, and "every Objective is achieved or names no
    // Warrant" is a reason — a legitimate one, not a defect — that the first
    // version of this function returned as silence. External review caught it:
    // the Markdown renderer flagged the case, the JSON did not.
    let nothing = match lowest_blocked {
        Some(o) => NothingActionable {
            objective: o.roadmap_ref.clone(),
            blocked_by: o
                .warrants
                .iter()
                .filter(|a| {
                    by_alias
                        .get(a.as_str())
                        .is_some_and(|w| w.rung < WarrantRung::WouldSatisfy)
                })
                .cloned()
                .collect(),
            why: "no milestone in the lowest unachieved Objective is unblocked; every one is \
                  waiting on a dependency that is not yet evidenced"
                .to_owned(),
        },
        None => NothingActionable {
            objective: None,
            blocked_by: vec![],
            why: "every Objective is either achieved or names no Warrant; there is no \
                  unachieved work to point at"
                .to_owned(),
        },
    };
    (vec![], Some(nothing))
}

/// SAS §106's requirement index: id → title. Empty if the SAS cannot be read.
fn section_106(repo: &Repository) -> BTreeMap<RequirementRef, String> {
    let dir = repo.root.join(&repo.config.paths.sas);
    let Ok(entries) = fs::read_dir(&dir) else {
        return BTreeMap::new();
    };
    let mut paths: Vec<Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.extension() == Some("md"))
        .collect();
    paths.sort();
    let mut out = BTreeMap::new();
    for p in paths {
        let Ok(text) = fs::read_to_string(&p) else {
            continue;
        };
        for line in text.lines() {
            // `| WAR-SAS-RQ-001 | Every WAR has immutable UUIDv7 identity |`
            let Some(rest) = line.strip_prefix("| ") else {
                continue;
            };
            let mut cells = rest.split(" | ");
            let (Some(id), Some(title)) = (cells.next(), cells.next()) else {
                continue;
            };
            if let Ok(r) = RequirementRef::parse(id) {
                out.insert(r, title.trim_end_matches(" |").trim().to_owned());
            }
        }
    }
    out
}

/// How many times the hand-maintained roadmap says "resolved".
fn hand_written_resolved_claims(repo: &Repository) -> usize {
    let dir = repo.root.join(&repo.config.paths.roadmap);
    let Ok(entries) = fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .map(|t| t.matches("**resolved**").count())
        .sum()
}

/// The Markdown projection, with its path.
pub fn corpus_status_md(repo: &Repository) -> Result<(Utf8PathBuf, String), RepoError> {
    let status = build(repo)?;
    Ok((
        repo.corpus_status_md_path(),
        openwarrant_compiler::render_corpus_status(&status),
    ))
}

/// The canonical JSON projection, with its path.
pub fn corpus_status_json(repo: &Repository) -> Result<(Utf8PathBuf, String), RepoError> {
    let status = build(repo)?;
    let json = openwarrant_compiler::corpus_status_json(&status).map_err(|e| {
        RepoError::Message(format!("could not canonicalize the corpus status: {e}"))
    })?;
    Ok((repo.corpus_status_json_path(), json + "\n"))
}

/// The page, with its path. Built from the same canonical JSON the agent
/// reads, so the three files cannot disagree.
pub fn corpus_status_html(repo: &Repository) -> Result<(Utf8PathBuf, String), RepoError> {
    let status = build(repo)?;
    let json = openwarrant_compiler::corpus_status_json(&status).map_err(|e| {
        RepoError::Message(format!("could not canonicalize the corpus status: {e}"))
    })?;
    Ok((
        repo.corpus_status_html_path(),
        openwarrant_compiler::render_corpus_status_html(&status, &json),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::status::{Achieved, ObjectiveStatus, WarrantLadder};

    /// The case external review found: every Objective achieved (or empty)
    /// must still produce a REASON in the JSON, never `(empty, None)`.
    #[test]
    fn nothing_actionable_always_carries_a_reason() {
        let all_done = vec![ObjectiveStatus {
            roadmap_ref: Some(RoadmapRef::parse("roadmap://OW-PHASE-1").expect("ok")),
            title: "t".to_owned(),
            exit_criterion: Some("e".to_owned()),
            exit_warrant: Some("OW-WAR-0001".to_owned()),
            warrants: vec!["OW-WAR-0001".to_owned()],
            ladder: WarrantLadder::default(),
            achieved: Achieved::Recorded,
        }];
        let (stages, nothing) = next_actionable(&all_done, &[], &BTreeMap::new());
        assert!(stages.is_empty());
        let nothing = nothing.expect("a reason, even when everything is achieved");
        assert!(nothing.why.contains("achieved"), "{}", nothing.why);

        let (stages, nothing) = next_actionable(&[], &[], &BTreeMap::new());
        assert!(stages.is_empty());
        assert!(nothing.is_some(), "an empty corpus still says why");
    }
}
