// SPDX-License-Identifier: Apache-2.0
//! `CLAUDE.md` and `AGENTS.md`: the managed pointer block, and their
//! sections as `instruction` records (M16, decision 13).
//!
//! # The block
//!
//! `war agents-md --block` keeps one small block, between
//! `<!-- openwarrant:begin -->` and `<!-- openwarrant:end -->`, in the
//! repository's instruction files. It says that ordinary coding needs no
//! Warrant and that `war prime` shows tracked work, and it carries the version
//! stamp [`crate::skew`] reads. It never holds the active Warrant's context:
//! the file stays stable, and what changes lives in `war prime`.
//!
//! - Which files: every root `AGENTS.md` and `CLAUDE.md` that exists, or
//!   `AGENTS.md` alone when neither does; `--file` names others.
//! - Idempotent: a second run changes nothing. Bytes outside the markers are
//!   never touched ([`openwarrant_core::instruction::upsert`]).
//! - Refused, each by rule and line, with nothing written to any file: a
//!   second block, a block that never closes, an end marker with no begin,
//!   and a file whose last fence never closes.
//!
//! `war init` writes the full AGENTS.md template (which ends with the block)
//! when there is none, and adds the block to an existing AGENTS.md or
//! CLAUDE.md. `war doctor` reports a block that is missing, stale (another
//! version's, or edited), or malformed.
//!
//! # Sections as records
//!
//! The root `CLAUDE.md` and `AGENTS.md`, and the nested files
//! `[instructions] nested` globs match, are read without being edited: each
//! `##` section is a record `md:<file>#<slug>` of type `instruction`, its
//! revision the digest of its byte span ([`openwarrant_core::instruction`]).
//! The block is no section. Relations cite a section by id
//! (`constrains md:CLAUDE.md#testing`), `war model` lists them, and
//! `war impact md:CLAUDE.md#testing` names what cites one.

use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};

use openwarrant_core::instruction::{self, BlockError, Change};

use crate::diagnostic::{Diagnostic, Report};
use crate::records::{Fault, Record};
use crate::repo::Repository;

/// The root instruction files, in the order they are read.
pub const ROOT_FILES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

/// The version this `war` stamps.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Directories a `**` walk never enters: a build or dependency tree holds no
/// instruction of this repository, and walking one is what makes a check slow.
fn skipped(name: &str) -> bool {
    name.starts_with('.') || name == "target" || name == "node_modules"
}

fn children(dir: &Utf8Path) -> Vec<(String, bool)> {
    let Ok(rd) = crate::vfs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, bool)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let ft = e.file_type().ok()?;
            let name = e.file_name().into_string().ok()?;
            // `file_type` does not follow a symbolic link: a link is neither.
            (ft.is_dir() || ft.is_file()).then_some((name, ft.is_dir()))
        })
        .collect();
    out.sort();
    out
}

fn expand(dir: &Utf8Path, segs: &[&str], out: &mut BTreeSet<Utf8PathBuf>) {
    match segs.split_first() {
        None => {}
        Some((&"**", rest)) => {
            expand(dir, rest, out);
            for (name, is_dir) in children(dir) {
                if is_dir && !skipped(&name) {
                    expand(&dir.join(&name), segs, out);
                }
            }
        }
        Some((seg, rest)) => {
            for (name, is_dir) in children(dir) {
                if !crate::acceptance::glob_matches(seg, &name) {
                    continue;
                }
                if rest.is_empty() && !is_dir {
                    out.insert(dir.join(&name));
                } else if !rest.is_empty() && is_dir {
                    expand(&dir.join(&name), rest, out);
                }
            }
        }
    }
}

/// The instruction files of `repo`: the root `AGENTS.md` and `CLAUDE.md` that
/// exist, then every file a `[instructions] nested` glob matches. Sorted by
/// path, each once.
#[must_use]
pub fn files(repo: &Repository) -> Vec<Utf8PathBuf> {
    files_in(&repo.root, &repo.config.instructions.nested)
}

