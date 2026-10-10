// SPDX-License-Identifier: Apache-2.0
//! The master document, `docs/generated/CURRENT.md` (OW-ADR-0022).
//!
//! One document, current by construction: every current subject fully
//! expanded, atom by atom in role order, verbatim; the SAS in force by its
//! normative statements; the accepted decisions; who governs each path; who
//! may sign; and the queue with every signing command already judged by the
//! dry run. A replaced subject is one line of lineage and nothing else.
//!
//! This module is pure: it renders a [`Corpus`] the CLI gathered from the
//! records. It decides nothing about currency — the corpus arrives with each
//! subject's currency already derived from its relations
//! (`relations::currencies`), and a second computation here would be a second
//! answer.
//!
//! # Where a role is tied to a rendering
//!
//! [`ROLE_SECTIONS`] is the only place. An atom carries its relation to the
//! projections by naming a role. A namespaced optional extension role (§16.4,
//! `x.review`) has no row: its atoms render verbatim under their role name in
//! the subject's *Extensions* section, after every row (OW-WAR-0113 AM-002).
//! Any other role no row names is refused by `war check`
//! (`atom.role-unprojected`), because an atom nothing renders is text nobody
//! reads. [`is_rendered_role`] is that test, shared by both.
//!
//! # Adding a section
//!
//! [`SECTIONS`] is the order; [`Section`] names each; `render` calls one
//! function per variant. A new section is a variant, a function, and a place
//! in the list — nothing else moves; the Roadmap (OW-WAR-0114 M5) was added
//! that way.

use std::fmt::Write as _;

/// One row of the profile's role→section map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleRow {
    /// The role as a manifest names it.
    pub role: &'static str,
    /// The heading this role's atoms render under, within a subject, in both
    /// projections.
    pub heading: &'static str,
}

/// The composition table (§16.1, §16.2): the roles an authored or bound atom
/// may carry, in the order a subject renders them. `control` and
/// `relations_and_integrity` are compiler-produced sections with no atom, so
/// they have no row.
pub const ROLE_SECTIONS: &[RoleRow] = &[
    RoleRow {
        role: "intent",
        heading: "Intent",
    },
    RoleRow {
        role: "basis",
        heading: "Basis",
    },
    RoleRow {
        role: "adr",
        heading: "Decision",
    },
    RoleRow {
        role: "work_order",
        heading: "Work order",
    },
    RoleRow {
        role: "milestones",
        heading: "Milestones",
    },
    RoleRow {
        role: "execution",
        heading: "Execution",
    },
    RoleRow {
        role: "assurance",
        heading: "Assurance",
    },
    RoleRow {
        role: "resolution",
        heading: "Resolution",
    },
    RoleRow {
        role: "validation",
        heading: "Validation",
    },
];

/// The row that renders `role`, if any projection renders it.
#[must_use]
pub fn role_row(role: &str) -> Option<(usize, &'static RoleRow)> {
    ROLE_SECTIONS
        .iter()
        .enumerate()
        .find(|(_, r)| r.role == role)
}

/// Whether a projection renders an atom of `role`: a row of
/// [`ROLE_SECTIONS`] names it, or it is a namespaced extension role (§16.4),
/// which renders under *Extensions*. The namespace test is the manifest's
/// own ([`openwarrant_core::is_namespaced_extension_role`]), so what the
/// grammar accepts (G-M8) and what the projections render cannot drift apart.
#[must_use]
pub fn is_rendered_role(role: &str) -> bool {
    role_row(role).is_some() || openwarrant_core::is_namespaced_extension_role(role)
}

/// The heading a subject's extension atoms render under.
pub const EXTENSIONS_HEADING: &str = "Extensions";

/// Where the master document is written, relative to the repository root.
pub const CURRENT_PATH: &str = "docs/generated/CURRENT.md";
/// Where the history is written, relative to the repository root.
pub const HISTORY_PATH: &str = "docs/generated/HISTORY.md";

/// One atom of a subject, as the records hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub ordinal: u32,
    pub role: String,
    /// Repository-relative path.
    pub source: String,
    /// The atom's text after its frontmatter, byte for byte.
    pub body: String,
    /// A structured atom (YAML) renders fenced; Markdown renders inline.
    pub structured: bool,
}

/// One declared deliverable and the state of its digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deliverable {
    pub id: String,
    pub title: String,
    pub target_ref: String,
    pub digest: String,
}

