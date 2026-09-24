// SPDX-License-Identifier: Apache-2.0
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
//! | 33 | superseding WAR makes old currency `superseded` | `relations.currency`, derived (OW-ADR-0022) |
//! | 34 | superseded WAR remains exportable | `relations.retired-available` |
//! | 35 | adopted unresolved children are explicit | `relations.adoption` |
//!
//! Test 24 — "a local choice inside autonomy does not require a new ADR" — is a
//! POSITIVE claim and is checked in `crate::check` where amendments are read,
//! because it is a statement about what must be ACCEPTED rather than rejected.

use std::collections::BTreeMap;

use openwarrant_core::ValidatedManifest;

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

/// A Warrant's currency, DERIVED from the relations around it (OW-ADR-0022).
///
/// §21.2 says the replaced Warrant's currency *becomes* superseded and that
/// the replaced Warrant stays immutable. Both at once are only possible if
/// currency is a fact about the SUCCESSOR's relation, computed here, and never
/// a field edited into the predecessor: OW-WAR-0073 was marked
/// `currency = "superseded"` and its signed contract digest moved, because
/// the manifest's bytes are in the preimage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "currency", rename_all = "snake_case")]
pub enum Derived {
    Current,
    /// An authorized Warrant declares `supersedes` → this one.
    Superseded {
        by: String,
    },
    /// The resolution standing says annulled.
    Annulled,
    /// Only a legacy bare `currency = "deprecated"`, tolerated and reported.
    /// A successor's `deprecates` relation would be the relation form; the
    /// manifest schema has no such field and is frozen under OW-WAR-0113.
    Deprecated,
}

impl Derived {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Superseded { .. } => "superseded",
            Self::Annulled => "annulled",
            Self::Deprecated => "deprecated",
        }
    }

    #[must_use]
    pub const fn is_current(&self) -> bool {
        matches!(self, Self::Current)
    }

    /// §21.4's retired states: still available, not for new execution.
    #[must_use]
    pub const fn retired(&self) -> bool {
        !self.is_current()
    }
}

impl std::fmt::Display for Derived {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One Warrant as the derivation needs it: identity, its outgoing
/// `supersedes`, whether its contract is authorized, whether its resolution
/// is annulled, and the legacy field it may still carry.
#[derive(Debug, Clone)]
pub struct CurrencyInput {
    pub alias: String,
    pub uuid: String,
    pub supersedes: Vec<String>,
    pub authorized: bool,
    pub annulled: bool,
    pub written: Option<String>,
}

/// The derivation over a whole corpus, and what it found on the way.
#[derive(Debug, Clone, Default)]
pub struct Currencies {
    by_alias: BTreeMap<String, Derived>,
    /// `(successor, predecessor)` declared by a successor not yet authorized:
    /// the relation is on record and not in force.
    pub pending: Vec<(String, String)>,
    /// `(successor, ref)` whose target is not in this corpus.
    pub external: Vec<(String, String)>,
    /// Each `supersedes` cycle once, as aliases starting from the least.
    pub cycles: Vec<Vec<String>>,
}

impl Currencies {
    /// The derived currency of `alias`; `Current` for an alias the corpus
    /// did not contain, which is the answer the relations give it.
    #[must_use]
    pub fn of(&self, alias: &str) -> Derived {
        self.by_alias
            .get(alias)
            .cloned()
            .unwrap_or(Derived::Current)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Derived)> {
        self.by_alias.iter().map(|(a, d)| (a.as_str(), d))
    }

