// SPDX-License-Identifier: Apache-2.0
//! Exact candidate data, exported from Git objects into one private temporary
//! tree. No checkout, hook, filter, export-subst, executable or signing runs.
//! One batch reader avoids starting a Git process for each source file.
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Component,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use camino::{Utf8Path, Utf8PathBuf};

use crate::repo::Repository;

pub(super) struct Snapshot {
    pub repository: Repository,
    pub history_root: Utf8PathBuf,
    _temporary: Temporary,
}

struct Temporary(Utf8PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl Temporary {
    fn create() -> Result<Self, String> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ow-candidate-{}-{nonce}", std::process::id()));
        let path = Utf8PathBuf::from_path_buf(path).map_err(|_| "non-UTF-8 temporary path")?;
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        // Ownership begins only after exclusive creation succeeds. A colliding
        // directory is never reused or removed by this guard.
        builder.create(&path).map_err(|e| e.to_string())?;
        Ok(Self(path))
    }
}

struct Blobs {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Drop for Blobs {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Blobs {
    fn open(root: &Utf8Path) -> Result<Self, String> {
        let mut child = Command::new("git")
            .args(["cat-file", "--batch"])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().expect("piped Git stdin");
        let output = BufReader::new(child.stdout.take().expect("piped Git stdout"));
        Ok(Self {
            child,
            input,
            output,
        })
    }

    fn copy(&mut self, object: &str, path: &Utf8Path) -> Result<(), String> {
        writeln!(self.input, "{object}").map_err(|e| e.to_string())?;
        self.input.flush().map_err(|e| e.to_string())?;
        let mut header = String::new();
        self.output
            .read_line(&mut header)
            .map_err(|e| e.to_string())?;
        let fields: Vec<_> = header.split_whitespace().collect();
        if fields.len() != 3 || fields[0] != object || fields[1] != "blob" {
            return Err(format!("Git could not read source object {object}"));
        }
        let size: u64 = fields[2].parse().map_err(|_| "invalid Git blob length")?;
        let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
        let copied = std::io::copy(&mut self.output.by_ref().take(size), &mut file)
            .map_err(|e| e.to_string())?;
        if copied != size {
            return Err("incomplete candidate source object".to_owned());
        }
        let mut newline = [0];
        self.output
            .read_exact(&mut newline)
            .map_err(|e| e.to_string())?;
        if newline != *b"\n" {
            return Err("invalid Git batch framing".to_owned());
        }
        Ok(())
    }
}

impl Snapshot {
    pub fn read(root: &Utf8Path, candidate: &str) -> Result<Self, String> {
        let listing = super::git(root, &["ls-tree", "-rz", "--full-tree", candidate])
            .ok_or("Git could not enumerate the candidate tree")?;
        let temporary = Temporary::create()?;
        let mut blobs = Blobs::open(root)?;
        let mut seen = std::collections::BTreeSet::new();
        for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let separator = entry
                .iter()
                .position(|b| *b == b'\t')
                .ok_or("invalid Git tree entry")?;
            let (metadata, tail) = entry.split_at(separator);
            let name = &tail[1..];
            let metadata = std::str::from_utf8(metadata).map_err(|_| "invalid Git metadata")?;
            let fields: Vec<_> = metadata.split_whitespace().collect();
            if fields.len() != 3 {
                return Err("invalid Git tree metadata".to_owned());
            }
            let relative =
                Utf8Path::new(std::str::from_utf8(name).map_err(|_| "non-UTF-8 candidate path")?);
            if relative
                .components()
                .any(|p| !matches!(p, camino::Utf8Component::Normal(_)))
                || relative
                    .as_std_path()
                    .components()
                    .any(|p| !matches!(p, Component::Normal(_)))
            {
                return Err("candidate source path is not relative without traversal".to_owned());
            }
            if !seen.insert(relative.to_owned()) {
                return Err("ambiguous duplicate candidate source path".to_owned());
            }
            // Never create symlinks or submodule entries. Required symlinked
            // sources were already refused by subject capture; omitting their
            // unsafe nodes cannot match an earlier regular-file review.
            if !matches!(fields[0], "100644" | "100755") || fields[1] != "blob" {
                continue;
            }
            if !fields[2].bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("invalid Git source identity".to_owned());
            }
            let path = temporary.0.join(relative);
            fs::create_dir_all(path.parent().ok_or("candidate source has no parent")?)
                .map_err(|e| e.to_string())?;
            blobs.copy(fields[2], &path)?;
        }
        // Candidate configuration is data. In particular, never activate a
        // protected authority store selected by an untrusted Git candidate.
        let config_path = temporary.0.join(crate::init::CONFIG_FILE);
        let config: openwarrant_core::RepositoryConfig =
            toml::from_str(&fs::read_to_string(&config_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        config.validate().map_err(|e| e.to_string())?;
        crate::repo::compat::check(&config, &config_path)?;
        for path in [
            &config.paths.sas,
            &config.paths.roadmap,
            &config.paths.adrs,
            &config.paths.warrants,
            &config.paths.gates,
            &config.paths.receipts,
        ] {
            if !within_tree(path) {
                return Err("candidate document path escapes its source tree".to_owned());
            }
        }
        let repository = Repository {
            root: temporary.0.clone(),
            config,
            profiles: crate::repo::load_profiles(&temporary.0).map_err(|e| e.to_string())?,
        };
        for dir in repository.warrant_dirs().map_err(|e| e.to_string())? {
            let manifest: openwarrant_core::Manifest = toml::from_str(
                &fs::read_to_string(dir.join("manifest.toml")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if manifest
                .atoms
                .iter()
                .filter_map(|a| a.path.as_deref())
                .any(|path| !within_tree(path))
            {
                return Err("candidate atom path escapes its source tree".to_owned());
            }
        }
        Ok(Self {
            repository,
            history_root: root.to_owned(),
            _temporary: temporary,
        })
    }
}

/// No data reference may turn the private snapshot into a host-file reader.
fn within_tree(path: &str) -> bool {
    !path.is_empty()
        && std::path::Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}
