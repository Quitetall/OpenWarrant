// SPDX-License-Identifier: Apache-2.0
//! `war init` — initialize repository configuration and directories (SAS §71.1).

use std::fmt;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};

pub mod guided;
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

const PROGRAM_SAS_TEMPLATE: &str = include_str!("../../templates/PROGRAM_SAS.md.tmpl");
const ROLES_EXAMPLE: &str = include_str!("../../../../docs/authority/roles.toml.example");
const ALLOWED_SIGNERS_EXAMPLE: &str =
    include_str!("../../../../docs/authority/allowed_signers.example");
const WAR_CHECK_GATE: &str =
    include_str!("../../../../docs/gates/software.repo.war-check@1.0.0.yaml");
const ADOPT_INTENT: &str = include_str!("../../templates/adopt/10-intent.md");
const ADOPT_BASIS: &str = include_str!("../../templates/adopt/20-basis.md");
const ADOPT_WORK_ORDER: &str = include_str!("../../templates/adopt/40-work-order.md");
const ADOPT_MILESTONES: &str = include_str!("../../templates/adopt/45-milestones.yaml");
const ADOPT_ASSURANCE: &str = include_str!("../../templates/adopt/60-assurance.md");

/// The agent instructions this repository ships, parameterised by namespace.
///
/// This legacy template is also the repository's linked workflow reference.
/// Root `AGENTS.md` adds project-specific routing and successor design guidance;
/// installing an additive context pointer is a separate, planned operation.
pub const AGENTS_MD_TEMPLATE: &str = include_str!("../../templates/AGENTS.md.tmpl");

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

    /// Keep the shipped legacy workflow equal to the linked reference, while
    /// allowing the root instructions to retain project-specific context.
    #[test]
    fn the_legacy_workflow_reference_is_the_rendered_template() {
        let repo = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let reference = "docs/agents/legacy-warrant-workflow.md";
        let committed = std::fs::read_to_string(repo.join(reference)).expect(reference);
        assert_eq!(committed, render_agents_md("OW"));
        let instructions = std::fs::read_to_string(repo.join("AGENTS.md")).expect("AGENTS.md");
        assert!(
            instructions.contains(reference),
            "root must route legacy work"
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

// ---------------------------------------------------------------------------
// `war init` as a conversation — the line front end (OW-WAR-0112 M4).
// ---------------------------------------------------------------------------

/// `war init` with no `--namespace`, at a terminal. Drives `guided::Machine`
/// with what the human types; applies each `Effect`; re-reads the tree.
///
/// The invariants the machine cannot hold are held here: the terminal gate
/// (`sign::at_a_terminal()`, checked by the caller and again before any
/// write), and write-once — an authority file that exists is never touched,
/// whatever was answered.
pub fn guided(root: Option<Utf8PathBuf>, program_hint: Option<&str>) -> Result<(), InitError> {
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
    if !root.is_dir() {
        return Err(InitError::RootMissing { path: root });
    }
    let io = |context: String| move |source: std::io::Error| InitError::Io { context, source };
    let now = {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        crate::gate_cmd::receipt::rfc3339_from_secs(secs)
    };
    let mut m = guided::Machine::new(guided::Facts::read(&root), now);
    let resumed = m.step != guided::Step::Program;
    println!("war init — setting up {}", root);
    if resumed {
        println!("resuming at: {}", m.step.title());
    }
    loop {
        use guided::{Answer, Effect, Step};
        println!();
        println!("{}", m.question());
        if m.step == Step::Done {
            return Ok(());
        }
        let answer = match m.step {
            Step::Program => {
                let default = program_hint
                    .map(str::to_owned)
                    .or_else(|| root.file_name().map(str::to_owned))
                    .unwrap_or_default();
                let Some(name) = ask(&format!("  program name [{default}]: "))? else {
                    return Ok(());
                };
                let name = if name.is_empty() { default } else { name };
                let Some(namespace) = ask("  namespace (A–Z): ")? else {
                    return Ok(());
                };
                Answer::Program {
                    name,
                    namespace: namespace.to_ascii_uppercase(),
                }
            }
            Step::Signer => {
                let git_name = git_user_name();
                let Some(name) = ask(&format!(
                    "  your name [{}]: ",
                    git_name.as_deref().unwrap_or("")
                ))?
                else {
                    return Ok(());
                };
                let name = if name.is_empty() {
                    git_name.unwrap_or_default()
                } else {
                    name
                };
                let default_principal: String = name
                    .to_ascii_lowercase()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                    .collect();
                let Some(principal) =
                    ask(&format!("  principal, no spaces [{default_principal}]: "))?
                else {
                    return Ok(());
                };
                let principal = if principal.is_empty() {
                    default_principal
                } else {
                    principal
                };
                let keys = loaded_keys();
                if keys.is_empty() {
                    println!(
                        "  (`ssh-add -L` lists no keys; paste one as `<keytype> <base64> [comment]`)"
                    );
                } else {
                    for (i, k) in keys.iter().enumerate() {
                        println!("  [{}] {}", i + 1, shorten(k));
                    }
                }
                let Some(pick) = ask("  which key (number, or paste a line): ")? else {
                    return Ok(());
                };
                let key = match pick.parse::<usize>() {
                    Ok(n) if n >= 1 && n <= keys.len() => keys[n - 1].clone(),
                    _ => pick,
                };
                Answer::Signer {
                    name,
                    principal,
                    key,
                }
            }
            Step::KeyLoaded => {
                let Some(a) = ask("  loaded with -c? [y/N]: ")? else {
                    return Ok(());
                };
                Answer::KeyLoaded(yes(&a))
            }
            Step::Sas | Step::Authorize => {
                let Some(a) = ask("  go ahead? [Y/n]: ")? else {
                    return Ok(());
                };
                if !a.is_empty() && !yes(&a) {
                    println!("stopped; run `war init` again to resume here");
                    return Ok(());
                }
                Answer::Proceed
            }
            Step::SignSas | Step::SignAuthorize => {
                let Some(a) = ask("  sign now? [y/N]: ")? else {
                    return Ok(());
                };
                Answer::SignNow(yes(&a))
            }
            Step::Done => unreachable!("handled above"),
        };
        let effects = match m.answer(answer) {
            Ok(e) => e,
            Err(why) => {
                println!("  refused: {why}");
                continue;
            }
        };
        for effect in effects {
            match effect {
                Effect::Scaffold { program, namespace } => {
                    run_program(&program, &namespace, Some(root.clone()))?;
                }
                Effect::Write { path, text } => {
                    // The two gates the machine cannot hold: a terminal, and
                    // write-once. Both are checked at the write, not before
                    // the questions, so a pipe that got this far still
                    // writes nothing.
                    if !crate::sign::at_a_terminal() {
                        return Err(InitError::Io {
                            context: format!("{path}: written only from answers at a terminal"),
                            source: std::io::Error::other("no terminal"),
                        });
                    }
                    let full = root.join(&path);
                    if full.exists() {
                        println!("  {path} exists — left untouched (a tool writes it once)");
                        continue;
                    }
                    println!("\n--- {path} ---\n{text}--- end ---");
                    let Some(ok) = ask(&format!("  write {path}? [Y/n]: "))? else {
                        return Ok(());
                    };
                    if !ok.is_empty() && !yes(&ok) {
                        println!("  not written");
                        continue;
                    }
                    if let Some(parent) = full.parent() {
                        fs::create_dir_all(parent)
                            .map_err(io(format!("could not create {parent}")))?;
                    }
                    fs::write(&full, text).map_err(io(format!("could not write {full}")))?;
                    println!("  wrote {path}");
                }
                Effect::ProposeSas { version } => {
                    child(&root, &["sas", "propose", &version])?;
                }
                Effect::Sign { target } => {
                    println!("  running: war sign {target} --ssh-sign");
                    child(&root, &["sign", &target, "--ssh-sign"])?;
                }
                Effect::Prepare { alias } => {
                    child(&root, &["check", &alias])?;
                    child(&root, &["compile"])?;
                    child(&root, &["authorize", &alias])?;
                    m.prepared();
                }
                Effect::Say(text) => println!("{text}"),
            }
        }
        let facts = guided::Facts::read(&root);
        m.observe(facts);
    }
}

/// One line from the human. `None` is end of input: a closed stdin stops the
/// conversation rather than spinning on empty answers.
fn ask(text: &str) -> Result<Option<String>, InitError> {
    use std::io::Write;
    let mut out = std::io::stdout();
    out.write_all(text.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|source| InitError::Io {
            context: "could not write the prompt".to_owned(),
            source,
        })?;
    let mut line = String::new();
    let read = std::io::stdin()
        .read_line(&mut line)
        .map_err(|source| InitError::Io {
            context: "could not read the answer".to_owned(),
            source,
        })?;
    if read == 0 {
        println!();
        return Ok(None);
    }
    Ok(Some(line.trim().to_owned()))
}

fn yes(a: &str) -> bool {
    matches!(a.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// `git config user.name`, when git and a name exist; the default answer,
/// never the recorded one.
fn git_user_name() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (out.status.success() && !s.is_empty()).then_some(s)
}

/// `ssh-add -L`: every loaded public key, one per line, in exactly the shape
/// `allowed_signers` wants after the principal.
fn loaded_keys() -> Vec<String> {
    std::process::Command::new("ssh-add")
        .arg("-L")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| l.starts_with("ssh-") || l.starts_with("sk-"))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn shorten(key: &str) -> String {
    let mut it = key.split_whitespace();
    let kt = it.next().unwrap_or("");
    let b64 = it.next().unwrap_or("");
    let comment = it.collect::<Vec<_>>().join(" ");
    let short = if b64.len() > 20 {
        format!("{}…{}", &b64[..8], &b64[b64.len() - 8..])
    } else {
        b64.to_owned()
    };
    format!("{kt} {short} {comment}").trim_end().to_owned()
}

/// Run this same `war` on the repository, inheriting the terminal — so
/// `war sign` can raise the key's dialog and the human can answer it. The
/// app (M2) does the same; nothing here holds a key.
fn child(root: &Utf8Path, args: &[&str]) -> Result<(), InitError> {
    let exe = std::env::current_exe().map_err(|source| InitError::Io {
        context: "could not find this executable".to_owned(),
        source,
    })?;
    let status = std::process::Command::new(exe)
        .arg("--root")
        .arg(root.as_str())
        .args(args)
        .status()
        .map_err(|source| InitError::Io {
            context: format!("could not run war {}", args.join(" ")),
            source,
        })?;
    if !status.success() {
        println!(
            "  (war {} exited {}; the conversation continues from what the tree says)",
            args.join(" "),
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