    /// The pure derivation. Separated from I/O so it is testable on
    /// synthetic corpora and so every reader calls the one computation.
    #[must_use]
    pub fn derive(inputs: &[CurrencyInput]) -> Self {
        let alias_of: BTreeMap<&str, &str> = inputs
            .iter()
            .map(|i| (i.uuid.as_str(), i.alias.as_str()))
            .collect();
        let mut out = Self::default();
        let mut superseded_by: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for i in inputs {
            for r in &i.supersedes {
                let uuid = r.trim_start_matches("war://");
                let Some(old) = alias_of.get(uuid) else {
                    out.external.push((i.alias.clone(), r.clone()));
                    continue;
                };
                if i.authorized {
                    superseded_by.entry(old).or_default().push(&i.alias);
                } else {
                    out.pending.push((i.alias.clone(), (*old).to_owned()));
                }
            }
        }
        for i in inputs {
            let d = if i.annulled {
                Derived::Annulled
            } else if let Some(by) = superseded_by.get(i.alias.as_str()) {
                let mut by = by.clone();
                by.sort_unstable();
                Derived::Superseded { by: by.join(", ") }
            } else if i.written.as_deref() == Some("deprecated") {
                Derived::Deprecated
            } else {
                Derived::Current
            };
            out.by_alias.insert(i.alias.clone(), d);
        }
        out.cycles = cycles(inputs, &alias_of);
        out
    }
}

/// Every `supersedes` cycle, each reported once from its least alias. A
/// cycle has no current subject at either end, so no currency derived over
/// it means anything (R-001).
fn cycles(inputs: &[CurrencyInput], alias_of: &BTreeMap<&str, &str>) -> Vec<Vec<String>> {
    let edges: BTreeMap<&str, Vec<&str>> = inputs
        .iter()
        .map(|i| {
            let mut to: Vec<&str> = i
                .supersedes
                .iter()
                .filter_map(|r| alias_of.get(r.trim_start_matches("war://")).copied())
                .collect();
            to.sort_unstable();
            (i.alias.as_str(), to)
        })
        .collect();
    let mut found: std::collections::BTreeSet<Vec<String>> = std::collections::BTreeSet::new();
    // Small graphs, and supersession is sparse: a DFS from every node that
    // records the path is enough and keeps the order deterministic.
    fn walk<'a>(
        node: &'a str,
        edges: &BTreeMap<&'a str, Vec<&'a str>>,
        path: &mut Vec<&'a str>,
        found: &mut std::collections::BTreeSet<Vec<String>>,
    ) {
        if let Some(pos) = path.iter().position(|p| *p == node) {
            let cycle = &path[pos..];
            let least = cycle
                .iter()
                .enumerate()
                .min_by_key(|(_, a)| **a)
                .map_or(0, |(i, _)| i);
            let mut rotated: Vec<String> = cycle[least..]
                .iter()
                .chain(cycle[..least].iter())
                .map(|a| (*a).to_owned())
                .collect();
            rotated.push(rotated[0].clone());
            found.insert(rotated);
            return;
        }
        path.push(node);
        for next in edges.get(node).map(Vec::as_slice).unwrap_or(&[]) {
            walk(next, edges, path, found);
        }
        path.pop();
    }
    for start in edges.keys() {
        if edges.get(start).is_some_and(|e| !e.is_empty()) {
            walk(start, &edges, &mut Vec::new(), &mut found);
        }
    }
    found.into_iter().collect()
}

