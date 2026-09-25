// SPDX-License-Identifier: Apache-2.0

//! `war schemas` (OW-WAR-0032): the JSON Schema pack, generated from the
//! record types and drift-checked like every other projection.
//!
//! One file per record under `schemas/oh.war/<record>/v1.json`, plus
//! `schemas/pack.json` with each file's sha256 and a transitive digest over
//! them. The pack's `version` is `SCHEMA_PACK_VERSION` — the value inside
//! every compiled contract's `format_basis`, which is inside every contract
//! digest, so it moves only with a deliberate format change, never with a
//! crate release. A hand-edited schema is the same defect class as a
//! hand-edited parent: `--check` reports it by file.
//!
//! Behind the `schema` cargo feature: the shipped binary carries no schema
//! generator; `cargo xtask gate` builds this on to check the pack.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const PACK_SCHEMA: &str = "oh.war/schema-pack/v1";
pub const PACK_DIR: &str = "schemas";

/// One schema in the pack: its record name and the type it is generated from.
struct Entry {
    record: &'static str,
    schema: fn(&mut SchemaGenerator) -> schemars::Schema,
}

fn entry<T: JsonSchema>(record: &'static str) -> Entry {
    Entry {
        record,
        schema: |g| g.root_schema_for::<T>(),
    }
}