/// One Warrant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    pub alias: String,
    pub title: String,
    /// Repository-relative manifest path.
    pub manifest: String,
    pub profile: String,
    /// The DERIVED currency: `current`, `superseded`, `annulled`,
    /// `deprecated`.
    pub currency: String,
    /// For a superseded subject, the Warrant(s) whose relation replaced it.
    pub replaced_by: Option<String>,
    /// The roadmap phase this subject belongs to, by id.
    pub phase: Option<String>,
    pub rung: String,
    /// The SAS revision the authorization names, when authorized.
    pub basis_revision: Option<String>,
    pub contract_revision: Option<u32>,
    pub contract_digest: Option<String>,
    /// Repository-relative record paths, when they exist.
    pub authorization: Option<String>,
    pub resolution: Option<String>,
    pub atoms: Vec<Atom>,
    pub deliverables: Vec<Deliverable>,
}

impl Subject {
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.currency == "current"
    }

    /// Atoms in role order, then ordinal.
    #[must_use]
    pub fn atoms_in_role_order(&self) -> Vec<&Atom> {
        let mut atoms: Vec<&Atom> = self.atoms.iter().collect();
        atoms.sort_by_key(|a| (role_row(&a.role).map_or(usize::MAX, |(i, _)| i), a.ordinal));
        atoms
    }
}

/// A roadmap phase, for grouping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase {
    pub id: String,
    pub title: String,
}

/// A current Warrant that names a roadmap phase, with its rung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub alias: String,
    pub rung: String,
}

/// One phase of the roadmap record, with what the records derive about it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoadmapPhase {
    pub id: String,
    pub title: String,
    pub outcome: String,
    pub exit: String,
    /// The tier, rendered: `1 Library and Standard`.
    pub tier: Option<String>,
    pub depends_on: Vec<String>,
    /// The Warrant whose ref carries the `exit` slug, when one does.
    pub exit_warrant: Option<String>,
    /// DERIVED from the exit Warrant's recorded resolution, as `war status`
    /// derives it; the record has no field to assert it with.
    pub achieved: String,
    /// DERIVED: the current Warrants whose own ref (or placement) names this
    /// phase. The record holds no member list.
    pub members: Vec<Member>,
    /// Slugs of work the phase needs that no Warrant carries yet.
    pub open: Vec<String>,
}

/// One revision record of the roadmap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoadmapRevisionEntry {
    pub revision: u32,
    /// `accepted` or `proposed`.
    pub state: String,
    pub sha256: String,
    pub predecessor: Option<u32>,
    /// Repository-relative path of the revision record.
    pub record: String,
    pub accepted_by: Option<String>,
    pub effective_time: Option<String>,
    /// The proposer's note, labelled as theirs.
    pub note: Option<String>,
    /// The phase-level diff against the predecessor, as the signing screen
    /// showed it.
    pub diff: String,
    /// The phases as that revision recorded them: id, title.
    pub phases: Vec<(String, String)>,
}

/// An earlier plan the roadmap record retires: one line of lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetiredPlan {
    pub path: String,
    pub lineage: String,
}

/// The roadmap record (OW-ADR-0023), gathered for both projections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roadmap {
    /// Repository-relative path of `roadmap.toml`.
    pub manifest: String,
    /// Repository-relative path of the phases atom.
    pub phases_source: String,
    /// sha256 over the manifest and atoms as they stand.
    pub digest: String,
    /// The newest accepted revision.
    pub accepted_revision: Option<u32>,
    /// Whether the atoms as they stand ARE that accepted revision.
    pub accepted: bool,
    /// A proposed revision recording the atoms as they stand, awaiting a
    /// signature.
    pub pending_revision: Option<u32>,
    /// The intent atom, verbatim.
    pub intent: Option<Atom>,
    /// In dependency order.
    pub phases: Vec<RoadmapPhase>,
    /// Every revision record, oldest first.
    pub revisions: Vec<RoadmapRevisionEntry>,
    pub retires: Vec<RetiredPlan>,
}

/// The SAS revision in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SasInForce {
    pub version: String,
    pub state: String,
    pub sha256: String,
    /// Repository-relative path of the document.
    pub source: String,
    /// Repository-relative path of the revision record.
    pub record: String,
    /// The ADR the acceptance carried (§101.3), when one did.
    pub adr: Option<String>,
    /// The normative statements, one Markdown line each, grouped by section.
    pub statements: String,
    pub statement_count: usize,
}

