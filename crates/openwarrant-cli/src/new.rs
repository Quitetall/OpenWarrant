// SPDX-License-Identifier: Apache-2.0
//! `war new` — create a draft Warrant (SAS §71.2).

use std::fs::{self, OpenOptions};
use std::io::Write as _;

use camino::Utf8PathBuf;
use openwarrant_core::role::{CoreProfile, ProfileDefinition};
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
    run_profile(repo, title, default_preset(&profile), &profile, None)
}

/// The presets `war new --preset` offers (OW-ADR-0022), with the profile
/// each composes. `feature` and `fix` are `delivery`; `decision` is the ADR
/// profile.
pub const PRESETS: &[(&str, Profile)] = &[
    ("feature", Profile::Delivery),
    ("fix", Profile::Delivery),
    ("decision", Profile::Decision),
];

/// The preset a caller that names only a profile gets — `war plan --apply`,
/// `war init`'s first Warrant and the MCP `war_new`, which overwrite or fill
/// the atoms themselves. The `TODO` skeleton is retired: every draft starts
/// from a preset's questions.
#[must_use]
pub const fn default_preset(profile: &Profile) -> &'static str {
    match profile.core() {
        CoreProfile::Delivery => "feature",
        CoreProfile::Decision => "decision",
    }
}

/// The preset names, for a refusal to list.
#[must_use]
pub fn preset_names() -> Vec<&'static str> {
    PRESETS.iter().map(|(n, _)| *n).collect()
}

/// The profile a preset composes, or `None` for a name no preset has.
#[must_use]
pub fn preset_profile(name: &str) -> Option<Profile> {
    PRESETS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, p)| p.clone())
}

/// One preset atom's body, embedded at build time so a preset never drifts
/// from the binary that writes it.
fn preset_body(preset: &str, file: &str) -> Option<&'static str> {
    Some(match (preset, file) {
        ("feature", "10-intent.md") => include_str!("../templates/presets/feature/10-intent.md"),
        ("feature", "20-basis.md") => include_str!("../templates/presets/feature/20-basis.md"),
        ("feature", "40-work-order.md") => {
            include_str!("../templates/presets/feature/40-work-order.md")
        }
        ("feature", "45-milestones.yaml") => {
            include_str!("../templates/presets/feature/45-milestones.yaml")
        }
        ("feature", "60-assurance.md") => {
            include_str!("../templates/presets/feature/60-assurance.md")
        }
        ("fix", "10-intent.md") => include_str!("../templates/presets/fix/10-intent.md"),
        ("fix", "20-basis.md") => include_str!("../templates/presets/fix/20-basis.md"),
        ("fix", "40-work-order.md") => include_str!("../templates/presets/fix/40-work-order.md"),
        ("fix", "45-milestones.yaml") => {
            include_str!("../templates/presets/fix/45-milestones.yaml")
        }
        ("fix", "60-assurance.md") => include_str!("../templates/presets/fix/60-assurance.md"),
        ("decision", "10-intent.md") => {
            include_str!("../templates/presets/decision/10-intent.md")
        }
        ("decision", "20-basis.md") => include_str!("../templates/presets/decision/20-basis.md"),
        ("decision", "30-decision.md") => {
            include_str!("../templates/presets/decision/30-decision.md")
        }
        ("decision", "60-assurance.md") => {
            include_str!("../templates/presets/decision/60-assurance.md")
        }
        _ => return None,
    })
}

/// Create a new draft Warrant from a named preset and return its directory.
pub fn run_preset(repo: &Repository, title: &str, preset: &str) -> Result<Utf8PathBuf, RepoError> {
    let Some(profile) = preset_profile(preset) else {
        return Err(RepoError::Message(format!(
            "new.unknown-preset: no preset is named {preset:?}; the presets are {}",
            preset_names().join(", ")
        )));
    };
    run_profile(repo, title, preset, &profile, None)
}

