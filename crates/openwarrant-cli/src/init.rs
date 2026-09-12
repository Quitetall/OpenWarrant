// SPDX-License-Identifier: Apache-2.0
//! `war init` — initialize repository configuration and directories (SAS §71.1).

use std::fmt;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::{Namespace, RepositoryConfig};

/// The repository configuration file name (§60).
pub const CONFIG_FILE: &str = "openwarrant.toml";

#[derive(Debug)]
pub enum InitError {
    Namespace(openwarrant_core::ConfigError),
    AlreadyInitialized {
        path: Utf8PathBuf,
    },
    RootMissing {
        path: Utf8PathBuf,
    },
    NonUtf8Path,
    Io {
        context: String,
        source: std::io::Error,
    },
    Serialize(toml::ser::Error),
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(source) => write!(f, "{source}"),
            Self::AlreadyInitialized { path } => write!(
                f,
                "{path} already exists; refusing to overwrite an initialized repository. \
                 Delete it deliberately if you intend to reinitialize."
            ),
            Self::RootMissing { path } => write!(f, "repository root {path} does not exist"),
            Self::NonUtf8Path => write!(f, "the current directory is not valid UTF-8"),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Serialize(source) => write!(f, "could not serialize configuration: {source}"),
        }
    }
}

impl std::error::Error for InitError {}

pub fn run(
    namespace: &str,
    name: Option<&str>,
    root: Option<Utf8PathBuf>,
) -> Result<(), InitError> {
    let root = match root {
        Some(path) => path,
        None => {
            let cwd = std::env::current_dir().map_err(|source| InitError::Io {
                context: "could not read the current directory".to_owned(),
                source,
            })?;
            Utf8PathBuf::from_path_buf(cwd).map_err(|_| InitError::NonUtf8Path)?
        }
    };

    if !root.is_dir() {
        return Err(InitError::RootMissing { path: root });
    }

    let config_path = root.join(CONFIG_FILE);
    // Refuse rather than clobber. A repository's namespace is baked into every
    // local alias already written; silently rewriting it would orphan them.
    if config_path.exists() {
        return Err(InitError::AlreadyInitialized { path: config_path });
    }

    let namespace = Namespace::parse(namespace).map_err(InitError::Namespace)?;

    let project_name = match name {
        Some(name) => name.to_owned(),
        None => root.file_name().unwrap_or("openwarrant").to_owned(),
    };

    let config = RepositoryConfig::new(project_name, namespace);
    // Validate what we are about to write. `war init` producing a file that
    // `war check` would reject is the kind of inconsistency that teaches people
    // to distrust the tool.
    config.validate().map_err(InitError::Namespace)?;

    let rendered = toml::to_string_pretty(&config).map_err(InitError::Serialize)?;
    fs::write(&config_path, rendered).map_err(|source| InitError::Io {
        context: format!("could not write {config_path}"),
        source,
    })?;

    for dir in [
        &config.paths.sas,
        &config.paths.roadmap,
        &config.paths.adrs,
        &config.paths.warrants,
    ] {
        let path = root.join(Utf8Path::new(dir));
        fs::create_dir_all(&path).map_err(|source| InitError::Io {
            context: format!("could not create {path}"),
            source,
        })?;
    }

    // An adopter's agents read AGENTS.md before their first Warrant. Written
    // once, never over an existing one: a repository may have tuned its copy.
    write_agents_md(&root, config.project.namespace.as_str(), false)?;

    // §76.3: silence on sound state is the ideal, but `init` is a mutation and
    // the operator needs to know what was created and where.
    println!("initialized {} ({})", config.project.name, config_path);
    Ok(())
}

