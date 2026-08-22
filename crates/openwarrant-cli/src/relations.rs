// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parent/child and supersession conformance — SAS §20, §21.
//!
//! OW-WAR-0043 OBL-004 asks for §91.4 test 24 and §91.5 tests 30–35 to be
//! planted. Four of those claims already had rules in `openwarrant_core`
//! (`lifecycle::needs_adr`, `Currency::remains_available`,
//! `Supersession::validate`) and none of them was reachable from `war check`,
//! so nothing could be planted against them — a rule the binary never consults
//! is exactly the "declared but never executed" state this fleet keeps finding.
//! This module is the wiring, plus the three §20 rules that did not exist.
//!
//! | test | claim | rule |
//! |---|---|---|
//! | 30 | parent source unchanged when child state changes | `relations.parent-source` |
//! | 31 | parent generated view lists child | `relations.child-listed` |
//! | 32 | child cannot silently replace parent rationale | `relations.parent-source` |
//! | 33 | superseding WAR makes old currency `superseded` | `relations.currency` |
//! | 34 | superseded WAR remains exportable | `relations.retired-available` |
//! | 35 | adopted unresolved children are explicit | `relations.adoption` |
//!
//! Test 24 — "a local choice inside autonomy does not require a new ADR" — is a
//! POSITIVE claim and is checked in `crate::check` where amendments are read,
//! because it is a statement about what must be ACCEPTED rather than rejected.

use std::collections::BTreeMap;

use openwarrant_core::ValidatedManifest;
use openwarrant_core::lifecycle::Currency;

use crate::diagnostic::{Diagnostic, Report};

/// One Warrant as this module needs to see it.
pub struct Related<'a> {
    pub alias: String,
    pub manifest: &'a ValidatedManifest,
    pub manifest_file: String,
    /// The committed generated view, if one is on disk.
    pub generated_view: Option<String>,
    /// The concatenated bytes of every authored atom.
    pub atom_source: String,
}