/// [`files`] for a root and its `nested` globs. A glob that is absolute or
/// climbs out with `..` matches nothing.
#[must_use]
pub fn files_in(root: &Utf8Path, nested: &[String]) -> Vec<Utf8PathBuf> {
    let mut out: BTreeSet<Utf8PathBuf> = ROOT_FILES
        .iter()
        .map(|f| root.join(f))
        .filter(|p| crate::vfs::is_file(p))
        .collect();
    for pattern in nested {
        let p = pattern.trim().trim_start_matches("./");
        if p.is_empty() || p.starts_with('/') || p.split('/').any(|s| s == "..") {
            continue;
        }
        let segs: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
        expand(root, &segs, &mut out);
    }
    out.into_iter().collect()
}

/// Every section of every instruction file, as records, and a fault per file
/// that could not be read or whose block is malformed. Malformed blocks are
/// read leniently (the sections outside them are records); the fault says
/// what a writer would refuse.
#[must_use]
pub fn read(repo: &Repository) -> (Vec<Record>, Vec<Fault>, usize) {
    let mut records = Vec::new();
    let mut faults = Vec::new();
    let paths = files(repo);
    for path in &paths {
        let rel = repo.relative(path);
        let text = match crate::vfs::read(path).map(String::from_utf8) {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                faults.push(Fault {
                    rule: "instruction.unreadable",
                    file: rel,
                    line: 0,
                    message: format!("not UTF-8, so its sections are no records: {e}"),
                });
                continue;
            }
            Err(e) => {
                faults.push(Fault {
                    rule: "instruction.unreadable",
                    file: rel,
                    line: 0,
                    message: format!("could not read: {e}"),
                });
                continue;
            }
        };
        let scan = instruction::scan(&text);
        for e in &scan.problems {
            faults.push(Fault {
                rule: "instruction.block-malformed",
                file: rel.clone(),
                line: e.line(),
                message: format!("{e} ({})", e.rule()),
            });
        }
        for s in scan.sections {
            records.push(Record {
                id: instruction::section_id(&rel, &s.slug),
                record_type: instruction::RECORD_TYPE.to_owned(),
                profile: String::new(),
                source: rel.clone(),
                line: s.line,
                revision: s.revision,
            });
        }
    }
    (records, faults, paths.len())
}

/// What `war agents-md --block` did to one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// Repository-relative.
    pub path: String,
    /// `inserted`, `updated`, `unchanged`, or `created` for a file that did
    /// not exist.
    pub change: &'static str,
}

/// Why `war agents-md --block` wrote nothing.
#[derive(Debug)]
pub enum Refused {
    /// One or more files' blocks, each by rule.
    Blocks(Report),
    /// A file could not be read or written.
    Io(String),
}

/// The files `--block` writes when none is named: every root file that
/// exists, or `AGENTS.md` when neither does.
#[must_use]
pub fn default_targets(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let present: Vec<Utf8PathBuf> = ROOT_FILES
        .iter()
        .map(|f| root.join(f))
        .filter(|p| p.is_file())
        .collect();
    if present.is_empty() {
        vec![root.join("AGENTS.md")]
    } else {
        present
    }
}

