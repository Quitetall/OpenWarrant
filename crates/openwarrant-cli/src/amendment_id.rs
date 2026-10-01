// SPDX-License-Identifier: Apache-2.0
//! Amendment ids two branches cannot both mint (t-dc28).
//!
//! §31 amendment records live at `docs/warrants/<alias>/amendments/<id>.yaml`.
//! They were numbered `AM-001`, `AM-002`, … by whoever wrote the next one, so
//! two branches amending the same Warrant both wrote `AM-002.yaml`, and the
//! merge renumbered one by hand (OW-WAR-0137's AM-003 was drafted as AM-002).
//!
//! # The scheme
//!
//! A new amendment is `AM-<n>-<hash>`: `<n>` is one more than the highest
//! ordinal on the branch that writes it (the sequential number people read and
//! cite), and `<hash>` is 4 to 16 lowercase hex digits taken from a fresh
//! UUIDv7's SHA-256, lengthened only when a shorter one is taken in the same
//! directory. Two branches that both write amendment 4 write `AM-004-1f3a`
//! and `AM-004-c07e`: different files, so the merge has nothing to resolve.
//!
//! Every record written before this — `AM-<n>` with no hash — keeps its name
//! and its bytes. Those files are cited by signed contracts, and nothing here
//! reads them differently: their order is their number, as it always was.
//!
//! # Order
//!
//! Amendments are ordered by ordinal `<n>`, then by the record's unindented
//! `effective_time:` (as written; RFC 3339 dates and times sort as strings),
//! then by file name. For a directory whose ordinals are distinct — every
//! directory before this scheme — that is exactly the old order. Two records
//! with the same ordinal from two branches fall to `effective_time`, and to
//! the name (the hash) only when both say the same moment: deterministic,
//! whichever branch merged first. A file whose name is not an amendment id
//! sorts after every well-formed one, and `war check` refuses it by name
//! (`amendment.id`).
//!
//! # What is a well-formed id
//!
//! `AM-` then 1 to 6 digits whose value is at least 1, then optionally `-`
//! and 4 to 16 lowercase hex digits. The record's own `id:` must equal its
//! file stem. Anything else — `AM-2x`, `AM-000`, `AM-004-C07E`, `am-004`,
//! `amendment.yaml` — is named, never silently read as some other number.

use std::cmp::Ordering;

use camino::{Utf8Path, Utf8PathBuf};

/// A parsed amendment id: the ordinal people read, and the collision-proof
/// suffix a branch-minted id carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmendmentId {
    /// The sequential number, `4` in `AM-004-1f3a`.
    pub ordinal: u64,
    /// The hash, `1f3a` in `AM-004-1f3a`; `None` for a record written before
    /// the scheme (`AM-004`).
    pub suffix: Option<String>,
}

impl AmendmentId {
    /// Parse `AM-<n>` or `AM-<n>-<hex>`, refusing anything else with the reason.
    pub fn parse(s: &str) -> Result<Self, String> {
        let why = |detail: &str| {
            format!(
                "{s:?} is not an amendment id ({detail}); an id is AM-<n> or AM-<n>-<hash>: \
                 1-6 digits, then optionally 4-16 lowercase hex digits"
            )
        };
        let rest = s.strip_prefix("AM-").ok_or_else(|| why("no AM- prefix"))?;
        let (number, suffix) = match rest.split_once('-') {
            Some((n, h)) => (n, Some(h)),
            None => (rest, None),
        };
        if number.is_empty() || number.len() > 6 || !number.bytes().all(|b| b.is_ascii_digit()) {
            return Err(why("the ordinal is not 1-6 digits"));
        }
        let ordinal: u64 = number.parse().map_err(|_| why("the ordinal"))?;
        if ordinal == 0 {
            return Err(why("the ordinal is 0; amendments count from 1"));
        }
        if let Some(h) = suffix
            && (!(4..=16).contains(&h.len())
                || !h
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        {
            return Err(why("the hash is not 4-16 lowercase hex digits"));
        }
        Ok(Self {
            ordinal,
            suffix: suffix.map(str::to_owned),
        })
    }
}

/// One file under `amendments/`, with what its name says.
#[derive(Debug, Clone)]
pub struct AmendmentFile {
    pub path: Utf8PathBuf,
    /// The file stem, which a well-formed record repeats as its `id:`.
    pub stem: String,
    /// The parsed stem, or why it is not an amendment id.
    pub id: Result<AmendmentId, String>,
    /// The unindented `effective_time:` value, empty when absent or unreadable.
    pub effective_time: String,
}

impl AmendmentFile {
    fn key_cmp(&self, other: &Self) -> Ordering {
        match (&self.id, &other.id) {
            (Ok(a), Ok(b)) => a
                .ordinal
                .cmp(&b.ordinal)
                .then_with(|| self.effective_time.cmp(&other.effective_time))
                .then_with(|| self.path.cmp(&other.path)),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => self.path.cmp(&other.path),
        }
    }
}

/// Every `.yaml`/`.yml` under `<warrant>/amendments/`, oldest first in the
/// order the module doc states. Empty when the directory does not exist.
#[must_use]
pub fn files(warrant_dir: &Utf8Path) -> Vec<AmendmentFile> {
    let Ok(entries) = warrant_dir.join("amendments").read_dir_utf8() else {
        return Vec::new();
    };
    let mut out: Vec<AmendmentFile> = entries
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .map(|path| {
            let stem = path.file_stem().unwrap_or_default().to_owned();
            let effective_time = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| {
                    text.lines().find_map(|l| {
                        l.strip_prefix("effective_time:")
                            .map(|v| v.trim().trim_matches('"').trim().to_owned())
                    })
                })
                .unwrap_or_default();
            AmendmentFile {
                id: AmendmentId::parse(&stem),
                stem,
                path,
                effective_time,
            }
        })
        .collect();
    out.sort_by(AmendmentFile::key_cmp);
    out
}

