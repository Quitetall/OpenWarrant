// SPDX-License-Identifier: Apache-2.0

//! What this build is: a release, a published crate, a checkout, or unknown
//! (OW-WAR-0143).
//!
//! A build from a checkout 62 commits past `v1.0.0-alpha.2` used to print
//! `war 1.0.0-alpha.2`, and `war update --check` then told it that it was
//! current. The version names a line of development; it does not name a build.
//! So a build carries its class, its commit and whether its tree was dirty,
//! and only a `release` build prints the bare `war <version>`.
//!
//! This file is shared: `build.rs` includes it by `#[path]` to classify the
//! workspace at build time, and the crate includes it to read back what
//! `build.rs` embedded and to answer `war version --probe <dir>`. It therefore
//! uses the standard library alone — `build.rs` has no dependencies.
//!
//! Law 15: a build with neither a git checkout nor a `.cargo_vcs_info.json`
//! is `unknown`, and `unknown` is never `release`.

use std::path::Path;
use std::process::Command;

/// The environment variable a release build is given: the tag it is built
/// from. Set by `.github/workflows/release.yml`'s Build step.
pub const RELEASE_TAG_ENV: &str = "OPENWARRANT_RELEASE_TAG";

/// A git checkout: the commit at HEAD, and whether tracked files differ from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Git {
    pub commit: String,
    pub dirty: bool,
}

/// What `cargo publish` recorded in `.cargo_vcs_info.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsInfo {
    pub sha1: String,
    pub dirty: bool,
}

/// Everything [`classify`] decides from, as read from a directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The package version (`CARGO_PKG_VERSION`).
    pub version: String,
    /// `OPENWARRANT_RELEASE_TAG`, when set and non-empty.
    pub tag: Option<String>,
    pub git: Option<Git>,
    pub vcs_info: Option<VcsInfo>,
}

/// The four kinds of build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Built from the tag `v<version>`, with a clean tree.
    Release,
    /// Built from a published crate; `.cargo_vcs_info.json` names its commit.
    Crate,
    /// Built from a git checkout that is not a release build.
    Unreleased,
    /// Neither source says what this is.
    Unknown,
}

impl Class {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Crate => "crate",
            Self::Unreleased => "unreleased",
            Self::Unknown => "unknown",
        }
    }

    /// The inverse of [`Class::as_str`]. Anything else is `Unknown`: an
    /// unreadable class is never promoted to `release`.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "release" => Self::Release,
            "crate" => Self::Crate,
            "unreleased" => Self::Unreleased,
            _ => Self::Unknown,
        }
    }
}

/// What a build is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub class: Class,
    pub commit: Option<String>,
    pub dirty: bool,
}

impl Identity {
    /// The text after `war ` in `war --version`.
    ///
    /// A `release` build prints its version alone, which is what the release
    /// workflow's Package step and `package.py` compare against. Every other
    /// class says what it is:
    /// `<version> (<class>[, commit <12 hex>][, dirty][, debug])`.
    #[must_use]
    pub fn version_text(&self, version: &str, debug: bool) -> String {
        if self.class == Class::Release {
            return version.to_owned();
        }
        let mut parts = vec![self.class.as_str().to_owned()];
        if let Some(commit) = &self.commit {
            parts.push(format!("commit {}", short(commit)));
        }
        if self.dirty {
            parts.push("dirty".to_owned());
        }
        if debug {
            parts.push("debug".to_owned());
        }
        format!("{version} ({})", parts.join(", "))
    }
}

/// The first twelve characters of a commit id.
#[must_use]
pub fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        // Whatever repository an enclosing process pointed git at is not the
        // one this directory is in.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn git_facts(dir: &Path) -> Option<Git> {
    let commit = git(dir, &["rev-parse", "HEAD"])?.trim().to_owned();
    if commit.len() < 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    // `--no-optional-locks`: a status that refreshed the index would touch
    // `.git/index`, which `build.rs` watches, and rerun itself once more.
    // A status that cannot run is not evidence of a clean tree.
    let dirty = git(
        dir,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain",
            "--untracked-files=no",
        ],
    )
    .is_none_or(|s| !s.trim().is_empty());
    Some(Git { commit, dirty })
}