/// One ADR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub alias: String,
    pub title: String,
    pub status: String,
    pub source: String,
    /// The atom body, frontmatter stripped.
    pub body: String,
}

/// One governed path (OW-ADR-0021).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Governed {
    pub path: String,
    pub warrant: String,
    pub deliverable: String,
    pub authorized_at: String,
    pub resolved: bool,
    /// The governing Warrant's manifest, for the link.
    pub manifest: Option<String>,
}

/// One actor on the authority register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    pub actor: String,
    pub kind: String,
    pub roles: Vec<String>,
    pub principal: Option<String>,
}

/// One act awaiting a human, as `war next` hands it over, judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Awaiting {
    pub warrant: String,
    pub action: String,
    pub command: String,
    pub why: String,
    /// `would record`, or `would refuse: <rule>`.
    pub judged: String,
}

/// One day of the recorded timeline (history only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    pub date: String,
    pub events: usize,
    pub by_type: Vec<(String, usize)>,
}

/// Everything both projections render, gathered by the CLI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Corpus {
    pub program: String,
    /// The latest recorded event date, so the document dates itself from
    /// the records rather than from a clock.
    pub tree_date: Option<String>,
    pub sas: Option<SasInForce>,
    pub decisions: Vec<Decision>,
    /// The roadmap record, when the program has one.
    pub roadmap: Option<Roadmap>,
    pub phases: Vec<Phase>,
    pub subjects: Vec<Subject>,
    pub governed: Vec<Governed>,
    pub signers: Vec<Signer>,
    pub awaiting: Vec<Awaiting>,
    pub timeline: Vec<Day>,
    /// The register file, for the link, when one exists.
    pub register: Option<String>,
}

/// The master document's sections, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    ReadThisFirst,
    InForce,
    Decisions,
    Roadmap,
    Warrants,
    Replaced,
    WhoGoverns,
    WhoMaySign,
    AwaitingAHuman,
}

/// The order `render` writes them in.
pub const SECTIONS: &[Section] = &[
    Section::ReadThisFirst,
    Section::InForce,
    Section::Decisions,
    Section::Roadmap,
    Section::Warrants,
    Section::Replaced,
    Section::WhoGoverns,
    Section::WhoMaySign,
    Section::AwaitingAHuman,
];

/// A link from `docs/generated/` to a repository-relative path. Links go to
/// atoms and records, never to another projection.
#[must_use]
pub fn link(path: &str) -> String {
    format!("../../{path}")
}

/// Render the master document.
#[must_use]
pub fn render(c: &Corpus) -> String {
    let mut out = String::new();
    for section in SECTIONS {
        match section {
            Section::ReadThisFirst => read_this_first(c, &mut out),
            Section::InForce => in_force(c, &mut out),
            Section::Decisions => decisions(c, &mut out),
            Section::Roadmap => roadmap(c, &mut out),
            Section::Warrants => warrants(c, &mut out),
            Section::Replaced => replaced(c, &mut out),
            Section::WhoGoverns => who_governs(c, &mut out),
            Section::WhoMaySign => who_may_sign(c, &mut out),
            Section::AwaitingAHuman => awaiting(c, &mut out),
        }
    }
    out
}

fn read_this_first(c: &Corpus, out: &mut String) {
    let current = c.subjects.iter().filter(|s| s.is_current()).count();
    let replaced = c.subjects.len() - current;
    let _ = write!(
        out,
        "<!--\nGENERATED BY OPENWARRANT (`war admin compile`). DO NOT EDIT.\nSource: the atoms and records of this repository. `war check --generated` refuses a hand edit.\n-->\n\n\
         # {program} — the current state\n\n\
         ## Read this first\n\n\
         This is the master document (OW-ADR-0022): what is authoritative in this \
         repository now, in one place. It is a projection — `war admin compile` writes it \
         from the authored atoms and the signed records, and it holds no authority of \
         its own. To change what it says, edit an atom or sign a record; never this \
         file.\n\n\
         - **Current by construction.** A Warrant is current unless an authorized \
         Warrant declares `supersedes` → it, or its resolution is annulled. Currency \
         is derived from those relations, never written.\n\
         - **Fully expanded.** Every current Warrant's atoms appear below verbatim, in \
         role order. Nothing is paraphrased.\n\
         - **Replaced subjects are one line.** {replaced} replaced subject(s) appear \
         under *Replaced* as lineage and nothing else; their text is in the history, \
         when this repository keeps one.\n\
         - **Records as of {date}** — the latest recorded event. {current} current \
         Warrant(s).\n\n",
        program = c.program,
        date = c.tree_date.as_deref().unwrap_or("no recorded event"),
    );
}

