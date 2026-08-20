// SPDX-License-Identifier: AGPL-3.0-or-later
//! Projections: the human Markdown parent and the canonical JSON (SAS §17, §103).
//!
//! §17.2: "The parent document is never authoritative merely because it is
//! committed to Git. It is a reproducible projection of the Compilation Basis."

use std::fmt::Write as _;

use openwarrant_core::frontmatter;

use crate::canonical::{CanonicalError, to_canonical_string};
use crate::ir::WarIr;
use crate::lower::CompilationBasis;

/// The projections §17.5 names.
///
/// §17.5 says the compiler "SHALL support at least" these. Support and
/// COMMITMENT are different things: every view here is renderable on demand, and
/// [`View::is_committed`] decides which are written to disk. Emitting all nine
/// per Warrant would put 360 generated files in a 40-Warrant repository, every
/// one of them drift-checked, to serve views a reader asks for occasionally.
///
/// `AdrOverview` is a corpus-level projection of the ADR set rather than of one
/// Warrant, so it renders from the ADR corpus and has no per-Warrant file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum View {
    FullWarrant,
    WorkOrder,
    AdrSection,
    AdrOverview,
    StageDispatch,
    AssuranceCase,
    Status,
    Audit,
    CanonicalJson,
}

impl View {
    /// §17.5's nine, in the specification's order.
    pub const ALL: [Self; 9] = [
        Self::FullWarrant,
        Self::WorkOrder,
        Self::AdrSection,
        Self::AdrOverview,
        Self::StageDispatch,
        Self::AssuranceCase,
        Self::Status,
        Self::Audit,
        Self::CanonicalJson,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FullWarrant => "full_warrant",
            Self::WorkOrder => "work_order",
            Self::AdrSection => "adr_section",
            Self::AdrOverview => "adr_overview",
            Self::StageDispatch => "stage_dispatch",
            Self::AssuranceCase => "assurance_case",
            Self::Status => "status",
            Self::Audit => "audit",
            Self::CanonicalJson => "canonical_json",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.as_str() == name)
    }

    /// Whether this view is written to disk and drift-checked.
    #[must_use]
    pub const fn is_committed(self) -> bool {
        matches!(self, Self::FullWarrant | Self::CanonicalJson)
    }

    /// The file a committed view is written to, relative to a Warrant's
    /// directory. `None` for a view rendered on demand.
    #[must_use]
    pub const fn filename(self) -> Option<&'static str> {
        match self {
            Self::FullWarrant => Some("generated/WAR.md"),
            Self::CanonicalJson => Some("generated/WAR.json"),
            _ => None,
        }
    }

    /// The filename of a view already known to be committed.
    ///
    /// `projections()` yields only committed views, so callers there have a
    /// filename by construction. This keeps that guarantee in one place instead
    /// of an `unwrap` at every use, and names what would be wrong if it failed.
    #[must_use]
    pub const fn committed_filename(self) -> &'static str {
        match self.filename() {
            Some(f) => f,
            None => panic!(
                "a view without a file was yielded as a committed projection; \
                 View::is_committed and View::filename must agree"
            ),
        }
    }
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Render only the atoms whose role is in `roles`, in manifest order.
fn atoms_by_role(basis: &CompilationBasis, roles: &[&str]) -> String {
    let mut out = String::new();
    for atom in basis
        .atoms
        .iter()
        .filter(|a| roles.contains(&a.role.as_str()))
    {
        let body = atom_body(&atom.bytes);
        if body.trim().is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(body.trim_end());
        out.push('\n');
    }
    out
}

/// A short header naming the view, so a rendered projection cannot be mistaken
/// for the full Warrant.
fn view_header(ir: &WarIr, view: View) -> String {
    format!(
        "{}\n# {}: {} — `{}` projection\n\n\
         *This is one projection of the Warrant, not the Warrant. \
         See `generated/WAR.md` for the full record.*\n",
        generated_header(ir),
        ir.identity.local_alias,
        ir.identity.title,
        view
    )
}

