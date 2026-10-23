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

/// The rule a `war` older than `[project] requires_war` is refused under;
/// `entrypoint` maps it to exit 2.
pub(crate) const TOO_OLD: &str = "compat.war-too-old";

/// The record families of docs/COMPATIBILITY.md and the highest schema major
/// this CLI actually reads. This includes explicit candidate codecs; reader
/// capability does not adopt a schema pack or reinterpret a signed contract.
pub(crate) const FROZEN: &[(&str, u32)] = &[
    ("manifest", 1),
    ("atom", 1),
    ("milestones", 1),
    ("deliverables", 1),
    ("authorization", 1),
    ("judgments", 1),
    ("resolution", 1),
    ("verification", 3),
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
        .map_err(|why| format!("{TOO_OLD}: {config_file}: requires_war {req:?}: {why}"))?;
    if parsed.matches(running) {
        return Ok(());
    }
    Err(format!(
        "{TOO_OLD}: {config_file} requires war {req} ([project] requires_war) and this \
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
    let Ok(rd) = crate::vfs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk(&path, out),
            Ok(t) if t.is_file() => out.push(path),
            _ => {}
        }
    }
}

/// [`schemas_in`], remembered per file for the life of the process while the
/// file's length and modification time stand. A Warrant is loaded many times
/// by one command (`war admin compile` loads the corpus once per projection); the
/// answer for an unchanged file does not change between loads.
fn cached_schemas(file: &Utf8Path) -> Vec<(String, u32)> {
    type Seen = std::collections::HashMap<
        Utf8PathBuf,
        (u64, Option<std::time::SystemTime>, Vec<(String, u32)>),
    >;
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Seen>> = std::sync::OnceLock::new();
    // Every exported/hosted compilation must record its own content witnesses.
    // A path/mtime cache from a previous compilation cannot stand in for one.
    if content_schema_scope() || crate::vfs::is_hosted() {
        return schemas_in(file);
    }
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

/// The `(record, major)` each schema declaration in `file` names, for a
/// file that could name a newer major than this `war` reads (none for any
/// other). A file that does not parse declares nothing here; its own reader
/// reports it.
type SchemaFacts = Vec<(String, u32)>;
#[derive(Default)]
struct ContentSchemas {
    facts: std::collections::BTreeMap<(String, String), SchemaFacts>,
    weight: usize,
}
thread_local! {
    static CONTENT_SCHEMAS: std::cell::RefCell<Option<ContentSchemas>> = const { std::cell::RefCell::new(None) };
}

/// A fresh compilation's content observations; no fact crosses compilations.
pub(crate) fn with_content_schema_cache<T>(work: impl FnOnce() -> T) -> T {
    struct Restore(Option<ContentSchemas>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CONTENT_SCHEMAS.with(|cache| *cache.borrow_mut() = self.0.take());
        }
    }
    let _restore =
        Restore(CONTENT_SCHEMAS.with(|cache| cache.replace(Some(ContentSchemas::default()))));
    work()
}

fn content_schema_scope() -> bool {
    CONTENT_SCHEMAS.with(|cache| cache.borrow().is_some())
}

fn schemas_in(file: &Utf8Path) -> Vec<(String, u32)> {
    let ext = file.extension().unwrap_or_default();
    if !matches!(ext, "toml" | "json" | "jsonl" | "yaml" | "yml" | "md") {
        return vec![];
    }
    if content_schema_scope() {
        let Ok(fingerprint) = crate::vfs::fingerprint(file.as_std_path()) else {
            return vec![];
        };
        let key = (ext.to_owned(), fingerprint.clone());
        if let Some(facts) = CONTENT_SCHEMAS.with(|cache| {
            cache
                .borrow()
                .as_ref()
                .and_then(|cache| cache.facts.get(&key).cloned())
        }) {
            return facts;
        }
        let bytes = crate::vfs::known_content(file.as_std_path(), &fingerprint)
            .map(Ok)
            .unwrap_or_else(|| crate::vfs::read(file));
        let Ok(bytes) = bytes else { return vec![] };
        let facts = schemas_from_bytes(bytes, ext);
        let weight = key.0.len()
            + key.1.len()
            + 96
            + facts.iter().map(|(name, _)| name.len() + 40).sum::<usize>();
        CONTENT_SCHEMAS.with(|cache| {
            if let Some(cache) = cache.borrow_mut().as_mut()
                && cache.facts.len() < 50_000
                && cache.weight.saturating_add(weight) <= 16 * 1024 * 1024
            {
                cache.weight += weight;
                cache.facts.insert(key, facts.clone());
            }
        });
        return facts;
    }
    let Ok(bytes) = crate::vfs::read(file) else {
        return vec![];
    };
    schemas_from_bytes(bytes, ext)
}