/// §20.4 — the parent's generated view SHALL list child WARs and their states.
///
/// A stale view is the realistic failure: a child is added, nobody recompiles,
/// and the parent's Relations section silently describes a smaller family than
/// exists. `generated.drift` does not name that — it reports that SOMETHING
/// differs from a fresh compile, which is true of any edit. This names the
/// missing child, which is what a reader needs.
fn child_listed(corpus: &[Related<'_>], report: &mut Report) {
    let by_uuid: BTreeMap<&str, &Related<'_>> = corpus
        .iter()
        .map(|r| (r.manifest.raw.uuid.as_str(), r))
        .collect();

    let mut children_of: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for child in corpus {
        for parent in &child.manifest.raw.parents {
            let uuid = parent.r#ref.trim_start_matches("war://");
            children_of.entry(uuid).or_default().push(&child.alias);
        }
    }

    for (parent_uuid, children) in &children_of {
        let Some(parent) = by_uuid.get(parent_uuid) else {
            // An external parent is not this repository's to render.
            continue;
        };
        let Some(view) = &parent.generated_view else {
            report.push(Diagnostic::unknown(
                "relations.child-listed",
                parent.manifest_file.clone(),
                format!(
                    "{}: has {} child WAR(s) but no generated view is committed, so §20.4's \
                     child list cannot be checked. Run `war compile`.",
                    parent.alias,
                    children.len()
                ),
            ));
            continue;
        };
        // Look ONLY inside the generated `### Children` section.
        //
        // The first version of this rule searched the whole view, and passed for
        // three of OW-WAR-0001's four children — because its atoms happen to say
        // "Those are OW-WAR-0002 through OW-WAR-0004" in narrative prose. A rule
        // satisfied by a Warrant being MENTIONED is fail-open: it reports §20.4
        // as met by a document that never projected a child list at all.
        let section = view
            .split("\n### ")
            .find(|s| s.starts_with("Children\n"))
            .unwrap_or("");
        let missing: Vec<&str> = children
            .iter()
            .filter(|child| !section.contains(**child))
            .copied()
            .collect();
        if missing.is_empty() {
            report.push(Diagnostic::pass(
                "relations.child-listed",
                format!(
                    "{}: generated view lists all {} child WAR(s)",
                    parent.alias,
                    children.len()
                ),
            ));
        } else {
            report.push(Diagnostic::error(
                "relations.child-listed",
                parent.manifest_file.clone(),
                format!(
                    "{}: child WAR(s) {} are not listed in the parent's generated view. \
                     §20.4: the parent's generated Relations section SHALL list child WARs \
                     and their current states. A child that exists but is not projected \
                     leaves the parent describing a smaller family than it has.",
                    parent.alias,
                    missing.join(", ")
                ),
            ));
        }
    }
}

/// §20.3 and §20.4 — a child's state belongs in the parent's GENERATED view,
/// never in the parent's authored source.
///
/// §20.4 is explicit that the child list "is a bound/generated projection, not
/// an edit to the parent's original contract", and §20.3 forbids a child
/// outcome becoming the parent's supposed original rationale. Both fail the same
/// way in practice: someone writes a child's status into a parent atom, and the
/// parent's authored basis now moves whenever the child does.
fn parent_source(corpus: &[Related<'_>], report: &mut Report) {
    let mut children_of: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for child in corpus {
        for parent in &child.manifest.raw.parents {
            children_of
                .entry(parent.r#ref.trim_start_matches("war://"))
                .or_default()
                .push(&child.alias);
        }
    }
    // A child's ALIAS beside a state word in a parent's authored atom.
    const STATE_WORDS: [&str; 6] = [
        "resolved",
        "unresolved",
        "in progress",
        "superseded",
        "blocked",
        "complete",
    ];

    for parent in corpus {
        let Some(children) = children_of.get(parent.manifest.raw.uuid.as_str()) else {
            continue;
        };
        let mut offences = vec![];
        for line in parent.atom_source.lines() {
            let lower = line.to_lowercase();
            if !STATE_WORDS.iter().any(|w| lower.contains(w)) {
                continue;
            }
            for child in children {
                if line.contains(*child) {
                    offences.push(format!("{child} in {:?}", line.trim()));
                }
            }
        }
        if offences.is_empty() {
            report.push(Diagnostic::pass(
                "relations.parent-source",
                format!(
                    "{}: authored atoms carry no child state; §20.4's list stays generated",
                    parent.alias
                ),
            ));
        } else {
            report.push(Diagnostic::error(
                "relations.parent-source",
                parent.manifest_file.clone(),
                format!(
                    "{}: a child's state is written into the parent's AUTHORED source — {}. \
                     §20.4: the child list is a bound/generated projection, not an edit to \
                     the parent's original contract. §20.3: a child outcome SHALL NOT become \
                     the supposed original rationale of the parent.",
                    parent.alias,
                    offences.join("; ")
                ),
            ));
        }
    }
}

/// §21.2 — the replaced WAR's canonical currency becomes `superseded`.
fn currency(corpus: &[Related<'_>], report: &mut Report) {
    let by_uuid: BTreeMap<&str, &Related<'_>> = corpus
        .iter()
        .map(|r| (r.manifest.raw.uuid.as_str(), r))
        .collect();

    for replacement in corpus {
        for superseded in &replacement.manifest.raw.supersedes {
            let uuid = superseded.r#ref.trim_start_matches("war://");
            let Some(old) = by_uuid.get(uuid) else {
                report.push(Diagnostic::unknown(
                    "relations.currency",
                    replacement.manifest_file.clone(),
                    format!(
                        "{}: supersedes {} which is not in this repository, so §21.2's \
                         currency cannot be checked",
                        replacement.alias, superseded.r#ref
                    ),
                ));
                continue;
            };
            match old.manifest.raw.currency.as_deref() {
                Some("superseded") => report.push(Diagnostic::pass(
                    "relations.currency",
                    format!(
                        "{}: currency is superseded, as {} requires",
                        old.alias, replacement.alias
                    ),
                )),
                other => report.push(Diagnostic::error(
                    "relations.currency",
                    old.manifest_file.clone(),
                    format!(
                        "{}: is superseded by {} but its currency is {}. §21.2: the replaced \
                         WAR's canonical currency becomes `superseded`. Until it does, the \
                         retired Warrant still reads as available for new execution.",
                        old.alias,
                        replacement.alias,
                        other.map_or("absent".to_owned(), |c| format!("{c:?}"))
                    ),
                )),
            }
        }
    }
}

/// §21.4 — superseded and deprecated WARs SHALL remain available.
///
/// "Available" is checked as: the manifest still loads and its atoms are still
/// on disk. A retired Warrant whose atoms were deleted is the deletion §21.4
/// forbids, whatever the manifest still says.
fn retired_available(corpus: &[Related<'_>], report: &mut Report) {
    for warrant in corpus {
        let Some(raw) = warrant.manifest.raw.currency.as_deref() else {
            continue;
        };
        let Ok(currency) = raw.parse::<Currency>() else {
            report.push(Diagnostic::error(
                "relations.retired-available",
                warrant.manifest_file.clone(),
                format!("{}: currency {raw:?} is not a §21 currency", warrant.alias),
            ));
            continue;
        };
        if !currency.retired_for_new_execution() {
            continue;
        }
        if warrant.atom_source.trim().is_empty() {
            report.push(Diagnostic::error(
                "relations.retired-available",
                warrant.manifest_file.clone(),
                format!(
                    "{}: currency is {currency} but its authored atoms are empty or missing. \
                     §21.4: superseded and deprecated WARs SHALL remain available for audit \
                     and relation traversal — retiring a Warrant is not deleting it.",
                    warrant.alias
                ),
            ));
        } else {
            report.push(Diagnostic::pass(
                "relations.retired-available",
                format!(
                    "{}: currency {currency} and its source remains available (§21.4)",
                    warrant.alias
                ),
            ));
        }
    }
}

/// §21.5 — a superseding WAR SHALL explicitly identify what it adopts.
///
/// The check is deliberately about EXPLICITNESS, not about the adoption being
/// correct. "Nothing is silently carried forward" is a statement about the
/// record saying what happened, and an empty `adopts` beside a superseded
/// Warrant that had children is silence.
fn adoption(corpus: &[Related<'_>], report: &mut Report) {
    let mut children_of: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for child in corpus {
        for parent in &child.manifest.raw.parents {
            children_of
                .entry(parent.r#ref.trim_start_matches("war://"))
                .or_default()
                .push(&child.alias);
        }
    }

    for replacement in corpus {
        for superseded in &replacement.manifest.raw.supersedes {
            let uuid = superseded.r#ref.trim_start_matches("war://");
            let inherited = children_of.get(uuid).map_or(0, Vec::len);
            if inherited == 0 {
                continue;
            }
            if superseded.adopts.is_empty() {
                report.push(Diagnostic::error(
                    "relations.adoption",
                    replacement.manifest_file.clone(),
                    format!(
                        "{}: supersedes {} which has {inherited} child WAR(s), and adopts \
                         nothing explicitly. §21.5: a superseding WAR SHALL explicitly \
                         identify which unresolved child WARs, deliverables, evidence or \
                         obligations it adopts — nothing is silently carried forward. \
                         Adopting none is a legitimate answer, but it has to be stated.",
                        replacement.alias, superseded.r#ref
                    ),
                ));
            } else {
                report.push(Diagnostic::pass(
                    "relations.adoption",
                    format!(
                        "{}: names {} adoption(s) from {} (§21.5)",
                        replacement.alias,
                        superseded.adopts.len(),
                        superseded.r#ref
                    ),
                ));
            }
        }
    }
}

/// Run every §20/§21 relation rule.
pub fn check(corpus: &[Related<'_>], report: &mut Report) {
    child_listed(corpus, report);
    parent_source(corpus, report);
    currency(corpus, report);
    retired_available(corpus, report);
    adoption(corpus, report);
}