/// §22 — the Work Order projection: what to do, without the rationale.
#[must_use]
pub fn work_order(ir: &WarIr, basis: &CompilationBasis) -> String {
    let mut out = view_header(ir, View::WorkOrder);
    out.push_str(&atoms_by_role(basis, &["work_order", "milestones"]));
    out
}

/// §19 — the ADRs this Warrant is governed by or implements.
#[must_use]
pub fn adr_section(ir: &WarIr, basis: &CompilationBasis) -> String {
    let mut out = view_header(ir, View::AdrSection);
    out.push_str(&atoms_by_role(basis, &["adr", "decisions"]));
    if ir.relations.implements.is_empty() && out.lines().count() < 8 {
        out.push_str("\nNo ADR relations are declared for this Warrant.\n");
    }
    out
}

/// §47 — the stage-level view a dispatch compiler starts from.
///
/// This renders the stage graph. It is NOT a Stage Dispatch: a dispatch is
/// compiled per stage per attempt with a context manifest and an attempt basis,
/// and calling this one would overstate what a projection can produce.
#[must_use]
pub fn stage_dispatch(ir: &WarIr, basis: &CompilationBasis) -> String {
    let mut out = view_header(ir, View::StageDispatch);
    out.push_str(
        "\n*The stage graph, not a Stage Dispatch. A dispatch is compiled per \
         stage per attempt against a context manifest and an attempt basis \
         (§47.1).*\n",
    );
    out.push_str(&atoms_by_role(basis, &["milestones", "work_order"]));
    out
}

/// §38 — the acceptance argument on its own.
#[must_use]
pub fn assurance_case(ir: &WarIr, basis: &CompilationBasis) -> String {
    let mut out = view_header(ir, View::AssuranceCase);
    out.push_str(&atoms_by_role(basis, &["assurance", "basis"]));
    out
}

/// §24 — the five-dimension state, and where it came from.
#[must_use]
pub fn status(ir: &WarIr) -> String {
    let mut out = view_header(ir, View::Status);
    let _ = writeln!(out, "\n| Field | Value |");
    let _ = writeln!(out, "|---|---|");
    let _ = writeln!(out, "| Local alias | `{}` |", ir.identity.local_alias);
    let _ = writeln!(out, "| Profile | `{}` |", ir.identity.profile);
    let _ = writeln!(
        out,
        "| Assurance level | `{}` |",
        ir.identity.assurance_level
    );
    let _ = writeln!(out, "| Contract revision | {} |", ir.contract_revision);
    let _ = writeln!(
        out,
        "| Enterprise ID | {} |",
        ir.identity
            .enterprise_id
            .as_deref()
            .map(|id| format!("`{id}`"))
            .unwrap_or_else(|| "*not allocated*".to_owned())
    );
    let _ = writeln!(
        out,
        "| Resolution | {} |",
        if ir.resolution.is_some() {
            "recorded"
        } else {
            "*none*"
        }
    );
    out.push_str(
        "\n*State shown here is DERIVED from the record. Nothing in this \
         repository records a §24 state directly until the local journal exists \
         (OW-WAR-0031).*\n",
    );
    out
}

/// The audit projection: digests and provenance, nothing else.
#[must_use]
pub fn audit(ir: &WarIr) -> String {
    let mut out = view_header(ir, View::Audit);
    let _ = writeln!(out, "\n## Integrity\n");
    let _ = writeln!(out, "- **algorithm:** `{}`", ir.integrity.algorithm);
    let _ = writeln!(
        out,
        "- **manifest digest:** `sha256:{}`",
        ir.source_and_composition.manifest_digest
    );
    let _ = writeln!(
        out,
        "- **composition revision digest:** `sha256:{}`",
        ir.integrity.composition_revision_digest
    );
    let _ = writeln!(
        out,
        "- **workspace basis digest:** `sha256:{}`",
        ir.integrity.workspace_basis_digest
    );

    let _ = writeln!(out, "\n## Contract coverage (§28.5)\n");
    let covered: Vec<String> = ir
        .contract_coverage
        .covered()
        .map(|e| format!("`{e}`"))
        .collect();
    let _ = writeln!(
        out,
        "Covers {} of 17 elements: {}",
        covered.len(),
        covered.join(", ")
    );

    let _ = writeln!(out, "\n## Source atoms\n");
    let _ = writeln!(out, "| Ordinal | Role | Jurisdiction | Source | Digest |");
    let _ = writeln!(out, "|---:|---|---|---|---|");
    for atom in &ir.source_and_composition.atoms {
        let _ = writeln!(
            out,
            "| {} | `{}` | `{}` | `{}` | `sha256:{}` |",
            atom.ordinal, atom.role, atom.jurisdiction, atom.source, atom.atom_source_digest
        );
    }
    out
}