/// The string value after `"key":` in a small, trusted JSON file. By hand:
/// `build.rs` has no JSON parser, and this file is two fields.
fn json_string(text: &str, key: &str) -> Option<String> {
    let at = text.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = text[at..].trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_owned())
}

fn json_true(text: &str, key: &str) -> bool {
    text.find(&format!("\"{key}\"")).is_some_and(|i| {
        text[i + key.len() + 2..]
            .trim_start()
            .strip_prefix(':')
            .is_some_and(|r| r.trim_start().starts_with("true"))
    })
}

fn vcs_info(dir: &Path) -> Option<VcsInfo> {
    let text = std::fs::read_to_string(dir.join(".cargo_vcs_info.json")).ok()?;
    let sha1 = json_string(&text, "sha1")?;
    if sha1.len() < 40 || !sha1.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(VcsInfo {
        sha1,
        dirty: json_true(&text, "dirty"),
    })
}

/// Read what `dir` says about the build made from it.
///
/// A `.cargo_vcs_info.json` in `dir` itself means an unpacked published crate,
/// and it is preferred over git: a crate unpacked under some other checkout
/// (a home directory kept in git, say) would otherwise borrow that checkout's
/// commit. A git checkout supplies HEAD and the dirty flag.
#[must_use]
pub fn gather(dir: &Path, version: &str) -> Facts {
    let tag = std::env::var(RELEASE_TAG_ENV)
        .ok()
        .filter(|t| !t.trim().is_empty());
    let vcs_info = vcs_info(dir);
    let git = if vcs_info.is_none() {
        git_facts(dir)
    } else {
        None
    };
    Facts {
        version: version.to_owned(),
        tag,
        git,
        vcs_info,
    }
}

/// Decide the class. An `Err` is a release build that is not what it says:
/// `build.rs` panics with it, so such a binary is never produced.
///
/// # Errors
///
/// A release tag that is not `v<version>`, a release tag over a dirty tree,
/// or a release tag with no source that could show the tree is clean.
pub fn classify(facts: &Facts) -> Result<Identity, String> {
    if let Some(tag) = &facts.tag {
        let want = format!("v{}", facts.version);
        if *tag != want {
            return Err(format!(
                "{RELEASE_TAG_ENV} is {tag}, but this is version {} — its release tag is {want}",
                facts.version
            ));
        }
        let (commit, dirty) = match (&facts.git, &facts.vcs_info) {
            (Some(g), _) => (g.commit.clone(), g.dirty),
            (None, Some(v)) => (v.sha1.clone(), v.dirty),
            (None, None) => {
                return Err(format!(
                    "{RELEASE_TAG_ENV} is {tag}, but neither git nor .cargo_vcs_info.json says \
                     what this source is; an unknown build is never a release"
                ));
            }
        };
        if dirty {
            return Err(format!(
                "{RELEASE_TAG_ENV} is {tag}, but the tree at commit {} is dirty: tracked files \
                 differ from the commit, so this build is not the release",
                short(&commit)
            ));
        }
        return Ok(Identity {
            class: Class::Release,
            commit: Some(commit),
            dirty: false,
        });
    }
    Ok(match (&facts.git, &facts.vcs_info) {
        (Some(g), _) => Identity {
            class: Class::Unreleased,
            commit: Some(g.commit.clone()),
            dirty: g.dirty,
        },
        (None, Some(v)) => Identity {
            class: Class::Crate,
            commit: Some(v.sha1.clone()),
            dirty: v.dirty,
        },
        (None, None) => Identity {
            class: Class::Unknown,
            commit: None,
            dirty: false,
        },
    })
}

