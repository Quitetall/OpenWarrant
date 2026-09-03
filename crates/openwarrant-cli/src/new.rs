// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war new` — create a draft Warrant (SAS §71.2).

use std::fs::{self, OpenOptions};
use std::io::Write as _;

use camino::Utf8PathBuf;
use openwarrant_core::{Profile, WarUuid};

use crate::repo::{RepoError, Repository};

/// How many times to retry when another process wins the allocation race.
///
/// Bounded rather than unbounded: a loop that retries forever turns a permanent
/// failure (an unwritable directory) into a hang.
const MAX_ALLOCATION_ATTEMPTS: u32 = 64;

/// Create a new draft Warrant and return its directory.
///
/// # Allocation is atomic
///
/// The next ordinal is chosen from what exists, and the manifest is created with
/// `create_new(true)` — `O_EXCL`. If another process took that ordinal between
/// the scan and the create, the create fails with `AlreadyExists` and we pick
/// again. Two concurrent `war new` invocations therefore produce two distinct
/// aliases, never one file with two authors.
///
/// This is not hypothetical. While these Warrants were being planned, an ADR was
/// committed in a sibling project while two others sat untracked holding their
/// numbers; a different pick would have collided. Scan-then-write without
/// `O_EXCL` is the same race with a wider window.
pub fn run(repo: &Repository, title: &str, profile: Profile) -> Result<Utf8PathBuf, RepoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(RepoError::Io {
            context: "a Warrant needs a title".to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty title"),
        });
    }

    let warrants = repo.warrants_dir();
    fs::create_dir_all(&warrants).map_err(|source| RepoError::Io {
        context: format!("could not create {warrants}"),
        source,
    })?;

    let namespace = repo.config.project.namespace.as_str();
    let mut next = next_ordinal(repo)?;

    for _ in 0..MAX_ALLOCATION_ATTEMPTS {
        let alias = format!("{namespace}-WAR-{next:04}");
        let dir = warrants.join(&alias);
        let manifest_path = dir.join("manifest.toml");

        fs::create_dir_all(dir.join("atoms")).map_err(|source| RepoError::Io {
            context: format!("could not create {dir}"),
            source,
        })?;

        // The atomic step. Everything above is idempotent; this is the claim.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&manifest_path)
        {
            Ok(mut file) => {
                let uuid = WarUuid::mint();
                file.write_all(manifest_template(&uuid, &alias, title, profile).as_bytes())
                    .map_err(|source| RepoError::Io {
                        context: format!("could not write {manifest_path}"),
                        source,
                    })?;
                write_atom_stubs(&dir, &uuid, profile)?;
                // §66.4 `draft.created` — the first journal entry, written by
                // the command that created the draft.
                crate::journal_cmd::record(
                    &dir,
                    &uuid.to_string(),
                    crate::journal_cmd::DRAFT_CREATED,
                    &format!("agent://{}", repo.performer()),
                    &format!("{{\"alias\":\"{alias}\",\"profile\":\"{profile}\"}}"),
                )?;
                return Ok(dir);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                next += 1;
                continue;
            }
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not create {manifest_path}"),
                    source,
                });
            }
        }
    }

    Err(RepoError::Io {
        context: format!(
            "could not allocate an alias after {MAX_ALLOCATION_ATTEMPTS} attempts; \
             {warrants} may be unwritable"
        ),
        source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "allocation exhausted"),
    })
}

/// One past the highest ordinal currently present.
fn next_ordinal(repo: &Repository) -> Result<u32, RepoError> {
    let mut highest = 0u32;
    for dir in repo.warrant_dirs()? {
        let Some(name) = dir.file_name() else {
            continue;
        };
        if let Some((_, digits)) = name.rsplit_once("-WAR-")
            && let Ok(n) = digits.parse::<u32>()
        {
            highest = highest.max(n);
        }
    }
    Ok(highest + 1)
}

fn manifest_template(uuid: &WarUuid, alias: &str, title: &str, profile: Profile) -> String {
    let mut out = format!(
        "# A Warrant is the contract for ONE bounded intervention inside a program,\n\
         # and it traces to that program's SAS through [[implements]] and [[roadmap]].\n\
         # Starting a program? Write its SAS instead (SAS §6.10; docs/DEFINITIONS.md).\n\
         schema = \"oh.war/manifest/v1\"\n\
         uuid = \"{uuid}\"\n\
         local_alias = \"{alias}\"\n\
         \n\
         # Allocated only by Knowledge Fabric (SAS §12.4). Leave empty.\n\
         enterprise_id = \"\"\n\
         \n\
         title = \"{title}\"\n\
         profile = \"{profile}\"\n\
         assurance_level = \"basic\"\n\
         \n\
         # [[implements]]\n\
         # ref = \"sas://WAR-SAS-RQ-000\"\n\
         # contribution = \"partial\"\n\
         \n"
    );
    for (ordinal, role, file) in template_atoms(profile) {
        out.push_str(&format!(
            "[[atoms]]\nordinal = {ordinal}\nrole = \"{role}\"\npath = \"atoms/{file}\"\nrequired = true\n\n"
        ));
    }
    out
}