/// Render any §17.5 view.
///
/// `AdrOverview` returns `None`: it is a projection of the ADR corpus, not of a
/// single Warrant, and is emitted separately by the ADR compiler. Returning
/// `None` rather than an empty string keeps the caller from rendering a blank
/// page and calling it an overview.
#[must_use]
pub fn render_view(view: View, ir: &WarIr, basis: &CompilationBasis) -> Option<String> {
    Some(match view {
        View::FullWarrant => full_warrant(ir, basis),
        View::WorkOrder => work_order(ir, basis),
        View::AdrSection => adr_section(ir, basis),
        View::StageDispatch => stage_dispatch(ir, basis),
        View::AssuranceCase => assurance_case(ir, basis),
        View::Status => status(ir),
        View::Audit => audit(ir),
        View::CanonicalJson => canonical_json(ir).ok()?,
        View::AdrOverview => return None,
    })
}

/// The machine-readable and human-visible banner every generated file carries
/// (§17.1).
fn generated_header(ir: &WarIr) -> String {
    format!(
        "<!--\n\
         GENERATED BY OPENWARRANT. DO NOT EDIT.\n\
         WAR: {alias}\n\
         Compilation basis: sha256:{basis}\n\
         Contract revision: {revision}\n\
         Source manifest: {manifest}\n\
         -->\n",
        alias = ir.identity.local_alias,
        basis = ir.integrity.workspace_basis_digest,
        revision = ir.contract_revision,
        manifest = ir.source_and_composition.manifest_source,
    )
}

/// Strip an atom's frontmatter, returning just its body.
///
/// An atom whose frontmatter will not parse is rendered whole rather than
/// dropped: losing content silently is worse than rendering a header, and
/// `war check` reports the parse failure separately.
fn atom_body(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    match frontmatter::parse(&text) {
        Ok(fm) => text[fm.body_offset..].trim_start_matches('\n').to_owned(),
        Err(_) => text.into_owned(),
    }
}