/// The pack's members. Names are the `oh.war/<record>/v1` families the
/// records themselves declare in their `schema` fields.
fn entries() -> Vec<Entry> {
    vec![
        entry::<openwarrant_core::manifest::Manifest>("manifest"),
        entry::<openwarrant_core::frontmatter::Frontmatter>("atom"),
        entry::<openwarrant_core::milestones::MilestoneGraph>("milestones"),
        entry::<crate::repo::DeliverableSet>("deliverables"),
        entry::<crate::authorize::AuthorizationRecord>("authorization"),
        entry::<crate::authorize::JudgmentRecord>("judgments"),
        entry::<crate::resolution_cmd::ResolutionRecord>("resolution"),
        entry::<openwarrant_core::verification::Verification>("verification"),
        entry::<openwarrant_core::correction::Correction>("correction"),
        // `sas-revision` likewise: core/sas.rs is pinned by resolved OW-WAR-0062,
        // the corrections fixture. Both schemas land with those corrections.
        // `amendment` is deliberately absent: its type lives in autonomy.rs,
        // pinned by resolved OW-WAR-0010 — the battery's positive fixture for
        // the thirteen — and a derive line there would drift the fixture until
        // the owner signs a correction. The schema lands with that correction.
        entry::<openwarrant_core::journal::JournalEvent>("journal-event"),
        entry::<openwarrant_core::execution::StageDispatch>("stage-dispatch"),
        entry::<openwarrant_core::execution::StageSubmission>("stage-submission"),
        entry::<crate::output::Envelope<'static>>("report"),
        entry::<crate::bonsai::BonsaiEvidence>("bonsai-evidence"),
        entry::<openwarrant_compiler::WarIr>("war"),
    ]
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Pack {
    pub schema: String,
    pub id: String,
    pub version: String,
    /// `<record>` → sha256 of `schemas/oh.war/<record>/v1.json`.
    pub files: BTreeMap<String, String>,
    /// sha256 over the sorted `<record>:<sha256>\n` lines: one digest a
    /// consumer pins instead of seventeen.
    pub transitive_digest: String,
}

/// Each root gets its own generator on purpose: every file is self-contained
/// (its referenced types inlined under `$defs`), so a consumer can take one
/// schema without the pack. Shared sub-types therefore appear in more than
/// one file; that is the trade, not an accident.
fn render_all() -> Result<(BTreeMap<String, String>, Pack), RepoError> {
    let settings = SchemaSettings::draft2020_12();
    let mut files = BTreeMap::new();
    for e in entries() {
        let mut g = SchemaGenerator::new(settings.clone());
        let mut schema = (e.schema)(&mut g);
        let id = format!("oh.war/{}/v1", e.record);
        if let Some(obj) = schema.as_object_mut() {
            obj.insert("$id".to_owned(), serde_json::Value::String(id.clone()));
        }
        // RFC 8785 canonical bytes, like every other committed projection:
        // key order is the standard's, not the generator's, so the file is the
        // same on every host and under every serde_json feature set.
        let text = serde_jcs::to_string(&schema)
            .map_err(|err| RepoError::Message(format!("could not render {id}: {err}")))?;
        files.insert(e.record.to_owned(), format!("{text}\n"));
    }
    let digests: BTreeMap<String, String> = files
        .iter()
        .map(|(k, v)| (k.clone(), openwarrant_compiler::sha256_hex(v.as_bytes())))
        .collect();
    let lines: String = digests.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    let pack = Pack {
        schema: PACK_SCHEMA.to_owned(),
        id: openwarrant_compiler::SCHEMA_PACK_ID.to_owned(),
        version: openwarrant_compiler::SCHEMA_PACK_VERSION.to_owned(),
        files: digests,
        transitive_digest: openwarrant_compiler::sha256_hex(lines.as_bytes()),
    };
    Ok((files, pack))
}

fn pack_text(pack: &Pack) -> Result<String, RepoError> {
    let canonical = serde_jcs::to_string(pack)
        .map_err(|err| RepoError::Message(format!("could not canonicalize pack.json: {err}")))?;
    Ok(format!("{canonical}\n"))
}

fn file_path(root: &Utf8Path, record: &str) -> Utf8PathBuf {
    root.join(PACK_DIR)
        .join("oh.war")
        .join(record)
        .join("v1.json")
}

/// Write the pack (`war schemas`), or compare it to the tree (`--check`).
pub fn run(repo: &Repository, check: bool) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (files, pack) = render_all()?;
    let pack_path = repo.root.join(PACK_DIR).join("pack.json");
    let pack_body = pack_text(&pack)?;
    let mut drift = 0usize;
    let mut compare = |path: &Utf8Path, want: &str, report: &mut Report| {
        match std::fs::read_to_string(path) {
            Ok(have) if have == want => {}
            Ok(_) => {
                drift += 1;
                report.push(Diagnostic::error(
                    "schemas.drift",
                    repo.relative(path),
                    "differs from what the types generate; run `war schemas` (cargo feature `schema`) and commit the result".to_owned(),
                ));
            }
            Err(_) => {
                drift += 1;
                report.push(Diagnostic::error(
                    "schemas.missing",
                    repo.relative(path),
                    "not in the tree; run `war schemas` and commit the result".to_owned(),
                ));
            }
        }
    };
    if check {
        for (record, text) in &files {
            compare(&file_path(&repo.root, record), text, &mut report);
        }
        compare(&pack_path, &pack_body, &mut report);
        if drift == 0 {
            report.push(Diagnostic::pass(
                "schemas.current",
                format!(
                    "{} schema(s) and pack.json match the types; pack {} {} transitive {}",
                    files.len(),
                    pack.id,
                    pack.version,
                    &pack.transitive_digest[..12]
                ),
            ));
        }
        return Ok(report);
    }
    for (record, text) in &files {
        let path = file_path(&repo.root, record);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                context: format!("could not create {parent}"),
                source,
            })?;
        }
        std::fs::write(&path, text).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    }
    std::fs::write(&pack_path, &pack_body).map_err(|source| RepoError::Io {
        context: format!("could not write {pack_path}"),
        source,
    })?;
    report.push(Diagnostic::pass(
        "schemas.written",
        format!(
            "{} schema(s) under {PACK_DIR}/oh.war/ and pack.json; pack {} {} transitive {}",
            files.len(),
            pack.id,
            pack.version,
            &pack.transitive_digest[..12]
        ),
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_schema_renders_and_the_pack_is_deterministic() {
        let (a, pa) = render_all().unwrap();
        let (b, pb) = render_all().unwrap();
        assert_eq!(a, b);
        assert_eq!(pa.transitive_digest, pb.transitive_digest);
        assert_eq!(a.len(), 15);
        for (record, text) in &a {
            let v: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(v["$id"], format!("oh.war/{record}/v1"));
            assert!(v.get("$schema").is_some(), "{record} names its draft");
        }
    }

    #[test]
    fn the_pack_names_the_version_inside_every_contract_digest() {
        let (_, p) = render_all().unwrap();
        assert_eq!(p.version, openwarrant_compiler::SCHEMA_PACK_VERSION);
        assert_eq!(p.id, openwarrant_compiler::SCHEMA_PACK_ID);
    }
}