/// Set the block in each of `targets` (absolute paths under `root`). Every
/// file is read and checked before any is written, so a refusal writes
/// nothing; a file that does not exist is created holding the block alone.
/// A target that is a link to another target (`CLAUDE.md` → `AGENTS.md`) is
/// written once, through the file it names, and reported as `linked`. A link
/// to a file outside the repository (a shared or global CLAUDE.md) is refused,
/// `agents-md.link-outside`: the block goes only into this repository's files.
pub fn write_blocks(root: &Utf8Path, targets: &[Utf8PathBuf]) -> Result<Vec<Written>, Refused> {
    /// The file a write lands in, its bytes before (none: absent), after.
    struct Pending {
        real: Utf8PathBuf,
        before: Option<Vec<u8>>,
        after: String,
    }
    let rel = |p: &Utf8Path| p.strip_prefix(root).unwrap_or(p).as_str().to_owned();
    // (target, its write, change) per target; no write for a link to a file
    // already planned.
    let mut planned: Vec<(String, Option<Pending>, &'static str)> = Vec::new();
    let mut seen: BTreeSet<Utf8PathBuf> = BTreeSet::new();
    let mut report = Report::default();
    let canonical = |p: &Utf8Path| {
        std::fs::canonicalize(p)
            .ok()
            .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
    };
    let real_root = canonical(root).unwrap_or_else(|| root.to_owned());
    for path in targets {
        let real = canonical(path).unwrap_or_else(|| path.clone());
        if real.is_absolute() && !real.starts_with(&real_root) {
            report.push(Diagnostic::error(
                "agents-md.link-outside",
                rel(path),
                format!(
                    "{} is a link to {real}, outside the repository; the block goes only into \
                     the repository's own files. Name another with --file. Nothing was written.",
                    rel(path)
                ),
            ));
            continue;
        }
        if !seen.insert(real.clone()) {
            planned.push((rel(path), None, "linked"));
            continue;
        }
        let existing = match std::fs::read(&real) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(Refused::Io(format!("could not read {path}: {e}"))),
        };
        let text = match existing.as_deref().map(std::str::from_utf8) {
            None => "",
            Some(Ok(t)) => t,
            Some(Err(_)) => {
                report.push(Diagnostic::error(
                    "agents-md.not-utf8",
                    rel(path),
                    format!(
                        "{} is not UTF-8; the block is text and goes only into a text file. \
                         Nothing was written.",
                        rel(path)
                    ),
                ));
                continue;
            }
        };
        match instruction::upsert(text, version()) {
            Ok((out, change)) => {
                let change = match (existing.is_some(), change) {
                    (false, _) => "created",
                    (true, Change::Inserted) => "inserted",
                    (true, Change::Updated) => "updated",
                    (true, Change::Unchanged) => "unchanged",
                };
                planned.push((
                    rel(path),
                    Some(Pending {
                        real,
                        before: existing,
                        after: out,
                    }),
                    change,
                ));
            }
            Err(e) => report.push(refusal(&rel(path), &e)),
        }
    }
    if !report.diagnostics.is_empty() {
        return Err(Refused::Blocks(report));
    }
    let mut out = Vec::new();
    for (path, write, change) in planned {
        if let Some(w) = write
            && change != "unchanged"
        {
            let prestate = w
                .before
                .as_deref()
                .map_or(crate::compile::atomic::Prestate::Absent, |b| {
                    crate::compile::atomic::Prestate::of(b)
                });
            crate::compile::atomic::write_if(&w.real, w.after.as_bytes(), &prestate)
                .map_err(|e| Refused::Io(e.to_string()))?;
        }
        out.push(Written { path, change });
    }
    Ok(out)
}

/// A block refusal as a diagnostic, by its rule, anchored to `file:line`.
#[must_use]
pub fn refusal(file: &str, e: &BlockError) -> Diagnostic {
    Diagnostic::error(
        e.rule(),
        format!("{file}:{}", e.line()),
        format!("{file}: {e}. Nothing was written."),
    )
}

/// `war doctor`'s findings on the block in the root instruction files: one
/// per file that lacks it, holds a stale one or a malformed one, and a pass
/// per file whose block is this `war`'s. A block a newer `war` wrote is not
/// stale here; [`crate::skew`] reports it as version skew.
#[must_use]
pub fn doctor(root: &Utf8Path) -> Vec<Diagnostic> {
    doctor_for(root, version())
}