fn in_force(c: &Corpus, out: &mut String) {
    out.push_str("## In force\n\n");
    let Some(sas) = &c.sas else {
        out.push_str("No SAS revision is recorded; nothing is normative by record.\n\n");
        return;
    };
    let _ = write!(
        out,
        "SAS revision **{version}** ({state}), sha256:`{sha}` — [document]({doc}), \
         [revision record]({rec}){adr}.\n\n\
         {n} normative statement(s), each with its section. The statement is what \
         binds; the section is where the reasoning lives.\n",
        version = sas.version,
        state = sas.state,
        sha = sas.sha256,
        doc = link(&sas.source),
        rec = link(&sas.record),
        adr = sas
            .adr
            .as_ref()
            .map(|a| format!("; accepted under {a}"))
            .unwrap_or_default(),
        n = sas.statement_count,
    );
    // Demote the statement groups one level so they nest under this section.
    for line in sas.statements.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            let _ = writeln!(out, "### {rest}");
        } else {
            let _ = writeln!(out, "{line}");
        }
    }
    out.push('\n');
}

fn decisions(c: &Corpus, out: &mut String) {
    out.push_str("## Decisions\n\n");
    let accepted: Vec<&Decision> = c
        .decisions
        .iter()
        .filter(|d| d.status == "accepted")
        .collect();
    let proposed: Vec<&Decision> = c
        .decisions
        .iter()
        .filter(|d| d.status == "proposed")
        .collect();
    let _ = writeln!(
        out,
        "{} accepted decision(s), expanded; {} proposed, one line each.\n",
        accepted.len(),
        proposed.len()
    );
    for d in accepted {
        let _ = writeln!(
            out,
            "### {} — {}\n\n[source]({})\n\n{}\n",
            d.alias,
            d.title,
            link(&d.source),
            d.body.trim_end()
        );
    }
    if !proposed.is_empty() {
        out.push_str("### Proposed\n\n");
        for d in proposed {
            let _ = writeln!(out, "- [{}]({}) — {}", d.alias, link(&d.source), d.title);
        }
        out.push('\n');
    }
}

