// SPDX-License-Identifier: AGPL-3.0-or-later
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

    // §76.3: silence on sound state is the ideal, but `init` is a mutation and
    // the operator needs to know what was created and where.
    println!("initialized {} ({})", config.project.name, config_path);
    Ok(())
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
