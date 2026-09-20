// SPDX-License-Identifier: Apache-2.0

//! Which `war` is this, where did it come from, and is it the newest one.
//!
//! One name on `PATH` can hide any number of binaries. On the development host
//! this module was written for, `~/.cargo/bin/war` was a shell script that
//! execed a months-old debug build, appended `--ssh-sign` to every
//! `war sign <target>`, silently `cd`ed into a different checkout for some
//! aliases, and routed three subcommands to three further frozen builds under
//! `~/.local/lib/openwarrant/builds/`. Five binaries behind one name. An
//! operator ran `war sign X` without `--ssh-sign`, was told
//! `sign.ssh-principal`, and spent an evening on it — the flag they had not
//! typed was added by the wrapper, and no command printed which file it was
//! running.
//!
//! So: [`observe`] answers "which binary is this" before anything else needs
//! asking, `war version` prints it, `war doctor` includes it, and
//! [`update`] replaces the install with a release from GitHub — refusing,
//! rather than clobbering, anything it did not put there.

use std::io::Read;

use camino::{Utf8Path, Utf8PathBuf};

use crate::diagnostic::{Diagnostic, Report};

/// Where releases come from. A fork changes this one line.
const RELEASES: &str = "https://api.github.com/repos/Quitetall/OpenWarrant/releases";

/// At most this many bytes from a release asset. A preview archive is ~20 MiB;
/// a hundred of them is not a release, it is a redirect loop or a mistake.
const MAX_ASSET_BYTES: usize = 200 * 1024 * 1024;

/// What the `war` on `PATH` actually is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A symlink, and what it resolves to.
    Symlink,
    /// A `#!` script. Every wrapper this module exists to expose is one.
    Script,
    /// A binary, run directly.
    Binary,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Symlink => "symlink",
            Self::Script => "script",
            Self::Binary => "binary",
        })
    }
}

/// One `war` found on `PATH`.
#[derive(Debug, Clone)]
pub struct OnPath {
    pub path: Utf8PathBuf,
    pub resolved: Utf8PathBuf,
    pub kind: Kind,
}

/// What this process is, and what else answers to its name.
#[derive(Debug, Clone)]
pub struct Install {
    pub version: &'static str,
    /// The binary actually executing, with symlinks resolved.
    pub running: Utf8PathBuf,
    /// Whether this build carries debug assertions — a `war` from
    /// `target/debug` is a development build, and saying so is cheaper than
    /// the operator discovering it from behaviour.
    pub debug_build: bool,
    /// Every `war` on `PATH`, in `PATH` order. More than one entry is the
    /// warning this type exists for.
    pub on_path: Vec<OnPath>,
    /// Where `war update` installs, and where a managed symlink points.
    pub install_root: Utf8PathBuf,
}

/// `~/.local/lib/openwarrant`, or `$XDG_DATA_HOME/openwarrant` when set.
#[must_use]
pub fn install_root() -> Utf8PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return Utf8PathBuf::from(xdg).join("openwarrant");
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Utf8PathBuf::from(home).join(".local/lib/openwarrant")
}

fn classify(path: &Utf8Path) -> Kind {
    if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Kind::Symlink;
    }
    let mut head = [0u8; 2];
    if std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok() && head == *b"#!"
    {
        return Kind::Script;
    }
    Kind::Binary
}

fn canonical(path: &Utf8Path) -> Utf8PathBuf {
    std::fs::canonicalize(path)
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
        .unwrap_or_else(|| path.to_owned())
}

/// Every `war` on `PATH`, plus which one is running.
#[must_use]
pub fn observe() -> Install {
    let running = std::env::current_exe()
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
        .map(|p| canonical(&p))
        .unwrap_or_default();
    let mut on_path = Vec::new();
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':').filter(|d| !d.is_empty()) {
            let candidate = Utf8PathBuf::from(dir).join("war");
            if !candidate.exists() {
                continue;
            }
            let kind = classify(&candidate);
            let resolved = canonical(&candidate);
            if on_path
                .iter()
                .any(|seen: &OnPath| seen.path == candidate || seen.resolved == resolved)
            {
                continue;
            }
            on_path.push(OnPath {
                path: candidate,
                resolved,
                kind,
            });
        }
    }
    Install {
        version: env!("CARGO_PKG_VERSION"),
        running,
        debug_build: cfg!(debug_assertions),
        on_path,
        install_root: install_root(),
    }
}