/// The Roadmap section (OW-ADR-0023 decision 6): the record's intent
/// verbatim, then every phase in dependency order with its exit, its members
/// and their rungs, and whether it is achieved — the last two derived, as
/// `war status` derives them. Earlier revisions are the history's.
fn roadmap(c: &Corpus, out: &mut String) {
    out.push_str("## Roadmap\n\n");
    let Some(r) = &c.roadmap else {
        out.push_str(
            "No roadmap record: this program's phases are read from its SAS (§98) until it \
             adopts one (OW-ADR-0023).\n\n",
        );
        return;
    };
    let standing = match (r.accepted, r.accepted_revision, r.pending_revision) {
        (true, Some(n), _) => {
            format!("Revision **{n}** is accepted, and the atoms are that revision.")
        }
        (_, accepted, Some(p)) => format!(
            "The atoms are revision **{p}**, proposed and awaiting one signature \
             (`war sign roadmap --ssh-sign`); {}.",
            accepted.map_or_else(
                || "no revision is accepted yet".to_owned(),
                |n| format!("revision {n} stays the accepted order of work until then")
            )
        ),
        (_, accepted, None) => format!(
            "The atoms (sha256:`{}`) are not a recorded revision (`war plan roadmap propose`); {}.",
            r.digest.get(..12).unwrap_or(&r.digest),
            accepted.map_or_else(
                || "no revision is accepted yet".to_owned(),
                |n| format!("revision {n} is the accepted order of work")
            )
        ),
    };
    let earlier = r
        .revisions
        .iter()
        .filter(|v| {
            Some(v.revision) != r.accepted_revision && Some(v.revision) != r.pending_revision
        })
        .count();
    let _ = write!(
        out,
        "The order of work: one record per program, beside the SAS (OW-ADR-0023) — \
         [manifest]({}), [phases]({}). {standing} A phase's members are the current \
         Warrants whose own ref names it, and it is achieved when its exit Warrant \
         resolves satisfied; the record holds neither fact. {} phase(s), in dependency \
         order.{}\n\n",
        link(&r.manifest),
        link(&r.phases_source),
        r.phases.len(),
        if earlier == 0 {
            String::new()
        } else {
            format!(" {earlier} earlier revision(s) are in the history.")
        },
    );
    if let Some(intent) = &r.intent {
        atom_block("###", "Intent", intent, out);
    }
    out.push_str("### Phases\n\n");
    for p in &r.phases {
        let _ = writeln!(out, "#### {} — {}\n", p.id, p.title);
        let _ = writeln!(out, "- **Exit:** {}", p.exit);
        if !p.outcome.is_empty() {
            let _ = writeln!(out, "- **Outcome:** {}", p.outcome);
        }
        let _ = writeln!(
            out,
            "- **Tier:** {} · **after:** {}",
            p.tier.as_deref().unwrap_or("none"),
            if p.depends_on.is_empty() {
                "nothing".to_owned()
            } else {
                p.depends_on.join(", ")
            }
        );
        let _ = writeln!(
            out,
            "- **Achieved:** {}{}",
            p.achieved,
            p.exit_warrant
                .as_ref()
                .map(|w| format!(" (exit Warrant {w})"))
                .unwrap_or_default()
        );
        if p.members.is_empty() {
            out.push_str("- **Members:** none\n");
        } else {
            let _ = writeln!(
                out,
                "- **Members ({}):** {}",
                p.members.len(),
                p.members
                    .iter()
                    .map(|m| format!("{} `{}`", m.alias, m.rung))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        if !p.open.is_empty() {
            let _ = writeln!(out, "- **No Warrant yet:** {}", p.open.join(", "));
        }
        out.push('\n');
    }
    if !r.retires.is_empty() {
        out.push_str(
            "### Retired plans\n\nEach earlier plan is one line of lineage; the record above \
             replaces it.\n\n",
        );
        for t in &r.retires {
            let _ = writeln!(out, "- `{}` → {}", t.path, t.lineage);
        }
        out.push('\n');
    }
}

/// Render one subject fully expanded. Shared with the history.
pub(crate) fn subject(s: &Subject, level: usize, out: &mut String) {
    let h = "#".repeat(level);
    let _ = writeln!(
        out,
        "{h} {} — {}\n\n[manifest]({}){}{} · profile `{}` · rung `{}` · currency `{}`",
        s.alias,
        s.title,
        link(&s.manifest),
        s.authorization
            .as_ref()
            .map(|p| format!(" · [authorization]({})", link(p)))
            .unwrap_or_default(),
        s.resolution
            .as_ref()
            .map(|p| format!(" · [resolution]({})", link(p)))
            .unwrap_or_default(),
        s.profile,
        s.rung,
        s.currency,
    );
    let _ = writeln!(
        out,
        "{}\n",
        match (&s.contract_revision, &s.basis_revision) {
            (Some(r), Some(b)) => format!(
                "\nContract revision {r}{}, Basis SAS {b}.",
                s.contract_digest
                    .as_ref()
                    .map(|d| format!(" (`{d}`)"))
                    .unwrap_or_default()
            ),
            (Some(r), None) => format!("\nContract revision {r}, Basis SAS not recorded."),
            _ => "\nNot authorized; no Basis is fixed.".to_owned(),
        }
    );
    let atoms = s.atoms_in_role_order();
    let (rows, extensions): (Vec<&Atom>, Vec<&Atom>) =
        atoms.into_iter().partition(|a| role_row(&a.role).is_some());
    for atom in rows {
        let heading = role_row(&atom.role).map_or("Unprojected", |(_, r)| r.heading);
        atom_block(&format!("{h}#"), heading, atom, out);
    }
    // §16.4 — a namespaced optional extension is preserved: verbatim, under
    // its own role name. A role that is neither a row nor namespaced never
    // reaches here; `war check` refuses it (`atom.role-unprojected`).
    if !extensions.is_empty() {
        let _ = writeln!(out, "{h}# {EXTENSIONS_HEADING}\n");
        for atom in extensions {
            atom_block(&format!("{h}##"), &format!("`{}`", atom.role), atom, out);
        }
    }
    if !s.deliverables.is_empty() {
        let _ = writeln!(out, "{h}# Deliverables\n");
        out.push_str("| id | title | target | digest |\n|---|---|---|---|\n");
        for d in &s.deliverables {
            let _ = writeln!(
                out,
                "| {} | {} | `{}` | {} |",
                d.id,
                d.title.replace('|', "\\|"),
                d.target_ref,
                d.digest
            );
        }
        out.push('\n');
    }
}

/// One atom, verbatim, under `heading` at heading marker `hashes`.
fn atom_block(hashes: &str, heading: &str, atom: &Atom, out: &mut String) {
    let _ = writeln!(
        out,
        "{hashes} {heading} — [{}]({})\n",
        atom.source,
        link(&atom.source)
    );
    if atom.structured {
        let _ = writeln!(out, "```yaml\n{}```\n", ensure_newline(&atom.body));
    } else {
        let _ = writeln!(
            out,
            "<!-- atom {} begins -->\n{}<!-- atom {} ends -->\n",
            atom.source,
            ensure_newline(&atom.body),
            atom.source
        );
    }
}

fn ensure_newline(s: &str) -> String {
    if s.ends_with('\n') {
        s.to_owned()
    } else {
        format!("{s}\n")
    }
}

fn warrants(c: &Corpus, out: &mut String) {
    out.push_str("## Warrants\n\n");
    let current: Vec<&Subject> = c.subjects.iter().filter(|s| s.is_current()).collect();
    let _ = writeln!(
        out,
        "{} current Warrant(s), grouped by roadmap phase, each expanded atom by atom.\n",
        current.len()
    );
    let mut groups: Vec<(String, Vec<&Subject>)> = Vec::new();
    for p in &c.phases {
        let members: Vec<&Subject> = current
            .iter()
            .filter(|s| s.phase.as_deref() == Some(p.id.as_str()))
            .copied()
            .collect();
        if !members.is_empty() {
            groups.push((format!("{} — {}", p.id, p.title), members));
        }
    }
    let unplaced: Vec<&Subject> = current
        .iter()
        .filter(|s| {
            s.phase
                .as_deref()
                .is_none_or(|ph| !c.phases.iter().any(|p| p.id == ph))
        })
        .copied()
        .collect();
    if !unplaced.is_empty() {
        groups.push(("No roadmap phase".to_owned(), unplaced));
    }
    for (title, members) in groups {
        let _ = writeln!(out, "### {title}\n");
        for s in members {
            subject(s, 4, out);
        }
    }
}

fn replaced(c: &Corpus, out: &mut String) {
    out.push_str("## Replaced\n\n");
    let replaced: Vec<&Subject> = c.subjects.iter().filter(|s| !s.is_current()).collect();
    if replaced.is_empty() {
        out.push_str("Nothing has been replaced.\n\n");
        return;
    }
    for s in replaced {
        let _ = writeln!(
            out,
            "- {} → {}",
            s.alias,
            s.replaced_by.as_deref().unwrap_or(&s.currency)
        );
    }
    out.push('\n');
}

fn who_governs(c: &Corpus, out: &mut String) {
    out.push_str("## Who governs what\n\n");
    if c.governed.is_empty() {
        out.push_str(
            "No path is governed: no authorization records an owned set (OW-ADR-0021).\n\n",
        );
        return;
    }
    out.push_str(
        "Each path is governed by the most recently authorized Warrant whose signed \
         declaration names it (OW-ADR-0021).\n\n| path | governed by | deliverable | authorized | resolved |\n|---|---|---|---|---|\n",
    );
    for g in &c.governed {
        let who = g.manifest.as_ref().map_or_else(
            || g.warrant.clone(),
            |m| format!("[{}]({})", g.warrant, link(m)),
        );
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} |",
            g.path,
            who,
            g.deliverable,
            g.authorized_at,
            if g.resolved { "yes" } else { "no" }
        );
    }
    out.push('\n');
}