/// Render the full human Warrant (§103).
pub fn full_warrant(ir: &WarIr, basis: &CompilationBasis) -> String {
    let mut out = generated_header(ir);

    let _ = write!(
        out,
        "\n# {}: {}\n",
        ir.identity.local_alias, ir.identity.title
    );

    // Control (§18.1) is a generated section: identity and standing, projected
    // rather than authored.
    out.push_str("\n## Control\n\n");
    let _ = writeln!(out, "| Field | Value |");
    let _ = writeln!(out, "|---|---|");
    let _ = writeln!(out, "| UUID | `{}` |", ir.identity.uuid);
    let _ = writeln!(out, "| Local alias | `{}` |", ir.identity.local_alias);
    let _ = writeln!(
        out,
        "| Enterprise ID | {} |",
        ir.identity
            .enterprise_id
            .as_deref()
            .map(|id| format!("`{id}`"))
            .unwrap_or_else(|| "*not allocated*".to_owned())
    );
    let _ = writeln!(out, "| Profile | `{}` |", ir.identity.profile);
    let _ = writeln!(
        out,
        "| Assurance level | `{}` |",
        ir.identity.assurance_level
    );

    // Atom bodies, in the manifest's declared order. §16.1 requires
    // inapplicable optional roles to be OMITTED rather than rendered as empty
    // ceremonial headings, which is why this iterates what exists instead of
    // walking the full role list.
    for atom in &basis.atoms {
        let body = atom_body(&atom.bytes);
        if body.trim().is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(body.trim_end());
        out.push('\n');
    }

    // Relations, Provenance, and Integrity (§103) — generated.
    out.push_str("\n## Relations, Provenance, and Integrity\n");

    if !ir.relations.implements.is_empty() {
        out.push_str("\n### Implements\n\n");
        for edge in &ir.relations.implements {
            let _ = writeln!(
                out,
                "- `{}`{}",
                edge.r#ref,
                edge.contribution
                    .as_deref()
                    .map(|c| format!(" — {c}"))
                    .unwrap_or_default()
            );
        }
    }
    if !ir.relations.parents.is_empty() {
        out.push_str("\n### Parents\n\n");
        for parent in &ir.relations.parents {
            let _ = writeln!(
                out,
                "- `{}` at contract revision {}",
                parent.r#ref, parent.contract_revision
            );
        }
    }
    if !ir.relations.roadmap.is_empty() {
        out.push_str("\n### Roadmap\n\n");
        for entry in &ir.relations.roadmap {
            let _ = writeln!(out, "- `{entry}`");
        }
    }

    out.push_str("\n### Source atoms\n\n");
    let _ = writeln!(out, "| Ordinal | Role | Jurisdiction | Source | Digest |");
    let _ = writeln!(out, "|---:|---|---|---|---|");
    for atom in &ir.source_and_composition.atoms {
        let _ = writeln!(
            out,
            "| {} | `{}` | `{}` | `{}` | `sha256:{}…` |",
            atom.ordinal,
            atom.role,
            atom.jurisdiction,
            atom.source,
            &atom.atom_source_digest[..16]
        );
    }

    out.push_str("\n### Integrity\n\n");
    let _ = writeln!(out, "- **algorithm:** `{}`", ir.integrity.algorithm);
    let _ = writeln!(
        out,
        "- **manifest digest:** `sha256:{}`",
        ir.source_and_composition.manifest_digest
    );
    let _ = writeln!(
        out,
        "- **composition revision digest:** `sha256:{}`",
        ir.integrity.composition_revision_digest
    );
    let _ = writeln!(
        out,
        "- **workspace basis digest:** `sha256:{}`",
        ir.integrity.workspace_basis_digest
    );

    out
}

