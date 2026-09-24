// SPDX-License-Identifier: Apache-2.0
//! `war` and its records negotiate in one place (OW-WAR-0130, option B of
//! its U-001).
//!
//! Two questions, each asked once:
//!
//! 1. **Does this repository admit this `war`?** `[project] requires_war` in
//!    `openwarrant.toml` is a version requirement. [`check`] compares it with
//!    the running version when the repository is discovered, before any
//!    record is read, and a `war` it does not admit is refused
//!    `compat.war-too-old`, naming both versions. No key: every `war` reads.
//! 2. **Does this `war` know the record in front of it?** Every frozen record
//!    (docs/COMPATIBILITY.md) names its schema, `oh.war/<record>/v<major>`.
//!    [`newer_records`] reads the major of each record under a Warrant, and
//!    one newer than this `war` knows is UNKNOWN `compat.newer-record`
//!    (Law 15): not PASS, because what it says was not read, and not ERROR,
//!    because nothing is known to be wrong with it. What a newer `war` wrote
//!    is a question this one cannot answer, and it says so.
//!
//! Unknown optional fields (§69.4's `x-…` extensions, option C of U-001) are
//! not negotiated here; see "Reading backward" in docs/COMPATIBILITY.md.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::RepositoryConfig;
use openwarrant_core::config::VersionReq;

use crate::diagnostic::Diagnostic;

/// The version of the running `war`.
pub(crate) const WAR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The frozen records of docs/COMPATIBILITY.md and the schema major this
/// `war` reads for each. A record's major moves only with a `v2`, which is a
/// release the owner re-authorizes into; this table moves with it.
pub(crate) const FROZEN: &[(&str, u32)] = &[
    ("manifest", 1),
    ("atom", 1),
    ("milestones", 1),
    ("deliverables", 1),
    ("authorization", 1),
    ("judgments", 1),
    ("resolution", 1),
    ("verification", 1),
    ("correction", 1),
    ("sas-revision", 1),
    ("amendment", 1),
    ("journal-event", 1),
    ("stage-dispatch", 1),
    ("stage-submission", 1),
    ("report", 1),
    ("bonsai-evidence", 1),
    ("war", 1),
];

/// `oh.war/<record>/v<major>` → `(record, major)`; anything else → `None`.
#[must_use]
pub(crate) fn parse_schema(schema: &str) -> Option<(&str, u32)> {
    let rest = schema.trim().strip_prefix("oh.war/")?;
    let (record, major) = rest.rsplit_once("/v")?;
    Some((record, major.parse().ok()?))
}

/// The major this `war` reads for a frozen record, `None` for any other.
#[must_use]
pub(crate) fn known_major(record: &str) -> Option<u32> {
    FROZEN.iter().find(|(r, _)| *r == record).map(|(_, m)| *m)
}

/// `[project] requires_war` against the running version. The error is the
/// whole refusal, rule first, naming both versions.
pub(crate) fn check(config: &RepositoryConfig, config_file: &Utf8Path) -> Result<(), String> {
    check_version(config, config_file, WAR_VERSION)
}

fn check_version(
    config: &RepositoryConfig,
    config_file: &Utf8Path,
    running: &str,
) -> Result<(), String> {
    let Some(req) = config.project.requires_war.as_deref() else {
        return Ok(());
    };
    // Validation already refused a malformed requirement; parsing again here
    // keeps this function honest on its own.
    let parsed = VersionReq::parse(req)
        .map_err(|why| format!("compat.war-too-old: {config_file}: requires_war {req:?}: {why}"))?;
    if parsed.matches(running) {
        return Ok(());
    }
    Err(format!(
        "compat.war-too-old: {config_file} requires war {req} ([project] requires_war) and this \
         is war {running}. Nothing was read: records this repository holds may have been written \
         by a war this one does not know. Install a war that satisfies {req}"
    ))
}

/// UNKNOWN `compat.newer-record` for every record under `warrant_dir` whose
/// schema major is newer than this `war` reads. `relative` spells a path as
/// the repository does.
#[must_use]
pub(crate) fn newer_records(
    warrant_dir: &Utf8Path,
    relative: &dyn Fn(&Utf8Path) -> String,
) -> Vec<Diagnostic> {
    let mut files = Vec::new();
    walk(warrant_dir, &mut files);
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let declared: std::collections::BTreeSet<(String, u32)> =
            cached_schemas(&file).into_iter().collect();
        for (record, major) in declared {
            let Some(known) = known_major(&record) else {
                continue;
            };
            if major > known {
                out.push(Diagnostic::unknown(
                    "compat.newer-record",
                    relative(&file),
                    format!(
                        "written as oh.war/{record}/v{major}; this war ({WAR_VERSION}) reads \
                         {record} up to v{known}. What it says is not read, so it is neither \
                         passed nor failed. A war that knows v{major} reads it; a repository \
                         that needs one says so with [project] requires_war"
                    ),
                ));
            }
        }
    }
    out
}

fn walk(dir: &Utf8Path, out: &mut Vec<Utf8PathBuf>) {
    let Ok(rd) = dir.read_dir_utf8() else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path().to_owned();
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk(&path, out),
            Ok(t) if t.is_file() => out.push(path),
            _ => {}
        }
    }
}