fn who_may_sign(c: &Corpus, out: &mut String) {
    out.push_str("## Who may sign\n\n");
    if c.signers.is_empty() {
        out.push_str("The authority register is empty: nobody may sign anything.\n\n");
        return;
    }
    if let Some(r) = &c.register {
        let _ = writeln!(out, "From the [authority register]({}).\n", link(r));
    }
    out.push_str("| actor | kind | roles | ssh principal |\n|---|---|---|---|\n");
    for s in &c.signers {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            s.actor,
            s.kind,
            s.roles.join(", "),
            s.principal.as_deref().unwrap_or("—")
        );
    }
    out.push('\n');
}

fn awaiting(c: &Corpus, out: &mut String) {
    out.push_str("## Awaiting a human\n\n");
    if c.awaiting.is_empty() {
        out.push_str("Nothing awaits a signature.\n");
        return;
    }
    out.push_str(
        "Every command here has been judged by the act's dry run (`war sign <target> \
         --dry-run`): the ingest ran with the write withheld. `would record` means the \
         signature is the only thing missing.\n\n| act | Warrant | command | judged | why |\n|---|---|---|---|---|\n",
    );
    for a in &c.awaiting {
        let _ = writeln!(
            out,
            "| {} | {} | `{}` | {} | {} |",
            a.action,
            a.warrant,
            a.command,
            a.judged,
            a.why.replace('|', "\\|")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(alias: &str, currency: &str, by: Option<&str>, body: &str) -> Subject {
        Subject {
            alias: alias.into(),
            title: format!("{alias} title"),
            manifest: format!("docs/warrants/{alias}/manifest.toml"),
            profile: "delivery".into(),
            currency: currency.into(),
            replaced_by: by.map(Into::into),
            phase: None,
            rung: "draft".into(),
            basis_revision: None,
            contract_revision: None,
            contract_digest: None,
            authorization: None,
            resolution: None,
            atoms: vec![
                Atom {
                    ordinal: 60,
                    role: "assurance".into(),
                    source: format!("docs/warrants/{alias}/atoms/60-assurance.md"),
                    body: "# Assurance\n".into(),
                    structured: false,
                },
                Atom {
                    ordinal: 10,
                    role: "intent".into(),
                    source: format!("docs/warrants/{alias}/atoms/10-intent.md"),
                    body: body.into(),
                    structured: false,
                },
            ],
            deliverables: vec![],
        }
    }

    #[test]
    fn a_replaced_subject_is_one_line_and_none_of_its_text() {
        let c = Corpus {
            program: "P".into(),
            subjects: vec![
                s("X-WAR-0001", "superseded", Some("X-WAR-0002"), "OLD TEXT\n"),
                s("X-WAR-0002", "current", None, "NEW TEXT\n"),
            ],
            ..Corpus::default()
        };
        let out = render(&c);
        assert!(out.contains("- X-WAR-0001 → X-WAR-0002\n"));
        assert!(!out.contains("OLD TEXT"));
        assert!(out.contains("NEW TEXT\n"));
        assert_eq!(out.matches("X-WAR-0001").count(), 1);
    }

    #[test]
    fn atoms_render_in_role_order_verbatim() {
        let c = Corpus {
            program: "P".into(),
            subjects: vec![s("X-WAR-0002", "current", None, "INTENT\n")],
            ..Corpus::default()
        };
        let out = render(&c);
        let intent = out.find("INTENT").unwrap();
        let assurance = out.find("# Assurance\n").unwrap();
        assert!(intent < assurance);
    }

    #[test]
    fn every_row_is_a_known_role_and_none_is_compiler_produced() {
        for row in ROLE_SECTIONS {
            assert!(!["control", "relations_and_integrity"].contains(&row.role));
        }
        assert!(role_row("ext.notes").is_none());
        assert!(role_row("intent").is_some());
    }

    #[test]
    fn a_namespaced_extension_is_rendered_and_a_bare_unknown_is_not() {
        assert!(is_rendered_role("intent"));
        assert!(is_rendered_role("x.review"));
        assert!(!is_rendered_role("hypothesis"));
        assert!(!is_rendered_role("relations_and_integrity"));
        assert!(!is_rendered_role(".leading"));
        assert!(!is_rendered_role("trailing."));
    }

    #[test]
    fn an_extension_atom_renders_verbatim_under_extensions_after_every_row() {
        let mut subject = s("X-WAR-0003", "current", None, "INTENT\n");
        subject.atoms.push(Atom {
            ordinal: 15,
            role: "x.review".into(),
            source: "docs/warrants/X-WAR-0003/atoms/15-review.md".into(),
            body: "# Review\n\nREVIEW TEXT, verbatim.\n".into(),
            structured: false,
        });
        let c = Corpus {
            program: "P".into(),
            subjects: vec![subject],
            ..Corpus::default()
        };
        let out = render(&c);
        let ext = out
            .find("##### Extensions\n")
            .expect("an Extensions section");
        let assurance = out.find("##### Assurance — ").unwrap();
        let review = out
            .find("###### `x.review` — [docs/warrants/X-WAR-0003/atoms/15-review.md]")
            .expect("the extension atom under its role name");
        assert!(assurance < ext && ext < review);
        assert!(out.contains("# Review\n\nREVIEW TEXT, verbatim.\n<!-- atom "));
        assert!(!out.contains("Unprojected"));
    }

    #[test]
    fn a_subject_without_extensions_has_no_extensions_section() {
        let c = Corpus {
            program: "P".into(),
            subjects: vec![s("X-WAR-0002", "current", None, "INTENT\n")],
            ..Corpus::default()
        };
        assert!(!render(&c).contains("Extensions"));
    }

    fn roadmap_corpus() -> Corpus {
        let phase = |id: &str, after: &[&str], members: &[(&str, &str)]| RoadmapPhase {
            id: id.into(),
            title: format!("{id} title"),
            exit: format!("{id} exits"),
            depends_on: after.iter().map(|s| (*s).to_owned()).collect(),
            achieved: "blocked by 1".into(),
            members: members
                .iter()
                .map(|(a, r)| Member {
                    alias: (*a).into(),
                    rung: (*r).into(),
                })
                .collect(),
            ..RoadmapPhase::default()
        };
        Corpus {
            roadmap: Some(Roadmap {
                manifest: "docs/roadmap/roadmap.toml".into(),
                phases_source: "docs/roadmap/atoms/20-phases.yaml".into(),
                digest: "ab".repeat(32),
                accepted_revision: Some(2),
                accepted: true,
                phases: vec![
                    phase("P-PHASE-0", &[], &[]),
                    phase("P-PHASE-1", &["P-PHASE-0"], &[("P-WAR-0001", "authorized")]),
                ],
                revisions: vec![
                    RoadmapRevisionEntry {
                        revision: 1,
                        ..RoadmapRevisionEntry::default()
                    },
                    RoadmapRevisionEntry {
                        revision: 2,
                        ..RoadmapRevisionEntry::default()
                    },
                ],
                retires: vec![RetiredPlan {
                    path: "docs/old-plan.md".into(),
                    lineage: "retired by the record".into(),
                }],
                ..Roadmap::default()
            }),
            ..Corpus::default()
        }
    }

    #[test]
    fn the_roadmap_renders_phases_in_order_with_exit_members_and_rungs() {
        let out = render(&roadmap_corpus());
        let section = out
            .split("## Roadmap\n")
            .nth(1)
            .and_then(|s| s.split("\n## ").next())
            .expect("a Roadmap section");
        let p0 = section.find("#### P-PHASE-0 — P-PHASE-0 title").unwrap();
        let p1 = section.find("#### P-PHASE-1 — P-PHASE-1 title").unwrap();
        assert!(p0 < p1, "{section}");
        assert!(section.contains("- **Exit:** P-PHASE-1 exits\n"));
        assert!(section.contains("- **Members (1):** P-WAR-0001 `authorized`\n"));
        assert!(section.contains("**after:** P-PHASE-0"));
        assert!(section.contains("Revision **2** is accepted"));
        assert!(section.contains("1 earlier revision(s) are in the history."));
        assert_eq!(section.matches("docs/old-plan.md").count(), 1);
        assert!(section.contains("- `docs/old-plan.md` → retired by the record\n"));
    }

    #[test]
    fn a_program_without_a_record_says_so() {
        let out = render(&Corpus::default());
        assert!(out.contains("## Roadmap\n\nNo roadmap record"));
    }

    #[test]
    fn sections_come_in_the_declared_order() {
        let out = render(&Corpus::default());
        let order: Vec<usize> = [
            "## Read this first",
            "## In force",
            "## Decisions",
            "## Roadmap",
            "## Warrants",
            "## Replaced",
            "## Who governs what",
            "## Who may sign",
            "## Awaiting a human",
        ]
        .iter()
        .map(|h| out.find(h).unwrap())
        .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]));
    }
}