impl Install {
    /// The facts, as diagnostics, so `war version` and `war doctor` report the
    /// same thing in the same words.
    #[must_use]
    pub fn report(&self) -> Report {
        let mut report = Report::default();
        report.push(Diagnostic::pass(
            "install.version",
            format!(
                "war {}{} running from {}",
                self.version,
                if self.debug_build {
                    " (debug build)"
                } else {
                    ""
                },
                self.running
            ),
        ));
        for entry in &self.on_path {
            let mine = entry.resolved == self.running;
            let line = format!(
                "{} — {}{}",
                entry.path,
                entry.kind,
                if entry.kind == Kind::Symlink || entry.resolved != entry.path {
                    format!(" → {}", entry.resolved)
                } else {
                    String::new()
                }
            );
            if mine {
                report.push(Diagnostic::pass("install.on-path", line));
            } else {
                // A second `war` earlier on PATH is what the next shell runs.
                report.push(Diagnostic::warn(
                    "install.shadowed",
                    entry.path.to_string(),
                    format!(
                        "{line} — a different `war` from the one reporting here; whichever comes \
                         first on PATH is what a shell runs"
                    ),
                ));
            }
            if entry.kind == Kind::Script {
                report.push(Diagnostic::warn(
                    "install.wrapper",
                    entry.path.to_string(),
                    format!(
                        "{} is a script, not the CLI. A wrapper may add flags you did not type — \
                         one on this project's development host appended `--ssh-sign` to every \
                         `war sign` and cost an evening. Read it, or replace it with a symlink \
                         into {}",
                        entry.path, self.install_root
                    ),
                ));
            }
        }
        report.push(Diagnostic::pass(
            "install.root",
            format!("`war update` installs into {}", self.install_root),
        ));
        report
    }

    #[must_use]
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": "oh.war/install/v1",
            "version": self.version,
            "running": self.running.as_str(),
            "debug_build": self.debug_build,
            "install_root": self.install_root.as_str(),
            "on_path": self.on_path.iter().map(|e| serde_json::json!({
                "path": e.path.as_str(),
                "resolved": e.resolved.as_str(),
                "kind": e.kind.to_string(),
                "is_running": e.resolved == self.running,
            })).collect::<Vec<_>>(),
        })
    }
}

/// The asset basename this host needs, or `None` when no release is built for
/// it — which is a refusal, not a guess at a near-enough architecture.
#[must_use]
pub fn host_asset(version: &str) -> Option<String> {
    let target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("macos", "aarch64") => "darwin-arm64",
        _ => return None,
    };
    Some(format!("openwarrant-v{version}-{target}.tar.gz"))
}

