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
//! asking, `war admin version` prints it, `war admin doctor` includes it, and
//! [`update`] replaces the install with a release from GitHub — refusing,
//! rather than clobbering, anything it did not put there.
//!
//! OW-WAR-0143 added what makes that answer honest over time: the build's
//! identity ([`crate::build_identity`]), SemVer precedence instead of string
//! equality, a default channel that follows the running build, verification of
//! everything downloaded before any link moves, and [`remedy`] — the one
//! command that gets this install to a given version.

use std::cmp::Ordering;
use std::io::Read;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};

use crate::build_identity::{self, Class, Identity};
use crate::diagnostic::{Diagnostic, Report};

/// Where releases come from. A fork changes this one line.
pub const RELEASES: &str = "https://api.github.com/repos/Quitetall/OpenWarrant/releases";

/// Overrides [`RELEASES`]; every use is reported `update.source`. The seam the
/// conformance plants use to serve fixture releases from a local server.
pub const RELEASES_ENV: &str = "OPENWARRANT_RELEASES_URL";

/// The bootstrap, for an install `war admin update` does not manage. `docs/INSTALL.md`
/// prints the same line; the conformance plant compares the two.
pub const INSTALL_SH_LINE: &str =
    "curl -fsSL https://raw.githubusercontent.com/Quitetall/OpenWarrant/main/install.sh | bash";

/// The update for a `war` that `cargo install` put in `~/.cargo/bin`.
pub const CARGO_LINE: &str = "cargo install --locked openwarrant-cli";

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
    /// What this build is: release, crate, unreleased or unknown.
    pub identity: Identity,
    /// Cargo's profile for this binary, as `build.rs` saw it.
    pub profile: &'static str,
    /// The binary actually executing, with symlinks resolved.
    pub running: Utf8PathBuf,
    /// Whether this build carries debug assertions — a `war` from
    /// `target/debug` is a development build, and saying so is cheaper than
    /// the operator discovering it from behaviour.
    pub debug_build: bool,
    /// Every `war` on `PATH`, in `PATH` order. More than one entry is the
    /// warning this type exists for.
    pub on_path: Vec<OnPath>,
    /// Where `war admin update` installs, and where a managed symlink points.
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

/// The releases list this process reads, and whether the environment chose it.
#[must_use]
pub fn releases_url() -> (String, bool) {
    match std::env::var(RELEASES_ENV) {
        Ok(url) if !url.trim().is_empty() => (url, true),
        _ => (RELEASES.to_owned(), false),
    }
}

fn source_warning(report: &mut Report) {
    let (url, overridden) = releases_url();
    if overridden {
        report.push(Diagnostic::warn(
            "update.source",
            RELEASES_ENV,
            format!(
                "releases are read from {url} ({RELEASES_ENV}), not {RELEASES}. Its checksums \
                 are believed as that server's"
            ),
        ));
    }
}

fn classify(path: &Utf8Path) -> Kind {
    if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Kind::Symlink;
    }
    let mut head = [0u8; 2];
    if std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && head == *b"#!"
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
        identity: build_identity::embedded(),
        profile: build_identity::embedded_profile(),
        running,
        debug_build: cfg!(debug_assertions),
        on_path,
        install_root: install_root(),
    }
}

impl Install {
    /// Whether `war admin update` put this entry in place: a symlink resolving
    /// inside the install root. Nothing else is ever repointed.
    #[must_use]
    pub fn is_managed(&self, entry: &OnPath) -> bool {
        entry.kind == Kind::Symlink
            && (entry.resolved.starts_with(&self.install_root)
                || entry.resolved.starts_with(canonical(&self.install_root)))
    }

    /// The one command that gets this install to at least `min_version`:
    /// `war admin update --to <v>` when a managed link is on PATH, the `install.sh`
    /// line otherwise. OW-WAR-0130's `compat.war-too-old` names it (Q-003).
    #[must_use]
    pub fn remedy(&self, min_version: &str) -> String {
        let min = min_version.trim_start_matches('v');
        if self.on_path.iter().any(|e| self.is_managed(e)) {
            format!("war admin update --to {min}")
        } else {
            INSTALL_SH_LINE.to_owned()
        }
    }