/// The id a new amendment to the Warrant at `warrant_dir` takes:
/// `AM-<max+1, at least 3 digits>-<hash>`, the hash lengthened past any id
/// already in the directory.
#[must_use]
pub fn mint(warrant_dir: &Utf8Path) -> String {
    let existing = files(warrant_dir);
    let next = existing
        .iter()
        .filter_map(|f| f.id.as_ref().ok().map(|i| i.ordinal))
        .max()
        .unwrap_or(0)
        + 1;
    let taken: std::collections::BTreeSet<&str> =
        existing.iter().map(|f| f.stem.as_str()).collect();
    let seed = openwarrant_core::WarUuid::mint().to_string();
    let hex = openwarrant_compiler::sha256_hex(seed.as_bytes());
    (4..=16)
        .map(|len| format!("AM-{next:03}-{}", &hex[..len]))
        .find(|id| !taken.contains(id.as_str()))
        .unwrap_or_else(|| format!("AM-{next:03}-{}", &hex[..16]))
}

/// `war amend <alias>`: write the skeleton of the next amendment under a
/// minted id, never over an existing file. The skeleton is deliberately not a
/// valid §31 record — its reason, authority, instruction, authorizer and
/// semantic diff are empty — so `war check` refuses it (`amendment.invalid`)
/// until a person or agent writes what changed and why. Refused for a resolved
/// Warrant: the resolution binds the contract as it was (§56.3).
pub fn amend(
    repo: &crate::repo::Repository,
    alias: &str,
    dry_run: bool,
) -> Result<crate::diagnostic::Report, crate::repo::RepoError> {
    use crate::diagnostic::{Diagnostic, Report};
    let mut report = Report::default();
    let dir = repo.warrant_dir(alias)?;
    if repo.load_resolution(&dir)?.is_some() {
        report.push(Diagnostic::error(
            "amend.resolved",
            repo.relative(&dir.join("resolution.toml")),
            format!(
                "{alias}: resolved; the resolution binds the contract as it was (§56.3), so \
                 there is nothing to amend. Nothing written"
            ),
        ));
        return Ok(report);
    }
    let id = mint(&dir);
    let path = dir.join("amendments").join(format!("{id}.yaml"));
    let rel = repo.relative(&path);
    if dry_run {
        report.push(Diagnostic::pass(
            "amend.would-write",
            format!("{alias}: {rel} (amendment {id}); nothing written"),
        ));
        return Ok(report);
    }
    let today = crate::gate_cmd::receipt::rfc3339_from_secs(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    );
    let today = today.get(..10).unwrap_or(&today).to_owned();
    std::fs::create_dir_all(dir.join("amendments")).map_err(|source| {
        crate::repo::RepoError::Io {
            context: format!("could not create {dir}/amendments"),
            source,
        }
    })?;
    let text = skeleton(&id, &today);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| crate::repo::RepoError::Io {
            context: format!("could not create {path} (never overwritten)"),
            source,
        })?;
    std::io::Write::write_all(&mut file, text.as_bytes()).map_err(|source| {
        crate::repo::RepoError::Io {
            context: format!("could not write {path}"),
            source,
        }
    })?;
    report.push(Diagnostic::pass(
        "amend.written",
        format!(
            "{alias}: {rel} — fill in reason, governing_adr_or_policy, \
             restart_or_repair_instruction, authorizer and semantic_diff; `war check {alias}` \
             refuses it until they are"
        ),
    ));
    Ok(report)
}