/// `war init --program`: everything `run` writes, plus a SAS the tool can read,
/// the authority examples, the repository's own gate, and a first Warrant
/// whose atoms are real. `war check` on the result exits 0, and `war sas
/// propose 0.1.0` records the SAS — asserted by a test, because a scaffold
/// the tool refuses would teach an adopter to distrust the tool on day one.
pub fn run_program(
    program: &str,
    namespace: &str,
    root: Option<Utf8PathBuf>,
) -> Result<Utf8PathBuf, InitError> {
    // §106 rows are `<PREFIX>-SAS-RQ-NNN` and the prefix must be letters;
    // a namespace with a digit or dash would make every ref unparseable.
    let invalid = |context: String| InitError::Io {
        context,
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "refused"),
    };
    if namespace.is_empty() || !namespace.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(invalid(format!(
            "--program needs a namespace of uppercase ASCII letters only, A–Z (it prefixes \
             `{namespace}-SAS-RQ-001`, and a §106 row's prefix must be letters); got {namespace:?}"
        )));
    }
    // The name lands in headings and table cells verbatim; a newline would
    // break both, and an empty name produces a heading with nothing in it.
    if program.trim().is_empty() || program.contains(['\n', '\r', '|']) {
        return Err(invalid(format!(
            "--program needs a non-empty name without a newline or `|`; got {program:?}"
        )));
    }
    run(namespace, Some(program), root.clone())?;
    let root = match root {
        Some(r) => r,
        None => {
            Utf8PathBuf::from_path_buf(std::env::current_dir().map_err(|source| InitError::Io {
                context: "could not read the current directory".to_owned(),
                source,
            })?)
            .map_err(|_| InitError::NonUtf8Path)?
        }
    };
    let io = |context: String| move |source: std::io::Error| InitError::Io { context, source };
    let program_file: String = program
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let fill = |t: &str, alias: &str| {
        t.replace("{{program}}", program)
            .replace("{{program_file}}", &program_file)
            .replace("{{namespace}}", namespace)
            .replace("{{alias}}", alias)
    };

    // The SAS: the three sections the tool reads, with everything else to write.
    let sas_dir = root.join("docs/sas");
    fs::create_dir_all(sas_dir.join("revisions"))
        .map_err(io(format!("could not create {sas_dir}")))?;
    let sas_path = sas_dir.join(format!("{program_file}_SAS.md"));
    fs::write(&sas_path, fill(PROGRAM_SAS_TEMPLATE, ""))
        .map_err(io(format!("could not write {sas_path}")))?;

    // Authority: the examples, never the real files — those are a human's.
    let auth = root.join("docs/authority");
    fs::create_dir_all(auth.join("responses")).map_err(io(format!("could not create {auth}")))?;
    for (name, text) in [
        ("roles.toml.example", ROLES_EXAMPLE),
        ("allowed_signers.example", ALLOWED_SIGNERS_EXAMPLE),
    ] {
        let p = auth.join(name);
        fs::write(&p, text).map_err(io(format!("could not write {p}")))?;
    }

    // The one gate every adopter has: `war check` itself, from PATH.
    let gates = root.join("docs/gates");
    fs::create_dir_all(&gates).map_err(io(format!("could not create {gates}")))?;
    fs::create_dir_all(root.join("docs/receipts"))
        .map_err(io("could not create docs/receipts".to_owned()))?;
    let gate_path = gates.join("software.repo.war-check@1.0.0.yaml");
    fs::write(
        &gate_path,
        WAR_CHECK_GATE
            .replace(
                r#"argv: ["./target/debug/war", "check", "--generated"]"#,
                r#"argv: ["war", "check", "--generated"]"#,
            )
            .replace(
                "conformance/plant.sh, run by cargo xtask gate",
                "OpenWarrant's conformance battery, at the release this binary was built from",
            ),
    )
    .map_err(io(format!("could not write {gate_path}")))?;

    // The first Warrant, through `war new` and then real atoms.
    let repo =
        crate::repo::Repository::discover(Some(root.clone())).map_err(|e| InitError::Io {
            context: format!("could not open the repository just initialized: {e}"),
            source: std::io::Error::other(e.to_string()),
        })?;
    let dir = crate::new::run(
        &repo,
        &format!("Adopt OpenWarrant in {program}"),
        openwarrant_core::Profile::Delivery,
    )
    .map_err(|e| InitError::Io {
        context: format!("could not create the first Warrant: {e}"),
        source: std::io::Error::other(e.to_string()),
    })?;
    let alias = dir
        .file_name()
        .ok_or_else(|| invalid(format!("{dir}: the Warrant directory has no name")))?
        .to_owned();
    let manifest_path = dir.join("manifest.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(io(format!("could not read {manifest_path}")))?;
    let uuid = toml::from_str::<toml::Value>(&manifest)
        .ok()
        .and_then(|v| {
            v.get("uuid")
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
        })
        .ok_or_else(|| {
            invalid(format!(
                "{manifest_path}: no `uuid` in the manifest `war new` wrote"
            ))
        })?;
    let frontmatter = |role: &str, ordinal: u32| {
        format!(
            "---\nschema: oh.war/atom/v1\nwarrant_uuid: {uuid}\nrole: {role}\njurisdiction: authored\norder: {ordinal}\nclassification: internal\n---\n\n"
        )
    };
    for (file, role, ordinal, body) in [
        ("10-intent.md", "intent", 10, ADOPT_INTENT),
        ("20-basis.md", "basis", 20, ADOPT_BASIS),
        ("40-work-order.md", "work_order", 40, ADOPT_WORK_ORDER),
        ("60-assurance.md", "assurance", 60, ADOPT_ASSURANCE),
    ] {
        let p = dir.join("atoms").join(file);
        fs::write(&p, frontmatter(role, ordinal) + &fill(body, &alias))
            .map_err(io(format!("could not write {p}")))?;
    }
    let p = dir.join("atoms/45-milestones.yaml");
    fs::write(&p, fill(ADOPT_MILESTONES, &alias)).map_err(io(format!("could not write {p}")))?;
    // Traceability into the SAS just written: the first row, the first phase.
    let refs = format!(
        "\n[[implements]]\nref = \"sas://{namespace}-SAS-RQ-001\"\ncontribution = \"complete\"\n\n[[roadmap]]\nref = \"roadmap://{namespace}-PHASE-1/adopt\"\n"
    );
    fs::write(&manifest_path, manifest + &refs)
        .map_err(io(format!("could not write {manifest_path}")))?;

    println!("scaffolded {program}: {sas_path}, docs/authority/*.example, {gate_path}, {alias}");
    println!(
        "next: edit the SAS, `war sas propose 0.1.0`, then a human signs it — `war next` says the rest"
    );
    Ok(root)
}