    /// `war --version`'s line for this binary.
    #[must_use]
    pub fn identity_line(&self) -> String {
        format!(
            "war {}",
            self.identity
                .version_text(self.version, self.profile == "debug")
        )
    }

    /// The facts, as diagnostics, so `war admin version` and `war admin doctor` report the
    /// same thing in the same words.
    #[must_use]
    pub fn report(&self) -> Report {
        let mut report = Report::default();
        let commit = self
            .identity
            .commit
            .as_deref()
            .map_or_else(String::new, |c| {
                format!(", commit {}", build_identity::short(c))
            });
        report.push(Diagnostic::pass(
            "install.version",
            format!(
                "war {} — {} build{commit}{}, {} profile{}; running from {}",
                self.version,
                self.identity.class.as_str(),
                if self.identity.dirty { ", dirty" } else { "" },
                self.profile,
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
            format!("`war admin update` installs into {}", self.install_root),
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
            // §69.2: an optional field a minor version may add.
            "build": {
                "class": self.identity.class.as_str(),
                "commit": self.identity.commit,
                "dirty": self.identity.dirty,
                "profile": self.profile,
                "releases_url": releases_url().0,
            },
        })
    }
}

/// The command that gets the `war` on PATH to at least `min_version`.
#[must_use]
pub fn remedy(min_version: &str) -> String {
    observe().remedy(min_version)
}

// ---------------------------------------------------------------------------
// SemVer 2.0.0 §11 precedence, by hand: one comparison is not worth a crate.

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Ident {
    // Declared first: numeric identifiers have lower precedence than
    // alphanumeric ones (§11.4.3), which the derived `Ord` gives.
    Num(u64),
    Alpha(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Semver {
    core: [u64; 3],
    pre: Vec<Ident>,
}

fn numeric(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) || (s.len() > 1 && s.starts_with('0'))
    {
        return None;
    }
    s.parse().ok()
}

fn parse(version: &str) -> Option<Semver> {
    let v = version.strip_prefix('v').unwrap_or(version);
    // Build metadata does not take part in precedence (§10).
    let v = v.split_once('+').map_or(v, |(a, _)| a);
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let mut parts = core.split('.');
    let core = [
        numeric(parts.next()?)?,
        numeric(parts.next()?)?,
        numeric(parts.next()?)?,
    ];
    if parts.next().is_some() {
        return None;
    }
    let mut idents = Vec::new();
    if let Some(pre) = pre {
        for ident in pre.split('.') {
            if ident.is_empty()
                || !ident
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return None;
            }
            if ident.bytes().all(|b| b.is_ascii_digit()) {
                idents.push(Ident::Num(numeric(ident)?));
            } else {
                idents.push(Ident::Alpha(ident.to_owned()));
            }
        }
    }
    Some(Semver { core, pre: idents })
}

/// SemVer precedence of two versions (a leading `v` is ignored). `None` when
/// either is not a version.
#[must_use]
pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    let (a, b) = (parse(a)?, parse(b)?);
    Some(a.core.cmp(&b.core).then_with(|| {
        match (a.pre.is_empty(), b.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            // A prerelease precedes its normal version (§11.3).
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            // Identifier by identifier; a shorter set precedes (§11.4.4).
            (false, false) => a.pre.cmp(&b.pre),
        }
    }))
}

/// `a` has lower precedence than `b`.
#[must_use]
pub fn precedes(a: &str, b: &str) -> bool {
    compare(a, b) == Some(Ordering::Less)
}

/// Whether `version` carries a prerelease part.
#[must_use]
pub fn is_prerelease(version: &str) -> bool {
    parse(version).is_some_and(|v| !v.pre.is_empty())
}

// ---------------------------------------------------------------------------

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

/// One GET. Sends the request line, `host`, and a `user-agent` naming this
/// package's version — no `accept`, no cookie, nothing about the caller.
fn get(url: &str, timeout: Option<Duration>) -> Result<Vec<u8>, String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(timeout)
        .accept(ureq::config::AutoHeaderValue::None)
        .accept_encoding(ureq::config::AutoHeaderValue::None)
        .user_agent(concat!("openwarrant/", env!("CARGO_PKG_VERSION")))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut response = agent.get(url).call().map_err(|e| format!("{url}: {e}"))?;
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

