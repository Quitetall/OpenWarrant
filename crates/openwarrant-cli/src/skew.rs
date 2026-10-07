// SPDX-License-Identifier: Apache-2.0
//! Version skew between the running `war` and the text an agent reads (M9).
//!
//! An agent follows AGENTS.md, the plugin's skills and its hooks; `war` is
//! whatever binary PATH finds. When the text is newer than the binary it
//! names commands and behaviour the binary does not have, and the agent
//! reads the binary's refusal as a rule. Two stamps say which `war` the text
//! came with:
//!
//! - **AGENTS.md** ends with `<!-- openwarrant agents-md: written by war X -->`,
//!   filled in by the `war` that wrote it (`war init`, `war agents-md`).
//! - **The Claude Code plugin manifest** (`.claude-plugin/plugin.json`)
//!   carries the `war` release it ships with as its `version`, bumped every
//!   release (CONTRIBUTING.md). The skills ship inside the plugin, so its
//!   version stands for them too.
//!
//! [`findings`] compares each stamp found with the running version. A stamp
//! newer than this `war` is a WARN naming both versions and how to update;
//! an equal or older one says nothing; a file with no stamp (written before
//! M9) says nothing either, because nothing is known about it. The plugin
//! manifest is read from the repository root and from `$CLAUDE_PLUGIN_ROOT`
//! when the harness sets it.

use camino::{Utf8Path, Utf8PathBuf};

use crate::diagnostic::{Diagnostic, Severity};

/// The rule a newer stamp is reported under.
pub const RULE: &str = "install.version-skew";

const AGENTS_STAMP: &str = "<!-- openwarrant agents-md: written by war ";

/// The version an AGENTS.md says wrote it, from its stamp line.
#[must_use]
pub fn agents_md_stamp(text: &str) -> Option<String> {
    text.lines().rev().find_map(|l| {
        let rest = l.trim().strip_prefix(AGENTS_STAMP)?;
        let v = rest.strip_suffix("-->")?.trim();
        (!v.is_empty()).then(|| v.to_owned())
    })
}

/// The plugin manifest's `version`.
#[must_use]
pub fn plugin_version(text: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()?
        .get("version")?
        .as_str()
        .map(str::to_owned)
}

/// One stamp compared with the running version: a WARN when the stamp is
/// newer, nothing otherwise. A stamp that is not a version is not guessed at.
#[must_use]
pub fn compare(what: &str, file: &str, stamp: &str, running: &str) -> Option<Diagnostic> {
    crate::install::precedes(running, stamp).then(|| {
        Diagnostic::new(
            Severity::Warn,
            RULE,
            Some(file.to_owned()),
            format!(
                "{what} expects war {stamp}; this war is {running}, older. Commands or behaviour \
                 it describes may be missing here, and a refusal from this binary is not a rule \
                 of the repository. Update war: {}",
                crate::install::remedy(stamp)
            ),
        )
    })
}

/// Every skew finding for the repository at `root`, against this `war`.
#[must_use]
pub fn findings(root: &Utf8Path) -> Vec<Diagnostic> {
    findings_for(root, env!("CARGO_PKG_VERSION"), plugin_root().as_deref())
}

fn plugin_root() -> Option<Utf8PathBuf> {
    std::env::var("CLAUDE_PLUGIN_ROOT")
        .ok()
        .filter(|s| !s.is_empty())
        .map(Utf8PathBuf::from)
}

/// [`findings`] with the running version and plugin root given, for tests.
#[must_use]
pub fn findings_for(root: &Utf8Path, running: &str, plugin: Option<&Utf8Path>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if let Ok(text) = std::fs::read_to_string(root.join("AGENTS.md"))
        && let Some(stamp) = agents_md_stamp(&text)
        && let Some(d) = compare("AGENTS.md", "AGENTS.md", &stamp, running)
    {
        out.push(d);
    }
    let mut manifests = vec![root.join(".claude-plugin/plugin.json")];
    if let Some(p) = plugin {
        let m = p.join(".claude-plugin/plugin.json");
        if !manifests.contains(&m) {
            manifests.push(m);
        }
    }
    for m in manifests {
        if let Ok(text) = std::fs::read_to_string(&m)
            && let Some(stamp) = plugin_version(&text)
            && let Some(d) = compare(
                "the OpenWarrant plugin (skills and hooks)",
                m.as_str(),
                &stamp,
                running,
            )
        {
            out.push(d);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stamp_is_read_from_the_last_line_that_carries_it() {
        let t = format!("# x\n\nbody\n\n{AGENTS_STAMP}1.2.0 -->\n");
        assert_eq!(agents_md_stamp(&t).as_deref(), Some("1.2.0"));
        assert_eq!(agents_md_stamp("# no stamp\n"), None);
        assert_eq!(
            plugin_version(r#"{"name":"openwarrant","version":"1.0.0-alpha.3"}"#).as_deref(),
            Some("1.0.0-alpha.3")
        );
    }

    #[test]
    fn only_a_newer_stamp_warns() {
        let d = compare("AGENTS.md", "AGENTS.md", "1.2.0", "1.0.0").expect("newer warns");
        assert_eq!(d.rule, RULE);
        assert!(d.message.contains("war 1.2.0") && d.message.contains("1.0.0"));
        // Refusal side: equal, older, and unparseable say nothing.
        assert!(compare("AGENTS.md", "AGENTS.md", "1.0.0", "1.0.0").is_none());
        assert!(compare("AGENTS.md", "AGENTS.md", "0.9.0", "1.0.0").is_none());
        assert!(compare("AGENTS.md", "AGENTS.md", "not a version", "1.0.0").is_none());
        // A prerelease is older than its release.
        assert!(compare("x", "x", "1.0.0", "1.0.0-alpha.2").is_some());
    }

    /// The plugin's version is the `war` release it ships with, so it moves
    /// every release: a version bump that forgets the plugin fails here.
    #[test]
    fn the_shipped_plugin_tracks_this_release() {
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join(".claude-plugin/plugin.json")).unwrap();
        assert_eq!(
            plugin_version(&text).as_deref(),
            Some(env!("CARGO_PKG_VERSION")),
            "bump .claude-plugin/plugin.json's version with the release (CONTRIBUTING.md)"
        );
        // And this repository's own text raises no skew against its build.
        assert!(findings_for(&root, env!("CARGO_PKG_VERSION"), None).is_empty());
    }

    #[test]
    fn findings_read_agents_md_and_both_plugin_manifests() {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-skew-{}", std::process::id()));
        let plugin = dir.join("plugin-cache");
        std::fs::create_dir_all(dir.join(".claude-plugin")).unwrap();
        std::fs::create_dir_all(plugin.join(".claude-plugin")).unwrap();
        std::fs::write(
            dir.join("AGENTS.md"),
            format!("x\n{AGENTS_STAMP}2.0.0 -->\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join(".claude-plugin/plugin.json"),
            r#"{"version":"1.0.0"}"#,
        )
        .unwrap();
        std::fs::write(
            plugin.join(".claude-plugin/plugin.json"),
            r#"{"version":"3.0.0"}"#,
        )
        .unwrap();
        let f = findings_for(&dir, "1.0.0", Some(&plugin));
        let files: Vec<&str> = f.iter().filter_map(|d| d.file.as_deref()).collect();
        assert_eq!(files.len(), 2, "{f:?}");
        assert!(files.contains(&"AGENTS.md"));
        assert!(files.iter().any(|p| p.contains("plugin-cache")));
        assert!(findings_for(&dir, "3.0.0", Some(&plugin)).is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