/// [`doctor`] with the running version given, for tests.
#[must_use]
pub fn doctor_for(root: &Utf8Path, running: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut any = false;
    for name in ROOT_FILES {
        let Ok(text) = std::fs::read_to_string(root.join(name)) else {
            continue;
        };
        any = true;
        let scan = instruction::scan(&text);
        if let Some(e) = scan.problems.first() {
            out.push(Diagnostic::warn(
                "doctor.agents-block-malformed",
                format!("{name}:{}", e.line()),
                format!(
                    "{name}: {e}. `war agents-md --block` refuses this file ({}) until it is \
                     fixed; ordinary work is unaffected",
                    e.rule()
                ),
            ));
            continue;
        }
        let Some(block) = scan.blocks.first() else {
            out.push(Diagnostic::warn(
                "doctor.agents-block-missing",
                name,
                format!(
                    "{name} has no openwarrant block. `war agents-md --block` adds one: a few \
                     lines saying ordinary coding needs no Warrant and `war prime` shows tracked \
                     work, with nothing else in the file changed"
                ),
            ));
            continue;
        };
        let current = instruction::block_text(running);
        if text[block.start..block.end] == current {
            out.push(Diagnostic::pass(
                "doctor.agents-block",
                format!("{name} carries the openwarrant block of war {running}"),
            ));
            continue;
        }
        let stamp = instruction::block_stamp(&text, block);
        if stamp
            .as_deref()
            .is_some_and(|s| crate::install::precedes(running, s))
        {
            // Newer than this war: version skew, said by `skew`.
            continue;
        }
        out.push(Diagnostic::warn(
            "doctor.agents-block-stale",
            format!("{name}:{}", block.begin_line),
            format!(
                "{name}: the openwarrant block {} differs from what war {running} writes. \
                 `war agents-md --block` rewrites the lines between its markers and nothing else",
                stamp.map_or_else(
                    || "has no version stamp and".to_owned(),
                    |s| format!("was written by war {s} and")
                )
            ),
        ));
    }
    if !any {
        out.push(Diagnostic::warn(
            "doctor.agents-block-missing",
            "AGENTS.md",
            "no AGENTS.md or CLAUDE.md at the root. `war agents-md --block` writes an AGENTS.md \
             holding the openwarrant block (ordinary coding needs no Warrant; `war prime` shows \
             tracked work), and `war agents-md` the full guide",
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> Utf8PathBuf {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "war-instructions-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_block_goes_into_every_root_file_and_a_refusal_writes_nothing() {
        let root = scratch("write");
        // Neither exists: AGENTS.md, holding the block alone.
        let t = default_targets(&root);
        assert_eq!(t, vec![root.join("AGENTS.md")]);
        let w = write_blocks(&root, &t).unwrap();
        assert_eq!(w[0].change, "created");
        let body = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert_eq!(body, format!("{}\n", instruction::block_text(version())));
        // Both exist: both, and a second run changes nothing.
        std::fs::write(root.join("CLAUDE.md"), "# Mine\n\n## Testing\nRun it.").unwrap();
        let t = default_targets(&root);
        assert_eq!(t.len(), 2);
        let w = write_blocks(&root, &t).unwrap();
        let changes: Vec<&str> = w.iter().map(|x| x.change).collect();
        assert_eq!(changes, ["unchanged", "inserted"]);
        let claude = std::fs::read_to_string(root.join("CLAUDE.md")).unwrap();
        assert!(claude.starts_with("# Mine\n\n## Testing\nRun it.\n<!-- openwarrant:begin -->"));
        let w = write_blocks(&root, &t).unwrap();
        assert!(w.iter().all(|x| x.change == "unchanged"));
        assert_eq!(
            std::fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
            claude
        );
        // A malformed CLAUDE.md refuses the run, and AGENTS.md is untouched too.
        std::fs::write(
            root.join("CLAUDE.md"),
            format!("{claude}\n{}\n", instruction::BLOCK_BEGIN),
        )
        .unwrap();
        std::fs::write(root.join("AGENTS.md"), "older\n").unwrap();
        match write_blocks(&root, &default_targets(&root)) {
            Err(Refused::Blocks(r)) => {
                assert_eq!(r.diagnostics.len(), 1);
                assert_eq!(r.diagnostics[0].rule, "agents-md.block-unterminated");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            std::fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            "older\n"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_written_once_and_a_link_outside_is_refused() {
        let root = scratch("links");
        let elsewhere = scratch("elsewhere");
        std::fs::write(root.join("AGENTS.md"), "# Ours\n").unwrap();
        std::os::unix::fs::symlink("AGENTS.md", root.join("CLAUDE.md")).unwrap();
        let w = write_blocks(&root, &default_targets(&root)).unwrap();
        let changes: Vec<&str> = w.iter().map(|x| x.change).collect();
        assert_eq!(changes, ["inserted", "linked"]);
        let text = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert_eq!(text.matches(instruction::BLOCK_BEGIN).count(), 1);
        // A CLAUDE.md that is a link to a file outside: refused, untouched.
        std::fs::remove_file(root.join("CLAUDE.md")).unwrap();
        std::fs::write(elsewhere.join("CLAUDE.md"), "global\n").unwrap();
        std::os::unix::fs::symlink(elsewhere.join("CLAUDE.md"), root.join("CLAUDE.md")).unwrap();
        match write_blocks(&root, &default_targets(&root)) {
            Err(Refused::Blocks(r)) => assert_eq!(r.diagnostics[0].rule, "agents-md.link-outside"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            std::fs::read_to_string(elsewhere.join("CLAUDE.md")).unwrap(),
            "global\n"
        );
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(elsewhere).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn nested_globs_find_files_and_skip_build_trees_hidden_dirs_and_links() {
        let root = scratch("nested");
        for dir in [
            "pkg/a",
            "pkg/b/deep",
            "target/x",
            ".hidden",
            "node_modules/y",
            "real",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for f in [
            "AGENTS.md",
            "pkg/a/CLAUDE.md",
            "pkg/b/deep/CLAUDE.md",
            "target/x/CLAUDE.md",
            ".hidden/CLAUDE.md",
            "node_modules/y/CLAUDE.md",
            "real/CLAUDE.md",
        ] {
            std::fs::write(root.join(f), "## A\n").unwrap();
        }
        std::os::unix::fs::symlink(root.join("real"), root.join("pkg/link")).unwrap();
        let rel = |v: Vec<Utf8PathBuf>| -> Vec<String> {
            v.iter()
                .map(|p| p.strip_prefix(&root).unwrap().to_string())
                .collect()
        };
        assert_eq!(rel(files_in(&root, &[])), ["AGENTS.md"]);
        assert_eq!(
            rel(files_in(&root, &["**/CLAUDE.md".to_owned()])),
            [
                "AGENTS.md",
                "pkg/a/CLAUDE.md",
                "pkg/b/deep/CLAUDE.md",
                "real/CLAUDE.md"
            ]
        );
        assert_eq!(
            rel(files_in(&root, &["pkg/*/CLAUDE.md".to_owned()])),
            ["AGENTS.md", "pkg/a/CLAUDE.md"]
        );
        // Out of the root, or absolute: nothing.
        assert_eq!(
            rel(files_in(
                &root,
                &["../*/CLAUDE.md".to_owned(), "/etc/*".to_owned()]
            )),
            ["AGENTS.md"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn doctor_reports_missing_stale_and_current_blocks() {
        let root = scratch("doctor");
        let rules = |r: &Utf8Path, v: &str| -> Vec<String> {
            doctor_for(r, v).into_iter().map(|d| d.rule).collect()
        };
        assert_eq!(rules(&root, "1.0.0"), ["doctor.agents-block-missing"]);
        std::fs::write(root.join("CLAUDE.md"), "# x\n").unwrap();
        assert_eq!(rules(&root, "1.0.0"), ["doctor.agents-block-missing"]);
        let (with, _) = instruction::upsert("# x\n", "1.0.0").unwrap();
        std::fs::write(root.join("CLAUDE.md"), &with).unwrap();
        assert_eq!(rules(&root, "1.0.0"), ["doctor.agents-block"]);
        // Written by an older war: stale. By a newer one: skew's to say.
        assert_eq!(rules(&root, "1.1.0"), ["doctor.agents-block-stale"]);
        assert!(rules(&root, "0.9.0").is_empty());
        // Edited inside the markers: stale, at the version that wrote it.
        std::fs::write(
            root.join("CLAUDE.md"),
            with.replace("Ordinary coding", "Most coding"),
        )
        .unwrap();
        assert_eq!(rules(&root, "1.0.0"), ["doctor.agents-block-stale"]);
        std::fs::write(root.join("CLAUDE.md"), format!("{with}{with}")).unwrap();
        assert_eq!(rules(&root, "1.0.0"), ["doctor.agents-block-malformed"]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