/// What `build.rs` embedded in this binary. A binary built without it (none
/// is, but the fallback must be honest) is `unknown`.
#[must_use]
pub fn embedded() -> Identity {
    let commit = option_env!("OW_BUILD_COMMIT")
        .filter(|c| !c.is_empty())
        .map(str::to_owned);
    Identity {
        class: Class::parse(option_env!("OW_BUILD_CLASS").unwrap_or("unknown")),
        commit,
        dirty: option_env!("OW_BUILD_DIRTY") == Some("true"),
    }
}

/// Cargo's profile for this binary (`debug` or `release`), as `build.rs` saw it.
#[must_use]
pub fn embedded_profile() -> &'static str {
    option_env!("OW_BUILD_PROFILE").unwrap_or("unknown")
}

/// The text after `war ` that `war --version` prints for this binary.
#[must_use]
pub fn version_text() -> &'static str {
    static TEXT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEXT.get_or_init(|| {
        embedded().version_text(env!("CARGO_PKG_VERSION"), embedded_profile() == "debug")
    })
}

/// `war --version`'s whole line.
#[must_use]
pub fn version_line() -> String {
    format!("war {}", version_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn facts(tag: Option<&str>, git: Option<bool>, vcs: Option<bool>) -> Facts {
        Facts {
            version: "1.2.3-alpha.4".to_owned(),
            tag: tag.map(str::to_owned),
            git: git.map(|dirty| Git {
                commit: SHA.to_owned(),
                dirty,
            }),
            vcs_info: vcs.map(|dirty| VcsInfo {
                sha1: SHA.to_owned(),
                dirty,
            }),
        }
    }

    #[test]
    fn only_a_clean_tagged_build_is_a_release_and_prints_its_version_alone() {
        let id = classify(&facts(Some("v1.2.3-alpha.4"), Some(false), None)).unwrap();
        assert_eq!(id.class, Class::Release);
        assert_eq!(id.version_text("1.2.3-alpha.4", true), "1.2.3-alpha.4");
        let id = classify(&facts(None, Some(true), None)).unwrap();
        assert_eq!(id.class, Class::Unreleased);
        assert_eq!(
            id.version_text("1.2.3-alpha.4", true),
            "1.2.3-alpha.4 (unreleased, commit 0123456789ab, dirty, debug)"
        );
    }

    #[test]
    fn a_release_tag_over_a_dirty_tree_a_wrong_tag_or_no_source_is_refused() {
        let dirty = classify(&facts(Some("v1.2.3-alpha.4"), Some(true), None)).unwrap_err();
        assert!(dirty.contains("dirty"), "{dirty}");
        let wrong = classify(&facts(Some("v9.9.9"), Some(false), None)).unwrap_err();
        assert!(
            wrong.contains("v9.9.9") && wrong.contains("v1.2.3-alpha.4"),
            "{wrong}"
        );
        let none = classify(&facts(Some("v1.2.3-alpha.4"), None, None)).unwrap_err();
        assert!(none.contains("never a release"), "{none}");
    }

    #[test]
    fn crate_and_unknown_are_named_and_unknown_is_never_release() {
        assert_eq!(
            classify(&facts(None, None, Some(false))).unwrap().class,
            Class::Crate
        );
        assert_eq!(
            classify(&facts(None, None, None)).unwrap().class,
            Class::Unknown
        );
        assert_eq!(Class::parse("garbage"), Class::Unknown);
    }

    #[test]
    fn the_vcs_info_file_is_read_by_hand() {
        let text = format!(
            "{{\n  \"git\": {{\n    \"sha1\": \"{SHA}\",\n    \"dirty\": true\n  }},\n  \"path_in_vcs\": \"crates/x\"\n}}"
        );
        assert_eq!(json_string(&text, "sha1").as_deref(), Some(SHA));
        assert!(json_true(&text, "dirty"));
        assert!(!json_true("{\"git\":{\"sha1\":\"x\"}}", "dirty"));
    }
}