/// Render the canonical JSON projection (§15.3).
pub fn canonical_json(ir: &WarIr) -> Result<String, CanonicalError> {
    // A trailing newline so the file is well-formed text; the canonical bytes
    // are what precedes it, and the drift check compares whole files either way.
    Ok(format!("{}\n", to_canonical_string(ir)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::{AtomSource, lower};
    use openwarrant_core::manifest::AtomEntry;
    use openwarrant_core::{MANIFEST_SCHEMA, Manifest};

    fn fixture() -> (WarIr, CompilationBasis) {
        let manifest = Manifest {
            schema: MANIFEST_SCHEMA.to_owned(),
            uuid: "01a018db-19fc-7f2a-8e39-69730f255e33".to_owned(),
            local_alias: "OW-WAR-0001".to_owned(),
            enterprise_id: String::new(),
            title: "Establish the repository".to_owned(),
            profile: "delivery".to_owned(),
            assurance_level: Some("basic".to_owned()),
            implements: vec![],
            roadmap: vec![],
            parents: vec![],
            atoms: ["intent", "basis", "work_order", "milestones", "assurance"]
                .iter()
                .enumerate()
                .map(|(i, role)| AtomEntry {
                    ordinal: (i as u32 + 1) * 10,
                    role: (*role).to_owned(),
                    path: Some(format!("atoms/{role}.md")),
                    r#ref: None,
                    required: true,
                })
                .collect(),
        };
        let validated = manifest.validate(Some("OW")).expect("valid");
        let atoms: Vec<AtomSource> = manifest
            .atoms
            .iter()
            .map(|a| AtomSource {
                ordinal: a.ordinal,
                role: a.role.clone(),
                jurisdiction: "authored".to_owned(),
                source: a.path.clone().expect("path"),
                bytes: format!(
                    "---\nschema: oh.war/atom/v1\nrole: {}\n---\n\n# {}\n\nSome body text.\n",
                    a.role, a.role
                )
                .into_bytes(),
                required: a.required,
            })
            .collect();
        let basis = CompilationBasis {
            manifest_source: "docs/warrants/OW-WAR-0001/manifest.toml".to_owned(),
            manifest_bytes: b"(manifest)".to_vec(),
            manifest,
            atoms,
        };
        let ir = lower(&basis, &validated).expect("lowers");
        (ir, basis)
    }

    #[test]
    fn the_generated_header_is_present_and_complete() {
        let (ir, basis) = fixture();
        let md = full_warrant(&ir, &basis);
        assert!(md.starts_with("<!--\nGENERATED BY OPENWARRANT. DO NOT EDIT."));
        for required in [
            "WAR: OW-WAR-0001",
            "Compilation basis: sha256:",
            "Contract revision: 1",
            "Source manifest: docs/warrants/OW-WAR-0001/manifest.toml",
        ] {
            assert!(md.contains(required), "header missing {required:?}");
        }
    }

    #[test]
    fn atom_frontmatter_is_stripped_from_the_parent() {
        let (ir, basis) = fixture();
        let md = full_warrant(&ir, &basis);
        assert!(
            !md.contains("schema: oh.war/atom/v1"),
            "frontmatter must not appear in the rendered parent"
        );
        assert!(md.contains("Some body text."), "bodies must appear");
    }

    /// §16.1: inapplicable optional roles are omitted, not rendered as empty
    /// ceremonial headings.
    #[test]
    fn absent_roles_produce_no_empty_headings() {
        let (ir, basis) = fixture();
        let md = full_warrant(&ir, &basis);
        for absent in ["## Execution", "## Resolution", "## Ongoing Validation"] {
            assert!(
                !md.contains(absent),
                "{absent} has no source and must not be rendered"
            );
        }
    }

    /// OW-WAR-0004 OBL-002: rendering is a pure function of the Basis, so two
    /// runs are byte-identical. If this fails, the drift check fires on every
    /// run and gets disabled within a week.
    #[test]
    fn rendering_is_byte_identical_across_runs() {
        let (ir, basis) = fixture();
        assert_eq!(full_warrant(&ir, &basis), full_warrant(&ir, &basis));
        assert_eq!(
            canonical_json(&ir).expect("json"),
            canonical_json(&ir).expect("json")
        );
    }

    /// §91.1 test 3 at the projection level: changing the rendering must not
    /// change the contract digest.
    #[test]
    fn the_rendered_parent_does_not_feed_the_contract_digest() {
        let (ir, basis) = fixture();
        let before = ir.contract_digest().expect("digest");
        let _ = full_warrant(&ir, &basis);
        assert_eq!(before, ir.contract_digest().expect("digest"));
    }

    #[test]
    fn an_unallocated_enterprise_id_renders_as_such() {
        let (ir, basis) = fixture();
        let md = full_warrant(&ir, &basis);
        assert!(md.contains("*not allocated*"));
    }

    #[test]
    fn canonical_json_ends_with_exactly_one_newline() {
        let (ir, _) = fixture();
        let json = canonical_json(&ir).expect("json");
        assert!(json.ends_with('\n'));
        assert!(!json.ends_with("\n\n"));
    }

    #[test]
    fn every_source_atom_appears_in_the_provenance_table() {
        let (ir, basis) = fixture();
        let md = full_warrant(&ir, &basis);
        for atom in &ir.source_and_composition.atoms {
            assert!(
                md.contains(&atom.source),
                "atom {} missing from the provenance table",
                atom.source
            );
        }
    }
}
