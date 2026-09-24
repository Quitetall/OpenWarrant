// SPDX-License-Identifier: Apache-2.0
//! `war compile` — write the configured projections (SAS §71.8, §17).

use std::fs;

use openwarrant_compiler::{
    CanonicalError, ChildRef, CompilationBasis, View, canonical_json, full_warrant, lower,
};
use openwarrant_compiler::{WarrantSummary, render_adr_overview, render_warrant_overview};
use openwarrant_core::ValidatedManifest;

use crate::repo::{RepoError, Repository};

// OW-WAR-0121: the one write path for records, declared here because the
// module list in `lib.rs` is not this Warrant's to change. Every other writer
// reaches it as `crate::compile::atomic`.
#[path = "atomic.rs"]
pub(crate) mod atomic;

/// §20.4's child list for one Warrant, computed from the whole corpus.
///
/// Lives here rather than in the compiler because it needs every manifest, and
/// `lower` is deliberately pure and single-Warrant. Sorted by alias so the
/// rendered view is stable — an unsorted list would make every recompile look
/// like drift.
#[must_use]
pub fn children_of(uuid: &str, corpus: &[crate::repo::Loaded]) -> Vec<ChildRef> {
    let claimed: Vec<&openwarrant_core::ValidatedManifest> = corpus
        .iter()
        .filter_map(|one| one.validated.as_ref())
        .filter(|v| {
            v.raw
                .parents
                .iter()
                .any(|p| p.r#ref.trim_start_matches("war://") == uuid)
        })
        .collect();
    if claimed.is_empty() {
        return vec![];
    }
    // The child's state is its DERIVED currency (OW-ADR-0022), from the one
    // derivation `war check` reads — never the manifest's field.
    let currencies = crate::relations::currencies(corpus);
    let mut out: Vec<ChildRef> = claimed
        .into_iter()
        .map(|validated| ChildRef {
            alias: validated.alias.to_string(),
            r#ref: format!("war://{}", validated.raw.uuid),
            state: currencies.of(validated.alias.as_str()).to_string(),
        })
        .collect();
    out.sort_by(|a, b| a.alias.cmp(&b.alias));
    out
}

/// Compile every projection this build implements.
///
/// Returns them as `(view, contents)` rather than writing, so `war check` can
/// compare against what is committed WITHOUT touching the working tree. A drift
/// check that had to write in order to compare would be a mutating gate, which
/// §44.8 treats as a distinct and more dangerous thing.
pub fn projections(
    basis: &CompilationBasis,
    validated: &ValidatedManifest,
    children: &[ChildRef],
) -> Result<Vec<(View, String)>, CanonicalError> {
    let ir = lower(basis, validated)?;
    Ok(vec![
        (View::FullWarrant, full_warrant(&ir, basis, children)),
        (View::CanonicalJson, canonical_json(&ir)?),
    ])
}

/// Compile the ADR Overview (§19.6, RQ-021), returning its path and contents.
///
/// Separate from the Warrant projections because it is a projection of the ADR
/// corpus, not of any one Warrant — but written and drift-checked by exactly the
/// same rules, since §19.7 forbids a manually maintained index for the same
/// reason §17.2 forbids an authoritative parent.
pub fn adr_overview(repo: &Repository) -> Result<(camino::Utf8PathBuf, String), RepoError> {
    let adrs = repo.load_adrs()?;
    Ok((repo.adr_overview_path(), render_adr_overview(&adrs.records)))
}

/// The SAS's normative projection (slice E1): `docs/sas/generated/NORMATIVE.md`
/// and `NORMATIVE.json` — every binding sentence with its section, and the
/// digest of the document they came from. Agents read this, not the whole
/// document; the drift check keeps it honest.
pub fn sas_normative(repo: &Repository) -> Result<Vec<(camino::Utf8PathBuf, String)>, RepoError> {
    let (path, bytes) = repo.sas_document()?;
    let text = String::from_utf8_lossy(&bytes);
    let sentences = openwarrant_core::normative_sentences(&text);
    let digest = {
        use sha2::Digest;
        let mut h = sha2::Sha256::new();
        h.update(&bytes);
        h.finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let revision = repo.load_sas_revisions().ok().and_then(|revs| {
        revs.into_iter()
            .find(|r| r.sha256 == digest)
            .map(|r| r.version)
    });
    let source = repo.relative(&path);
    let dir = repo.root.join(&repo.config.paths.sas).join("generated");

    // Markdown: one line per sentence, grouped by section, headed by what it
    // was derived from and roughly what it costs to read.
    let mut md = String::new();
    let json_body = serde_json::json!({
        "schema": "oh.war/sas-normative/v1",
        "source": source,
        "sha256": digest,
        "revision": revision,
        "sentences": sentences,
    });
    let json = serde_jcs::to_string(&json_body)
        .map_err(|e| RepoError::Message(format!("could not canonicalise NORMATIVE.json: {e}")))?
        + "\n";
    let md_body = normative_body(&sentences);
    let estimated_tokens = (md_body.len() + 400) / 4;
    md.push_str(&format!(
        "# Normative sentences of {source}\n\n<!-- GENERATED by `war compile` from sha256:{digest}{}; do not edit. -->\n\n{} sentence(s) — every SHALL, SHALL NOT, MUST, SHOULD and MAY of the document, with its section. \
         Read this instead of the document: it is ~{estimated_tokens} tokens (bytes/4) against the document's ~{}. \
         The sentence is what binds; the section number is where the reasoning lives.\n",
        revision.as_deref().map(|v| format!(" (revision {v})")).unwrap_or_default(),
        sentences.len(),
        bytes.len() / 4
    ));
    md.push_str(&md_body);
    Ok(vec![
        (dir.join("NORMATIVE.md"), md),
        (dir.join("NORMATIVE.json"), json),
    ])
}

/// The SAS's section index (OW-WAR-0125): `docs/sas/generated/SECTIONS.json`
/// and `SECTIONS.md` — every section of the document in force with its id,
/// title, byte range and sha256, and the revision the bytes match. Derived
/// from the pinned document, never authored (OW-ADR-0028 step one), and
/// drift-checked beside `NORMATIVE.*`.
pub fn sas_sections(repo: &Repository) -> Result<Vec<(camino::Utf8PathBuf, String)>, RepoError> {
    use openwarrant_core::sas_sections::{self, SectionKind};
    let (path, bytes) = repo.sas_document()?;
    let digest = openwarrant_compiler::sha256_hex(&bytes);
    let sections = sas_sections::split(&bytes);
    // The split is lossless or it is not published: an index whose sections
    // do not join back to the document would describe some other document.
    let joined = openwarrant_compiler::sha256_hex(&sas_sections::join(&sections));
    if joined != digest {
        return Err(RepoError::Message(format!(
            "the SAS split does not join back to the document (sha256:{joined}, document \
             sha256:{digest}); no section index is written from a lossy split"
        )));
    }
    let revision = repo.load_sas_revisions().ok().and_then(|revs| {
        revs.into_iter()
            .find(|r| r.sha256 == digest)
            .map(|r| r.version)
    });
    let namespace = sas_sections::namespace(&String::from_utf8_lossy(&bytes));
    let source = repo.relative(&path);
    let dir = repo.root.join(&repo.config.paths.sas).join("generated");

    let json_body = serde_json::json!({
        "schema": "oh.war/sas-sections/v1",
        "source": source,
        "sha256": digest,
        "revision": revision,
        "namespace": namespace,
        "sections": sections,
    });
    let json = serde_jcs::to_string(&json_body)
        .map_err(|e| RepoError::Message(format!("could not canonicalise SECTIONS.json: {e}")))?
        + "\n";

    let cite = namespace.as_deref().unwrap_or("<NS>");
    let headings = sections
        .iter()
        .filter(|s| matches!(s.kind, SectionKind::Part | SectionKind::Appendix))
        .count();
    let numbered = sections
        .iter()
        .filter(|s| s.kind == SectionKind::Numbered)
        .count();
    let subsections: usize = sections.iter().map(|s| s.subsections.len()).sum();
    let mut md = format!(
        "# Sections of {source}\n\n<!-- GENERATED by `war compile` from sha256:{digest}{}; do not edit. -->\n\n\
         {} unit(s): the preamble, {headings} part and appendix heading(s), {numbered} numbered \
         section(s), and {subsections} subsection(s) within them. Joined in order, the units are the \
         document byte for byte. A numbered section is cited as `sas://{cite}-SAS-<n>`, a subsection \
         as `sas://{cite}-SAS-<n>.<m>` (OW-ADR-0028); `war check` resolves each such citation in an \
         amendment against the revision its Warrant is pinned to.\n\n\
         | id | title | bytes | sha256 (first 16) |\n| --- | --- | ---: | --- |\n",
        revision
            .as_deref()
            .map(|v| format!(" (revision {v})"))
            .unwrap_or_default(),
        sections.len(),
    );
    let cell = |s: &str| s.replace('|', "\\|");
    for s in &sections {
        md.push_str(&format!(
            "| `{}` | {} | {}–{} | `{}` |\n",
            s.id,
            cell(&s.title),
            s.start,
            s.end,
            &s.sha256[..16]
        ));
        for sub in &s.subsections {
            md.push_str(&format!(
                "| `{}` | ↳ {} | {}–{} | `{}` |\n",
                sub.id,
                cell(&sub.title),
                sub.start,
                sub.end,
                &sub.sha256[..16]
            ));
        }
    }
    Ok(vec![
        (dir.join("SECTIONS.md"), md),
        (dir.join("SECTIONS.json"), json),
    ])
}

/// The normative statements as Markdown, one line each, grouped by section.
/// Shared by `NORMATIVE.md` and the master document's *In force*, so the two
/// cannot state the SAS differently.
fn normative_body(sentences: &[openwarrant_core::NormativeSentence]) -> String {
    let mut body = String::new();
    let mut last_section = String::new();
    for s in sentences {
        if s.section != last_section {
            body.push_str(&format!("\n## §{} {}\n\n", s.section, s.heading));
            last_section.clone_from(&s.section);
        }
        body.push_str(&format!(
            "- **{}** §{}: {}\n",
            s.keyword.as_str(),
            s.section,
            s.sentence
        ));
    }
    body
}

/// A value's serde name (`snake_case` for the status enums), for rendering.
fn word<T: serde::Serialize>(t: &T) -> String {
    match serde_json::to_value(t) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => "unknown".to_owned(),
    }
}

/// `a/b/../c` → `a/c`, for an atom path written relative to its manifest.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// An atom's text after its frontmatter, byte for byte. A structured atom
/// (YAML) has none and is returned whole.
#[must_use]
pub fn atom_body(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---\n")
    {
        return rest[end + 5..].trim_start_matches('\n');
    }
    text
}

/// Gather what both master projections render (OW-ADR-0022). One
/// gathering, two renderings; the currency in it is the derivation
/// `relations::currencies` computes, and the queue is `war next`'s, judged.
pub fn master_corpus(repo: &Repository) -> Result<openwarrant_compiler::CurrentCorpus, RepoError> {
    use openwarrant_compiler::current as c;

    let loaded: Vec<crate::repo::Loaded> = repo
        .warrant_dirs()?
        .iter()
        .filter_map(|d| repo.load_warrant(d).ok())
        .collect();
    let currencies = crate::relations::currencies(&loaded);
    let status = crate::status::build(repo)?;

    // Roadmap phases: the record's order and titles when there is one; the
    // membership is status's, as `war roadmap` reads it.
    let mut phases: Vec<c::Phase> = match crate::roadmap_cmd::load(repo) {
        Ok(Some(l)) => l
            .phases
            .in_dependency_order()
            .into_iter()
            .map(|p| c::Phase {
                id: p.id.clone(),
                title: p.title.clone(),
            })
            .collect(),
        _ => vec![],
    };
    let mut phase_of: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for o in &status.objectives {
        let Some(r) = &o.roadmap_ref else { continue };
        let id = format!("{}-PHASE-{}", r.prefix, r.phase);
        if !phases.iter().any(|p| p.id == id) {
            phases.push(c::Phase {
                id: id.clone(),
                title: o.title.clone(),
            });
        }
        for w in &o.warrants {
            phase_of.entry(w.clone()).or_insert_with(|| id.clone());
        }
    }

    let mut subjects = Vec::new();
    for one in &loaded {
        let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
            continue;
        };
        let alias = validated.alias.to_string();
        let st = status.warrants.iter().find(|w| w.alias == alias);
        let auth = repo.load_authorization(&one.dir).ok().flatten();
        let manifest_dir = basis
            .manifest_source
            .rsplit_once('/')
            .map_or("", |(d, _)| d)
            .to_owned();
        let derived = currencies.of(&alias);
        subjects.push(c::Subject {
            alias: alias.clone(),
            title: basis.manifest.title.clone(),
            manifest: basis.manifest_source.clone(),
            profile: validated.profile.to_string(),
            currency: derived.to_string(),
            replaced_by: match &derived {
                crate::relations::Derived::Superseded { by } => Some(by.clone()),
                _ => None,
            },
            phase: phase_of.get(&alias).cloned(),
            rung: st.map_or_else(|| "unknown".to_owned(), |w| word(&w.rung)),
            basis_revision: auth.as_ref().and_then(|a| a.sas_revision.clone()),
            contract_revision: auth.as_ref().and_then(|a| {
                (a.revision.state == openwarrant_core::RevisionState::Authorized)
                    .then_some(a.revision.revision)
            }),
            contract_digest: auth.as_ref().and_then(|a| {
                (a.revision.state == openwarrant_core::RevisionState::Authorized)
                    .then(|| a.revision.contract_digest.clone())
            }),
            authorization: auth
                .as_ref()
                .map(|_| repo.relative(&one.dir.join("authorization.toml"))),
            resolution: one
                .dir
                .join("resolution.toml")
                .is_file()
                .then(|| repo.relative(&one.dir.join("resolution.toml"))),
            atoms: basis
                .atoms
                .iter()
                .map(|a| {
                    let text = String::from_utf8_lossy(&a.bytes);
                    c::Atom {
                        ordinal: a.ordinal,
                        role: a.role.clone(),
                        source: normalize(&format!("{manifest_dir}/{}", a.source)),
                        body: atom_body(&text).to_owned(),
                        structured: !a.source.ends_with(".md"),
                    }
                })
                .collect(),
            deliverables: st
                .map(|w| {
                    w.deliverables
                        .iter()
                        .map(|d| c::Deliverable {
                            id: d.id.clone(),
                            title: d.title.clone(),
                            target_ref: d.target_ref.clone(),
                            digest: word(&d.digest),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    subjects.sort_by(|a, b| a.alias.cmp(&b.alias));

    let sas = repo.load_sas_revisions().ok().and_then(|revs| {
        let pin = crate::sas::pin_of(&revs)?.clone();
        let (path, bytes) = repo.sas_document().ok()?;
        let sentences = openwarrant_core::normative_sentences(&String::from_utf8_lossy(&bytes));
        Some(c::SasInForce {
            version: pin.version.clone(),
            state: word(&pin.state),
            sha256: pin.sha256.clone(),
            source: repo.relative(&path),
            record: repo.relative(&repo.sas_revision_path(&pin.version)),
            adr: pin.acceptance.as_ref().and_then(|a| a.adr_ref.clone()),
            statement_count: sentences.len(),
            statements: normative_body(&sentences),
        })
    });

    let decisions = repo
        .load_adrs()?
        .records
        .into_iter()
        .map(|a| c::Decision {
            alias: a.local_alias,
            title: a.title,
            status: a.status.to_string(),
            source: a.source,
            body: a.body,
        })
        .collect();

    let ownership = crate::ownership::Ownership::index_with(repo, &currencies)?;
    let governed = ownership
        .paths()
        .filter_map(|p| {
            let o = ownership.current(p)?;
            let manifest = repo.warrants_dir().join(&o.alias).join("manifest.toml");
            Some(c::Governed {
                path: p.to_owned(),
                warrant: o.alias.clone(),
                deliverable: o.deliverable_id.clone(),
                authorized_at: o.authorized_at.clone(),
                resolved: o.resolved,
                manifest: manifest.is_file().then(|| repo.relative(&manifest)),
            })
        })
        .collect();

    let register_path = repo.root.join("docs/authority/roles.toml");
    let signers = repo
        .load_authority_register()?
        .assignments
        .iter()
        .map(|a| c::Signer {
            actor: a.actor.clone(),
            kind: word(&a.actor_kind),
            roles: a.roles.iter().map(ToString::to_string).collect(),
            principal: a.ssh_principal.clone(),
        })
        .collect();

    let awaiting = crate::next::run_with(repo, &status)?
        .actions
        .into_iter()
        .filter(|a| a.actor == crate::next::Actor::Human)
        .map(|a| c::Awaiting {
            judged: a
                .judged
                .as_ref()
                .map_or_else(|| "not judged".to_owned(), crate::next::Judged::word),
            warrant: a.warrant,
            action: a.action,
            command: a.command,
            why: a.why,
        })
        .collect();

    let timeline = crate::timeline::build_timeline(repo)?;
    let days: Vec<c::Day> = timeline
        .days
        .iter()
        .map(|d| c::Day {
            date: d.date.clone(),
            events: d.events,
            by_type: d.by_type.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        })
        .collect();
    let tree_date = days
        .iter()
        .rev()
        .map(|d| d.date.clone())
        .find(|d| d.as_bytes().first().is_some_and(u8::is_ascii_digit));

    Ok(openwarrant_compiler::CurrentCorpus {
        program: repo.config.project.name.clone(),
        tree_date,
        sas,
        decisions,
        phases,
        subjects,
        governed,
        signers,
        awaiting,
        timeline: days,
        register: register_path
            .is_file()
            .then(|| repo.relative(&register_path)),
    })
}

/// The master document, and the history when `[generated] history = true`.
pub fn master_documents(
    repo: &Repository,
) -> Result<Vec<(camino::Utf8PathBuf, String)>, RepoError> {
    use openwarrant_compiler::current::{CURRENT_PATH, HISTORY_PATH};
    let corpus = master_corpus(repo)?;
    let mut out = vec![(
        repo.root.join(CURRENT_PATH),
        openwarrant_compiler::render_current(&corpus),
    )];
    if repo.config.generated.history {
        out.push((
            repo.root.join(HISTORY_PATH),
            openwarrant_compiler::render_history(&corpus),
        ));
    }
    Ok(out)
}

/// Derive a Warrant's state from the record's shape (SAS §24).
///
/// Every Warrant derives to `draft`, and that is not a placeholder — it is the
/// truthful answer. §24.7's `draft → proposed → authorized` chain requires an
/// AUTHORIZATION, and contract revisions do not exist until OW-WAR-0009. Nothing
/// in this repository has been authorized by anything, so nothing has left draft.
///
/// Deriving a later phase from "the work looks done" is exactly the fabrication
/// this system exists to prevent: it would let a tool award itself a lifecycle it
/// never transitioned through. The provenance marker says `derived` so no reader
/// mistakes this for a recorded transition.
fn derive_state() -> openwarrant_core::WarrantState {
    openwarrant_core::WarrantState::draft(openwarrant_core::Provenance::Derived)
}

/// Compile the Warrant Overview (§17.5 `status`), returning its path and contents.
pub fn warrant_overview(repo: &Repository) -> Result<(camino::Utf8PathBuf, String), RepoError> {
    let mut summaries = Vec::new();
    // Resolve parent UUIDs to aliases where the parent is in this corpus, so the
    // Relations section reads as names rather than as opaque identifiers.
    let mut alias_by_uuid = std::collections::BTreeMap::new();
    let mut loaded = Vec::new();
    for dir in repo.warrant_dirs()? {
        let one = repo.load_warrant(&dir)?;
        if let Some(v) = &one.validated {
            alias_by_uuid.insert(v.uuid.to_string(), v.alias.to_string());
        }
        loaded.push(one);
    }

    let currencies = crate::relations::currencies(&loaded);
    for one in &loaded {
        // A Warrant that would not validate is OMITTED rather than rendered with
        // blank fields — an entry that looks like a Warrant but describes nothing
        // is worse than a missing one. `war check` reports it separately.
        let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
            continue;
        };
        summaries.push(WarrantSummary {
            alias: validated.alias.to_string(),
            uuid: validated.uuid.to_string(),
            title: basis.manifest.title.clone(),
            profile: validated.profile.to_string(),
            assurance_level: validated.assurance_level.to_string(),
            implements: basis
                .manifest
                .implements
                .iter()
                .map(|i| i.r#ref.clone())
                .collect(),
            roadmap: basis
                .manifest
                .roadmap
                .iter()
                .map(|r| r.r#ref.clone())
                .collect(),
            parents: basis
                .manifest
                .parents
                .iter()
                .map(|p| {
                    let uuid = p.r#ref.strip_prefix("war://").unwrap_or(&p.r#ref);
                    alias_by_uuid
                        .get(uuid)
                        .cloned()
                        .unwrap_or_else(|| p.r#ref.clone())
                })
                .collect(),
            atom_count: basis.atoms.len(),
            source: basis.manifest_source.clone(),
            state: {
                // Recorded from the journal when one exists (OW-WAR-0031).
                let outcome = repo
                    .load_resolution(&one.dir)
                    .ok()
                    .flatten()
                    .and_then(|r| r.resolution.common_outcome.to_string().parse().ok());
                crate::journal_cmd::load(&one.dir)
                    .ok()
                    .and_then(|j| crate::journal_cmd::recorded_state(&j, outcome))
                    .unwrap_or_else(derive_state)
            },
            currency: currencies.of(validated.alias.as_str()).to_string(),
            milestone_count: basis
                .atoms
                .iter()
                .filter(|a| a.role == "milestones")
                .find_map(|a| {
                    openwarrant_core::milestones::parse(&String::from_utf8_lossy(&a.bytes)).ok()
                })
                .map(|g| (g.milestones.len(), g.stages.len())),
        });
    }
    Ok((
        repo.warrant_overview_path(),
        render_warrant_overview(&summaries),
    ))
}

/// Compile one Warrant, or all of them, writing into each `generated/`.
pub fn run(repo: &Repository, only: Option<&str>) -> Result<(), RepoError> {
    let dirs = match only {
        Some(alias) => vec![repo.warrant_dir(alias)?],
        None => repo.warrant_dirs()?,
    };

    if dirs.is_empty() {
        println!("no Warrants found in {}", repo.config.paths.warrants);
        return Ok(());
    }

    let mut written = 0usize;
    let mut skipped = Vec::new();

    // §20.4's child list needs every manifest, so load the corpus once even when
    // only one Warrant was asked for: a parent rendered without its children is
    // a projection that quietly under-reports the family.
    let corpus: Vec<crate::repo::Loaded> = repo
        .warrant_dirs()?
        .iter()
        .filter_map(|d| repo.load_warrant(d).ok())
        .collect();

    for dir in &dirs {
        let loaded = repo.load_warrant(dir)?;
        let alias = loaded.alias();

        let (Some(basis), Some(validated)) = (&loaded.basis, &loaded.validated) else {
            // Refuse to emit a projection of a Warrant we could not validate.
            // A generated document is a claim about its sources; producing one
            // from sources we rejected would be the falsest thing this tool
            // could do.
            skipped.push(alias);
            continue;
        };

        let children = children_of(&validated.raw.uuid, &corpus);
        let views = match projections(basis, validated, &children) {
            Ok(views) => views,
            Err(err) => {
                skipped.push(format!("{alias} ({err})"));
                continue;
            }
        };

        for (view, contents) in views {
            let path = dir.join(view.committed_filename());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                    context: format!("could not create {parent}"),
                    source,
                })?;
            }
            // Write only when the bytes actually change, so recompiling a clean
            // tree does not churn mtimes and make every build look dirty. The
            // bytes read to decide that are the prestate the write checks, so
            // a view changed meanwhile is refused rather than overwritten, and
            // a crash mid-write leaves the old view whole (§86).
            let before = atomic::prestate(&path)?;
            if before != atomic::Prestate::of(contents.as_bytes()) {
                atomic::write_if(&path, &contents, &before)?;
                written += 1;
            }
        }
        println!("compiled {alias}");
    }

    if !skipped.is_empty() {
        // Never silent. A Warrant skipped without a word reads as compiled.
        eprintln!("\nnot compiled ({}): {}", skipped.len(), skipped.join(", "));
        eprintln!("run `war check` for the reason.");
    }

    // The ADR Overview covers the whole corpus, so it is compiled once rather
    // than per Warrant, and only on a full run.
    if only.is_none() {
        let mut projections = vec![
            warrant_overview(repo)?,
            adr_overview(repo)?,
            crate::status::corpus_status_md(repo)?,
            crate::status::corpus_status_json(repo)?,
            crate::status::corpus_status_html(repo)?,
            crate::timeline::corpus_timeline_json(repo)?,
            crate::timeline::corpus_pending_json(repo)?,
        ];
        // A repository with no SAS document has nothing to project; one with
        // an unreadable one is refused as it always was, by sas_document.
        if repo.sas_document().is_ok() {
            projections.extend(sas_normative(repo)?);
            projections.extend(sas_sections(repo)?);
        }
        // Last: the master document reads what the others read, and the
        // queue it carries is judged by the dry run (OW-ADR-0022).
        projections.extend(master_documents(repo)?);
        for (path, contents) in projections {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                    context: format!("could not create {parent}"),
                    source,
                })?;
            }
            let before = atomic::prestate(&path)?;
            if before != atomic::Prestate::of(contents.as_bytes()) {
                atomic::write_if(&path, &contents, &before)?;
                written += 1;
            }
            println!("compiled {}", path.file_name().unwrap_or("overview"));
        }
    }

    println!(
        "\n{} file(s) written, {} Warrant(s) skipped",
        written,
        skipped.len()
    );
    Ok(())
}