const PROGRAM_SAS_TEMPLATE: &str = include_str!("../templates/PROGRAM_SAS.md.tmpl");
const ROLES_EXAMPLE: &str = include_str!("../../../docs/authority/roles.toml.example");
const ALLOWED_SIGNERS_EXAMPLE: &str =
    include_str!("../../../docs/authority/allowed_signers.example");
const WAR_CHECK_GATE: &str = include_str!("../../../docs/gates/software.repo.war-check@1.0.0.yaml");
const ADOPT_INTENT: &str = include_str!("../templates/adopt/10-intent.md");
const ADOPT_BASIS: &str = include_str!("../templates/adopt/20-basis.md");
const ADOPT_WORK_ORDER: &str = include_str!("../templates/adopt/40-work-order.md");
const ADOPT_MILESTONES: &str = include_str!("../templates/adopt/45-milestones.yaml");
const ADOPT_ASSURANCE: &str = include_str!("../templates/adopt/60-assurance.md");

/// The agent instructions this repository ships, parameterised by namespace.
///
/// One source: this template is the file, and the repository's own `AGENTS.md`
/// is asserted byte-identical to its rendering for `OW` by a test, so the
/// rules an adopter's agents get are the rules this repository's agents get.
pub const AGENTS_MD_TEMPLATE: &str = include_str!("../templates/AGENTS.md.tmpl");

#[must_use]
pub fn render_agents_md(namespace: &str) -> String {
    AGENTS_MD_TEMPLATE.replace("{{namespace}}", namespace)
}

