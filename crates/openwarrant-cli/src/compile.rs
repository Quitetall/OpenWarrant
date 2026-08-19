// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war compile` — write the configured projections (SAS §71.8, §17).

use std::fs;

use openwarrant_compiler::render_adr_overview;
use openwarrant_compiler::{
    CanonicalError, CompilationBasis, View, canonical_json, full_warrant, lower,
};
use openwarrant_core::ValidatedManifest;

use crate::repo::{RepoError, Repository};

/// Compile every projection this build implements.
///
/// Returns them as `(view, contents)` rather than writing, so `war check` can
/// compare against what is committed WITHOUT touching the working tree. A drift
/// check that had to write in order to compare would be a mutating gate, which
/// §44.8 treats as a distinct and more dangerous thing.
pub fn projections(
    basis: &CompilationBasis,
    validated: &ValidatedManifest,
) -> Result<Vec<(View, String)>, CanonicalError> {
    let ir = lower(basis, validated)?;
    Ok(vec![
        (View::FullWarrant, full_warrant(&ir, basis)),
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

        let views = match projections(basis, validated) {
            Ok(views) => views,
            Err(err) => {
                skipped.push(format!("{alias} ({err})"));
                continue;
            }
        };

        for (view, contents) in views {
            let path = dir.join(view.filename());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                    context: format!("could not create {parent}"),
                    source,
                })?;
            }
            // Write only when the bytes actually change, so recompiling a clean
            // tree does not churn mtimes and make every build look dirty.
            let unchanged = fs::read_to_string(&path)
                .map(|existing| existing == contents)
                .unwrap_or(false);
            if !unchanged {
                fs::write(&path, &contents).map_err(|source| RepoError::Io {
                    context: format!("could not write {path}"),
                    source,
                })?;
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
        let (path, contents) = adr_overview(repo)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                context: format!("could not create {parent}"),
                source,
            })?;
        }
        let unchanged = fs::read_to_string(&path)
            .map(|existing| existing == contents)
            .unwrap_or(false);
        if !unchanged {
            fs::write(&path, &contents).map_err(|source| RepoError::Io {
                context: format!("could not write {path}"),
                source,
            })?;
            written += 1;
        }
        println!("compiled ADR Overview");
    }

    println!(
        "\n{} file(s) written, {} Warrant(s) skipped",
        written,
        skipped.len()
    );
    Ok(())
}