/// The atoms a new Warrant starts with, by profile (§16.3).
fn template_atoms(profile: Profile) -> Vec<(u32, &'static str, &'static str)> {
    match profile {
        Profile::Delivery => vec![
            (10, "intent", "10-intent.md"),
            (20, "basis", "20-basis.md"),
            (40, "work_order", "40-work-order.md"),
            (45, "milestones", "45-milestones.yaml"),
            (60, "assurance", "60-assurance.md"),
        ],
        Profile::Decision => vec![
            (10, "intent", "10-intent.md"),
            (20, "basis", "20-basis.md"),
            (30, "adr", "30-decision.md"),
            (60, "assurance", "60-assurance.md"),
        ],
    }
}

fn write_atom_stubs(
    dir: &camino::Utf8Path,
    uuid: &WarUuid,
    profile: Profile,
) -> Result<(), RepoError> {
    for (ordinal, role, file) in template_atoms(profile) {
        let path = dir.join("atoms").join(file);
        if path.exists() {
            continue;
        }
        let body = if file.ends_with(".yaml") {
            String::from(
                "schema: \"oh.war/milestones/v1\"\n\n\
                 milestones:\n  \
                 - id: \"M1\"\n    title: \"\"\n    stage_refs: [\"STAGE-001\"]\n\n\
                 stages:\n  \
                 - id: \"STAGE-001\"\n    title: \"\"\n    executor_kind: \"human\"\n    responsibility_tier: \"T2\"\n",
            )
        } else {
            format!(
                "---\n\
                 schema: oh.war/atom/v1\n\
                 warrant_uuid: {uuid}\n\
                 role: {role}\n\
                 jurisdiction: authored\n\
                 order: {ordinal}\n\
                 classification: internal\n\
                 ---\n\n\
                 # {}\n\n\
                 TODO\n",
                heading(role)
            )
        };
        fs::write(&path, body).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    }
    Ok(())
}

fn heading(role: &str) -> &'static str {
    match role {
        "intent" => "Intent",
        "basis" => "Basis",
        "work_order" => "Work Order",
        "assurance" => "Assurance",
        "adr" => "Decision",
        _ => "Section",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::{Manifest, Namespace, RepositoryConfig};

    fn scratch(label: &str) -> Repository {
        let mut root = Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("temp dir is utf-8");
        root.push(format!("ow-new-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("scratch");
        let config = RepositoryConfig::new("Scratch", Namespace::parse("OW").expect("ns"));
        fs::write(
            root.join("openwarrant.toml"),
            toml::to_string_pretty(&config).expect("serialize"),
        )
        .expect("write config");
        Repository::open(root).expect("opens")
    }

    #[test]
    fn new_creates_a_valid_delivery_warrant() {
        let repo = scratch("delivery");
        let dir = run(&repo, "A first warrant", Profile::Delivery).expect("creates");
        assert!(dir.join("manifest.toml").is_file());

        let text = fs::read_to_string(dir.join("manifest.toml")).expect("read");
        let manifest: Manifest = toml::from_str(&text).expect("parses");
        let validated = manifest
            .validate(Some("OW"))
            .expect("the template must validate");
        assert_eq!(validated.alias.as_str(), "OW-WAR-0001");
        assert_eq!(validated.profile, Profile::Delivery);

        // Every declared atom stub must exist, or `war check` fails on a
        // freshly created Warrant — which would teach people to distrust it.
        for atom in &manifest.atoms {
            let path = dir.join(atom.path.as_deref().expect("path"));
            assert!(path.is_file(), "missing stub {path}");
        }

        let _ = fs::remove_dir_all(&repo.root);
    }

    #[test]
    fn new_creates_a_valid_decision_warrant() {
        let repo = scratch("decision");
        let dir = run(&repo, "A decision", Profile::Decision).expect("creates");
        let text = fs::read_to_string(dir.join("manifest.toml")).expect("read");
        let manifest: Manifest = toml::from_str(&text).expect("parses");
        let validated = manifest.validate(Some("OW")).expect("validates");
        assert_eq!(validated.profile, Profile::Decision);
        assert!(manifest.atoms.iter().any(|a| a.role == "adr"));
        let _ = fs::remove_dir_all(&repo.root);
    }

    #[test]
    fn ordinals_increment() {
        let repo = scratch("increment");
        let a = run(&repo, "First", Profile::Delivery).expect("creates");
        let b = run(&repo, "Second", Profile::Delivery).expect("creates");
        assert_eq!(a.file_name(), Some("OW-WAR-0001"));
        assert_eq!(b.file_name(), Some("OW-WAR-0002"));
        let _ = fs::remove_dir_all(&repo.root);
    }

    /// The allocation race. Threads all call `run` at once; every alias must be
    /// distinct. A scan-then-write implementation without `O_EXCL` fails here.
    #[test]
    fn concurrent_allocation_never_collides() {
        let repo = scratch("race");
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let repo = repo.clone();
                std::thread::spawn(move || run(&repo, &format!("Warrant {i}"), Profile::Delivery))
            })
            .collect();

        let mut aliases = std::collections::BTreeSet::new();
        for handle in threads {
            let dir = handle.join().expect("thread").expect("allocates");
            let alias = dir.file_name().expect("name").to_owned();
            assert!(aliases.insert(alias.clone()), "duplicate alias {alias}");
        }
        assert_eq!(aliases.len(), 8, "eight distinct aliases");

        let _ = fs::remove_dir_all(&repo.root);
    }

    #[test]
    fn an_empty_title_is_refused() {
        let repo = scratch("empty-title");
        assert!(run(&repo, "   ", Profile::Delivery).is_err());
        let _ = fs::remove_dir_all(&repo.root);
    }
}