/// The derivation over loaded Warrants: reads each one's `authorization.toml`
/// and `resolution.toml` beside its manifest. The one place currency is
/// computed; `check`, `ownership`, `compile` and both projections read it.
#[must_use]
pub fn currencies(corpus: &[crate::repo::Loaded]) -> Currencies {
    let inputs: Vec<CurrencyInput> = corpus
        .iter()
        .filter_map(|one| {
            let v = one.validated.as_ref()?;
            let read = |name: &str| std::fs::read_to_string(one.dir.join(name)).ok();
            let authorized = !v.raw.supersedes.is_empty()
                && read("authorization.toml")
                    .and_then(|t| toml::from_str::<crate::authorize::AuthorizationRecord>(&t).ok())
                    .is_some_and(|r| {
                        r.revision.state == openwarrant_core::RevisionState::Authorized
                    });
            let annulled = read("resolution.toml")
                .and_then(|t| toml::from_str::<crate::resolution_cmd::ResolutionRecord>(&t).ok())
                .is_some_and(|r| {
                    r.resolution.standing == openwarrant_core::ResolutionStanding::Annulled
                });
            Some(CurrencyInput {
                alias: v.alias.to_string(),
                uuid: v.raw.uuid.clone(),
                supersedes: v.raw.supersedes.iter().map(|s| s.r#ref.clone()).collect(),
                authorized,
                annulled,
                written: v.raw.currency.clone(),
            })
        })
        .collect();
    Currencies::derive(&inputs)
}

/// §21.2 — the replaced WAR's canonical currency becomes `superseded`, by
/// relation. Three rules, one derivation:
///
/// - `relations.currency` — PASS on each supersession in force, naming the
///   successor; a declared one awaiting authorization is on record and not
///   yet in force.
/// - `relations.currency-authored` — a manifest that WRITES `superseded` or
///   `annulled` is refused: those are facts about a successor or a
///   resolution, and the predecessor's bytes are under a signature. A bare
///   `deprecated` is a legacy form, tolerated and reported.
/// - `relations.currency-cycle` — a `supersedes` cycle is refused.
fn currency(corpus: &[Related<'_>], currencies: &Currencies, report: &mut Report) {
    let file_of: BTreeMap<&str, &str> = corpus
        .iter()
        .map(|r| (r.alias.as_str(), r.manifest_file.as_str()))
        .collect();
    for (alias, d) in currencies.iter() {
        if let Derived::Superseded { by } = d {
            report.push(Diagnostic::pass(
                "relations.currency",
                format!(
                    "{alias}: superseded, derived from {by}'s authorized `supersedes`; \
                      nothing is written in {alias}'s manifest"
                ),
            ));
        }
    }
    for (successor, old) in &currencies.pending {
        report.push(Diagnostic::pass(
            "relations.currency",
            format!(
                "{successor}: declares `supersedes` → {old}, not in force until {successor} \
                  is authorized; {old} reads current"
            ),
        ));
    }
    for (successor, r) in &currencies.external {
        report.push(Diagnostic::unknown(
            "relations.currency",
            file_of
                .get(successor.as_str())
                .copied()
                .unwrap_or_default()
                .to_owned(),
            format!(
                "{successor}: supersedes {r} which is not in this repository, so its §21.2 \
                  currency cannot be derived here"
            ),
        ));
    }
    for w in corpus {
        let Some(written) = w.manifest.raw.currency.as_deref() else {
            continue;
        };
        match written {
            "superseded" | "annulled" => report.push(Diagnostic::error(
                "relations.currency-authored",
                w.manifest_file.clone(),
                format!(
                    "{}: the manifest writes `currency = {written:?}`. Currency is derived \
                      from relations (OW-ADR-0022): superseded when an authorized Warrant \
                      declares `supersedes` → this one, annulled from the resolution \
                      standing. Writing it into this manifest moves a signed contract \
                      digest (OW-WAR-0073). Remove the field; the successor's relation \
                      already says it",
                    w.alias
                ),
            )),
            "deprecated" | "current" => report.push(Diagnostic::warn(
                "relations.currency-authored",
                w.manifest_file.clone(),
                format!(
                    "{}: the manifest writes `currency = {written:?}` — a legacy field, \
                      tolerated. Currency is derived from relations (OW-ADR-0022); this \
                      reads {}",
                    w.alias,
                    currencies.of(&w.alias)
                ),
            )),
            other => report.push(Diagnostic::error(
                "relations.currency-authored",
                w.manifest_file.clone(),
                format!(
                    "{}: `currency = {other:?}` is not a §21 currency, and currency is not \
                      written at all: it is derived from relations (OW-ADR-0022)",
                    w.alias
                ),
            )),
        }
    }
    for cycle in &currencies.cycles {
        report.push(Diagnostic::error(
            "relations.currency-cycle",
            file_of
                .get(cycle[0].as_str())
                .copied()
                .unwrap_or_default()
                .to_owned(),
            format!(
                "`supersedes` cycle: {}. Each Warrant in it is replaced by another in it, \
                  so none is current and no currency derived over it means anything. Break \
                  the cycle: supersession runs one way",
                cycle.join(" → ")
            ),
        ));
    }
}

/// §21.4 — superseded and deprecated WARs SHALL remain available.
///
/// "Available" is checked as: the manifest still loads and its atoms are still
/// on disk. A retired Warrant whose atoms were deleted is the deletion §21.4
/// forbids, whatever the manifest still says. Retired is the DERIVED currency.
fn retired_available(corpus: &[Related<'_>], currencies: &Currencies, report: &mut Report) {
    for warrant in corpus {
        let currency = currencies.of(&warrant.alias);
        if !currency.retired() {
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
pub fn check(corpus: &[Related<'_>], currencies: &Currencies, report: &mut Report) {
    child_listed(corpus, report);
    parent_source(corpus, report);
    currency(corpus, currencies, report);
    retired_available(corpus, currencies, report);
    adoption(corpus, report);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(alias: &str, supersedes: &[&str], authorized: bool) -> CurrencyInput {
        CurrencyInput {
            alias: alias.into(),
            uuid: format!("u-{alias}"),
            supersedes: supersedes.iter().map(|a| format!("war://u-{a}")).collect(),
            authorized,
            annulled: false,
            written: None,
        }
    }

    #[test]
    fn an_authorized_successor_supersedes_and_a_draft_does_not() {
        let c = Currencies::derive(&[w("A", &[], false), w("B", &["A"], true)]);
        assert_eq!(c.of("A"), Derived::Superseded { by: "B".into() });
        assert_eq!(c.of("B"), Derived::Current);
        let c = Currencies::derive(&[w("A", &[], false), w("B", &["A"], false)]);
        assert_eq!(c.of("A"), Derived::Current);
        assert_eq!(c.pending, vec![("B".to_owned(), "A".to_owned())]);
    }

    #[test]
    fn a_written_field_does_not_make_a_warrant_superseded() {
        let mut a = w("A", &[], false);
        a.written = Some("superseded".into());
        assert_eq!(Currencies::derive(&[a]).of("A"), Derived::Current);
    }

    #[test]
    fn annulled_comes_from_the_resolution_and_deprecated_from_the_legacy_field() {
        let mut a = w("A", &[], false);
        a.annulled = true;
        let mut b = w("B", &[], false);
        b.written = Some("deprecated".into());
        let c = Currencies::derive(&[a, b]);
        assert_eq!(c.of("A"), Derived::Annulled);
        assert_eq!(c.of("B"), Derived::Deprecated);
    }

    #[test]
    fn a_cycle_is_found_once() {
        let c = Currencies::derive(&[
            w("A", &["C"], true),
            w("B", &["A"], true),
            w("C", &["B"], true),
            w("D", &[], true),
        ]);
        assert_eq!(c.cycles.len(), 1);
        assert_eq!(c.cycles[0].first(), Some(&"A".to_owned()));
        assert_eq!(c.cycles[0].len(), 4);
        let none = Currencies::derive(&[w("A", &[], true), w("B", &["A"], true)]);
        assert!(none.cycles.is_empty());
    }
}