/// Write `AGENTS.md` at the root. Refuses to overwrite unless `force`: an
/// adopter may have edited theirs, and silently replacing it would be the
/// "change a document to make a tool happy" failure the file itself forbids.
/// Returns whether a file was written.
pub fn write_agents_md(root: &Utf8Path, namespace: &str, force: bool) -> Result<bool, InitError> {
    let path = root.join("AGENTS.md");
    if path.exists() && !force {
        return Ok(false);
    }
    fs::write(&path, render_agents_md(namespace)).map_err(|source| InitError::Io {
        context: format!("could not write {path}"),
        source,
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> Utf8PathBuf {
        let mut path = Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("temp dir is utf-8");
        path.push(format!("openwarrant-init-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("scratch dir");
        path
    }

    #[test]
    fn init_writes_config_and_directories() {
        let root = scratch("ok");
        run("OW", Some("OpenWarrant"), Some(root.clone())).expect("init succeeds");

        let config_path = root.join(CONFIG_FILE);
        assert!(config_path.exists(), "config file written");

        let text = fs::read_to_string(&config_path).expect("readable");
        let parsed: RepositoryConfig = toml::from_str(&text).expect("round trips");
        assert_eq!(parsed.project.namespace.as_str(), "OW");
        assert_eq!(parsed.project.name, "OpenWarrant");
        assert_eq!(parsed.validate(), Ok(()));

        for dir in ["docs/sas", "docs/roadmap", "docs/adr", "docs/warrants"] {
            assert!(root.join(dir).is_dir(), "{dir} created");
        }

        let _ = fs::remove_dir_all(&root);
    }

    /// Reinitializing must refuse. Rewriting the namespace would orphan every
    /// alias already minted under the old one.
    #[test]
    fn init_refuses_to_overwrite() {
        let root = scratch("twice");
        run("OW", None, Some(root.clone())).expect("first init succeeds");
        let err = run("XX", None, Some(root.clone())).expect_err("second init refuses");
        assert!(
            matches!(err, InitError::AlreadyInitialized { .. }),
            "expected AlreadyInitialized, got {err:?}"
        );

        // And the original namespace survived the refused call.
        let text = fs::read_to_string(root.join(CONFIG_FILE)).expect("readable");
        let parsed: RepositoryConfig = toml::from_str(&text).expect("parses");
        assert_eq!(parsed.project.namespace.as_str(), "OW");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn init_refuses_a_malformed_namespace() {
        let root = scratch("badns");
        let err = run("lowercase", None, Some(root.clone())).expect_err("refuses");
        assert!(matches!(err, InitError::Namespace(_)), "got {err:?}");
        assert!(
            !root.join(CONFIG_FILE).exists(),
            "nothing written on a refused init"
        );
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod program_tests {
    use super::*;

    /// The scaffold passes the tool that will judge it: `war check` exits 0
    /// (warnings only — no independence, no SAS revision yet), `war sas
    /// propose 0.1.0` records the SAS with its three §106 rows, and after
    /// that `sas.unrecorded` is gone.
    #[test]
    fn a_scaffolded_program_passes_check_and_proposes_its_sas() {
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-init-program-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        run_program("Demo Program", "DM", Some(root.clone())).expect("scaffolds");
        let repo = crate::repo::Repository::discover(Some(root.clone())).unwrap();
        let report = crate::check::run(&repo, None, false).unwrap();
        assert!(
            report.is_ready(),
            "the scaffold must pass war check: {:?}",
            report
                .diagnostics
                .iter()
                .filter(|d| !matches!(d.severity, crate::diagnostic::Severity::Pass))
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.rule == "sas.unrecorded")
        );
        let proposed = crate::sas::propose(&repo, "0.1.0").unwrap();
        assert!(proposed.is_ready(), "{proposed:?}");
        assert!(root.join("docs/sas/revisions/0.1.0.toml").is_file());
        let after = crate::check::run(&repo, None, false).unwrap();
        assert!(after.is_ready());
        assert!(!after.diagnostics.iter().any(|d| d.rule == "sas.unrecorded"));
        // The first Warrant traces into the SAS just written.
        let manifest =
            fs::read_to_string(root.join("docs/warrants/DM-WAR-0001/manifest.toml")).unwrap();
        assert!(manifest.contains("sas://DM-SAS-RQ-001"));
        assert!(manifest.contains("roadmap://DM-PHASE-1/adopt"));
        // A namespace with a digit cannot prefix a §106 row: refused by name.
        let bad = root.join("bad");
        fs::create_dir_all(&bad).unwrap();
        let err = run_program("X", "D1", Some(bad.clone()))
            .unwrap_err()
            .to_string();
        assert!(err.contains("uppercase ASCII letters only"), "{err}");
        let err = run_program("two\nlines", "DM", Some(bad))
            .unwrap_err()
            .to_string();
        assert!(err.contains("without a newline"), "{err}");
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod agents_md_tests {
    use super::*;

    #[test]
    fn the_template_renders_for_a_namespace_with_no_placeholder_left() {
        let out = render_agents_md("XX");
        assert!(out.contains("XX-WAR-NNNN"));
        assert!(!out.contains("{{"), "unrendered placeholder");
        assert!(out.contains("war sign"), "the loop names the human's act");
        assert!(out.contains("Never verify your own work"));
    }

    /// One source of rules: this repository's own AGENTS.md is the template
    /// rendered for `OW`. If they differ, the adopter and this repository are
    /// being told different things.
    #[test]
    fn the_repositorys_agents_md_is_the_rendered_template() {
        let repo = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let committed = std::fs::read_to_string(repo.join("AGENTS.md")).expect("AGENTS.md");
        assert_eq!(
            committed,
            render_agents_md("OW"),
            "run `war agents-md --force`"
        );
    }

    #[test]
    fn init_writes_agents_md_once_and_force_rewrites_it() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-init-agents-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        run("ZZ", Some("demo"), Some(root.clone())).expect("init");
        let path = root.join("AGENTS.md");
        assert!(path.is_file());
        std::fs::write(&path, "tuned by the adopter\n").unwrap();
        assert!(
            !write_agents_md(&root, "ZZ", false).unwrap(),
            "must not clobber"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "tuned by the adopter\n"
        );
        assert!(write_agents_md(&root, "ZZ", true).unwrap());
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("ZZ-WAR-NNNN")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
