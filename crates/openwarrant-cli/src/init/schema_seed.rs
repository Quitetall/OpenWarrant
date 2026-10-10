// SPDX-License-Identifier: Apache-2.0
//! Retain the build's checked schema sources at setup. This grants no authority.
use super::InitError;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write};

const PACK: &[u8] = include_bytes!("../../../../schemas/pack.json");
macro_rules! member {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!("../../../../schemas/oh.war/", $name, "/v1.json")).as_slice(),
        )
    };
}
const MEMBERS: &[(&str, &[u8])] = &[
    member!("atom"),
    member!("authorization"),
    member!("bonsai-evidence"),
    member!("correction"),
    member!("deliverables"),
    member!("impact"),
    member!("journal-event"),
    member!("judgments"),
    member!("ledger-file"),
    member!("liminal-request"),
    member!("liminal-response"),
    member!("manifest"),
    member!("milestones"),
    member!("model"),
    member!("projection"),
    member!("report"),
    member!("resolution"),
    member!("roadmap"),
    member!("score"),
    member!("score-statement"),
    member!("stage-dispatch"),
    member!("stage-submission"),
    member!("standing-authorization"),
    member!("verification"),
    member!("war"),
];

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    schema: String,
    id: String,
    version: String,
    files: BTreeMap<String, String>,
    transitive_digest: String,
}

fn refused(message: impl Into<String>) -> InitError {
    InitError::Io {
        context: "schema pack setup refused".into(),
        source: std::io::Error::other(message.into()),
    }
}

pub(super) struct Seed(Vec<(Utf8PathBuf, &'static [u8])>);

impl Seed {
    /// Check the bundled pack and every destination before init changes the tree.
    pub(super) fn prepare(root: &Utf8Path) -> Result<Self, InitError> {
        let pack: Pack = serde_json::from_slice(PACK).map_err(|e| refused(e.to_string()))?;
        let canonical =
            openwarrant_compiler::to_canonical_bytes(&pack).map_err(|e| refused(e.to_string()))?;
        let digests: BTreeMap<_, _> = MEMBERS
            .iter()
            .map(|(name, bytes)| ((*name).to_owned(), openwarrant_compiler::sha256_hex(bytes)))
            .collect();
        let preimage: String = digests
            .iter()
            .map(|(name, digest)| format!("{name}:{digest}\n"))
            .collect();
        if PACK.strip_suffix(b"\n").unwrap_or(PACK) != canonical
            || pack.schema != "oh.war/schema-pack/v1"
            || pack.id != openwarrant_compiler::SCHEMA_PACK_ID
            || pack.version != openwarrant_compiler::SCHEMA_PACK_VERSION
            || pack.files != digests
            || pack.transitive_digest != openwarrant_compiler::sha256_hex(preimage.as_bytes())
        {
            return Err(refused(
                "bundled schema pack differs from checked member bytes",
            ));
        }
        let mut files = vec![(root.join("schemas/pack.json"), PACK)];
        files.extend(
            MEMBERS
                .iter()
                .map(|(name, bytes)| (root.join(format!("schemas/oh.war/{name}/v1.json")), *bytes)),
        );
        for (path, bytes) in &files {
            for parent in path.ancestors().skip(1).take_while(|p| *p != root) {
                match fs::symlink_metadata(parent) {
                    Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
                        return Err(refused(format!(
                            "existing schema directory is not a plain directory: {parent}"
                        )));
                    }
                    Ok(_) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(refused(format!("cannot inspect {parent}: {e}"))),
                }
            }
            match fs::symlink_metadata(path) {
                Ok(meta) => {
                    if !meta.is_file()
                        || meta.file_type().is_symlink()
                        || meta.len() != bytes.len() as u64
                        || fs::read(path)
                            .map_err(|e| refused(format!("cannot read {path}: {e}")))?
                            != *bytes
                    {
                        return Err(refused(format!(
                            "existing schema source differs; preserved without overwrite: {path}"
                        )));
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(refused(format!("cannot inspect {path}: {e}"))),
            }
        }
        Ok(Self(files))
    }

    /// New files only. Existing identical sources were checked, never rewritten.
    pub(super) fn install(self) -> Result<(), InitError> {
        for (path, bytes) in self.0 {
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.is_file()
                    && !meta.file_type().is_symlink()
                    && meta.len() == bytes.len() as u64
                    && fs::read(&path).map_err(|e| refused(format!("cannot read {path}: {e}")))?
                        == bytes
                {
                    continue;
                }
                return Err(refused(format!(
                    "schema source changed during setup; preserved without overwrite: {path}"
                )));
            }
            fs::create_dir_all(path.parent().expect("schema paths have a parent"))
                .map_err(|e| refused(format!("cannot create schema directory: {e}")))?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| refused(format!("cannot create {path} without overwriting: {e}")))?;
            file.write_all(bytes)
                .map_err(|e| refused(format!("cannot write {path}: {e}")))?;
        }
        Ok(())
    }
}