/// Every published release. Drafts are not offered: a draft is not published.
///
/// # Errors
///
/// The list could not be read or is not a list of releases.
pub fn releases(timeout: Option<Duration>) -> Result<Vec<Release>, String> {
    let (url, _) = releases_url();
    let body = get(&url, timeout)?;
    let value: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("{url}: {e}"))?;
    let array = value
        .as_array()
        .ok_or_else(|| format!("{url}: expected a list of releases"))?;
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

/// What `war admin update` was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Published releases that are not prereleases.
    Stable,
    /// Anything published, prereleases included.
    Preview,
}

/// The channel the running build belongs to: a prerelease version is on the
/// preview channel. Every release published so far is a prerelease, and a
/// default of `stable` found nothing for any of them.
#[must_use]
pub fn default_channel() -> Channel {
    if is_prerelease(env!("CARGO_PKG_VERSION")) {
        Channel::Preview
    } else {
        Channel::Stable
    }
}

#[must_use]
pub const fn label(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => "stable",
        Channel::Preview => "preview",
    }
}

/// The release `want` names, or the newest on `channel` by precedence.
///
/// # Errors
///
/// No release matches.
pub fn pick(all: Vec<Release>, channel: Channel, want: Option<&str>) -> Result<Release, String> {
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
        .filter(|r| match channel {
            Channel::Stable => !r.prerelease,
            Channel::Preview => true,
        })
        .filter(|r| parse(&r.version).is_some())
        .max_by(|a, b| compare(&a.version, &b.version).unwrap_or(Ordering::Equal))
        .ok_or_else(|| format!("no published {} release", label(channel)))
}

/// How this build stands against `release`, as the one diagnostic `check`
/// and `update` both give. `None` when the running build is older: the caller
/// decides whether that is a notice or an install.
fn standing(install: &Install, release: &Release, channel: Channel) -> Option<Diagnostic> {
    let current = install.version;
    match compare(current, &release.version) {
        Some(Ordering::Less) => None,
        Some(Ordering::Equal) if install.identity.class == Class::Release => {
            Some(Diagnostic::pass(
                "update.current",
                format!("{current} is the newest {} release", label(channel)),
            ))
        }
        Some(Ordering::Equal) => Some(Diagnostic::warn(
            "update.unreleased",
            "build",
            format!(
                "this build is {} {}{}{}, not the release {}: same version, not the same build. \
                 `war admin update --to {current} --force` installs the release",
                install.identity.class.as_str(),
                current,
                install
                    .identity
                    .commit
                    .as_deref()
                    .map_or_else(String::new, |c| format!(
                        " at commit {}",
                        build_identity::short(c)
                    )),
                if install.identity.dirty {
                    " (dirty)"
                } else {
                    ""
                },
                release.tag
            ),
        )),
        Some(Ordering::Greater) => Some(Diagnostic::pass(
            "update.ahead",
            format!(
                "this build ({current}) is newer than {}, the newest {} release. Nothing is \
                 installed unless --to names a version",
                release.tag,
                label(channel)
            ),
        )),
        None => Some(Diagnostic::unknown(
            "update.unavailable",
            "releases",
            format!(
                "{current} and {} cannot be ordered as versions",
                release.tag
            ),
        )),
    }
}

/// Report what is available and stop.
#[must_use]
pub fn check(channel: Channel) -> Report {
    let install = observe();
    let mut report = install.report();
    source_warning(&mut report);
    match releases(None).and_then(|all| pick(all, channel, None)) {
        Ok(release) => match standing(&install, &release, channel) {
            Some(d) => report.push(d),
            None => {
                let command = install.remedy(&release.version);
                report.push(Diagnostic::warn(
                    "update.available",
                    command.clone(),
                    format!(
                        "{} is published; this is {}. `{command}` installs it",
                        release.tag, install.version
                    ),
                ));
            }
        },
        Err(why) => report.push(Diagnostic::unknown("update.unavailable", "releases", why)),
    }
    report
}