/// Create a new draft of `profile` from a preset of its core profile
/// (OW-WAR-0140): the preset's atoms, then a stub for each namespaced role
/// the profile's definition requires, written from that definition. With
/// `parent`, the manifest cites it as [`run_with_parent`] does.
pub fn run_profile(
    repo: &Repository,
    title: &str,
    preset: &str,
    profile: &Profile,
    parent: Option<&str>,
) -> Result<Utf8PathBuf, RepoError> {
    let Some(composes) = preset_profile(preset) else {
        return Err(RepoError::Message(format!(
            "new.unknown-preset: no preset is named {preset:?}; the presets are {}",
            preset_names().join(", ")
        )));
    };
    if composes.core() != profile.core() {
        return Err(RepoError::Message(format!(
            "new.preset-profile: preset {preset:?} composes the `{composes}` profile, not `{profile}`"
        )));
    }
    let Some(definition) = repo.profiles.definition(profile) else {
        return Err(RepoError::Message(format!(
            "new.unknown-profile: this repository's registry does not define `{profile}`; \
             the profiles are {}",
            repo.profiles.names().join(", ")
        )));
    };
    // Read BEFORE anything is created, so a bad parent leaves no directory.
    let citation = match parent {
        Some(parent) => Some(parent_citation(repo, parent)?),
        None => None,
    };
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
                let mut manifest = manifest_template(&uuid, &alias, title, profile, definition);
                if let Some(citation) = &citation {
                    manifest.push_str(citation);
                }
                file.write_all(manifest.as_bytes())
                    .map_err(|source| RepoError::Io {
                        context: format!("could not write {manifest_path}"),
                        source,
                    })?;
                write_preset(&dir, &uuid, profile, preset)?;
                write_profile_stubs(&dir, &uuid, definition)?;
                // §66.4 `draft.created` — the first journal entry, written by
                // the command that created the draft.
                crate::journal_cmd::record(
                    &dir,
                    &uuid.to_string(),
                    crate::journal_cmd::DRAFT_CREATED,
                    &format!("agent://{}", repo.performer()),
                    &format!(
                        "{{\"alias\":\"{alias}\",\"profile\":\"{profile}\",\"preset\":\"{preset}\"}}"
                    ),
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

/// `war new <title> --parent <alias>` (SAS §71.2, OW-WAR-0123): a new draft
/// whose manifest cites its parent exactly — the parent's `war://` identity,
/// its latest authorized revision and that revision's digest, read from the
/// parent's `authorization.toml` (§20.2, RQ-023).
///
/// A citation typed by hand is how OW-WAR-0002 to 0005 came to say "revision
/// 1" at revision 2's digest; the tool writes both from the one record. The
/// parent is read BEFORE anything is created, so an unknown alias or a parent
/// with no authorized revision leaves no directory behind (U-002: a draft has
/// no revision the child could rest on).
pub fn run_with_parent(
    repo: &Repository,
    title: &str,
    preset: &str,
    parent: &str,
) -> Result<Utf8PathBuf, RepoError> {
    let Some(profile) = preset_profile(preset) else {
        return Err(RepoError::Message(format!(
            "new.unknown-preset: no preset is named {preset:?}; the presets are {}",
            preset_names().join(", ")
        )));
    };
    run_profile(repo, title, preset, &profile, Some(parent))
}

/// The `[[parents]]` table citing `alias` at its latest authorized revision.
fn parent_citation(repo: &Repository, alias: &str) -> Result<String, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let loaded = repo.load_warrant(&dir)?;
    let Some(validated) = loaded.validated.as_ref() else {
        return Err(RepoError::Message(format!(
            "--parent {alias}: its manifest does not validate, so it has no identity to cite; \
             nothing was created"
        )));
    };
    let Some(authorization) = repo.load_authorization(&dir)? else {
        return Err(RepoError::Message(format!(
            "--parent {alias}: {alias} has no authorized revision, so there is no exact \
             revision to cite (§20.2); authorize it first. Nothing was created"
        )));
    };
    if authorization.revision.state != openwarrant_core::contract::RevisionState::Authorized {
        return Err(RepoError::Message(format!(
            "--parent {alias}: its authorization record is not in the authorized state; \
             nothing was created"
        )));
    }
    Ok(format!(
        "# The exact parent revision this child rests on (SAS §20.2, RQ-023), written\n\
         # by `war new --parent {alias}` from its authorization.toml.\n\
         [[parents]]\n\
         ref = \"war://{uuid}\"\n\
         contract_revision = {revision}\n\
         contract_digest = \"sha256:{digest}\"\n",
        uuid = validated.uuid,
        revision = authorization.revision.revision,
        digest = authorization.revision.contract_digest,
    ))
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

fn manifest_template(
    uuid: &WarUuid,
    alias: &str,
    title: &str,
    profile: &Profile,
    definition: &ProfileDefinition,
) -> String {
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
         {pin}\
         assurance_level = \"basic\"\n\
         \n\
         # [[implements]]\n\
         # ref = \"sas://WAR-SAS-RQ-000\"\n\
         # contribution = \"partial\"\n\
         \n",
        // OW-ADR-0031: the profile file this draft is composed against. The
        // manifest's bytes are inside the contract digest, so the signature
        // covers the type; `war check` reports `profile.pin-drift` when the
        // file moves. A program with no profile file has nothing to pin.
        pin = definition
            .digest
            .as_deref()
            .map(|d| {
                format!(
                    "# The profile file this Warrant is composed against (OW-ADR-0031).\n\
                     profile_digest = \"{d}\"\n"
                )
            })
            .unwrap_or_default()
    );
    for (ordinal, role, file) in template_atoms(profile) {
        out.push_str(&format!(
            "[[atoms]]\nordinal = {ordinal}\nrole = \"{role}\"\npath = \"atoms/{file}\"\nrequired = true\n\n"
        ));
    }
    // The namespaced roles the profile requires, in ordinal order after the
    // core atoms (§16.4): composition entries like any other, and nothing
    // in the core learns their names.
    let mut extensions: Vec<_> = definition.required_extension_roles.iter().collect();
    extensions.sort_by_key(|r| r.ordinal);
    for required in extensions {
        out.push_str(&format!(
            "[[atoms]]\nordinal = {}\nrole = \"{}\"\npath = \"atoms/{}\"\nrequired = true\n\n",
            required.ordinal, required.role, required.file
        ));
    }
    out
}

/// The atoms a new Warrant starts with, by profile (§16.3).
fn template_atoms(profile: &Profile) -> Vec<(u32, &'static str, &'static str)> {
    match profile.core() {
        CoreProfile::Delivery => vec![
            (10, "intent", "10-intent.md"),
            (20, "basis", "20-basis.md"),
            (40, "work_order", "40-work-order.md"),
            (45, "milestones", "45-milestones.yaml"),
            (60, "assurance", "60-assurance.md"),
        ],
        CoreProfile::Decision => vec![
            (10, "intent", "10-intent.md"),
            (20, "basis", "20-basis.md"),
            (30, "adr", "30-decision.md"),
            (60, "assurance", "60-assurance.md"),
        ],
    }
}

/// Write each atom the profile requires from the preset: the frontmatter
/// this Warrant needs, then the preset's headings and the question each one
/// asks. The answers are the author's; `war check` names a required heading
/// left unanswered (`atom.preset-unanswered`).
fn write_preset(
    dir: &camino::Utf8Path,
    uuid: &WarUuid,
    profile: &Profile,
    preset: &str,
) -> Result<(), RepoError> {
    for (ordinal, role, file) in template_atoms(profile) {
        let path = dir.join("atoms").join(file);
        if path.exists() {
            continue;
        }
        let Some(body) = preset_body(preset, file) else {
            return Err(RepoError::Message(format!(
                "new.preset-incomplete: preset {preset:?} has no {file} for role `{role}`"
            )));
        };
        let text = if file.ends_with(".yaml") {
            body.to_owned()
        } else {
            format!(
                "---\n\
                 schema: oh.war/atom/v1\n\
                 warrant_uuid: {uuid}\n\
                 role: {role}\n\
                 jurisdiction: authored\n\
                 order: {ordinal}\n\
                 classification: internal\n\
                 ---\n\n{body}"
            )
        };
        fs::write(&path, text).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    }
    Ok(())
}

/// A stub for each namespaced role the profile requires, from its
/// definition's own text: the stub is data, so a new profile brings its own.
fn write_profile_stubs(
    dir: &camino::Utf8Path,
    uuid: &WarUuid,
    definition: &ProfileDefinition,
) -> Result<(), RepoError> {
    for required in &definition.required_extension_roles {
        let path = dir.join("atoms").join(&required.file);
        if path.exists() {
            continue;
        }
        let text = format!(
            "---\n\
             schema: oh.war/atom/v1\n\
             warrant_uuid: {uuid}\n\
             role: {role}\n\
             jurisdiction: authored\n\
             order: {ordinal}\n\
             classification: internal\n\
             ---\n\n{body}",
            role = required.role,
            ordinal = required.ordinal,
            body = required.stub,
        );
        fs::write(&path, text).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// §12 / OW-WAR-0029 OBL-001 — allocating an alias never changes the
    /// identity: the UUID minted for the draft is the UUID in its manifest.
    #[test]
    fn allocation_preserves_the_uuid() {
        let uuid = WarUuid::mint();
        let registry = openwarrant_core::role::ProfileRegistry::builtin();
        let definition = registry.definition(&Profile::Delivery).expect("core");
        let m = manifest_template(&uuid, "OW-WAR-0001", "x", &Profile::Delivery, definition);
        assert!(m.contains(&format!("uuid = \"{uuid}\"")));
        assert!(m.starts_with("# A Warrant is the contract for ONE bounded intervention"));
    }

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

    /// OW-WAR-0140: a profile defined only as data in `profiles/` gets its
    /// core preset's atoms plus a stub for each role it requires, and the
    /// draft validates against the repository's registry — no code names it.
    #[test]
    fn new_writes_a_program_profiles_stubs() {
        let repo = scratch("program-profile");
        fs::create_dir_all(repo.root.join("profiles")).expect("profiles/");
        fs::write(
            repo.root.join("profiles/lab.toml"),
            "schema = \"oh.war/profile/v1\"\nname = \"lab\"\nextends = \"delivery\"\n\
             approved = false\n\n[[requires]]\nrole = \"lab.protocol\"\nordinal = 71\n\
             file = \"71-lab-protocol.md\"\nstub = \"# Protocol\\n\\nNot written yet.\\n\"\n",
        )
        .expect("write lab");
        let repo = Repository::open(repo.root.clone()).expect("reopens with the profile");
        let lab = repo.profiles.resolve("lab").expect("admitted");
        let dir = run(&repo, "A lab warrant", lab.clone()).expect("creates");
        let text = fs::read_to_string(dir.join("manifest.toml")).expect("read");
        let manifest: Manifest = toml::from_str(&text).expect("parses");
        let validated = manifest
            .validate_in(Some("OW"), &repo.profiles)
            .expect("validates in the registry");
        assert_eq!(validated.profile, lab);
        assert!(
            manifest.validate(Some("OW")).is_err(),
            "unknown to the built-in registry"
        );
        let stub = fs::read_to_string(dir.join("atoms/71-lab-protocol.md")).expect("stub");
        assert!(stub.contains("role: lab.protocol") && stub.contains("Not written yet."));
        assert!(
            dir.join("atoms/40-work-order.md").is_file(),
            "the feature preset's atoms"
        );
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

    /// OW-WAR-0123 OBL-005's refusals: a parent that does not exist, and a
    /// parent with no authorized revision, create nothing.
    #[test]
    fn a_parent_without_an_authorized_revision_is_refused_and_nothing_is_created() {
        let repo = scratch("parent-refused");
        let draft = run(&repo, "A draft parent", Profile::Delivery).expect("creates");
        assert!(!draft.join("authorization.toml").exists());
        for parent in ["OW-WAR-9999", "OW-WAR-0001"] {
            let err = run_with_parent(&repo, "A child", "feature", parent).expect_err("refused");
            assert!(
                !repo.warrants_dir().join("OW-WAR-0002").exists(),
                "{parent}: {err}"
            );
        }
        let message = run_with_parent(&repo, "A child", "feature", "OW-WAR-0001")
            .expect_err("refused")
            .to_string();
        assert!(message.contains("has no authorized revision"), "{message}");
        let _ = fs::remove_dir_all(&repo.root);
    }

    #[test]
    fn every_preset_has_every_atom_its_profile_requires_and_no_todo() {
        for (name, profile) in PRESETS {
            for (_, _, file) in template_atoms(profile) {
                let body =
                    preset_body(name, file).unwrap_or_else(|| panic!("preset {name} lacks {file}"));
                assert!(!body.contains("TODO"), "{name}/{file} carries TODO");
                if file.ends_with(".md") {
                    assert!(
                        body.contains("<!-- required -->"),
                        "{name}/{file} asks nothing"
                    );
                }
            }
        }
    }

    #[test]
    fn a_preset_draft_is_typed_and_an_unknown_preset_is_refused() {
        let repo = scratch("preset");
        let dir = run_preset(&repo, "A fix", "fix").expect("creates");
        let intent = fs::read_to_string(dir.join("atoms/10-intent.md")).expect("intent");
        assert!(intent.contains("role: intent"));
        assert!(intent.contains("## Problem\n<!-- required -->"));
        let err = run_preset(&repo, "x", "nonsense").unwrap_err().to_string();
        assert!(err.contains("new.unknown-preset") && err.contains("feature, fix, decision"));
        let _ = fs::remove_dir_all(&repo.root);
    }

    #[test]
    fn an_empty_title_is_refused() {
        let repo = scratch("empty-title");
        assert!(run(&repo, "   ", Profile::Delivery).is_err());
        let _ = fs::remove_dir_all(&repo.root);
    }
}
