// SPDX-License-Identifier: AGPL-3.0-or-later
//! Stage-relevant context selection (slice C1; SAS §47.2 "select only
//! stage-relevant context", §33 context manifest).
//!
//! Before this, a Dispatch's context was required-vs-optional: every required
//! atom included whole, every optional one omitted, whatever the stage. A
//! stage may now declare what it needs — whole atoms, named sections of an
//! atom, repository artifacts, external references — and the selection is:
//!
//!   included = required atoms ∪ declared atoms ∪ declared sections
//!            ∪ declared artifacts ∪ declared external references
//!   omitted  = everything else, each with a TRUE reason.
//!
//! A declared section that does not exist is refused by name, with the
//! headings that do; a declared atom or artifact that does not exist is
//! refused. Sorted, so the manifest is deterministic.

use camino::Utf8Path;
use openwarrant_compiler::CompilationBasis;
use openwarrant_core::context::{
    ContextItem, ContextRole, Holder, Omission, Precedence, TrustClass,
};
use openwarrant_core::milestones::Stage;
use sha2::Digest;

use crate::repo::Repository;

/// One refusal, by rule, for `war check`-style reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub rule: &'static str,
    pub message: String,
}

pub struct Selection {
    pub included: Vec<ContextItem>,
    pub omitted: Vec<Omission>,
    /// The bytes each included item actually carries: whole atoms, section
    /// bodies, artifact files; an external ref carries none. Sorted by id.
    pub bytes: Vec<(String, u64)>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn role_for(atom_role: &str) -> ContextRole {
    match atom_role {
        "basis" => ContextRole::Governing,
        "intent" | "work_order" | "milestones" | "assurance" => ContextRole::Normative,
        _ => ContextRole::Informative,
    }
}

/// Select the context for `stage` out of `basis`.
pub fn select(
    repo: &Repository,
    warrant_dir: &Utf8Path,
    basis: &CompilationBasis,
    stage: &Stage,
    commit: Option<&str>,
) -> Result<Selection, Vec<Refusal>> {
    let mut refusals = Vec::new();
    let mut included: Vec<ContextItem> = Vec::new();
    let mut omitted: Vec<Omission> = Vec::new();
    let mut bytes: Vec<(String, u64)> = Vec::new();
    let holder = |path: String| Holder {
        kind: "git".to_owned(),
        repository: repo.config.project.name.clone(),
        commit_sha: commit.unwrap_or_default().to_owned(),
        path,
    };
    let atom_by_name = |name: &str| {
        basis
            .atoms
            .iter()
            .find(|a| a.source == name || a.source.rsplit('/').next() == Some(name))
    };

    // Whole atoms: every required one, plus the declared ones.
    let declared_atoms: Vec<&str> = stage.context_atoms.iter().map(String::as_str).collect();
    for name in &declared_atoms {
        if atom_by_name(name).is_none() {
            refusals.push(Refusal {
                rule: "dispatch.unknown-atom",
                message: format!(
                    "{}: context_atoms names {name:?}, which is not an atom of this Warrant; declared: {}",
                    stage.id,
                    basis.atoms.iter().map(|a| a.source.as_str()).collect::<Vec<_>>().join(", ")
                ),
            });
        }
    }
    // Sections: `<atom>#<heading>`.
    let mut section_items: Vec<(String, String, String)> = Vec::new(); // (atom source, heading, body)
    for sel in &stage.context_sections {
        let Some((atom_name, heading)) = sel.split_once('#') else {
            refusals.push(Refusal {
                rule: "dispatch.section-missing",
                message: format!(
                    "{}: context_sections entry {sel:?}: not of the form `<atom>#<heading>`",
                    stage.id
                ),
            });
            continue;
        };
        let Some(atom) = atom_by_name(atom_name) else {
            refusals.push(Refusal {
                rule: "dispatch.unknown-atom",
                message: format!("{}: context_sections names atom {atom_name:?}, which this Warrant does not have", stage.id),
            });
            continue;
        };
        let text = String::from_utf8_lossy(&atom.bytes);
        match openwarrant_core::sections::find(&text, heading) {
            Some(body) => {
                section_items.push((atom.source.clone(), heading.trim().to_owned(), body))
            }
            None => {
                let known: Vec<String> = openwarrant_core::sections::headings(&text)
                    .into_iter()
                    .map(|(_, t)| t)
                    .collect();
                refusals.push(Refusal {
                    rule: "dispatch.section-missing",
                    message: format!(
                        "{}: {atom_name} has no section {heading:?}; its headings are: {}",
                        stage.id,
                        known.join(" | ")
                    ),
                });
            }
        }
    }
    // Artifacts: repository paths that exist.
    let mut artifact_items = Vec::new();
    for path in &stage.context_artifacts {
        let full = repo.root.join(path);
        match std::fs::read(&full) {
            Ok(content) => {
                artifact_items.push((path.clone(), sha256_hex(&content), content.len() as u64))
            }
            Err(e) => refusals.push(Refusal {
                rule: "dispatch.artifact-missing",
                message: format!("{}: context_artifacts names {path:?}: {e}", stage.id),
            }),
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }

    for atom in &basis.atoms {
        let declared = declared_atoms
            .iter()
            .any(|n| *n == atom.source || atom.source.rsplit('/').next() == Some(n));
        let sectioned = section_items.iter().any(|(s, _, _)| *s == atom.source);
        if atom.required || declared {
            bytes.push((atom.source.clone(), atom.bytes.len() as u64));
            included.push(ContextItem {
                id: atom.source.clone(),
                role: role_for(&atom.role),
                required: atom.required,
                holder: holder(repo.relative(&warrant_dir.join(&atom.source))),
                content_digest: format!("sha256:{}", sha256_hex(&atom.bytes)),
                selector_sections: vec![],
                classification: "internal".to_owned(),
                trust: TrustClass::AuthoritativeInternal,
                taints: vec![],
                precedence: Some(Precedence::AuthorizedWarContract),
            });
        } else if !sectioned {
            omitted.push(Omission {
                id: atom.source.clone(),
                reason: format!(
                    "optional atom of role {:?}; {} declares neither it nor a section of it (§47.2 \"select only stage-relevant context\")",
                    atom.role, stage.id
                ),
                required: false,
            });
        }
    }
    for (source, heading, body) in &section_items {
        // A section of an atom that is included whole (every required atom
        // is, §47.2 "preserve every required normative source") narrows that
        // item: the section is recorded on it as a selector, and the packet's
        // context digest moves with the selection.
        if let Some(item) = included.iter_mut().find(|i| i.id == *source) {
            item.selector_sections.push(heading.clone());
            item.selector_sections.sort();
            item.selector_sections.dedup();
            continue;
        }
        bytes.push((format!("{source}#{heading}"), body.len() as u64));
        let atom = atom_by_name(source).expect("resolved above");
        included.push(ContextItem {
            id: format!("{source}#{heading}"),
            role: role_for(&atom.role),
            required: false,
            holder: holder(repo.relative(&warrant_dir.join(source))),
            content_digest: format!("sha256:{}", sha256_hex(body.as_bytes())),
            selector_sections: vec![heading.clone()],
            classification: "internal".to_owned(),
            trust: TrustClass::AuthoritativeInternal,
            taints: vec![],
            precedence: Some(Precedence::AuthorizedWarContract),
        });
    }
    for (path, digest, len) in artifact_items {
        bytes.push((path.clone(), len));
        included.push(ContextItem {
            id: path.clone(),
            role: ContextRole::Input,
            required: false,
            holder: holder(path),
            content_digest: format!("sha256:{digest}"),
            selector_sections: vec![],
            classification: "internal".to_owned(),
            trust: TrustClass::InternalUnverified,
            taints: vec![],
            precedence: Some(Precedence::InformativeSource),
        });
    }
    // The program's glossary (OW-WAR-0068, after mattpocock/skills
    // domain-modeling): a term defined once at the root costs one word in
    // every Dispatch, so the selector carries it whenever it exists.
    let glossary = repo.root.join("CONTEXT.md");
    if let Ok(body) = std::fs::read(&glossary) {
        bytes.push(("CONTEXT.md".to_owned(), body.len() as u64));
        included.push(ContextItem {
            id: "CONTEXT.md".to_owned(),
            role: ContextRole::Informative,
            required: false,
            holder: holder("CONTEXT.md".to_owned()),
            content_digest: format!("sha256:{}", sha256_hex(&body)),
            selector_sections: vec![],
            classification: "internal".to_owned(),
            trust: TrustClass::AuthoritativeInternal,
            taints: vec![],
            precedence: Some(Precedence::InformativeSource),
        });
    }
    for uri in &stage.context_external {
        included.push(ContextItem {
            id: uri.clone(),
            role: ContextRole::Informative,
            required: false,
            holder: Holder {
                kind: "external".to_owned(),
                repository: String::new(),
                commit_sha: String::new(),
                path: uri.clone(),
            },
            // Recorded, never fetched: no digest can honestly be claimed.
            content_digest: String::new(),
            selector_sections: vec![],
            classification: "internal".to_owned(),
            trust: TrustClass::ExternalUntrusted,
            taints: vec!["unfetched".to_owned()],
            precedence: Some(Precedence::InformativeSource),
        });
    }
    included.sort_by(|a, b| a.id.cmp(&b.id));
    omitted.sort_by(|a, b| a.id.cmp(&b.id));
    bytes.sort();
    Ok(Selection {
        included,
        omitted,
        bytes,
    })
}