/// The skeleton `war amend` writes: every §31 field present, the ones only a
/// person can say left empty.
fn skeleton(id: &str, today: &str) -> String {
    format!(
        "schema: \"oh.war/amendment/v1\"\n\
         \n\
         id: {id:?}\n\
         band: \"manual_revision\"\n\
         reason: \"\"\n\
         governing_adr_or_policy: \"\"\n\
         artifact_admissibility: \"remain_admissible\"\n\
         restart_or_repair_instruction: \"\"\n\
         re_preflight_required: \"false\"\n\
         authorizer: \"\"\n\
         effective_time: {today:?}\n\
         \n\
         semantic_diff: []\n\
         \n\
         affected_stages: []\n\
         affected_milestones: []\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skeleton_parses_and_is_refused_until_filled() {
        let text = skeleton("AM-004-1f3a", "2026-09-25");
        let doc = openwarrant_core::structured::parse(&text).expect("parses");
        let r = openwarrant_core::autonomy::from_structured(&doc);
        assert!(r.is_err(), "an empty skeleton must not validate: {r:?}");
        assert!(text.contains("id: \"AM-004-1f3a\"\n"));
    }

    fn scratch(tag: &str) -> Utf8PathBuf {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-amid-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("amendments")).unwrap();
        dir
    }

    fn put(dir: &Utf8Path, stem: &str, time: &str) {
        std::fs::write(
            dir.join("amendments").join(format!("{stem}.yaml")),
            format!("id: \"{stem}\"\neffective_time: \"{time}\"\n"),
        )
        .unwrap();
    }

    #[test]
    fn legacy_and_hashed_ids_parse() {
        assert_eq!(
            AmendmentId::parse("AM-003").unwrap(),
            AmendmentId {
                ordinal: 3,
                suffix: None
            }
        );
        let h = AmendmentId::parse("AM-004-1f3a").unwrap();
        assert_eq!((h.ordinal, h.suffix.as_deref()), (4, Some("1f3a")));
        assert_eq!(AmendmentId::parse("AM-1000").unwrap().ordinal, 1000);
    }

    #[test]
    fn a_malformed_id_is_refused_with_its_name() {
        for bad in [
            "AM-2x",
            "AM-000",
            "AM-",
            "am-004",
            "AM-004-C07E",
            "AM-004-abc",
            "AM-004-xyzw",
            "AM-1234567",
            "amendment",
            "AM-004-1f3a-extra",
        ] {
            let e = AmendmentId::parse(bad).expect_err(bad);
            assert!(e.contains(&format!("{bad:?}")), "{bad}: {e}");
        }
    }

    #[test]
    fn two_independent_mints_differ_and_both_load_in_order() {
        // Two branches from the same base: each sees AM-001..AM-003 and mints.
        let a = scratch("a");
        let b = scratch("b");
        for d in [&a, &b] {
            put(d, "AM-001", "2026-09-01");
            put(d, "AM-002", "2026-09-02");
            put(d, "AM-003", "2026-09-03");
        }
        let id_a = mint(&a);
        let id_b = mint(&b);
        assert!(id_a.starts_with("AM-004-") && id_b.starts_with("AM-004-"));
        assert_ne!(id_a, id_b, "two branches minted the same file name");
        // The merge: both land in one directory, nothing to resolve.
        put(&a, &id_a, "2026-09-25T10:00:00Z");
        put(&a, &id_b, "2026-09-25T09:00:00Z");
        let order: Vec<String> = files(&a).into_iter().map(|f| f.stem).collect();
        assert_eq!(order, ["AM-001", "AM-002", "AM-003", &id_b, &id_a]);
        assert_eq!(mint(&a).get(..7), Some("AM-005-"));
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }

    #[test]
    fn a_legacy_corpus_keeps_its_order_and_malformed_names_sort_last() {
        let d = scratch("legacy");
        // Effective times deliberately out of order: the ordinal decides.
        put(&d, "AM-010", "2026-01-01");
        put(&d, "AM-002", "2026-12-01");
        put(&d, "AM-1000", "2025-01-01");
        put(&d, "AM-2x", "2020-01-01");
        let order: Vec<String> = files(&d).into_iter().map(|f| f.stem).collect();
        assert_eq!(order, ["AM-002", "AM-010", "AM-1000", "AM-2x"]);
        assert_eq!(mint(&d).get(..8), Some("AM-1001-"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_missing_directory_mints_the_first() {
        let d = scratch("none");
        let _ = std::fs::remove_dir_all(d.join("amendments"));
        assert!(files(&d).is_empty());
        assert!(mint(&d).starts_with("AM-001-"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