/// [`schemas_in`], remembered per file for the life of the process while the
/// file's length and modification time stand. A Warrant is loaded many times
/// by one command (`war compile` loads the corpus once per projection); the
/// answer for an unchanged file does not change between loads.
fn cached_schemas(file: &Utf8Path) -> Vec<(String, u32)> {
    type Seen = std::collections::HashMap<
        Utf8PathBuf,
        (u64, Option<std::time::SystemTime>, Vec<(String, u32)>),
    >;
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Seen>> = std::sync::OnceLock::new();
    let Ok(meta) = std::fs::metadata(file) else {
        return vec![];
    };
    let stamp = (meta.len(), meta.modified().ok());
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(seen) = cache.lock()
        && let Some((len, modified, found)) = seen.get(file)
        && (*len, *modified) == stamp
    {
        return found.clone();
    }
    let found = schemas_in(file);
    if let Ok(mut seen) = cache.lock() {
        seen.insert(file.to_owned(), (stamp.0, stamp.1, found.clone()));
    }
    found
}

/// The `(record, major)` each schema declaration in `file` names. A file
/// that does not parse declares nothing here; its own reader reports it.
fn schemas_in(file: &Utf8Path) -> Vec<(String, u32)> {
    let ext = file.extension().unwrap_or_default();
    if !matches!(ext, "toml" | "json" | "jsonl" | "yaml" | "yml" | "md") {
        return vec![];
    }
    let Ok(text) = std::fs::read_to_string(file) else {
        return vec![];
    };
    let named = |s: &str| parse_schema(s).map(|(r, m)| (r.to_owned(), m));
    match ext {
        "toml" => toml::from_str::<toml::Value>(&text)
            .ok()
            .and_then(|v| v.get("schema")?.as_str().and_then(named))
            .into_iter()
            .collect(),
        "json" => serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("schema")?.as_str().and_then(named))
            .into_iter()
            .collect(),
        // The journal: one oh.war/journal-event per line, its major in `v`.
        "jsonl" => text
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter_map(|v| v.get("v")?.as_u64())
            .filter_map(|v| u32::try_from(v).ok())
            .map(|v| ("journal-event".to_owned(), v))
            .collect(),
        // YAML records and Markdown front matter: an unindented `schema:`.
        _ => {
            let body: Box<dyn Iterator<Item = &str>> = if ext == "md" {
                let mut lines = text.lines();
                if lines.next().map(str::trim) != Some("---") {
                    return vec![];
                }
                Box::new(lines.take_while(|l| l.trim() != "---"))
            } else {
                Box::new(text.lines())
            };
            body.filter_map(|l| l.strip_prefix("schema:"))
                .map(|v| v.trim().trim_matches(['"', '\'']))
                .filter_map(named)
                .take(1)
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(req: Option<&str>) -> RepositoryConfig {
        let mut c = RepositoryConfig::new(
            "Compat",
            openwarrant_core::Namespace::parse("CP").expect("namespace"),
        );
        c.project.requires_war = req.map(str::to_owned);
        c
    }

    #[test]
    fn requires_war_refuses_an_older_war_naming_both_versions() {
        let file = Utf8Path::new("openwarrant.toml");
        let err = check_version(&config(Some(">=99")), file, "1.0.0").expect_err("refused");
        assert!(err.starts_with("compat.war-too-old: "), "{err}");
        assert!(err.contains(">=99") && err.contains("war 1.0.0"), "{err}");
        assert_eq!(
            check_version(&config(Some("=1.0.0")), file, "1.0.0"),
            Ok(())
        );
        assert_eq!(check_version(&config(None), file, "1.0.0"), Ok(()));
        assert_eq!(check(&config(Some(WAR_VERSION)), file), Ok(()));
    }

    #[test]
    fn a_newer_major_is_unknown_and_an_unrelated_schema_is_ignored() {
        assert_eq!(
            parse_schema("oh.war/verification/v9"),
            Some(("verification", 9))
        );
        assert_eq!(
            parse_schema("oh.war/stage-dispatch/v1"),
            Some(("stage-dispatch", 1))
        );
        assert_eq!(parse_schema("something/else"), None);
        assert_eq!(known_major("verification"), Some(1));
        assert_eq!(known_major("verification-response"), None);

        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf-8")
            .join(format!("war-compat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("verifications")).expect("mkdir");
        std::fs::write(
            root.join("verifications/OBL-001.toml"),
            "schema = \"oh.war/verification/v9\"\nobligation = \"OBL-001\"\n",
        )
        .expect("write");
        std::fs::write(
            root.join("verifications/OBL-002.toml"),
            "schema = \"oh.war/verification/v1\"\n",
        )
        .expect("write");
        std::fs::write(
            root.join("request.toml"),
            "schema = \"oh.war/verification-request/v7\"\n",
        )
        .expect("write");
        let found = newer_records(&root, &|p: &Utf8Path| p.to_string());
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].rule, "compat.newer-record");
        assert_eq!(found[0].severity, crate::diagnostic::Severity::Unknown);
        assert!(
            found[0]
                .file
                .as_deref()
                .is_some_and(|f| f.ends_with("OBL-001.toml"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