fn schemas_from_bytes(bytes: Vec<u8>, ext: &str) -> Vec<(String, u32)> {
    // Only a newer major is ever reported, and parsing every record under a
    // Warrant to learn its schema was most of what loading one cost. A file
    // that cannot name a newer major declares nothing worth parsing for.
    if !may_name_newer(&bytes, ext) {
        return vec![];
    }
    let Ok(text) = String::from_utf8(bytes) else {
        return vec![];
    };
    let named = |s: &str| parse_schema(s).map(|(r, m)| (r.to_owned(), m));
    match ext {
        "toml" if known_toml_header(&text) => vec![],
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

/// Whether `text` could declare a record major newer than this `war` reads:
/// a superset of what [`schemas_in`]'s parse would find, from the bytes
/// alone. Every `oh.war/<record>/v<major>` spelled out, or for a journal
/// every `"v": <major>`, is read; an escape that could spell either without
/// its plain bytes (`\u`, `\U`, `\/`) sends the file to the parse. TOML
/// also admits hex escapes and line-ending backslashes, which can split a
/// family or major across physical lines.
fn may_name_newer(text: &[u8], ext: &str) -> bool {
    fn skip_ws(b: &[u8]) -> &[u8] {
        &b[b.iter().take_while(|c| c.is_ascii_whitespace()).count()..]
    }
    if memchr::memchr_iter(b'\\', text).any(|at| {
        text.get(at + 1).is_some_and(|next| {
            matches!(*next, b'u' | b'U' | b'/')
                || (ext == "toml" && (*next == b'x' || next.is_ascii_whitespace()))
        })
    }) {
        return true;
    }
    let newer = |record: &[u8], major: &[u8]| {
        let (Ok(record), Ok(major)) = (std::str::from_utf8(record), std::str::from_utf8(major))
        else {
            return false;
        };
        // A major too large for u32 is not read by the parse either.
        major
            .parse::<u32>()
            .is_ok_and(|m| known_major(record).is_some_and(|k| m > k))
    };
    let digits = |b: &[u8]| b.iter().take_while(|c| c.is_ascii_digit()).count();
    if ext == "jsonl" {
        for at in memchr::memmem::find_iter(text, b"\"v\"") {
            let Some(value) = skip_ws(&text[at + 3..]).strip_prefix(b":") else {
                continue;
            };
            let value = skip_ws(value);
            let n = digits(value);
            if n > 0 && newer(b"journal-event", &value[..n]) {
                return true;
            }
        }
        return false;
    }
    for at in memchr::memmem::find_iter(text, b"oh.war/") {
        let token = &text[at + b"oh.war/".len()..];
        let end = token
            .iter()
            .position(|c| {
                c.is_ascii_whitespace() || matches!(c, b'"' | b'\'' | b',' | b'}' | b']' | b'\\')
            })
            .unwrap_or(token.len());
        // `parse_schema`: the record is what precedes the last `/v`.
        let token = &token[..end];
        if let Some(slash) = memchr::memmem::rfind(token, b"/v") {
            let major = &token[slash + 2..];
            if newer(&token[..slash], major) {
                return true;
            }
        }
    }
    false
}

/// A complete first root assignment fixes TOML's schema: a later assignment
/// to that key would make the document invalid, not replace it. Returning no
/// finding for a known/irrelevant version is therefore the same for a valid
/// or malformed body. Newer frozen versions still take the full parser below,
/// preserving its handling of malformed documents. Tables, multiline strings
/// and unusual headers also fall back; this is an optimization, not a grammar.
fn known_toml_header(text: &str) -> bool {
    let Some(line) = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
    else {
        return false;
    };
    if line.len() > 1024 {
        return false;
    }
    let Ok(table) = toml::from_str::<toml::Table>(line) else {
        return false;
    };
    if table.len() != 1 {
        return false;
    }
    let Some(schema) = table.get("schema").and_then(toml::Value::as_str) else {
        return false;
    };
    !parse_schema(schema)
        .is_some_and(|(record, major)| known_major(record).is_some_and(|known| major > known))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_only_duplicates_record_one_complete_body() {
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "war-schema-content-{}",
                openwarrant_core::WarUuid::mint()
            ));
        std::fs::create_dir(&root).unwrap();
        let text = r#"{"schema":"oh.war/verification/v4"}"#;
        let first = root.join("first.json");
        let second = root.join("second.json");
        std::fs::write(&first, text).unwrap();
        std::fs::write(&second, text).unwrap();
        let recorder = std::rc::Rc::new(std::cell::RefCell::new(crate::vfs::Recorder {
            root: root.clone(),
            ..Default::default()
        }));
        let found = with_content_schema_cache(|| {
            crate::vfs::recording(std::rc::Rc::clone(&recorder), || {
                (schemas_in(&first), schemas_in(&second))
            })
        });
        assert_eq!(found.0, vec![("verification".to_owned(), 4)]);
        assert_eq!(found.0, found.1);
        let seen = recorder.borrow();
        assert_eq!(
            seen.files.len(),
            1,
            "equivalent schema facts need one complete content observation"
        );
        assert!(seen.seen.contains("second.json"));
        drop(seen);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hosted_schema_facts_do_not_make_withheld_bytes_readable() {
        let bytes = br#"{"schema":"oh.war/verification/v4"}"#.to_vec();
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes));
        let root = Utf8PathBuf::from("/schema-basis");
        let tree = std::rc::Rc::new(
            crate::vfs::Tree::new(
                root.clone(),
                [("available.json".into(), bytes)].into_iter().collect(),
                ["withheld.json".into()].into_iter().collect(),
            )
            .with_content_digests(
                [
                    ("available.json".into(), digest.clone()),
                    ("withheld.json".into(), digest),
                ]
                .into_iter()
                .collect(),
            ),
        );
        with_content_schema_cache(|| {
            crate::vfs::hosted(std::rc::Rc::clone(&tree), || {
                // Ask for the withheld path first: ordering cannot supply a fact.
                assert_eq!(
                    schemas_in(&root.join("withheld.json")),
                    vec![("verification".into(), 4)]
                );
                assert!(tree.consulted().withheld_read.is_empty());
                assert!(crate::vfs::read(root.join("withheld.json")).is_err());
                assert!(tree.consulted().withheld_read.contains("withheld.json"));
            })
        });
    }

    #[test]
    fn schema_facts_require_a_content_witness_in_each_compilation() {
        let bytes = br#"{"schema":"oh.war/verification/v4"}"#.to_vec();
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes));
        let root = Utf8PathBuf::from("/schema-isolation");
        let available = std::rc::Rc::new(crate::vfs::Tree::new(
            root.clone(),
            [("available.json".into(), bytes)].into_iter().collect(),
            Default::default(),
        ));
        with_content_schema_cache(|| {
            crate::vfs::hosted(available, || {
                assert_eq!(
                    schemas_in(&root.join("available.json")),
                    vec![("verification".into(), 4)]
                );
            })
        });
        let missing = std::rc::Rc::new(
            crate::vfs::Tree::new(
                root.clone(),
                Default::default(),
                ["withheld.json".into()].into_iter().collect(),
            )
            .with_content_digests([("withheld.json".into(), digest)].into_iter().collect()),
        );
        with_content_schema_cache(|| {
            crate::vfs::hosted(std::rc::Rc::clone(&missing), || {
                assert!(schemas_in(&root.join("withheld.json")).is_empty());
                assert!(missing.consulted().withheld_read.contains("withheld.json"));
            })
        });
    }

    #[test]
    fn changed_bytes_cannot_reuse_a_schema_fact() {
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "war-schema-changed-{}",
                openwarrant_core::WarUuid::mint()
            ));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("record.json");
        with_content_schema_cache(|| {
            std::fs::write(&file, br#"{"schema":"oh.war/verification/v4"}"#).unwrap();
            assert_eq!(schemas_in(&file), vec![("verification".into(), 4)]);
            std::fs::write(&file, br#"{"schema":"oh.war/verification/v5"}"#).unwrap();
            assert_eq!(schemas_in(&file), vec![("verification".into(), 5)]);
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn performance_cache_preserves_toml_schema_escapes() {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-compat-escaped-schema-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in [
            (
                "joined-family",
                "schema = \"\"\"oh.war/verifi\\\ncation/v4\"\"\"\n",
            ),
            (
                "joined-major",
                "schema = \"\"\"oh.war/verification/\\\nv4\"\"\"\n",
            ),
            (
                "spaced-crlf",
                "schema = \"\"\"oh.war/verifi\\ \t\r\ncation/v4\"\"\"\n",
            ),
            ("hex-prefix", "schema = \"\\x6fh.war/verification/v4\"\n"),
            ("hex-major", "schema = \"oh.war/verification/v\\x34\"\n"),
        ] {
            let parsed: toml::Value = toml::from_str(text).unwrap();
            assert_eq!(parsed["schema"].as_str(), Some("oh.war/verification/v4"));
            let file = dir.join(format!("{name}.toml"));
            std::fs::write(&file, text).unwrap();
            let found = schemas_in(&file);
            assert!(
                found
                    .iter()
                    .any(|(family, major)| family == "verification" && *major == 4),
                "{name}: future schema was lost: {found:?}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn only_a_file_that_could_name_a_newer_major_is_parsed() {
        // A current or unknown record is never parsed for.
        assert!(!may_name_newer(
            b"schema = \"oh.war/verification/v3\"\n",
            "toml"
        ));
        assert!(!may_name_newer(
            b"{\"schema\": \"oh.war/verification-request/v7\"}",
            "json"
        ));
        assert!(!may_name_newer(b"no schema here", "md"));
        // A newer frozen major is, however it is quoted or placed.
        assert!(may_name_newer(
            b"schema = \"oh.war/verification/v9\"\n",
            "toml"
        ));
        assert!(may_name_newer(b"schema = 'oh.war/atom/v2'\n", "toml"));
        assert!(may_name_newer(b"---\nschema: oh.war/atom/v2\n---\n", "md"));
        assert!(may_name_newer(b"{\"x\":[\"oh.war/report/v2\"]}", "json"));
        // An escape that could spell one is parsed rather than guessed at.
        assert!(may_name_newer(
            b"{\"schema\": \"oh.war\\/atom\\/v2\"}",
            "json"
        ));
        assert!(may_name_newer(b"schema = \"oh.w\\u0061r/atom/v2\"", "toml"));
        // A journal: each line's `v`.
        assert!(!may_name_newer(
            b"{\"v\":1,\"type\":\"x\"}\n{\"v\": 1}\n",
            "jsonl"
        ));
        assert!(may_name_newer(
            b"{\"v\":1}\n{\"v\" : 2,\"type\":\"x\"}\n",
            "jsonl"
        ));
        // What the parse would find, the prefilter lets through to it.
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf-8")
            .join(format!("war-compat-prefilter-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        for (name, body) in [
            ("a.toml", "schema = \"oh.war/verification/v9\"\n"),
            ("b.json", "{\"schema\":\"oh.war/atom/v2\"}"),
            ("c.jsonl", "{\"v\":1}\n{\"v\":2}\n"),
            ("d.md", "---\nschema: oh.war/atom/v5\n---\n"),
        ] {
            let file = dir.join(name);
            std::fs::write(&file, body).expect("write");
            assert!(
                schemas_in(&file)
                    .iter()
                    .any(|(r, m)| known_major(r).is_some_and(|k| *m > k)),
                "{name}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

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
        assert_eq!(known_major("verification"), Some(3));
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