fn get(url: &str) -> Result<Vec<u8>, String> {
    let mut response = ureq::get(url)
        .header("user-agent", concat!("openwarrant/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    let mut body = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_ASSET_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|e| format!("{url}: {e}"))?;
    if body.len() > MAX_ASSET_BYTES {
        return Err(format!(
            "{url}: larger than {MAX_ASSET_BYTES} bytes; refused rather than read further"
        ));
    }
    Ok(body)
}

/// One release, reduced to what an update needs.
#[derive(Debug, Clone)]
pub struct Release {
    pub tag: String,
    pub version: String,
    pub prerelease: bool,
    pub assets: Vec<(String, String)>,
}

/// Releases newest first. Drafts are not offered: a draft is not published.
pub fn releases() -> Result<Vec<Release>, String> {
    let body = get(RELEASES)?;
    let value: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("{RELEASES}: {e}"))?;
    let array = value
        .as_array()
        .ok_or_else(|| format!("{RELEASES}: expected a list of releases"))?;
    let mut out = Vec::new();
    for release in array {
        if release["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let Some(tag) = release["tag_name"].as_str() else {
            continue;
        };
        let assets = release["assets"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|asset| {
                        Some((
                            asset["name"].as_str()?.to_owned(),
                            asset["browser_download_url"].as_str()?.to_owned(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.push(Release {
            version: tag.trim_start_matches('v').to_owned(),
            tag: tag.to_owned(),
            prerelease: release["prerelease"].as_bool().unwrap_or(false),
            assets,
        });
    }
    Ok(out)
}

/// What `war update` was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Published releases that are not prereleases.
    Stable,
    /// Anything published, prereleases included.
    Preview,
}

/// Report what is available and stop.
pub fn check(channel: Channel) -> Report {
    let mut report = observe().report();
    match pick(channel, None) {
        Ok(release) => {
            let current = env!("CARGO_PKG_VERSION");
            if release.version == current {
                report.push(Diagnostic::pass(
                    "update.current",
                    format!("{current} is the newest {} release", label(channel)),
                ));
            } else {
                report.push(Diagnostic::warn(
                    "update.available",
                    "war update".to_owned(),
                    format!(
                        "{} is published; this is {current}. `war update` installs it",
                        release.tag
                    ),
                ));
            }
        }
        Err(why) => report.push(Diagnostic::unknown("update.unavailable", "releases", why)),
    }
    report
}

const fn label(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => "stable",
        Channel::Preview => "preview",
    }
}

fn pick(channel: Channel, want: Option<&str>) -> Result<Release, String> {
    let all = releases()?;
    if let Some(want) = want {
        let tag = if want.starts_with('v') {
            want.to_owned()
        } else {
            format!("v{want}")
        };
        return all
            .into_iter()
            .find(|r| r.tag == tag)
            .ok_or_else(|| format!("no published release is tagged {tag}"));
    }
    all.into_iter()
        .find(|r| match channel {
            Channel::Stable => !r.prerelease,
            Channel::Preview => true,
        })
        .ok_or_else(|| format!("no published {} release", label(channel)))
}

/// Download, verify and install a release; repoint what this tool installed.
///
/// Never touches a `war` it did not put in place: a script or a binary sitting
/// on `PATH` is somebody's deliberate arrangement, and replacing it silently is
/// how an operator ends up running something they did not choose. Those are
/// reported with the command that would repoint them, and left alone.
pub fn update(channel: Channel, want: Option<&str>, force: bool) -> Report {
    let install = observe();
    let mut report = Report::default();
    let release = match pick(channel, want) {
        Ok(r) => r,
        Err(why) => {
            report.push(Diagnostic::error("update.no-release", "releases", why));
            return report;
        }
    };
    let Some(asset_name) = host_asset(&release.version) else {
        report.push(Diagnostic::error(
            "update.no-build",
            "host",
            format!(
                "no release is built for {}/{}; build from source instead",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
        ));
        return report;
    };
    if release.version == install.version && !force {
        report.push(Diagnostic::pass(
            "update.current",
            format!(
                "{} is already installed; --force reinstalls it",
                release.tag
            ),
        ));
        return report;
    }
    let Some((_, archive_url)) = release.assets.iter().find(|(n, _)| *n == asset_name) else {
        report.push(Diagnostic::error(
            "update.no-asset",
            release.tag.clone(),
            format!("{} carries no {asset_name}", release.tag),
        ));
        return report;
    };
    let Some((_, sum_url)) = release
        .assets
        .iter()
        .find(|(n, _)| *n == format!("{asset_name}.sha256"))
    else {
        report.push(Diagnostic::error(
            "update.no-checksum",
            release.tag.clone(),
            format!(
                "{asset_name} has no .sha256 beside it. An archive nothing attests to is not \
                 installed"
            ),
        ));
        return report;
    };

    let archive = match get(archive_url) {
        Ok(b) => b,
        Err(why) => {
            report.push(Diagnostic::error("update.download", asset_name, why));
            return report;
        }
    };
    let expected = match get(sum_url) {
        Ok(b) => String::from_utf8_lossy(&b)
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned(),
        Err(why) => {
            report.push(Diagnostic::error("update.checksum", asset_name, why));
            return report;
        }
    };
    let actual = openwarrant_compiler::sha256_hex(&archive);
    if actual != expected {
        report.push(Diagnostic::error(
            "update.checksum-mismatch",
            asset_name,
            format!(
                "the release records sha256:{expected} and the bytes are sha256:{actual}. \
                 Nothing is installed"
            ),
        ));
        return report;
    }
    report.push(Diagnostic::pass(
        "update.verified",
        format!("{asset_name} matches sha256:{}", &actual[..16]),
    ));

    let target = install.install_root.join(&release.version);
    let staging = install.install_root.join(format!(
        "{}.incoming.{}",
        release.version,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&staging);
    if let Err(e) = std::fs::create_dir_all(&staging) {
        report.push(Diagnostic::error(
            "update.install",
            staging.to_string(),
            format!("could not create {staging}: {e}"),
        ));
        return report;
    }
    let archive_path = staging.join(&asset_name);
    if let Err(e) = std::fs::write(&archive_path, &archive) {
        report.push(Diagnostic::error(
            "update.install",
            archive_path.to_string(),
            format!("could not write {archive_path}: {e}"),
        ));
        let _ = std::fs::remove_dir_all(&staging);
        return report;
    }
    // `tar` rather than a decompression crate: the archive format is the one
    // the release notes already tell a human to extract by hand, and two new
    // dependencies for one command is a supply chain for a convenience.
    let extracted = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(archive_path.as_str())
        .arg("-C")
        .arg(staging.as_str())
        .status();
    match extracted {
        Ok(status) if status.success() => {}
        Ok(status) => {
            report.push(Diagnostic::error(
                "update.extract",
                archive_path.to_string(),
                format!("tar exited {status}; nothing is installed"),
            ));
            let _ = std::fs::remove_dir_all(&staging);
            return report;
        }
        Err(e) => {
            report.push(Diagnostic::error(
                "update.extract",
                archive_path.to_string(),
                format!("could not run tar: {e}"),
            ));
            let _ = std::fs::remove_dir_all(&staging);
            return report;
        }
    }
    let _ = std::fs::remove_file(&archive_path);
    if !staging.join("bin/war").is_file() {
        report.push(Diagnostic::error(
            "update.archive-shape",
            asset_name,
            "the archive carries no bin/war; nothing is installed".to_owned(),
        ));
        let _ = std::fs::remove_dir_all(&staging);
        return report;
    }
    let _ = std::fs::remove_dir_all(&target);
    if let Err(e) = std::fs::rename(&staging, &target) {
        report.push(Diagnostic::error(
            "update.install",
            target.to_string(),
            format!("could not move {staging} to {target}: {e}"),
        ));
        let _ = std::fs::remove_dir_all(&staging);
        return report;
    }
    let installed = target.join("bin/war");
    report.push(Diagnostic::pass(
        "update.installed",
        format!("{} installed at {installed}", release.tag),
    ));

    // Repoint only the links this tool owns: a symlink whose target is inside
    // the install root. Everything else is reported, never rewritten.
    let mut repointed = 0;
    for entry in &install.on_path {
        let ours =
            entry.kind == Kind::Symlink && entry.resolved.starts_with(&install.install_root);
        if !ours {
            report.push(Diagnostic::warn(
                "update.not-ours",
                entry.path.to_string(),
                format!(
                    "{} is a {} this tool did not install, so it is untouched. To point it at \
                     the new version: ln -sfn {installed} {}",
                    entry.path, entry.kind, entry.path
                ),
            ));
            continue;
        }
        let tmp = Utf8PathBuf::from(format!("{}.incoming.{}", entry.path, std::process::id()));
        let linked = std::os::unix::fs::symlink(installed.as_str(), tmp.as_str())
            .and_then(|()| std::fs::rename(&tmp, &entry.path));
        match linked {
            Ok(()) => {
                repointed += 1;
                report.push(Diagnostic::pass(
                    "update.repointed",
                    format!("{} → {installed}", entry.path),
                ));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                report.push(Diagnostic::error(
                    "update.repoint",
                    entry.path.to_string(),
                    format!("could not repoint {}: {e}", entry.path),
                ));
            }
        }
    }
    if repointed == 0 && install.on_path.is_empty() {
        report.note(format!(
            "No `war` is on PATH. Add one: ln -sfn {installed} ~/.local/bin/war"
        ));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_with_no_release_is_refused_rather_than_approximated() {
        // The two targets the release workflow builds, and nothing else.
        let asset = host_asset("1.2.3");
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => {
                assert_eq!(asset.as_deref(), Some("openwarrant-v1.2.3-linux-x86_64.tar.gz"));
            }
            ("macos", "aarch64") => {
                assert_eq!(asset.as_deref(), Some("openwarrant-v1.2.3-darwin-arm64.tar.gz"));
            }
            _ => assert!(asset.is_none()),
        }
    }

    #[test]
    fn the_running_binary_is_identified_and_listed_once() {
        let install = observe();
        assert!(!install.running.as_str().is_empty());
        let mut seen = install
            .on_path
            .iter()
            .map(|e| e.resolved.clone())
            .collect::<Vec<_>>();
        let before = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), before, "one binary must not be listed twice");
    }

    #[test]
    fn a_script_on_path_is_classified_as_one() {
        let dir = std::env::temp_dir().join(format!("ow-install-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("war");
        std::fs::write(&script, "#!/bin/sh\nexec /somewhere/else/war \"$@\" --ssh-sign\n").unwrap();
        let path = Utf8PathBuf::from_path_buf(script.clone()).unwrap();
        assert_eq!(classify(&path), Kind::Script);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