/// The newest release on `channel`, for the notice. A read with a timeout.
///
/// # Errors
///
/// The list could not be read, or holds nothing on the channel.
pub fn newest(channel: Channel, timeout: Duration) -> Result<Release, String> {
    pick(releases(Some(timeout))?, channel, None)
}

/// The command that brings one unmanaged `war` under `war admin update`.
fn unmanaged_remedy(entry: &OnPath) -> String {
    let under_cargo = entry
        .path
        .parent()
        .is_some_and(|dir| dir.ends_with(".cargo/bin"));
    if entry.kind == Kind::Binary && under_cargo {
        format!(
            "`cargo install` put it there; update it the same way: {CARGO_LINE}. (`war admin update` \
             manages only the links it creates.)"
        )
    } else {
        format!(
            "remove it first (rm {}), then install a managed one: {INSTALL_SH_LINE}",
            entry.path
        )
    }
}

fn manifest_check(staging: &Utf8Path) -> Result<usize, String> {
    let text = std::fs::read(staging.join("MANIFEST.json"))
        .map_err(|e| format!("the archive carries no readable MANIFEST.json: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&text).map_err(|e| format!("MANIFEST.json: {e}"))?;
    let files = value["files"]
        .as_object()
        .ok_or_else(|| "MANIFEST.json has no `files` map".to_owned())?;
    if !files.contains_key("bin/war") {
        return Err("MANIFEST.json does not list bin/war".to_owned());
    }
    for (name, item) in files {
        let unsafe_name = name.is_empty()
            || name.starts_with('/')
            || name.contains('\\')
            || name
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..");
        if unsafe_name {
            return Err(format!("MANIFEST.json lists an unsafe path {name:?}"));
        }
        let bytes = std::fs::read(staging.join(name))
            .map_err(|e| format!("MANIFEST.json lists {name}, which the archive lacks: {e}"))?;
        let actual = openwarrant_compiler::sha256_hex(&bytes);
        let recorded = item["sha256"].as_str().unwrap_or_default();
        let size = item["bytes"].as_u64();
        if recorded != actual || size != Some(bytes.len() as u64) {
            return Err(format!(
                "{name}: MANIFEST.json records sha256:{recorded} ({} bytes); the archive holds \
                 sha256:{actual} ({} bytes)",
                size.map_or_else(|| "?".to_owned(), |s| s.to_string()),
                bytes.len()
            ));
        }
    }
    Ok(files.len())
}

fn identity_check(binary: &Utf8Path, version: &str) -> Result<String, String> {
    let out = std::process::Command::new(binary.as_str())
        .arg("--version")
        .output()
        .map_err(|e| format!("could not run {binary} --version: {e}"))?;
    let line = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let want = format!("war {version}");
    if line == want || line.starts_with(&format!("{want} ")) {
        Ok(line)
    } else {
        Err(format!(
            "the archive's bin/war says {line:?}, not {want:?}; it is not the release it is \
             published as"
        ))
    }
}

/// Download, verify and install a release; repoint what this tool installed.
///
/// Never touches a `war` it did not put in place: a script or a binary sitting
/// on `PATH` is somebody's deliberate arrangement, and replacing it silently is
/// how an operator ends up running something they did not choose. When no
/// `war` on PATH is managed, the update is refused before anything is
/// downloaded, with the command for each; beside a managed link they are
/// reported and left alone.
#[must_use]
pub fn update(channel: Channel, want: Option<&str>, force: bool) -> Report {
    let install = observe();
    let mut report = Report::default();
    source_warning(&mut report);

    if !install.on_path.is_empty() && !install.on_path.iter().any(|e| install.is_managed(e)) {
        for entry in &install.on_path {
            report.push(Diagnostic::error(
                "update.unmanaged",
                entry.path.to_string(),
                format!(
                    "{} is a {} `war admin update` did not install, and nothing on PATH is. It is \
                     never overwritten; {}",
                    entry.path,
                    entry.kind,
                    unmanaged_remedy(entry)
                ),
            ));
        }
        return report;
    }

    let release = match releases(None).and_then(|all| pick(all, channel, want)) {
        Ok(r) => r,
        Err(why) => {
            report.push(Diagnostic::error("update.no-release", "releases", why));
            return report;
        }
    };
    if !force {
        match compare(install.version, &release.version) {
            // Asked for by name: an older version is a choice, not an accident.
            Some(Ordering::Greater) if want.is_some() => {}
            Some(Ordering::Less) => {}
            _ => {
                if let Some(d) = standing(&install, &release, channel) {
                    report.push(d);
                }
                return report;
            }
        }
    }
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

    let archive = match get(archive_url, None) {
        Ok(b) => b,
        Err(why) => {
            report.push(Diagnostic::error("update.download", asset_name, why));
            return report;
        }
    };
    let expected = match get(sum_url, None) {
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
    // 1. The checksum published beside the archive.
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

    let root = &install.install_root;
    let root_existed = root.exists();
    let target = root.join(&release.version);
    let staging = root.join(format!(
        "{}.incoming.{}",
        release.version,
        std::process::id()
    ));
    // Every refusal from here leaves the root as it was found.
    let discard = |report: &mut Report, rule: &str, file: String, message: String| {
        let _ = std::fs::remove_dir_all(&staging);
        if !root_existed {
            let _ = std::fs::remove_dir(root);
        }
        report.push(Diagnostic::error(rule, file, message));
    };
    let _ = std::fs::remove_dir_all(&staging);
    if let Err(e) = std::fs::create_dir_all(&staging) {
        discard(
            &mut report,
            "update.install",
            staging.to_string(),
            format!("could not create {staging}: {e}"),
        );
        return report;
    }
    let archive_path = staging.join(&asset_name);
    if let Err(e) = std::fs::write(&archive_path, &archive) {
        discard(
            &mut report,
            "update.install",
            archive_path.to_string(),
            format!("could not write {archive_path}: {e}"),
        );
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
            discard(
                &mut report,
                "update.extract",
                archive_path.to_string(),
                format!("tar exited {status}; nothing is installed"),
            );
            return report;
        }
        Err(e) => {
            discard(
                &mut report,
                "update.extract",
                archive_path.to_string(),
                format!("could not run tar: {e}"),
            );
            return report;
        }
    }
    let _ = std::fs::remove_file(&archive_path);
    if !staging.join("bin/war").is_file() {
        discard(
            &mut report,
            "update.archive-shape",
            asset_name,
            "the archive carries no bin/war; nothing is installed".to_owned(),
        );
        return report;
    }
    // 2. Every file the archive's own manifest lists.
    match manifest_check(&staging) {
        Ok(n) => report.push(Diagnostic::pass(
            "update.manifest",
            format!("all {n} files match MANIFEST.json"),
        )),
        Err(why) => {
            discard(
                &mut report,
                "update.manifest-mismatch",
                asset_name,
                format!("{why}. Nothing is installed"),
            );
            return report;
        }
    }
    // 3. The binary says it is the version it is published as.
    match identity_check(&staging.join("bin/war"), &release.version) {
        Ok(line) => report.push(Diagnostic::pass(
            "update.identity",
            format!("the archive's bin/war says {line}"),
        )),
        Err(why) => {
            discard(
                &mut report,
                "update.identity-mismatch",
                asset_name,
                format!("{why}. Nothing is installed"),
            );
            return report;
        }
    }
    // 4. What none of that establishes (20-basis.md Q-001).
    report.push(Diagnostic::warn(
        "update.unsigned",
        release.tag.clone(),
        format!(
            "{} is verified against the .sha256 published in the same release. A checksum from \
             the same release is not a signature: it catches corruption and truncation, not a \
             release or an account replaced whole",
            release.tag
        ),
    ));

    // The switch: an existing directory for this version is set aside, not
    // deleted, and put back if the new one cannot take its place.
    let aside = root.join(format!(
        "{}.previous.{}",
        release.version,
        std::process::id()
    ));
    let had_target = std::fs::symlink_metadata(&target).is_ok();
    if had_target && let Err(e) = std::fs::rename(&target, &aside) {
        discard(
            &mut report,
            "update.install",
            target.to_string(),
            format!("could not set {target} aside: {e}"),
        );
        return report;
    }
    if let Err(e) = std::fs::rename(&staging, &target) {
        if had_target {
            let _ = std::fs::rename(&aside, &target);
        }
        discard(
            &mut report,
            "update.install",
            target.to_string(),
            format!("could not move {staging} to {target}: {e}"),
        );
        return report;
    }
    let installed = target.join("bin/war");
    report.push(Diagnostic::pass(
        "update.installed",
        format!("{} installed at {installed}", release.tag),
    ));

    // Repoint only the links this tool owns: a symlink whose target is inside
    // the install root. Everything else is reported, never rewritten.
    for entry in &install.on_path {
        if !install.is_managed(entry) {
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
    if had_target {
        let _ = std::fs::remove_dir_all(&aside);
    }
    if install.on_path.is_empty() {
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
                assert_eq!(
                    asset.as_deref(),
                    Some("openwarrant-v1.2.3-linux-x86_64.tar.gz")
                );
            }
            ("macos", "aarch64") => {
                assert_eq!(
                    asset.as_deref(),
                    Some("openwarrant-v1.2.3-darwin-arm64.tar.gz")
                );
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
        std::fs::write(
            &script,
            "#!/bin/sh\nexec /somewhere/else/war \"$@\" --ssh-sign\n",
        )
        .unwrap();
        let path = Utf8PathBuf::from_path_buf(script.clone()).unwrap();
        assert_eq!(classify(&path), Kind::Script);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// SemVer 2.0.0 §11.4's own example, in order, and the core example.
    #[test]
    fn precedence_follows_the_semver_example_ordering() {
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "2.0.0",
            "2.1.0",
            "2.1.1",
        ];
        for (i, a) in ordered.iter().enumerate() {
            for (j, b) in ordered.iter().enumerate() {
                assert_eq!(compare(a, b), Some(i.cmp(&j)), "{a} vs {b}");
                assert_eq!(precedes(a, b), i < j, "{a} precedes {b}");
            }
        }
        // Build metadata is ignored; a leading `v` is a tag, not a version part.
        assert_eq!(compare("v1.0.0+abc", "1.0.0"), Some(Ordering::Equal));
        // Not versions: no order, and so never "newer".
        for bad in ["1.0", "1.0.0-", "01.0.0", "1.0.0-alpha..1", "1.0.0-01", "x"] {
            assert_eq!(compare(bad, "1.0.0"), None, "{bad}");
            assert!(!precedes(bad, "9.9.9"), "{bad}");
        }
    }

    #[test]
    fn the_newest_release_is_chosen_by_precedence_not_list_order() {
        let r = |tag: &str, pre: bool| Release {
            tag: tag.to_owned(),
            version: tag.trim_start_matches('v').to_owned(),
            prerelease: pre,
            assets: Vec::new(),
        };
        let all = vec![
            r("v1.0.0-alpha.2", true),
            r("v1.0.0-alpha.10", true),
            r("v0.9.0", false),
            r("not-a-version", true),
        ];
        assert_eq!(
            pick(all.clone(), Channel::Preview, None).unwrap().tag,
            "v1.0.0-alpha.10"
        );
        assert_eq!(
            pick(all.clone(), Channel::Stable, None).unwrap().tag,
            "v0.9.0"
        );
        assert_eq!(
            pick(all, Channel::Stable, Some("1.0.0-alpha.2"))
                .unwrap()
                .tag,
            "v1.0.0-alpha.2"
        );
        assert!(is_prerelease("1.0.0-alpha.2") && !is_prerelease("1.0.0"));
    }

    #[test]
    fn remedy_names_update_for_a_managed_link_and_install_sh_otherwise() {
        let root = Utf8PathBuf::from("/nonexistent-ow-root/openwarrant");
        let mut install = observe();
        install.install_root = root.clone();
        install.on_path = vec![OnPath {
            path: "/bin-a/war".into(),
            resolved: "/bin-a/war".into(),
            kind: Kind::Binary,
        }];
        assert_eq!(install.remedy("99.0.0"), INSTALL_SH_LINE);
        install.on_path.push(OnPath {
            path: "/bin-b/war".into(),
            resolved: root.join("1.0.0/bin/war"),
            kind: Kind::Symlink,
        });
        assert_eq!(install.remedy("v99.0.0"), "war admin update --to 99.0.0");
    }
}
